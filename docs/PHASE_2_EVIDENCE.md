# Phase 2 Yon Handler Boundary Evidence

> Historical evidence: the counts and claims in this file describe the original
> Phase 2 gate for JavaScript and Python. ADR 0017 (architecture decision
> record) and its newer workspace tests define the current contract. That
> contract covers eight languages, mandatory layers, relay, and streaming.

## Status

Phase 2 was completed as a development milestone on 2026-07-26. It is not a
preview release or a supported Tachyon release. The required contract and the
features that were explicitly postponed are in
[`PHASE_2_SPEC.md`](PHASE_2_SPEC.md).

## Delivered Outcome

The compiled Rust `ty` executable finds JavaScript and Python Yon handlers. It
calls handlers in either language through Handler Protocol v1. For each call,
the implementation:

- validates the source and the request;
- starts one language-adapter process directly;
- limits the size of framed input and output, and limits how long the process
  lives;
- returns one typed response or a stable diagnostic.

Route Manifest v1 records two kinds of routes in a fixed order:

- routes that have only a handler;
- routes where a handler is in the same place as a Tac view.

Handler values are HTTP responses. The build never merges them into the
context of a view.

The current successor supports eight languages. It adds a readiness probe
before the development server writes output or binds to an address. The probe
has limits and does not repeat the same check twice. When a runtime is
missing, the result is `TY2112`. This is true both during the preflight check
and when the runtime disappears before a later spawn. Doctor reports and
structured server events use only logical runtime labels. They do not include
configured paths or operating-system errors. The C# readiness check builds a
small source file that the framework owns, with a limit. Because of this, the
check reports `Microsoft.NETCore.App` without an SDK as incomplete before the
server binds.

## Toolchain and Platforms

- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`.
- Local JavaScript runtime: Node.js 23.5.0.
- Local Python runtime: CPython 3.14.4.
- macOS arm64: the full workspace, the compiled CLI, and the real JavaScript
  and Python process tests run locally on Darwin 25.5.0.
- Linux x86_64: the same locked suite runs with real Node.js and CPython child
  processes in the official `rust:1.97.1-bookworm` image with digest
  `sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa`.
- Windows x86_64: all workspace targets and features compile locally for
  `x86_64-pc-windows-gnu`. The repository's `windows-latest` job installs
  Node.js 24.22.2 and CPython 3.14.6. This job is the official check that
  processes run natively. The cross-compilation shows only that the code
  builds.

The native GitHub Actions matrix fixes the versions of Rust and the language
runtimes. On Ubuntu, macOS, and Windows it:

- runs the same locked workspace suite;
- builds the documentation with no warnings;
- runs the release binary.

## Behavioral Evidence

The workspace has 69 deterministic Rust tests:

- 6 Phase 1 acceptance tests that use the real binary;
- 5 Phase 2 handler acceptance tests that use the real binary;
- 6 shared tests with real JavaScript/Python processes and adversarial input;
- 5 Handler Protocol and Route Manifest contract tests;
- 7 repository policy tests;
- 34 core unit tests for discovery, HTML, the compiler, the server, handlers,
  and the scaffold;
- 6 diagnostic contract tests.

The Phase 2 tests prove these things:

- JavaScript and Python receive the same typed request contract. Both support
  synchronous and asynchronous static HTTP methods.
- These values pass to the handler and back without change: request IDs, all
  seven HTTP methods, repeated headers, UTF-8 bodies, Unicode data, and
  project paths that contain spaces and Unicode.
- Routes with only a handler, and routes with a view and a handler, produce
  canonical manifest entries.
- By default, handlers do not get host environment values. The runtime copies
  a value only when its name is on an explicit allowlist.
- Queue admission, startup, execution, framing, and process exit all share one
  deadline. They also share a limit on how many run at the same time.
- A timeout or an explicit cancellation stops the child process and waits for
  it to exit. This works for children that are sleeping or that do not
  cooperate.
- A process crash affects only that process. The next call succeeds in a new
  child process.
- These cases produce stable failures of limited size: missing runtimes,
  classes, or methods; syntax errors; exceptions; and values that cannot be
  serialized.
- Processes fail closed when they send malformed, oversized, or trailing
  output, a mismatched ID, extra data hidden in stdout, or too much stderr.

## Quality and Security Gates

The final gate covers these checks:

- formatting;
- checking all targets with all features;
- strict Clippy;
- all tests;
- rustdoc with no warnings;
- dependency advisories, licenses, and sources;
- GitHub Action references that cannot change;
- running the release binary;
- the legacy Bun type and test compatibility suite.

LLVM coverage is:

| Metric | Result | Required |
| --- | ---: | ---: |
| Lines | 90.67% | 90% |
| Functions | 92.62% | 90% |
| Regions | 91.95% | 90% |

All three metrics still have the same minimum. The focused adversarial process
tests are still required, whatever the total coverage is.

## Recovery, Limits, and Compatibility

The supervisor owns every child process until the child exits or is forced to
stop. The supervisor:

- reads stdout and stderr at the same time;
- writes a typed cancellation frame;
- waits for a limited grace period;
- kills the process when needed;
- waits for the process to exit;
- never reuses a process that failed.

These things all have limits: diagnostic content, frame size, handler source,
request fields, stderr, concurrency, and lifetime.

This is not an operating-system security sandbox. A handler keeps the file
system and network access of the developer account that runs it. Phase 2 does
not enforce kernel limits on CPU or memory. [`THREAT_MODEL.md`](THREAT_MODEL.md)
and [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md) state this limitation clearly.

The legacy implementation has not changed. It still passes its type and test
suites. Phase 2 adds no route-context or view behavior. This keeps a clean
compatibility boundary for Phase 3.
