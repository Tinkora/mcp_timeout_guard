# MCP Timeout Guard 产品规格

[English](PRODUCT_SPEC.md)

## 目标

许多 MCP 客户端对本地 stdio server 使用固定或缺失的超时。JVM server、首次
执行的 `npx -y` 安装可能在初始化前被终止，真正挂起的 server 则会让 Agent 会话
无限等待。`mcp-timeout-guard` 是一个本地进程包装器，让任意 MCP 客户端获得有界
的响应期限，而不需要修改客户端配置。

MVP 的成功标准是：用户把该二进制放在现有 stdio MCP 命令前面后，正常流量保持
兼容；server 挂起时得到确定性的 JSON-RPC 超时，而不是无限阻塞。

## 证据与替代方案

公开 Issue 已反复报告这个问题：

- [Alexi #1367](https://github.com/ausardcompany/alexi/issues/1367)：3 秒默认值
  无法覆盖 JVM server 和首次 `npx -y` 安装，需要可配置启动超时。
- [Alexi #1386](https://github.com/ausardcompany/alexi/issues/1386)：请求每个
  server 的 MCP 超时覆盖。
- [RustyClawd #1570](https://github.com/rysweet/RustyClawd/issues/1570)：缺少
  `MCP_TOOL_TIMEOUT` 能力。
- [Curia #1666](https://github.com/josephfung/curia/issues/1666)：调用路径没有传递
  timeout 或 abort signal。

MCP Inspector 适合交互式协议调试，但不是可直接放入任意客户端配置的超时包装器。
完整 MCP gateway/proxy 通常还包含路由、认证、重试或托管能力，超出本工具的窄范围。

## 命令接口

```text
mcp-timeout-guard [OPTIONS] -- COMMAND [ARGUMENT ...]

选项：
  --startup-timeout-ms <N>   首个需要响应的请求期限（默认 30,000）
  --request-timeout-ms <N>  后续每个请求期限（默认 30,000）
  --max-frame-bytes <N>     单个换行 JSON frame 最大大小（默认 8 MiB）
```

包装器从 stdin 读取 MCP JSON-RPC frame，写入子进程 stdin；子进程 stdout 原样转发
到客户端 stdout，stderr 继承到当前终端。`--` 是命令边界，包装器不会解释 shell
元字符。

## 协议行为

- 带非 null JSON-RPC `id` 的请求会被追踪，直到收到同一 id 的响应。
- 没有 id 的 notification 立即转发，不参与超时。
- 首个被追踪的请求使用 `--startup-timeout-ms`，其余使用
  `--request-timeout-ms`。
- 超时后返回带原 id 的通用 JSON-RPC 错误，终止子进程，并以状态码 `124` 退出。
- 子进程提前退出时，不回显请求参数，只返回通用错误。
- 非法或超大的客户端 frame 在本地拒绝，不发送给子进程。

## 非目标

- 不提供重试、重放、去重、熔断、限流或 payload 改写。
- 不提供 shell 执行、网络 transport、认证、沙箱或后代进程监管。
- 不发现 MCP 配置；静态配置诊断使用 `mcp_doctor`。
- 不分析 trace；事后分析使用 `tool_call_trace`。

## 验证标准

1. 正常 initialize/tools/list/tools/call 流量保持转发。
2. 延迟响应产生一次带原 id 的超时错误，并在期限后以 `124` 退出。
3. Notification 不会产生 pending 项或超时错误。
4. 非法和超大 frame 在子进程收到前失败。
5. 测试和诊断输出不包含用户提供的 secret 或完整 payload。
6. 公开发布前完成双语 README、完整 CI、供应链检查和可验证 Release。
