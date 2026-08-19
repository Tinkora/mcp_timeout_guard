# Contributing

[简体中文](CONTRIBUTING.zh-CN.md)

## Before opening a pull request

Keep changes focused and preserve the product boundary. Add an outcome-focused
test before changing protocol or process behavior. Do not add payload logging,
network access, shell interpolation, retries, or descendant-process claims
without a documented product decision.

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

Use English Conventional Commits, for example `fix: preserve numeric request ids`.
Keep code comments in English and include a changelog entry for user-visible
behavior.
