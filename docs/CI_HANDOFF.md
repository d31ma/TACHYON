# CI handoff — 2026-10-17 (status updated 2026-10-18)

This note hands the open CI and integration work over to a maintainer who
continues on their own machines. It lists what is on `develop`, what is still on
`work/*` branches, why `develop` CI is red, and how to check each branch
locally. Read it with `docs/HANDOFF.md`, `AGENTS.md` and
`docs/ENGINEERING_STANDARDS.md`.

## Status on 2026-10-18

All the work this note handed over is resolved. The sections after this one describe the state on 2026-10-17 and are kept as history.

- **Every branch in "Branches still open" is merged into `develop`, except the diagnostic and bisect branches.** That covers the three CI branches, `cpp-yon-ci`, `fsharp-ci`, `python-ci`, `fakes-ci`, `fakes-rest` and `ci-self-hosted`, plus the macOS Swift test from `ci-macos-windows`. `ci-windows-bisect`, `ci-windows-bisect-prelude` and the WebView2 diagnostics were not for merging. All the `work/*` branches have been deleted.
- **Every red job in "Why develop CI is red" has a fix on `develop`:**
  - **Windows WebView2 gate:** the Windows host now sends each native call to the companion that owns the service (`08746f7f`).
  - **Phase 6:** the scaffold differences are declared migrations (`8f0b6a21`). The Phase 6 job now installs Node 24.18.0 (`f062fb18`).
  - **Android:** the gate expects the native event by its model name (`54374177`).
  - **Phase 3 in Firefox:** the storage gate waits for components to activate (`0de0ecf1`).
- **New fixes found while running the gate on Windows and WSL:**
  - Kotlin compilers that exist only as `.bat` launchers
  - `em++` on Windows
  - the mruby build folder on Windows
  - Lua behavior clocks on Windows
  - the handler cache lock when the clock steps back
  - the .NET readiness probe environment
  - F# `Needs` construction for primary-constructor types
- **CI now runs locally:**
  - GitHub Actions runs only for pull requests from `develop` into `main`.
  - Day-to-day CI is the same gate on the maintainer's machines: macOS, plus Windows ARM64 and WSL. It runs under `cargo nextest` and `sccache`.
  - On slow machines, set `TAC_HANDLER_COMPILE_SECONDS` (default 60, cap 900) to allow longer handler compiles.
  - Load-sensitive tests tolerate a busy machine, and the `relay_boundaries` tests run in nextest's `serial` group (`4603885c`).
- **Decision taken:** Phase 6 keeps the scaffold comparison as a tripwire for drift, with every intended difference declared, instead of declaring a deliberate break.
- **Still open (see the last two sections):**
  - the release candidate
  - in the product, a call that reaches a shared service before its component activates uses the default `tac-client` partition
  - on Windows ARM64, Node 24 sometimes deadlocks at process exit (about 7% of Ruby web drivers under load). Its 2-minute deadline turns that into one failed test.
  - about 40 other `node` driver call sites have no deadline

## Where develop is

`develop` had never run CI before 2026-10-16: `rust-ci.yml` only ran for
`main` and `codex/rust-rewrite`. It now runs on pushes to `develop` and
`work/**`, and on pull requests into `main` and `develop`.

> **Update 2026-10-18:** GitHub Actions now runs `rust-ci.yml` and
> `ruby-runtime.yml` only for pull requests from `develop` into `main` (every job
> is guarded on the head branch), for the tagged release in `publish.yml`, and
> on demand; scheduled fuzzing is on demand too. Day-to-day CI runs on the
> maintainer's own machines (macOS, plus Windows and WSL on the Surface) with
> the same gate: fmt, clippy `-D warnings`, the full workspace tests and rustdoc.
> All the branches below are merged into `develop`.

These batches landed on `develop` on 2026-10-16 and 2026-10-17. Each was checked
locally first (fmt, clippy `-D warnings`, rustdoc, the full workspace test suite
compared with a `develop` baseline, and the relevant Chromium gates):

- C# and Kotlin native services are page singletons; Go server tests select a
  small module closure; `:safe-html`, `theme_color`, smaller route scripts,
  Go `json` tags, the Go page-exit fix, loop and `<select>` retention.
- `bun check` (Bun 1.4.3) replaces `tsc` for the repository's own type checks;
  dependency and GitHub Actions bumps; `rustls` 0.23.46 for a cargo-deny
  advisory.
- Docs corrections, the doctor tests no longer depend on `PATH`, F# in the Yon
  rule table, website snippet coverage with a strict CI step, Pyodide/CPython
  licence notices, PHP test units clear Fakes.
- CI runs on `develop` and `work/**`; the SVG namespace, logic controls and
  scaffold browser gates run in CI.
- Java `@Checked` model runtime test, JS/TS services share one Setup class,
  Ruby and Lua refuse a `Needs` field that names another service (TY2015).
- C++ test units: Fakes, HTTP answers, `@ValueSource`, coverage.
- Cohesion refactors of the Yon parser, route types, handler cache, template
  expressions and `ty migrate` fixes. Baselines: cohesion 2268, structure 2898,
  duplicated blocks 109, file names 156.

## Branches still open

Each `*-ci` branch is a finished change rebased onto `develop` and pushed so CI
runs the full matrix. Merge a branch once its CI shows no failure that
`develop` does not also have (see "Reading a CI run" below). Heads are as of
this note; `git fetch origin` and compare before starting.

| Branch | Head | What it does | State |
|---|---|---|---|
| `work/ci-emscripten` | `73f86751` | Stops exporting `EM_CONFIG`/`EMSDK` (it broke the .NET wasm workload's Emscripten); strips them from `dotnet publish` in the product; Windows uses `emcc.exe`; longer deadline for the C# web publish | Unblocks Phase 3 Chromium, C# tests on Ubuntu/macOS, Windows native |
| `work/ci-toolchains` | `955e9367` | JDK 21 everywhere, Node/Ruby/Emscripten/Dart/Python/Lua for coverage and sanitizers, Lua for the macOS website job, sanitizer flags kept out of nested builds | Fixed the Java/TeaVM test; iterating |
| `work/ci-small-fixes` | `cd05b247` | Android bridge test regex, managed-storage fixture (TY1310), fuzz crash in `template/frontend.rs` `doctype()`, stderr in the dev-server readiness test | Iterating |
| `work/ci-macos-windows` | `5724a923` | macOS Swift singleton test; diagnostics for the Windows WebView2 gate, whose page renders empty on develop | Root cause of the Windows gate not found yet |
| `work/ci-windows-bisect` | `c9a1b9f3` | Diagnostic probes for the Windows gate (page errors, a bridge probe, the staged bundle, WebView2 arguments) | **Not for merging** |
| `work/ci-windows-bisect-prelude` | `bf9a93a7` | Bisect probe: restores the per-activation C# prelude from before `e1921367` to test whether that change empties the Windows gate page | **Not for merging**; head carries `[skip ci]`, run it on Windows by hand |
| `work/fsharp-ci` | `c8397342` | F# typed HTTPClient injection, typed answers, caching and test Fakes; fixes the .NET first-app gate flake and a shared browser-HTTP race (a declared-length answer is read whole) | No new failures vs develop on Ubuntu/macOS native; Firefox/WebKit not reached yet because Phase 3 stops early |
| `work/python-ci` | `ffe55532` | Python `async def` service methods as page tasks; LocalState/SessionState for Python | Awaiting CI |
| `work/fakes-ci` | `f565ff12` | Java/Groovy, Python and JS/TS test units clear Fakes before every case; per-language table in `docs/BEHAVIOR_ANNOTATIONS.md`; Lua runtime notices; Go `LICENSE` shipped next to `wasm_exec.js` | Awaiting CI |
| `work/fakes-rest` | `056dd059` | Same Fake clearing for Kotlin, C#, Dart, Go and Rust (on top of `work/fakes-ci`) | Kotlin, C# and Dart fixes not yet tested by any run (head carries `[skip ci]`); run `test_units::every_typed_language_clears_its_fakes_before_each_case` on a machine with kotlinc, dotnet and dart. Locally, one `behavior_jvm` test and four `tachyon-core` lib tests failed and were not yet compared with `develop` |
| `work/cpp-yon-ci` | `94c013a3` | C++ Yon route controllers compiled with clang++, with the review fixes (route types limited to what the JSON codec reads, default parameter values refused, snippet check runs the C++ examples) | Based on `cd1e55cd`; CI run 39050254718 shows no failure that `develop` lacks, and the strict language-gap step passes. Needs another rebase: `develop` has since lowered the cohesion baselines (`31855d03`) and refactored `ty migrate` and template code, so re-measure the baselines on a clean tree after rebasing |
| `work/ci-self-hosted` | `c127798b` | Opt-in switch (repository variable `TACHYON_SELF_HOSTED`) that runs push-triggered jobs on self-hosted macOS, Windows and WSL runners while pull requests, manual runs and tag releases stay on GitHub-hosted runners (Android also needs `TACHYON_SELF_HOSTED_ANDROID`); setup guide in `docs/SELF_HOSTED_RUNNERS.md` | Head carries `[skip ci]`. The policy test passes locally; the full gate and `cargo deny` were not run. Any new job in `rust-ci.yml` must use its guarded `runs-on` form |

Already merged into `develop` or superseded, safe to delete:

| Branch | Head | Where its work is |
|---|---|---|
| `work/cohesion-yon` | `4fa56905` | Merged into `develop` |
| `work/cohesion-template-migrate` | `2c2262d4` | Merged into `develop` |
| `work/cpp-test-units` | `e24c98f4` | Merged into `develop` |
| `work/cpp-yon` | `3247ba6a` | Superseded by `work/cpp-yon-ci` |
| `work/java-checked-models` | `ba06e71d` | Merged into `develop` |
| `work/scaffold-gate` | `c18dde22` | Merged into `develop` |
| `work/tac-page-program-js` | `b3c08198` | Merged into `develop` |
| `work/fsharp-http` | `068504e0` | Superseded by `work/fsharp-ci` |
| `work/python-tac` | `65617876` | Superseded by `work/python-ci` |

These branches were integrated by cherry-pick from the integration machine and
never pushed. Nothing on them is missing from `develop` or the branches above:
`work/ci-gates`, `work/ci-fixes`, `work/core-small`, `work/docs-loose-ends`,
`work/snippet-coverage`, `work/notices-fakes` and `work/lua-ruby-needs` are in
`develop`; `work/fakes-notices-2` is in `work/fakes-ci`; the review branches
for C++ test units and C++ Yon are in `develop` and `work/cpp-yon-ci`.

## Why develop CI is red

Most jobs stop at their first failing step, so later steps have not run on any
branch yet. Expect another round of failures once these are fixed.

| Job | First failure | Fix |
|---|---|---|
| Dependency, license, and source policy | rustls advisories | Fixed on develop (0.23.46) |
| Native Rust gate (ubuntu, macOS) | C# web tests: `"$CFGDIR/upstream/bin/clang"` not found (leaked `EM_CONFIG`); Java `release version 21 not supported`; macOS Swift singleton probe | `work/ci-emscripten`, `work/ci-toolchains`, `work/ci-macos-windows` |
| Native Rust gate (windows) | `emcc.bat` is not recognized | `work/ci-emscripten` |
| Phase 3 Chromium | storage gate fixture: same `EM_CONFIG` leak | `work/ci-emscripten` |
| Real managed SQL, document and Redis | FYLO route answers 503 (fixture breaks TY1310) | `work/ci-small-fixes` |
| Rust coverage ratchet | missing Node 24, Ruby, JDK 21, Emscripten | `work/ci-toolchains` |
| Phase 7 sanitizers | Go 1.27, JDK 21, Dart, Ruby, wasm32 target missing; a timing budget under ASan | `work/ci-toolchains` |
| Phase 7 fuzzing | `template_frontend` panics on a non-char-boundary slice | `work/ci-small-fixes` |
| Phase 4 macOS parity | `ty doctor website`: Lua not on PATH | `work/ci-toolchains` |
| Phase 5 Android | bridge unit test regex looks for the old `NATIVE_SHIM` name | `work/ci-small-fixes` |
| Windows WebView2 gate | page renders no controls after 20 s; the same gate passes on `main` | `work/ci-macos-windows` (open) |
| Phase 6 legacy differential | the scaffold differs from the v26.30.04 oracle in ~25 places | Needs a decision (below) |

Suggested merge order: `work/ci-emscripten`, `work/ci-toolchains`,
`work/ci-small-fixes`, then re-run the rest on top so their later steps execute.

## Reading a CI run

A branch is fine when it adds no failing test that `develop` does not already
have. To compare, list the failing tests of the same job on `develop` and on
the branch:

```bash
gh api "repos/d31ma/TACHYON/actions/runs/<run-id>/jobs?per_page=50" \
  --jq '.jobs[] | select(.conclusion=="failure") | "\(.id) \(.name)"'
gh api "repos/d31ma/TACHYON/actions/jobs/<job-id>/logs" \
  | sed -E 's/^[0-9TZ:.-]+ //' \
  | awk '/^failures:$/{f=1;next} f&&/^test result/{f=0} f' \
  | grep -E '^\s+[a-z_:0-9]+$' | sort -u
```

## Checking a branch locally

The canonical gate is in `AGENTS.md`. On a machine without every SDK, compare
the failing tests with an untouched `develop` checkout instead of expecting a
clean run. Things that matter:

- Go harnesses write `go 1.27` into `go.mod`; put Go 1.27 first on `PATH`
  (`GOTOOLCHAIN=go1.27.0 go version` downloads it).
- Yon JavaScript needs Node 24.5 or newer. With an older Node, set
  `YON_JAVASCRIPT_RUNTIME=bun` (CI does this for the scaffold gate).
- Rust web services need `rustup target add wasm32-unknown-unknown`.
- Browser gates take `TAC_BIN=<path to a built ty>` and usually a
  `TAC_*_BROWSERS=chromium` variable to limit engines. The repository pins
  Playwright 1.62.1; its browsers must match that version.
- `cargo test -p tachyon-contracts --test repository_policy` bakes the source
  path in at compile time (`env!("CARGO_MANIFEST_DIR")`). Do not share one
  `CARGO_TARGET_DIR` between two checkouts, or the policy tests may scan the
  other checkout.
- The cohesion scan counts untracked files too. Measure baselines on a clean
  tree: `git archive HEAD | tar -x -C <dir>` and run
  `cargo run -p cohesion-scan -- <dir> --json`. Baselines are a ratchet: set
  them to the measured total, never above it.
- The `relay_boundaries` tests are timing-sensitive and fail at random on a
  heavily loaded machine; run them alone before treating a failure as real.

## Decisions waiting on the maintainer

- ~~`@Check` status code~~: decided by
  [ADR 0101](adr/0101-check-rejections-answer-422-problem-details.md). A
  rejection answers 422 with RFC 9457 problem details in every language.
- ~~`ty preview` sends `Cache-Control: no-store` and `Vary`, so the offline
  service worker caches nothing in preview, in any language.~~ Fixed on
  2026-10-18: `ty start` and `ty preview --static` send `no-cache` for the
  files in the worker's asset table (ADR 0093 amendment).
- ~~Phase 6 legacy differential~~: decided on 2026-10-18. The migration list in
  `scripts/compat/differential.mjs` declares each intended difference.
- Python and Lua `HTTPClient`; who is the trusted authority for native live
  updates; menu command recipients other than Rust; whether the planned
  languages and Zig still stand.
- The release candidate: version 26.42.x stamped on the release date, closing
  `[Unreleased]`, merging `develop` into `main` and tagging.

## Smaller follow-ups found on the way

- The relay boundary tests should tolerate a loaded runner.
- A Ruby `#[Needs]` field typed as a browser API class is probably left nil;
  a `Needs` field without a `@return` type is not checked.
- C++ Yon: a root `middleware.hpp` is compiled as a handler and fails with a
  clang++ link error (`TY2004`) instead of a clear "C++ middleware comes later"
  refusal.
- C++ Yon: invalid UTF-8 in a returned string gives an opaque 500; a body
  nested 64 levels gives 500 instead of 400; the handler's exception line is
  never logged.
- `ModelParser::jsdoc_models` can panic on a `/**/` slice in JavaScript
  sources (noticed during the cohesion work, not fixed).
