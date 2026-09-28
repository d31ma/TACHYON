# WebAssembly event continuation - 2026-09-25

## Recovered work

Claude's latest Tachyon session stopped after adding incoming event wiring
for Rust, C# and Go WebAssembly services, before browser verification.
This continuation completes that feature. It does not claim a release or
requalify the earlier Go implementation in ADR 0045.

## Contract

- Every WebAssembly service exposes its compiled listener metadata to the
  page runtime and accepts owner-addressed listener delivery.
- The page owns event delivery for WebAssembly, including the publisher's
  own listeners. Conditions and Order apply regardless of the publishing
  language. Async listeners run in a browser microtask and do not republish
  their return values.
- A synchronous listener's returned event is published once. Retained
  events replay into later owners. Removing a prototype owner unregisters
  its listeners and disposes its activation; replacing it creates a new one.
- Native-host delivery retains its existing local drain. Browser events
  do not gain a network transport, native permissions or Yon resources.
- Existing finite-JSON payload bounds, owner checks and stale-owner
  rejection remain. The page's event runtime stays below its 10,653-byte
  budget.

## Implementation and review

The generated web module registers hidden listener methods with the same
event bus used by browser services. The Rust/C#/Go preludes describe those
listeners and route the internal `event` operation to the live owner.
WebAssembly publication drains emit to the page instead of running a second
local listener pass. This keeps self-delivery, peer delivery, ordering and
async scheduling under one owner.

Incoming listener errors now return a fixed bridge error instead of a
successful reply. The browser marks `listener_failed` while completing the
publisher and continuing healthy listeners. Rust Result errors, Go errors
and C# exceptions have executable coverage; unrecoverable traps remain
outside that recovery contract.

Review found that merely skipping the publisher on the page would deliver
a high-order self-listener before a lower-order peer. A regression reproduced
that defect, and the web-only drain change fixes it. Native Rust and C# use
their original delivery path.

The full suite also exposed a stale source-extraction anchor in
`native/rust.rs`: it expected an attribute no longer present on `TacListener`.
The anchor now names the declaration, preserving the existing test that
only TacValue types enter client test units.

The browser CI job installs the pinned .NET SDK and wasm-tools workload as
well as Rust's WebAssembly target and Go. C# is required in CI; a local run
without its workload reports that skip explicitly.

## Verification

Logs live in
`/Volumes/ANNEX/build/tachyon-wasm-events-20260925.jx1MdQ/`.

- `browser-negative.log`: disabling the new listener registration in a
  temporary local build reproduced the missing cross-service delivery.
  Registration was restored before the successful runs.
- `browser-native-order-red.log`: before the web-only drain change, a Go
  publisher's own listener ran before a lower-order JavaScript listener.
- `browser-all-origin.log`: real Chromium passed for Rust, C# and Go,
  including external delivery, conditions, returned events, cross-language
  Order, same-owner delivery exactly once, deferred Async, retained replay,
  prototype removal and replacement. Both page errors and the event-failure
  marker were checked.
- `browser-listener-failure-red.log`: the interrupted event bridge reported
  a failing listener as success, leaving the browser failure marker unset.
- `browser-qualified.log`: the complete browser matrix passes, including
  error visibility, healthy peer delivery and publisher completion after
  failures in Rust, C# and Go.
- `website-event-snippets.log`: the exact published HTML and Rust/C#/Go
  sources were built and driven in Chromium. Every example produced
  `settled = 25` and `audited = 25`, with no page or event errors. The log
  includes source hashes; authored snippets were not rewritten for this test.
- `final-verification.log`: all canonical Rust gates pass: formatting,
  workspace check, strict Clippy, workspace tests, strict rustdoc and
  dependency advisories/bans/licenses/sources. `cargo deny` reports allowed
  duplicate-version warnings, not policy failures.
- `workspace-tests.log`: 1,153 passed, zero failed and two ignored across
  43 suites. The ignored tests require dedicated Redis and TLS SQL instances
  at `YON_TEST_REDIS_URL` and `YON_TEST_DATASOURCE_URL`; they were not run.
- `typecheck.log`, `application-events.log` and `tac-events.log`: all pass.
- `website-tests.log`: 44 tests pass, zero fail, with 1,528 expectations.
- `native-events-final.log`: native Rust and C# pass activation, ordering,
  conditions, Async/republish, bounded cycles, disposal and stale-owner
  contracts using the final preludes.

The final runtime sizes are 58,558 bytes for `tac-client.js` and 10,569 bytes
for `application-events.js`, within their existing budgets. Scoped
`git diff --check` also passes. Graphify is refreshed for this feature's
code, documentation and CI changes without claiming to reindex unrelated
work in the existing worktree.

This evidence is local macOS and Chromium execution. It is not a new native
emulator run, a hosted GitHub Actions run or a release-readiness claim.
