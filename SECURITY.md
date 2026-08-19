# Security Policy

[简体中文](SECURITY.zh-CN.md)

## Scope

MCP Timeout Guard is a local process wrapper. It does not sandbox a child,
inspect its payloads, authenticate remote services, or guarantee termination of
descendants. Do not treat a timeout as a security boundary.

## Reporting

Please use [GitHub private vulnerability reporting](https://github.com/Tinkora/mcp_timeout_guard/security/advisories/new)
when available. Do not open a public issue containing secrets, exploit details,
or private configuration values. Include a minimal reproduction with all
credentials removed.

## Design commitments

- Never log request, response, argument, environment, or command-line values.
- Never insert a shell between the client and the configured command.
- Enforce frame limits before buffering unbounded input.
- Keep timeout errors generic and preserve only the JSON-RPC request id.
