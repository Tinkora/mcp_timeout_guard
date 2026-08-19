# Repository Guide for AI Agents

## Product boundary

`mcp_timeout_guard` is a local stdio JSON-RPC proxy for MCP clients that do
not expose a reliable per-request timeout. It forwards newline-delimited MCP
frames, enforces bounded response deadlines, and emits protocol-safe timeout
errors without recording payloads or making network requests.

It is not an MCP server, a sandbox, a security scanner, a retry framework, a
gateway, or an observability service. It cannot guarantee that child processes
or their descendants terminate on every operating system.

## Commit language

Write public commit subjects and bodies in English using Conventional Commits.

## Required checks

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

## Safety invariants

- The proxy never logs request or response payloads, arguments, environment
  values, or command-line secrets.
- The child command is explicit; no shell is inserted by the proxy.
- Input and output frames have a bounded size before buffering.
- A request timeout returns a JSON-RPC error with the original request id and
  then terminates the child to avoid accepting a late ambiguous response.
- Notifications without an id are forwarded but never held in the pending
  timeout map.
- The proxy does not contact the network or modify MCP configuration files.

## Rust style

- Use UTF-8 source and English comments only.
- Keep protocol parsing and process management in small, testable modules.
- Add outcome-focused tests for success, malformed frames, timeout, process
  exit, frame limits, and notifications.
