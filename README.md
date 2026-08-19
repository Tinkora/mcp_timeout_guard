# MCP Timeout Guard

`mcp-timeout-guard` is a local stdio JSON-RPC proxy for MCP clients that do
not expose a reliable per-request timeout. It forwards newline-delimited MCP
frames, applies bounded deadlines, and returns a deterministic timeout error
without logging payloads or making network requests.

[简体中文](README.zh-CN.md)

## Why

Slow JVM servers and first-run package installs can exceed a client startup
budget, while a hung server can block an Agent session indefinitely. Public
reports include [Alexi #1367](https://github.com/ausardcompany/alexi/issues/1367),
[Alexi #1386](https://github.com/ausardcompany/alexi/issues/1386),
[RustyClawd #1570](https://github.com/rysweet/RustyClawd/issues/1570), and
[Curia #1666](https://github.com/josephfung/curia/issues/1666).

## Quick start

Build the wrapper, then place `--` before the existing server command:

```bash
cargo build --release --locked
mcp-timeout-guard --request-timeout-ms 30000 -- node ./server.js
```

The wrapper reads MCP JSON-RPC frames from stdin and forwards child stdout to
stdout. Child stderr is inherited. It never invokes a shell, so shell syntax is
not interpreted by the wrapper.

```text
mcp-timeout-guard [OPTIONS] -- COMMAND [ARGUMENT ...]

--startup-timeout-ms <N>  First request deadline (default: 30000)
--request-timeout-ms <N> Later request deadline (default: 30000)
--max-frame-bytes <N>    Maximum input/output frame size (default: 8388608)
```

Requests with an `id` are tracked. Notifications are forwarded immediately and
are never timed out. On timeout, the wrapper writes one JSON-RPC error with the
original id, terminates the child, and exits with status `124`:

```json
{"jsonrpc":"2.0","id":7,"error":{"code":-32001,"message":"MCP request timed out after 30000 ms"}}
```

## Boundaries

- Local process wrapper only; no network transport or hosted service.
- No retries, replay, deduplication, circuit breaker, rate limiting, or payload
  transformation.
- No shell execution, authentication, sandboxing, or descendant-process
  supervision guarantee.
- No MCP configuration discovery; use [MCP Doctor](https://github.com/Tinkora/mcp_doctor).
- No trace analysis; use [Tool Call Trace](https://github.com/Tinkora/tool_call_trace).
- Timeouts prevent ambiguous waits; they do not prove that a child or every
  descendant process has stopped on every operating system.

## Development

Requirements: Rust 1.95.0 or newer.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

See the [product specification](docs/PRODUCT_SPEC.md),
[contributing guide](CONTRIBUTING.md), [security policy](SECURITY.md), and
[changelog](CHANGELOG.md).

## Support

[Support Tinkora on Ko-fi](https://ko-fi.com/tinkora)

## License

MIT. See [LICENSE](LICENSE).
