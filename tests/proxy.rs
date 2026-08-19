#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn guard() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mcp-timeout-guard"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn forwards_requests_and_notifications_without_mutating_frames() {
    let mut process = guard()
        .args([
            "--startup-timeout-ms",
            "1000",
            "--",
            "sh",
            "-c",
            "while IFS= read -r line; do printf '%s\\n' \"$line\"; done",
        ])
        .spawn()
        .expect("spawn guard");
    let mut input = process.stdin.take().expect("guard stdin");
    let mut output = BufReader::new(process.stdout.take().expect("guard stdout"));
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
        .expect("write request");
    input.flush().expect("flush request");

    let mut first = String::new();
    output.read_line(&mut first).expect("read notification");
    let mut second = String::new();
    output.read_line(&mut second).expect("read request");
    assert_eq!(
        first.trim(),
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}"
    );
    assert_eq!(
        second.trim(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}"
    );

    drop(input);
    let status = process.wait().expect("wait guard");
    assert!(status.success());
}

#[test]
fn emits_timeout_error_and_exits_124_for_delayed_response() {
    let mut process = guard()
        .args([
            "--startup-timeout-ms",
            "80",
            "--",
            "sh",
            "-c",
            "IFS= read -r line; sleep 1; printf '%s\\n' \"$line\"",
        ])
        .spawn()
        .expect("spawn guard");
    let mut input = process.stdin.take().expect("guard stdin");
    let mut output = BufReader::new(process.stdout.take().expect("guard stdout"));
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":42,\"method\":\"tools/call\",\"params\":{\"secret\":\"do-not-echo\"}}\n")
        .expect("write request");
    input.flush().expect("flush request");

    let started = Instant::now();
    let mut line = String::new();
    output.read_line(&mut line).expect("read timeout");
    assert!(started.elapsed() < Duration::from_millis(800));
    assert!(line.contains("\"id\":42"));
    assert!(line.contains("-32001"));
    assert!(!line.contains("do-not-echo"));

    drop(input);
    let status = process.wait().expect("wait guard");
    assert_eq!(status.code(), Some(124));
}

#[test]
fn rejects_oversized_frames_before_child_dispatch() {
    let mut process = guard()
        .args([
            "--max-frame-bytes",
            "32",
            "--",
            "sh",
            "-c",
            "while IFS= read -r line; do printf '%s\\n' \"$line\"; done",
        ])
        .spawn()
        .expect("spawn guard");
    let mut input = process.stdin.take().expect("guard stdin");
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"this-is-too-large\"}\n")
        .expect("write oversized request");
    drop(input);
    let output = process.wait_with_output().expect("wait guard");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("frame exceeds 32 bytes"));
}

#[test]
fn reports_child_exit_for_a_pending_request_without_echoing_input() {
    let mut process = guard()
        .args(["--startup-timeout-ms", "1000", "--", "sh", "-c", "exit 7"])
        .spawn()
        .expect("spawn guard");
    let mut input = process.stdin.take().expect("guard stdin");
    let mut output = BufReader::new(process.stdout.take().expect("guard stdout"));
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"secret\":\"do-not-echo\"}}\n")
        .expect("write request");
    input.flush().expect("flush request");
    let mut line = String::new();
    output.read_line(&mut line).expect("read child exit error");
    assert!(line.contains("\"id\":9"));
    assert!(line.contains("-32002"));
    assert!(!line.contains("do-not-echo"));
    drop(input);
    let status = process.wait().expect("wait guard");
    assert_eq!(status.code(), Some(7));
}
