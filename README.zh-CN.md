# MCP Timeout Guard

`mcp-timeout-guard` 是一个本地 stdio JSON-RPC 包装器，为没有可靠的逐请求超时
配置的 MCP 客户端提供有界期限。它转发换行分隔的 MCP frame，超时后返回确定性的
错误，不记录 payload，也不发起网络请求。

[English](README.md)

## 为什么需要

慢启动的 JVM server 和首次执行的包安装可能超过客户端启动预算，挂起的 server
则会让 Agent 会话无限阻塞。公开报告包括 [Alexi #1367](https://github.com/ausardcompany/alexi/issues/1367)、
[Alexi #1386](https://github.com/ausardcompany/alexi/issues/1386)、
[RustyClawd #1570](https://github.com/rysweet/RustyClawd/issues/1570) 和
[Curia #1666](https://github.com/josephfung/curia/issues/1666)。

## 快速开始

先构建包装器，再在现有 server 命令前使用 `--`：

```bash
cargo build --release --locked
mcp-timeout-guard --request-timeout-ms 30000 -- node ./server.js
```

包装器从 stdin 读取 MCP JSON-RPC frame，并将子进程 stdout 转发到 stdout；stderr
继承到当前终端。包装器不会调用 shell，因此不会解释 shell 语法。

带 `id` 的请求会被追踪；notification 立即转发且不参与超时。超时时返回一个带原
id 的 JSON-RPC 错误，终止子进程，并以状态码 `124` 退出。

## 边界

- 仅本地进程包装，不提供网络 transport 或托管服务。
- 不提供重试、重放、去重、熔断、限流或 payload 改写。
- 不提供 shell 执行、认证、沙箱或后代进程监管保证。
- 不发现 MCP 配置，静态诊断使用 [MCP Doctor](https://github.com/Tinkora/mcp_doctor)。
- 不分析 trace，事后分析使用 [Tool Call Trace](https://github.com/Tinkora/tool_call_trace)。
- 超时用于避免不明确的等待，并不保证每个操作系统上的子进程及其所有后代都已停止。

## 开发

要求：Rust 1.95.0 或更高版本。

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

详见[产品规格](docs/PRODUCT_SPEC.zh-CN.md)、[贡献指南](CONTRIBUTING.zh-CN.md)、
[安全策略](SECURITY.zh-CN.md)和[变更日志](CHANGELOG.md)。

## 支持

[在 Ko-fi 支持 Tinkora](https://ko-fi.com/tinkora)

## 许可证

MIT，详见 [LICENSE](LICENSE)。
