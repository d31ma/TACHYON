# 0012 — Whole-workspace coverage ratchet

## Status

Accepted.

## Context

Phase 0 through Phase 4 measured a target-neutral subset of the Rust core.
That subset stayed above 90% for lines, functions and regions. Phases 5
through 7 added more code:

- CLI orchestration;
- five native hosts;
- external toolchain probes;
- WebAssembly compiler drivers;
- server lifecycle code;
- a fuzz-only interface.

Real-process, real-browser and native-host gates are the main tests for these
paths. LLVM still counts them in its whole-workspace denominator.

The first clean Linux pull-request measurement of that larger denominator was
81.26% lines, 80.49% functions and 80.97% regions. We could keep a 90% check
and call a narrower measurement "workspace coverage", but that claim would be
false. We could exclude every real-world boundary only to keep the percentage
high, but that would make the metric less useful.

## Decision

The release gate enforces one 80% floor for lines, functions and regions. It
measures the whole workspace with all features enabled. Only two files stay
excluded: `native/compiler.rs` and `native/macos.rs`. LLVM instrumentation
cannot meaningfully represent the work they do in external processes: the
Swift compiler, code signing and AppKit. The native application gates must
test their behavior.

Coverage is one ratchet: a limit that may move up but not down. It is not the
definition of test quality. These boundaries also require their own focused
tests:

- parser;
- path;
- template;
- compiler;
- handler protocol;
- server;
- native planner.

Those tests are negative, adversarial, fuzz, process, browser or native-host
tests, as each boundary needs. A change may not replace those tests with
aggregate coverage.

To raise the floor, the full CI measurement must show spare room above it. To
lower the floor, another accepted ADR must give a measured reason.

## Consequences

- The number in CI describes the code that the release workspace actually
  contains. It does not describe a hand-selected target-neutral subset.
- A one-point regression in the lowest metric fails the pull request.
- The historical evidence from Phase 0 through Phase 4 stays valid for the
  narrower source set. That evidence continues to state its scope.
- Native, browser, fuzz, sanitizer, compatibility and lifecycle jobs stay
  required. Aggregate source coverage cannot replace running the code in the
  environment where we claim the behavior.
