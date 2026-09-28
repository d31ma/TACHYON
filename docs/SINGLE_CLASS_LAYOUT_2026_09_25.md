# Authored single-class files - 2026-09-25

## Scope

This continuation implements ADR 0044 after the Gateway review. A framework
annotated file holds one class and imports. Plain Yon route handlers, workers
and root middleware follow the same rule. Nested types and Rust impl blocks
belonging to that one type remain valid. Generated composition output and
external relay/command programs are not checked as authored role files.

An empty or comment-only `yon.<ext>` route marker is allowed only when a
controller for that route exists. Without a controller, the marker fails with
TY2015 because it does not declare a Yon Handler. This exception does not
allow executable statements or helper code in a marker file.

## Changes

- A shared lexical validator checks top-level items across the ten source
  languages and emits source-anchored TY1310 migration guidance.
- TestConfiguration is selected from an exact same-language sibling file,
  named after its class. Configuration reads use the existing confined,
  bounded no-follow source access. Missing, mismatched and duplicate names
  fail before execution. Existing production Configuration selection remains
  supported.
- Test-only configurations are not part of production role discovery.
- Bounded role-folder discovery includes behavior-only annotated classes
  without turning them into injectable providers. Unannotated helpers remain
  outside automatic composition.
- Root middleware is a class named `Middleware` with `before(request)` and
  `after(request)`. Every runtime calls `before` before the route and `after`
  after it, for every HTTP method; a missing method lets the phase pass. A
  `Handler` class in a middleware file fails with TY1310. PHP middleware
  keeps response headers and model imports.
- Scaffolds, website examples and execution fixtures were migrated
  together. Pure types belong in models; annotated helpers have dedicated
  files and explicit roles only when they need dependency injection.

## Verification

The implementation and migration passed the following gates on 2026-09-25:

| Gate | Result |
| --- | --- |
| Rust formatting, workspace check and Clippy | Passed; Clippy denies warnings |
| Workspace tests, all targets and features | 996 passed, 0 failed, 2 integration tests ignored by default |
| Rustdoc with warnings denied | Passed |
| Dependency advisories, bans, licenses and sources | Passed; existing duplicate-version warnings remain |
| CLI build and fuzz-target compilation | Passed |
| Semantic JavaScript/TypeScript and declaration checks | Passed |
| Website tests | 44 passed, 1,455 assertions |
| Website service test units | 38 passed; 42 of 42 public annotated methods named by tests |
| Compatibility ledger | 65 claims confirmed, 0 wrong |
| Browser/runtime/script gates | All 15 passed, including no SSR, HMR, lifecycle, focus, events and storage |
| Managed-resource transport probes | 148 assertions passed across Java, Kotlin, C# and PHP |
| Dedicated Redis integration | Passed against an isolated temporary Redis container |
| Complete strict snippet verification | 142 bundles, 225 HTTP method probes, 16 test units, 19 syntax checks and 6 JSON checks; 0 failures |

The negative tests first reproduced acceptance of top-level helpers and missing
sibling configuration composition. Review regressions also reproduced and
fixed misplaced test-only modules entering production composition, a service
being accepted as a configuration fallback, and a nested type supplying the
configuration annotation. PHP generated-source tests preserve preamble,
environment-capture and startup-budget behavior behind the authored boundary.

The snippet runner explicitly leaves 197 descriptive or incomplete excerpts
unexecuted. Native-only Tac examples are source-staged where indicated, not
counted as device-tested. The live SQL integration test still requires a
dedicated TLS database and was not run. The other default-ignored integration
test, Redis, was run separately and passed. These results qualify this feature
on the current host, not a new platform-support tier or release promotion.

Graphify refresh covers the changed source boundary, fixtures, website examples
and documentation; unrelated historical graph context is preserved.

Logs are retained in
`/Volumes/ANNEX/build/tachyon-single-class-20260925.UNgQYF/`.

No commits, publication or native platform support promotion are part of this
continuation. Existing unrelated changes are preserved.
