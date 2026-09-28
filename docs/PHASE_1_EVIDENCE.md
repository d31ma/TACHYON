# Phase 1 Web Vertical Slice Evidence

## Status

Phase 1 was completed as a development milestone on 2026-07-26. It is not a
preview release or a supported Tachyon release. The required behavior and the
features that were explicitly postponed are in
[`PHASE_1_SPEC.md`](PHASE_1_SPEC.md).

## Delivered Outcome

The compiled Rust `ty` executable does these things:

- it initializes a project;
- it finds static Tac views and Yon REST-handler routes;
- it writes web output and Route Manifest v1, and the output is the same every
  time;
- it serves the generated files from a development server that is safe on
  loopback.

This path does not use Bun, Node.js, a handler runtime, or any application
dependency.

The implementation rejects Phase 2 and Phase 3 behavior on purpose. This
behavior includes companions, handler execution, context injection, control
tags, components, bindings, and islands.

## Toolchain and Platforms

- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`.
- macOS arm64: the full workspace suite and the acceptance suite that uses the
  real binary run locally on Darwin 25.5.0.
- Linux x86_64: the same locked suite runs in the official
  `rust:1.97.1-bookworm` image with digest
  `sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa`.
- Windows x86_64: all workspace targets and features compile locally for
  `x86_64-pc-windows-gnu`. The repository's `windows-latest` CI (continuous
  integration) job is the required check that the code runs natively. The
  cross-compilation shows only that the code builds.

The GitHub Actions native matrix runs on Ubuntu, macOS, and Windows. On each
system it uses the exact toolchain and runs the checks, the tests, the
documentation build, and a smoke test of the release binary.

## Behavioral Evidence

The workspace has 47 deterministic Rust tests:

- 6 Phase 1 acceptance tests that use the real binary;
- 4 contract/schema tests;
- 7 repository policy tests;
- 24 core tests for discovery, HTML, the compiler, the scaffold, and the server;
- 6 diagnostic contract tests.

The acceptance suite proves these things:

- `ty init`, `build`, `bundle`, `dev`, and `serve` use the compiled binary.
- Two builds of the generated project are identical byte for byte.
- Static Tac views and Yon handler routes appear in the manifest in the
  canonical order.
- Failures give stable human and JSON diagnostics.
- After a collision or an unsupported companion, the last good output stays in
  place.
- Unsafe initialization and serving on an address that is not loopback fail
  closed. This means they stop and do nothing unsafe.
- GET and HEAD requests succeed. Requests for missing paths and path traversal
  do not show source files. The server sends security headers.

## Quality and Security Gates

The final local gate covers these checks:

- formatting;
- checking all targets with all features;
- strict Clippy;
- all tests;
- rustdoc with no warnings;
- dependency advisories, licenses, and sources;
- running the release binary;
- legacy Bun type and test compatibility.

LLVM coverage is:

| Metric | Result | Required |
| --- | ---: | ---: |
| Lines | 91.04% | 90% |
| Functions | 94.00% | 90% |
| Regions | 91.53% | 90% |

Adversarial tests use harmful or broken input on purpose. They cover:

- malformed, oversized, and non-UTF-8 HTML;
- NUL input;
- tokenizer errors;
- scripts and inline events;
- unknown tags and control tags;
- invalid and dynamic route segments;
- collisions;
- companions;
- symlinked sources and targets;
- unsafe output paths;
- binds to addresses that are already in use;
- output rollback;
- missing HTTP paths;
- raw path traversal requests.

## Recovery and Compatibility

The build writes its output to a staging directory next to the destination.
It publishes the output only after every route and the canonical manifest
succeed. If compilation fails, the build restores or keeps the previous output.
Initialization writes through a temporary directory next to the target. It
never overwrites a target that is not empty.

The legacy implementation has not changed. It still passes its type and test
suites. Phase 1 adds no handler or template behavior that could conflict with
the Phase 2 and Phase 3 compatibility work.
