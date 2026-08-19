# 贡献指南

[English](CONTRIBUTING.md)

请保持改动聚焦并遵守产品边界。修改协议或进程行为前先增加结果导向测试。没有
产品决策时，不要加入 payload 日志、网络访问、shell 插值、重试或后代进程保证。

提交前运行：

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

提交信息使用英文 Conventional Commits，代码注释使用英文；用户可见行为需要更新
变更日志。
