# 安全策略

[English](SECURITY.md)

MCP Timeout Guard 是本地进程包装器，不是沙箱。它不检查 payload、不认证远程服务，
也不保证所有操作系统上都能终止后代进程。不要把超时当作安全边界。

请使用 [GitHub 私密漏洞报告](https://github.com/Tinkora/mcp_timeout_guard/security/advisories/new)。
不要在公开 Issue 中提交 secret、漏洞利用细节或私有配置值；复现材料必须删除凭据。

设计承诺：不记录请求/响应/参数/环境/命令行值；不插入 shell；在有界缓冲前限制
frame 大小；错误只保留 JSON-RPC request id。
