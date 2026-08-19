use crate::protocol::{
    RequestId, child_exit_error, is_request_with_id, parse_client_frame, request_id, response_id,
    timeout_error, write_frame,
};
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const TIMEOUT_EXIT_CODE: u8 = 124;

#[derive(Clone, Debug)]
pub struct ProxyOptions {
    pub command: String,
    pub args: Vec<String>,
    pub startup_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub max_frame_bytes: usize,
}

impl ProxyOptions {
    fn validate(&self) -> Result<(), ProxyError> {
        if self.command.trim().is_empty() {
            return Err(ProxyError::Usage(
                "child command cannot be empty".to_string(),
            ));
        }
        if self.startup_timeout_ms == 0 || self.request_timeout_ms == 0 {
            return Err(ProxyError::Usage(
                "timeouts must be greater than zero".to_string(),
            ));
        }
        if self.max_frame_bytes == 0 {
            return Err(ProxyError::Usage(
                "max-frame-bytes must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("usage error: {0}")]
    Usage(String),
    #[error("cannot start child process: {0}")]
    Spawn(#[source] io::Error),
    #[error("child process I/O failed: {0}")]
    Io(#[source] io::Error),
}

impl ProxyError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Spawn(_) | Self::Io(_) => 1,
        }
    }
}

#[derive(Clone, Debug)]
struct PendingRequest {
    id: RequestId,
    deadline: Instant,
    timeout_ms: u64,
}

#[derive(Debug)]
struct SharedState {
    pending: Mutex<VecDeque<PendingRequest>>,
    output: Mutex<io::BufWriter<io::Stdout>>,
    shutdown: AtomicBool,
    timed_out: AtomicBool,
    output_done: AtomicBool,
}

impl SharedState {
    fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            output: Mutex::new(io::BufWriter::new(io::stdout())),
            shutdown: AtomicBool::new(false),
            timed_out: AtomicBool::new(false),
            output_done: AtomicBool::new(false),
        }
    }

    fn write_value(&self, value: &serde_json::Value) -> Result<(), ProxyError> {
        let bytes = write_frame(value);
        let mut output = self.output.lock().expect("output mutex poisoned");
        output.write_all(&bytes).map_err(ProxyError::Io)?;
        output.flush().map_err(ProxyError::Io)
    }

    fn has_pending(&self) -> bool {
        !self
            .pending
            .lock()
            .expect("pending mutex poisoned")
            .is_empty()
    }
}

enum Event {
    Input(Result<(), ProxyError>),
    Output(Result<(), ProxyError>),
}

pub fn run_proxy(options: ProxyOptions) -> Result<u8, ProxyError> {
    options.validate()?;
    let mut child = Command::new(&options.command)
        .args(&options.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(ProxyError::Spawn)?;
    let child_stdin = child.stdin.take().ok_or_else(|| {
        ProxyError::Io(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "child stdin unavailable",
        ))
    })?;
    let child_stdout = child.stdout.take().ok_or_else(|| {
        ProxyError::Io(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "child stdout unavailable",
        ))
    })?;

    let state = Arc::new(SharedState::new());
    let child = Arc::new(Mutex::new(child));
    let (event_tx, event_rx) = mpsc::channel();

    spawn_input_thread(
        child_stdin,
        Arc::clone(&state),
        options.clone(),
        event_tx.clone(),
    );
    spawn_output_thread(
        child_stdout,
        Arc::clone(&state),
        options.max_frame_bytes,
        event_tx,
    );
    spawn_watchdog(Arc::clone(&state), Arc::clone(&child));

    supervise(child, state, event_rx)
}

fn spawn_input_thread(
    child_stdin: ChildStdin,
    state: Arc<SharedState>,
    options: ProxyOptions,
    event_tx: mpsc::Sender<Event>,
) {
    thread::spawn(move || {
        let result = forward_client_frames(io::stdin().lock(), child_stdin, state, &options);
        let _ = event_tx.send(Event::Input(result));
    });
}

fn spawn_output_thread(
    child_stdout: impl Read + Send + 'static,
    state: Arc<SharedState>,
    max_frame_bytes: usize,
    event_tx: mpsc::Sender<Event>,
) {
    thread::spawn(move || {
        let result = forward_server_frames(child_stdout, state, max_frame_bytes);
        let _ = event_tx.send(Event::Output(result));
    });
}

fn spawn_watchdog(state: Arc<SharedState>, child: Arc<Mutex<Child>>) {
    thread::spawn(move || {
        loop {
            if state.shutdown.load(Ordering::SeqCst) {
                return;
            }
            thread::sleep(Duration::from_millis(5));
            let expired = {
                let mut pending = state.pending.lock().expect("pending mutex poisoned");
                let now = Instant::now();
                let mut expired = Vec::new();
                pending.retain(|request| {
                    if request.deadline <= now {
                        expired.push(request.clone());
                        false
                    } else {
                        true
                    }
                });
                expired
            };
            if expired.is_empty() {
                continue;
            }
            state.timed_out.store(true, Ordering::SeqCst);
            for request in expired {
                let _ = state.write_value(&timeout_error(&request.id, request.timeout_ms));
            }
            state.shutdown.store(true, Ordering::SeqCst);
            let _ = child.lock().expect("child mutex poisoned").kill();
            return;
        }
    });
}

fn supervise(
    child: Arc<Mutex<Child>>,
    state: Arc<SharedState>,
    event_rx: mpsc::Receiver<Event>,
) -> Result<u8, ProxyError> {
    loop {
        if let Some(status) = child
            .lock()
            .expect("child mutex poisoned")
            .try_wait()
            .map_err(ProxyError::Io)?
        {
            if !state.output_done.load(Ordering::SeqCst) && state.has_pending() {
                let _ = event_rx.recv_timeout(Duration::from_millis(100));
            }
            state.shutdown.store(true, Ordering::SeqCst);
            return Ok(exit_code(status, &state));
        }
        match event_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Event::Input(Ok(()))) => {
                state.shutdown.store(true, Ordering::SeqCst);
                let _ = child.lock().expect("child mutex poisoned").kill();
                return Ok(exit_code_after_shutdown(&state));
            }
            Ok(Event::Output(Ok(()))) => {
                state.shutdown.store(true, Ordering::SeqCst);
                let status = child
                    .lock()
                    .expect("child mutex poisoned")
                    .wait()
                    .map_err(ProxyError::Io)?;
                return Ok(exit_code(status, &state));
            }
            Ok(Event::Input(Err(error))) | Ok(Event::Output(Err(error))) => {
                state.shutdown.store(true, Ordering::SeqCst);
                let _ = child.lock().expect("child mutex poisoned").kill();
                return Err(error);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                state.shutdown.store(true, Ordering::SeqCst);
                let _ = child.lock().expect("child mutex poisoned").kill();
                return Ok(exit_code_after_shutdown(&state));
            }
        }
    }
}

fn exit_code(status: ExitStatus, state: &SharedState) -> u8 {
    if state.timed_out.load(Ordering::SeqCst) {
        TIMEOUT_EXIT_CODE
    } else {
        status
            .code()
            .map(|code| code.clamp(0, 255) as u8)
            .unwrap_or(1)
    }
}

fn exit_code_after_shutdown(state: &SharedState) -> u8 {
    if state.timed_out.load(Ordering::SeqCst) {
        TIMEOUT_EXIT_CODE
    } else {
        0
    }
}

fn forward_client_frames(
    input: impl Read,
    mut child_stdin: ChildStdin,
    state: Arc<SharedState>,
    options: &ProxyOptions,
) -> Result<(), ProxyError> {
    let mut reader = BufReader::new(input);
    let mut frame = Vec::new();
    let mut first_tracked_request = true;
    loop {
        frame.clear();
        if !read_bounded_frame(&mut reader, &mut frame, options.max_frame_bytes)? {
            return Ok(());
        }
        let value = parse_client_frame(&frame)
            .map_err(|message| ProxyError::Usage(format!("invalid client frame: {message}")))?;
        if is_request_with_id(&value) {
            let id = request_id(&value).expect("request id was checked");
            let timeout_ms = if first_tracked_request {
                first_tracked_request = false;
                options.startup_timeout_ms
            } else {
                options.request_timeout_ms
            };
            state
                .pending
                .lock()
                .expect("pending mutex poisoned")
                .push_back(PendingRequest {
                    id,
                    deadline: Instant::now() + Duration::from_millis(timeout_ms),
                    timeout_ms,
                });
        }
        child_stdin.write_all(&frame).map_err(ProxyError::Io)?;
        child_stdin.write_all(b"\n").map_err(ProxyError::Io)?;
        child_stdin.flush().map_err(ProxyError::Io)?;
    }
}

fn forward_server_frames(
    child_stdout: impl Read,
    state: Arc<SharedState>,
    max_frame_bytes: usize,
) -> Result<(), ProxyError> {
    let mut reader = BufReader::new(child_stdout);
    let mut frame = Vec::new();
    loop {
        frame.clear();
        if !read_bounded_frame(&mut reader, &mut frame, max_frame_bytes)? {
            let pending = state
                .pending
                .lock()
                .expect("pending mutex poisoned")
                .drain(..)
                .collect::<Vec<_>>();
            for request in pending {
                if !state.shutdown.load(Ordering::SeqCst) {
                    state.write_value(&child_exit_error(&request.id))?;
                }
            }
            state.output_done.store(true, Ordering::SeqCst);
            state.shutdown.store(true, Ordering::SeqCst);
            return Ok(());
        }
        if state.shutdown.load(Ordering::SeqCst) {
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&frame) {
            if let Some(id) = response_id(&value) {
                remove_pending(&state, &id);
            }
        }
        let mut output = state.output.lock().expect("output mutex poisoned");
        output.write_all(&frame).map_err(ProxyError::Io)?;
        output.write_all(b"\n").map_err(ProxyError::Io)?;
        output.flush().map_err(ProxyError::Io)?;
    }
}

fn remove_pending(state: &SharedState, id: &RequestId) {
    let mut pending = state.pending.lock().expect("pending mutex poisoned");
    if let Some(position) = pending.iter().position(|request| &request.id == id) {
        pending.remove(position);
    }
}

fn read_bounded_frame<R: BufRead>(
    reader: &mut R,
    frame: &mut Vec<u8>,
    max_frame_bytes: usize,
) -> Result<bool, ProxyError> {
    loop {
        let available = reader.fill_buf().map_err(ProxyError::Io)?;
        if available.is_empty() {
            return Ok(!frame.is_empty());
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |index| index + 1);
        if frame.len().saturating_add(take) > max_frame_bytes {
            return Err(ProxyError::Usage(format!(
                "frame exceeds {max_frame_bytes} bytes"
            )));
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            if frame.last() == Some(&b'\n') {
                frame.pop();
            }
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(true);
        }
    }
}
