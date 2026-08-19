# MCP Timeout Guard Product Specification

[简体中文](PRODUCT_SPEC.zh-CN.md)

## Objective

MCP clients frequently launch local stdio servers with a fixed or missing
timeout. Slow JVM servers and first-run package installs can be killed before
they initialize, while a hung server can stall an agent session indefinitely.
`mcp-timeout-guard` is a local process wrapper that gives any MCP client a
bounded response deadline without requiring client-specific configuration.

The MVP succeeds when a user can place the binary before an existing stdio MCP
server command and receive a deterministic JSON-RPC timeout instead of an
unbounded hang, while normal MCP traffic remains byte-compatible.

## Evidence and alternatives

The problem is supported by current public issue reports:

- [Alexi #1367](https://github.com/ausardcompany/alexi/issues/1367) reports a
  3-second default that rejects JVM-based servers and first-run `npx -y`
  installs, and requests a configurable startup timeout.
- [Alexi #1386](https://github.com/ausardcompany/alexi/issues/1386) requests
  per-server MCP timeout overrides.
- [RustyClawd #1570](https://github.com/rysweet/RustyClawd/issues/1570) reports
  a missing `MCP_TOOL_TIMEOUT` capability.
- [Curia #1666](https://github.com/josephfung/curia/issues/1666) reports an MCP
  call path that does not pass a timeout or abort signal.

MCP Inspector is useful for interactive protocol debugging but is not a
drop-in timeout wrapper. Full MCP gateways and policy proxies add routing,
authentication, retries, or hosted operations outside this narrow need. The
proxy deliberately does not compete with them.

## Interface

```text
mcp-timeout-guard [OPTIONS] -- COMMAND [ARGUMENT ...]

Options:
  --startup-timeout-ms <N>   Deadline for the first request that expects a response
                             (default: 30,000)
  --request-timeout-ms <N>  Deadline for each later request (default: 30,000)
  --max-frame-bytes <N>     Maximum newline-delimited JSON frame (default: 8 MiB)
```

The proxy reads MCP JSON-RPC frames from stdin and writes frames to the child
stdin. Child stdout is forwarded to the client stdout. Child stderr is
inherited by the wrapper. `--` is required before the child command so shell
metacharacters are never interpreted by the proxy.

## Protocol behavior

- Requests with a non-null JSON-RPC `id` are tracked until a response with the
  same id is observed.
- Notifications are forwarded immediately and are not timed out.
- The first tracked request uses `--startup-timeout-ms`; all later tracked
  requests use `--request-timeout-ms`.
- When a deadline expires, the proxy writes:

  ```json
  {"jsonrpc":"2.0","id":<original id>,"error":{"code":-32001,"message":"MCP request timed out"}}
  ```

  It then terminates the child and exits with status `124`.
- If the child exits before a pending response, the proxy reports a generic
  JSON-RPC error without echoing request arguments.
- Invalid client frames and oversized frames are rejected locally and never
  sent to the child.

## Non-goals

- No retry, replay, deduplication, circuit breaker, rate limiting, or payload
  transformation.
- No shell execution, network transport, authentication, sandboxing, or
  descendant process supervision.
- No MCP configuration discovery; use `mcp_doctor` for static configuration
  diagnostics.
- No trace analysis; use `tool_call_trace` for post-hoc inspection.

## Tech stack and structure

- Rust 1.85 or newer, edition 2024.
- `serde_json` for bounded JSON-RPC envelope inspection only.
- `clap` for the command-line contract.
- `src/proxy.rs` for process and frame forwarding.
- `src/protocol.rs` for id extraction and error envelopes.
- `tests/` for process-level and unit regressions.

## Testing strategy

- Unit tests cover id extraction, notification handling, timeout validation,
  and error envelope serialization.
- Process tests use a deterministic fixture mode in the test binary to cover
  successful forwarding, delayed responses, child exit, and frame limits.
- CI runs formatting, locked tests, Clippy, cargo-deny, cargo-audit, CodeQL,
  and workflow security checks.

## Boundaries

- Always validate frame size and JSON-RPC id shape before tracking a request.
- Always keep payloads out of logs and error messages.
- Ask first before adding a new transport, retry policy, or payload logging.
- Never treat the wrapper as a security sandbox or claim it kills arbitrary
  descendant processes.

## Success criteria

1. A normal initialize/tools/list/tools/call exchange is forwarded unchanged.
2. A delayed response produces one timeout error with the original id and a
   `124` process exit within the configured deadline.
3. Notifications never create pending entries or timeout errors.
4. Malformed and oversized client frames fail before child dispatch.
5. No test or command output contains a supplied secret or full payload.
6. The repository has bilingual README entry points, release documentation,
   reproducible CI, and a verified first release before public publication.
