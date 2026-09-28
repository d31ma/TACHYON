# Gateway review and recovery - 2026-09-25

## Recovered work

The latest Tachyon Claude session is
`51b938e2-3ade-41fd-9545-23174ab55a1a`. After the September 24 cleanup, it
implemented models across the service languages (ADR 0043), website fixes,
and all six Gateway slices (ADR 0042). It stopped during independent review.
Its detailed implementation record remains in
`/Volumes/ANNEX/build/simplify/gateway/PROGRESS.md`.

The old gate wrapper returned success even though its snippet step failed.
The preserved summary lists three Rust snippet readiness failures and skipped
Redis verification. Those results are not an all-green qualification. The
wrapper now propagates a failing step, and this continuation uses separate
logs under `/Volumes/ANNEX/build/tachyon-recovery-20260925.YiQvgp/`.

## Confirmed defects repaired

- An explicit allowlist could restore private environment names in command
  gateways. Reserved application, resource and protocol names are now excluded
  case-insensitively, even when allowlisted. Tests use synthetic values in a
  subprocess; ordinary allowlisted values still arrive.
- A command could overflow stderr and still return success. Overflow and
  stream-read failures now produce the fixed `adapter` error. Its aborted
  stdin writer is joined before command cleanup returns.
- PHP captured relay timing and environment after application initialization.
  The runtime now runs as a confined auto-prepend file, with a separate
  auto-append entry. Startup delays count against the deadline and application
  environment changes do not reach relay programs. Strict types, namespace
  blocks and imports remain valid. The standalone transport fixture explicitly
  invokes the entry after loading its runtime and handler.
- Resource endpoint timeouts and Drop could abort live Gateway futures before
  process-group cleanup. Regressions reproduced surviving leaders and
  descendants. Endpoint shutdown now signals cancellation and joins calls;
  only request reads, ordinary storage calls and response writes have aborting
  time bounds. Gateway calls retain ownership through termination and reaping.
- Rust targets with consuming or mutable receivers reached native compilation
  even though providers are shared. They now fail with actionable `TY1312`;
  supported target methods use `&self`.
- Overload selection depended on declaration order. Matching now uses arity,
  concrete instance status and JSON types. Ambiguous matches and conflicting
  requested arities fail deterministically. The chosen arity reaches generated
  dispatch and its cache identity. A real Java invocation verifies the result.

## Verification

The targeted regressions passed after reproducing the defects. Fresh canonical
gates passed: formatting, locked workspace check, warning-free Clippy,
958 workspace tests across 39 suites, warning-free rustdoc and dependency
policy. The CLI build and the locked fuzz-target compilation also passed.
Two external-service tests were ignored by the workspace run; they are not
included in the passing count.

Website checks passed: type checking, 43 website tests, 37 framework test
cases, the compatibility ledger and 15 browser/runtime/schema scripts.
The direct Java/Kotlin/C#/PHP transport suite passed 148 assertions, and the
dedicated Redis JSON/TTL/delete test passed against a disposable local
container. The TLS database integration test remains unexecuted.
The strict full-snippet check passed with two workers and required toolchains:
142 bundles, 225 HTTP method probes, 16 test units, 19 syntax checks and
6 JSON checks, with zero failures. It explicitly left 197 descriptive,
standalone or incomplete examples unexecuted; this is not a claim that every
displayed excerpt ran. Native bundle checks labelled source staging do not
prove execution on the target operating system. Individual exit codes, rather
than the old wrapper's exit code, are the evidence.

The temporary Redis container had no persistent volume and was removed after
verification. The test logs remain in the ANNEX directory named above.

No native Windows, device, store-signing or real Firecracker qualification is
claimed. Gateway remains refused under Firecracker. This work is not a release
publication, measured coverage result or unqualified production-readiness claim.

## Next feature

Implement [ADR 0044](adr/0044-an-annotated-file-holds-one-class.md) only after
the Gateway review gate is complete. The user explicitly included plain Yon
handlers, workers and middleware in the one-class-plus-imports rule. External
relay and command-gateway programs remain exempt. Test configurations need
their own files and must be composed without entering browser bundles.

Validate authored sources before composition or generated runtime insertion.
Update scaffolds, fixtures, website examples and test units together; do not
enforce the rule on a generated unit containing framework-owned helper types.
Keep this behavior-changing feature separate from the Gateway fixes.

Nothing was committed, pushed or published. Existing unrelated changes remain
in the user's working tree.
