# Simplification recovery — 2026-09-24

## Source of this work

The latest Tachyon Claude session is
`51b938e2-3ade-41fd-9545-23174ab55a1a`. It stopped at its usage limit during
the simplification pass. Its working plan and compatibility captures are in
`/Volumes/ANNEX/build/simplify/`. The repository contains substantial
uncommitted work from that session; this recovery preserves it.

The recovery and continuation records below distinguish completed cleanup
from the separate Gateway feature and release qualification. ADRs 0030–0041
supersede earlier API and layout examples in the historical handoff.

## Recovered work

- Restored the complete server test module where an unfinished edit left
  `@@TESTS@@`. All 104 original assertions remain.
- Restored the browser hot-update JavaScript where an unfinished edit left
  a placeholder inside the served string. The 3,340-byte script matches the
  saved pre-refactor source exactly. Added a regression test of the served
  script's event connection, document lifecycle, and reload behavior.
- Removed temporary annotation and event capture tests after comparing
  their fresh outputs with the saved pre-refactor captures. Annotation
  composition output is byte-identical (48,095 bytes), as is the event
  container output across the captured language cases.
- Removed temporary Yon capture wrappers, environment-controlled disk
  writes, and the mutation-capture test. The three Yon source files then
  matched their saved pre-instrumentation versions exactly. No parser
  behavior was changed in this recovery.
- Stopped the specific leftover Claude mutation-capture test process. It
  had run for more than 52 minutes. Its partial scratch results remain on
  ANNEX; they are not a completed compatibility result.
- Confirmed all 153 prepared X1/X2 website prose fields were already
  applied. Then changed five section bodies and two headings (the "eight"
  language headings became "nine"), without changing examples or inline
  code.
- Applied Rust formatting to the interrupted working tree.
- Repaired strict TypeScript checking for the new realm import paths and
  compiler-composed website services. Their ambient names use the actual
  imported class types, with negative type tests; no tests were excluded.
  Added declarations and positive/negative tests for the existing test
  annotations. Restored exact JSDoc parameter types in release checks and
  made the snippet scratch-directory selection narrow optional strings.
  No runtime API was added.

The annotation scanner, server, application/migration, and event refactors
were already partly or fully saved. Recovery does not mean every planned
R5 simplification has been completed.

## Verification

Commands use `/Volumes/ANNEX/toolchains/env.sh` and
`CARGO_TARGET_DIR=/Volumes/ANNEX/build/simplify-target`.

- Locked workspace check, all targets and features: passed.
- Locked workspace Clippy, all targets and features, warnings denied:
  passed again after the new hot-update regression test.
- `cargo fmt --check`: passed.
- Core library tests, all features: 583 passed, 2 ignored. The ignored
  tests require dedicated Redis and TLS SQL instances. This run preceded
  the new hot-update regression test, which then passed separately.
- Ten selected core integration suites: 83 passed. These cover annotation
  layout/discovery, compiled and dynamic controllers, constructor inference,
  JSON returns, Dart, HTTP dispatch, managed resources, and local events.
- CLI migration tests: 5 passed. Signal lifecycle tests: 7 passed, including
  the 60-second three-server test.
- Repository policy tests: 12 passed, including foundation-document links.
- Workspace rustdoc with warnings denied: passed.
- Dependency policy: advisories, bans, licenses, and sources passed. Existing
  duplicate-dependency warnings remain.
- Browser hot updates, application-event runtime, and Tac event browser
  contracts: passed against the freshly built recovery binary.
- Fresh website tests: 43 passed, 1,323 assertions.
- Website framework units (`ty test website`): 34 passed. The compiler's
  method-name check found tests naming all 41 public annotated methods;
  this is not runtime coverage measurement.
- Website browser input gate: passed on desktop and mobile for home,
  documentation, and feature routes, including continuous typing, caret
  preservation, custom slots, and Escape handling.
- Full `bun run typecheck`: passed for the website, release checks, and
  standard and legacy decorators. Negative type tests verify that composed
  services retain their real members and test annotations reject invalid
  options.
- Schema HTTP smoke test: passed for body/header/parameter validation,
  dynamic OPTIONS, and the API reference.
- Annotation and event generated-output captures: exact byte matches.
- Runtime size limits: `tac-client.js` is 58,558 bytes (limit 58,563);
  `application-events.js` is 10,551 bytes (limit 10,653).

These are fresh recovery runs. The full workspace test matrix, all snippet
executions, platform qualification, and remaining browser scripts have not
been rerun as one final simplification gate.

Repository-wide `git diff --check` also reports existing CRLF/trailing-space
changes in the legacy migration fixture. These fixture bytes were not
rewritten by this recovery.

## Continuation: R5, R6, and X3

The remaining cleanup is implemented and reviewed:

- Yon parsing, constructor inference, and lowering now use smaller helpers.
  Constructor tests retain their original assertions and add two exact-source
  checks. Lowering passed 456,720 exact before/after result comparisons across
  nine language variants and invalid plans. Evidence is in
  `/Volumes/ANNEX/build/simplify/scratch/R5/lowering-check/RESULTS.md`.
- Removed the unreachable Tac companion-discovery branch. Moved the shared
  role-folder list into its owning type without changing discovery order.
- CLI execution separates command setup, test execution, case reporting,
  exit status, and handler request construction. Seventeen CLI comparisons
  preserve exit status, stdout, and stderr exactly, including help, mixed
  test outcomes, repeated headers, and JSON diagnostics.
- R6 integration and contract tests use focused setup/assertion helpers.
  Original assertions and embedded fixture text remain. Already-cohesive
  files were reviewed without unnecessary edits.
- `PRODUCT_VERSION` and `CONTRACTS` stay at file level, because other crates
  and the CLI's clap version attribute use them. (`JSON_SCHEMA_DRAFT` and
  `DIAGNOSTIC_PREFIX` were kept here too, but only their own crates use them;
  the follow-up review below moved them.)
- X3 changes 79 of 84 prose fields across five website data files: 26 are
  the prepared drafts as written, 53 are rewrites, and 5 are unchanged. Examples,
  identifiers, headings, and inline code were preserved. Corrections cover
  bundled runtimes, configuration layout, shared role discovery, event
  returns, hot-update fallbacks, platform rendering differences, and the
  Firecracker transport's actual scope. Guide headings now correctly name
  the nine Yon examples, including Dart. Test-unit prose describes aggregated
  failures and distinguishes method-name checks from runtime coverage.
- The compatibility ledger now checks Rust WebAssembly support and current
  Autowired resources. Retired Resource syntax has its own rejection check.
- A real snippet run exposed a stale verifier fixture path for the complete
  configuration example. It now uses the documented
  `server/configurations/ProductionConfiguration.ts` shared role folder.

Fresh continuation verification:

- Canonical formatting, locked workspace check, Clippy, rustdoc, and
  dependency policy: passed. Duplicate-dependency warnings remain.
- Full locked workspace tests: 824 passed, zero failed, two ignored because
  they require dedicated Redis and TLS SQL instances. Log:
  `/Volumes/ANNEX/build/simplify/scratch/R6/workspace-tests.log`.
- Website tests: 43 passed with 1,323 assertions. Framework units: 34 passed,
  with all 41 public methods named by tests.
- Compatibility ledger: 62 confirmed, zero wrong.
- Strict TypeScript checks, including standard and legacy decorators: passed.
- All fourteen selected browser/runtime/HTTP scripts passed: Phase 3,
  managed clients, application events, Tac events, HMR, browser events,
  service workers, expressions, storage, runtime lifecycle, behavior runtime,
  behavior browser, website input, and CHEX schema HTTP checks.
- Full strict-toolchain snippet rerun with real Redis: 137 bundles,
  207 HTTP probes, 16 test units, 13 syntax checks, six JSON checks, and zero
  failures. The 193 explicitly unexecuted entries are reported individually
  in `/Volumes/ANNEX/build/simplify/scratch/R6/snippets-final.log`; these include
  descriptive commands and incomplete excerpts. Native source staging is
  labeled separately and is not device execution.
- The dedicated Redis storage test then passed separately. The TLS SQL test
  remains unexecuted in this continuation. The temporary loopback Redis
  container used no persistent volume.
- Runtime size limits remain unchanged: 58,558 bytes for `tac-client.js`
  and 10,551 bytes for `application-events.js`.

Use `TAC_BIN=/Volumes/ANNEX/build/simplify-target/debug/ty` explicitly for
website checks. The old repository-local `target/debug/ty` is a Linux ELF
artifact and cannot execute on this macOS host. It was not overwritten.

These checks do not constitute a new cross-platform release qualification,
a release publication, or measured CRAP/coverage scores.

## Independent verification and follow-up (Claude, 2026-09-24)

Every gate reported above was rerun separately and passed with the same
numbers: formatting, Clippy, the fuzz workspace check, 824 workspace tests
(0 failed, 2 ignored), 43 website tests, 34 framework units, the ledger
(62 confirmed, 0 wrong), strict TypeScript checks, 15 browser and runtime
scripts, and the snippet verifier (0 failures without the dedicated Redis
probe). Three read-only reviews then compared every change with the saved
tree. They confirmed the Rust changes keep every limit, message, diagnostic
code and assertion, and that the restored `server.rs` tests and hot-update
script match the saved versions. They also found problems, now fixed:

- Website prose: a correction was applied in one place but not in others
  (Yon language counts, Dart missing from lists, `ty test` stopping at the
  first failure, a reversed negation, the Firecracker "same build" claim, the
  streaming-marker claim, `application.json` called private). Every data
  file was searched for each old statement.
- The parity ledger's Rust WebAssembly row needed the `wasm32` target, which
  CI did not install, and it did not check that WebAssembly was produced.
  CI now installs the target, the row checks the built bundle, and a machine
  without the target records the matching diagnostic. `PARITY_LEDGER.md`
  rows that contradicted the script were updated.
- `website/composed-services.d.ts` was never type-checked; it is now
  `website/composed-services.ts`.
- The comment sweep had cut the `@deprecated` messages in
  `types/tachyon-env.d.ts`; they are restored for the current API.
- `website/scripts/verify-snippets.mjs` hard-coded a machine path; it now
  uses `TAC_SNIPPETS_SCRATCH` or the system temporary folder.
- The embedded Python events runtime had gained new top-level helpers after
  user code; they are now members of `_YonEvents`, with byte-identical
  scenario output.
- The unreachable Tac companion plumbing was removed end to end, with
  byte-identical build output. The hot-update test is pinned to the exact
  script (3,340 bytes and its SHA-256). `JSON_SCHEMA_DRAFT` is now
  `Contract::JSON_SCHEMA_DRAFT` and `DIAGNOSTIC_PREFIX` is now
  `DiagnosticCode::PREFIX`.
- The website's install command was too long for the responsive check, and
  `/docs/annotations` overflowed on phones; both are fixed and the
  responsive check passes at every width.

## Remaining feature work

Implement the accepted [Gateway decision](adr/0042-a-gateway-method-calls-another-language.md)
as a separate behavior-changing feature. Start with relay environment and
remaining-deadline fixes, then follow the language slices in
`/Volumes/ANNEX/build/simplify/gateway-plan.md`. The annotation is not yet
implemented. The supervisor currently encodes the request before admission;
the relay fix must account for queue and startup time as well as elapsed
time inside the language runner. A runner-only timeout change is insufficient.

Do not publish, commit, or replace the existing dirty tree merely to satisfy
these checkpoints. Keep platform qualification distinct from compilation.
