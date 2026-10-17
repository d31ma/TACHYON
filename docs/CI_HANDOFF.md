# CI handoff — 2026-10-17

This note hands the open CI and integration work over to a maintainer who
continues on their own machines. It lists what is on `develop`, what is still on
`work/*` branches, why `develop` CI is red, and how to check each branch
locally. Read it with `docs/HANDOFF.md`, `AGENTS.md` and
`docs/ENGINEERING_STANDARDS.md`.

## Where develop is

`develop` had never run CI before 2026-10-16: `rust-ci.yml` only ran for
`main` and `codex/rust-rewrite`. It now runs on pushes to `develop` and
`work/**`, and on pull requests into `main` and `develop`.

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
  expressions and `ty migrate` fixes. Baselines: cohesion 2269, structure 2898,
  duplicated blocks 109, file names 156.

## Branches still open

Each `*-ci` branch is a finished change rebased onto `develop` and pushed so CI
runs the full matrix. Merge a branch once its CI shows no failure that
`develop` does not also have (see "Reading a CI run" below).

| Branch | What it does | State |
|---|---|---|
| `work/ci-emscripten` | Stops exporting `EM_CONFIG`/`EMSDK` (it broke the .NET wasm workload's Emscripten); strips them from `dotnet publish` in the product; Windows uses `emcc.exe`; longer deadline for the C# web publish | Unblocks Phase 3 Chromium, C# tests on Ubuntu/macOS, Windows native |
| `work/ci-toolchains` | JDK 21 everywhere, Node/Ruby/Emscripten/Dart/Python/Lua for coverage and sanitizers, Lua for the macOS website job, sanitizer flags kept out of nested builds | Fixed the Java/TeaVM test; iterating |
| `work/ci-small-fixes` | Android bridge test regex, managed-storage fixture (TY1310), fuzz crash in `template/frontend.rs` `doctype()`, stderr in the dev-server readiness test | Iterating |
| `work/ci-macos-windows` | macOS Swift singleton test; diagnostics for the Windows WebView2 gate, whose page renders empty on develop | Root cause of the Windows gate not found yet |
| `work/ci-windows-bisect` | Diagnostic probes for the Windows gate | **Not for merging** |
| `work/fsharp-ci` | F# typed HTTPClient injection, typed answers, caching and test Fakes; fixes the .NET first-app gate flake and a shared browser-HTTP race (a declared-length answer is read whole) | No new failures vs develop on Ubuntu/macOS native; Firefox/WebKit not reached yet because Phase 3 stops early |
| `work/python-ci` | Python `async def` service methods as page tasks; LocalState/SessionState for Python | Awaiting CI |
| `work/fakes-ci` | Java/Groovy, Python and JS/TS test units clear Fakes before every case; per-language table in `docs/BEHAVIOR_ANNOTATIONS.md`; Lua runtime notices; Go `LICENSE` shipped next to `wasm_exec.js` | Awaiting CI |
| `work/fakes-rest` | Same Fake clearing for Kotlin, C#, Dart, Go and Rust (on top of `work/fakes-ci`) | Kotlin/C#/Dart verified only in CI |
| `work/cpp-yon-ci` | C++ Yon route controllers compiled with clang++, with the review fixes (route types limited to what the JSON codec reads, default parameter values refused, snippet check runs the C++ examples) | Rebased onto develop; awaiting CI |

Already merged or superseded, safe to delete: `work/cohesion-yon`,
`work/cohesion-template-migrate`, `work/cpp-test-units`, `work/cpp-yon`,
`work/java-checked-models`, `work/scaffold-gate`, `work/tac-page-program-js`,
`work/fsharp-http` (see `work/fsharp-ci`), `work/python-tac` (see
`work/python-ci`).

A local-only branch, `work/ci-self-hosted`, is being prepared: an opt-in switch
(repository variable `TACHYON_SELF_HOSTED`) that runs push-triggered jobs on
self-hosted macOS, Windows and WSL runners while pull requests stay on
GitHub-hosted runners, plus `docs/SELF_HOSTED_RUNNERS.md`.

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

- `@Check` rejections answer a generic 500 in every language (asserted by the
  Java checked-model test). Should they answer 400 or 422 with field details?
  That needs an ADR.
- `ty preview` sends `Cache-Control: no-store` and `Vary`, so the offline
  service worker caches nothing in preview, in any language.
- Phase 6 legacy differential: declare the new scaffold a deliberate break, or
  update the migration list in `scripts/compat/differential.mjs`.
- Python and Lua `HTTPClient`; who is the trusted authority for native live
  updates; menu command recipients other than Rust; whether the planned
  languages and Zig still stand.
- The release candidate: version 26.42.x stamped on the release date, closing
  `[Unreleased]`, merging `develop` into `main` and tagging.

## Smaller follow-ups found on the way

- The relay boundary tests should tolerate a loaded runner.
- A Ruby `#[Needs]` field typed as a browser API class is probably left nil;
  a `Needs` field without a `@return` type is not checked.
- C++ Yon: invalid UTF-8 in a returned string gives an opaque 500; a body
  nested 64 levels gives 500 instead of 400; the handler's exception line is
  never logged.
- `ModelParser::jsdoc_models` can panic on a `/**/` slice in JavaScript
  sources (noticed during the cohesion work, not fixed).
