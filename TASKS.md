# Implementation Tasks

- [ ] Add the Rust package, CLI argument contract, and protocol id helpers.
  - Acceptance: invalid timeout values and missing child commands fail with a
    stable usage error.
  - Verify: unit tests and `cargo test --workspace --locked`.
- [ ] Add bounded stdin/stdout frame forwarding.
  - Acceptance: normal JSON-RPC frames and notifications are forwarded without
    payload mutation; oversized or malformed client frames are rejected.
  - Verify: process-level tests with the fixture child.
- [ ] Add first-request and per-request deadlines.
  - Acceptance: a delayed response returns one generic JSON-RPC timeout error,
    kills the child, and exits 124; child exit is reported without payload echo.
  - Verify: deterministic delayed fixture tests.
- [ ] Add bilingual README, security policy, contribution guide, changelog, and
  release checklist.
  - Acceptance: English is the default entry point and Chinese is linked.
  - Verify: documentation contract script.
- [ ] Add fixed-SHA CI, CodeQL, cargo-deny, cargo-audit, Dependabot, and release
  workflow with checksums, SBOM, provenance, and attestations.
  - Acceptance: all required checks pass on `main` and release artifacts are
    reproducible.
  - Verify: hosted workflow and downloaded asset checks.
- [ ] Publish only after the local contract and hosted checks pass.
  - Acceptance: immutable SemVer pre-release with verified assets and a clear
    Alpha boundary.
  - Verify: GitHub Release inspection.
