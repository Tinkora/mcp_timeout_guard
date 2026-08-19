use std::process::ExitCode;

use clap::Parser;
use mcp_timeout_guard::{ProxyOptions, run_proxy};

#[derive(Debug, Parser)]
#[command(
    name = "mcp-timeout-guard",
    version,
    about = "Bounded timeout proxy for local stdio MCP JSON-RPC"
)]
struct Cli {
    /// Deadline for the first request that expects a response.
    #[arg(long, default_value_t = 30_000, value_name = "MILLISECONDS")]
    startup_timeout_ms: u64,

    /// Deadline for each request after the first tracked request.
    #[arg(long, default_value_t = 30_000, value_name = "MILLISECONDS")]
    request_timeout_ms: u64,

    /// Maximum size of one newline-delimited JSON frame.
    #[arg(long, default_value_t = 8 * 1024 * 1024, value_name = "BYTES")]
    max_frame_bytes: usize,

    /// Child command and arguments. Put `--` before the command.
    #[arg(last = true, required = true, value_name = "COMMAND", num_args = 1..)]
    command: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some((command, args)) = cli.command.split_first() else {
        eprintln!("usage error: a child command is required after --");
        return ExitCode::from(2);
    };

    let options = ProxyOptions {
        command: command.clone(),
        args: args.to_vec(),
        startup_timeout_ms: cli.startup_timeout_ms,
        request_timeout_ms: cli.request_timeout_ms,
        max_frame_bytes: cli.max_frame_bytes,
    };
    match run_proxy(options) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("mcp-timeout-guard: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
