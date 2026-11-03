# Phase 0 Evidence

Status: **complete — local acceptance passed on 2026-07-26**

This file records a run on macOS only. The native Linux and Windows results
come from CI (continuous integration). This file does not assume them from the
macOS run.

## Baseline

- Rewrite branch: `codex/rust-rewrite`
- Legacy baseline commit: `47775135e456cd8db3c80f4dfde0afaf1273296b`
- Product foundation version: `0.0.0-phase0`
- Required Rust toolchain: `1.97.1`

## Local Gate

- Host: macOS arm64.
- Installed and selected toolchain:
  - `rustc 1.97.1 (8bab26f4f 2026-07-14)`;
  - `cargo 1.97.1 (c980f4866 2026-06-30)`.
- `cargo fmt --check`: passed.
- `cargo check --workspace --all-targets --all-features --locked`: passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: passed.
- `cargo test --workspace --all-targets --all-features --locked`: passed, 15
  Rust tests.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
  --locked`: passed.
- `cargo llvm-cov --workspace --all-features --locked --fail-under-lines 95
  --fail-under-functions 90 --fail-under-regions 90`: passed. Coverage was
  95.09% of lines, 92.00% of functions, and 91.23% of regions.
- `git diff --check`: passed.

The host also has a Homebrew Cargo that comes earlier on `PATH`. We added the
Rustup `1.97.1` toolchain directory to the front of `PATH` to collect the local
evidence. We did not use the older Homebrew binary.

## Contract Gate

- We checked seven canonical Draft 2020-12 schemas against the meta-schema.
- Each schema accepted its valid example and rejected its invalid example.
- The Rust registry that is embedded in the code matched the schema files on
  disk exactly.
- Repository policy tests checked these things:
  - the required foundation documents exist;
  - there are five accepted ADRs (architecture decision records);
  - GitHub Actions are pinned to fixed versions that cannot change;
  - relative links point to files that exist;
  - the workspace, the toolchain, and CI use one exact Rust version.

## Supply-Chain Gate

- `cargo deny check advisories bans licenses sources`: passed with
  `cargo-deny 0.19.7`.
- `cargo install --locked cargo-llvm-cov --version 0.8.6` installed the pinned
  coverage runner. Both the local run and CI use it.
- All GitHub workflow and issue-form YAML files parsed without errors.
  `actionlint 1.7.7` accepted every workflow.
- Legacy compatibility check: Bun `1.3.11` installed the existing lockfile.
  Type checking passed. The legacy suite passed 637 tests in 62 files, with
  zero failures.

## Native Matrix

| Target | Evidence |
| --- | --- |
| macOS arm64 | builds; the full local gate above passed |
| Linux x86_64 | waiting for GitHub Actions |
| Windows x86_64 | waiting for GitHub Actions |

"Phase 0 complete" means the foundation is complete and it passed the local
checks. It does not claim that remote CI jobs passed, because they have not
run yet.
