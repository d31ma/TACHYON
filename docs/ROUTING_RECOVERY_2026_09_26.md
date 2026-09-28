# Routing recovery - 2026-09-26

This continues the interrupted Claude routing wave. It is not completion of
the later queued Ruby-language, error-handling, typed-argument or guard phases.
The accepted routing contract is [ADR 0048](adr/0048-yon-routes-come-from-annotations.md).

## Public contract

- Yon controllers live in `server/controllers/<Name>Controller.<ext>`.
- `RequestRoute` supplies the class path. `GetRoute`, `PostRoute`, `PutRoute`,
  `PatchRoute` and `DeleteRoute` supply the verb and optional method path.
- Route method names start with the matching verb, using their language's casing.
- `FromPath`, `FromQuery`, `FromBody`, `FromHeader` and `FromCookie` bind input.
  Signature types and models validate input and model responses.
- `HTTPClient` and the five request method annotations replace `Client`,
  `WebCall` and the Exchange names. Client parameters use `PathVariable`,
  `RequestParam`, `RequestBody` and `RequestHeader`.
- Tac pages remain folder-routed. Old Yon route folders, markers and schema
  files are refused; migration converts safe cases and reports manual work.

## Website and editor recovery

The website no longer publishes executable examples using the retired entry
paths or annotations. The limited-store and configuration guides now carry
the same complete ten-language examples as the annotation reference.
Kotlin is included and executed, not treated as an unavailable tab.

The routing annotation glossary appears in navigation and is linked from
routing and client guidance. Its typed-path example covers every Yon language.
TypeScript declarations provide plain-language hover descriptions. Retired
markers are absent from declarations and covered by negative type tests.

Current reference documents describe annotation-owned routes, including the
README, product context, engineering standards, Yon annotations, behavior
annotations, source layout, configuration and browser migration guidance.
Historical evidence retains its original contract. Superseding ADR links
explain which earlier routing decisions no longer apply.

The previous `tac.catalog.title` fix remains intact. Every Settings schema
declares TITLE, PAGE_SIZE and CACHE_ENABLED. This recovery did not weaken
prefix validation or restore Go's filtered configuration workaround.

## Executed website checks

All commands source `/Volumes/ANNEX/toolchains/env.sh`. They use the rebuilt
checkout binary `/Volumes/ANNEX/build/simplify-target/debug/ty`, not an installed
release. Snippet probes use concurrency 1 and require all selected toolchains.

Logs are in
`/Volumes/ANNEX/build/simplify/scratch/CODEX-RECOVERY-2026-09-26/site/`.

| Check | Result | Log |
| --- | --- | --- |
| Website tests and bundle | 51 passed, 4,613 assertions | `website-tests-qualified.log` |
| Full JavaScript and TypeScript checks, including declaration modes | Passed | `typecheck-qualified.log` |
| Published language suites | 8 bundles, 20 HTTP probes | `languages.log` |
| Checking values and method arguments | 20 HTTP probes | `validation.log` |
| Limited Store and Query | 20 HTTP probes | `stores.log` |
| Reading settings and choosing configuration | 20 HTTP probes, 1 JSON check | `settings.log` |
| Glossary typed paths | 20 HTTP probes | `glossary.log` |
| Outgoing HTTP: Yon | 20 HTTP probes | `yon-clients.log` |
| Calling a method in another language | 20 HTTP probes | `gateway-guide.log` |
| Outgoing HTTP: Tac | 8 bundles | `tac-clients-qualified.log` |
| Configuration binding: Tac | 8 bundles, 6 browser probes, 2 native test units | `tac-settings.log` |
| Storage client recipe | 2 bundles | `storage-clients.log` |
| Event-driven client recipe | 2 bundles | `cookbook-clients.log` |
| Public configuration loader | 2 bundles | `public-configuration.log` |
| Fetch-cache feature | 2 bundles; 7 constituent fragments not run independently | `fetch-cache.log` |

The seven ten-language HTTP groups each exercise OPTIONS schema and a route
method in JavaScript, TypeScript, Python, PHP, Java, Kotlin, C#, Rust, Dart and
Go. They have zero skips or failures. The fetch-cache fragments are compiled
inside its two page projects; that command does not independently execute
each fragment or assert browser network behavior. Outgoing HTTP: Tac performs
native source staging for Swift, Kotlin, C# and Rust, not execution of those
native HTTP adapters. Its other four language tabs build browser bundles.

These focused snippet runs total 32 bundles, 140 HTTP probes, 6 browser probes,
2 native test units and 1 JSON check. All selected language toolchains were
available. The seven fetch-cache fragments described above are the only
entries not independently executed. Exact Python JSON dump formatting was
preserved for docs.json, features.json and languages.json; scoped diff checks
also passed. This document does not claim an unqualified stable release or
remote CI execution.

Timing matters for these totals. The logs for languages, stores, settings,
Outgoing HTTP: Yon and the gateway guide (100 of the 140 HTTP probes) ran
between 01:10 and 01:18. That is before strict model records (01:38) and
before the PHP promoted-field fix (01:48). Strict records change the
published OPTIONS schemas and can refuse a response, so those five groups
show the state before strict records; they were not re-run. Only the
validation guide was re-run after both changes
(`validation-strict-records-qualified.log`). The typed-path guide was last run
at 01:46 (`glossary-final-models.log`), after strict records but before the
PHP fix.

## Defects found by execution

`Outgoing HTTP: Tac` initially failed for six compiled service languages.
Five examples needed an explicit Service marker alongside HTTPClient.
Adding it also exposed a compiler defect: qualification removed or ignored
Service before interpreting HTTPClient as transport metadata. Dart showed
the same defect even though its example already declared both markers.
The compiler now keeps Service while it classifies HTTPClient and removes
the role marker only after request metadata is lowered. Dart applies the
same ordering before its role erasure. The examples keep both annotations;
they are not weakened to hide the defect. The final eight-tab retry passes.

Red logs: `tac-clients.log`, `tac-clients-service-markers.log` and
`tac-clients-final.log`. The final passing log is `tac-clients-qualified.log`.

## Core follow-up: strict model records

Declared model records reject undeclared fields in requests and responses,
including nested records. Their published schemas use
`additionalProperties: false`. An explicitly open `Any` field remains open.
This preserves validation safety while replacing separate schema files.

The strict check exposed a PHP parsing defect in the published Product model.
Its fields are public constructor-promoted properties. The parser previously
read that model as an empty record, so it rejected valid name, sku and level
values. The model and website data were correct and were not weakened.
The parser now reads public promoted fields, including readonly, nullable and
defaulted properties. Private properties and ordinary constructor parameters
do not become public model fields.

Six focused route-type tests pass. Added regressions cover PHP promoted-field
discovery, required versus defaulted fields, private and ordinary parameter
exclusion, nested undeclared-field rejection, open Any fields and the published
schema. After rebuilding the CLI, the complete ten-language "Checking values
and method arguments" guide passed again with required toolchains enabled:
20 HTTP probes, zero skips and zero failures. This includes OPTIONS and POST
for every language, not only PHP. Final follow-up log:
`/Volumes/ANNEX/build/simplify/scratch/CODEX-RECOVERY-2026-09-26/site/validation-strict-records-qualified.log`.

## Combined qualification

The coordinator and independent core reviewer reported these additional checks:

- Core library after the final parser fix: 867 passed, 2 ignored.
- Core all-targets, all-features Clippy: passed with warnings denied.
- Controller execution: all ten Yon languages passed
  (`routing/controller-tests.log`, 01:07, before strict records; the later
  qualified workspace test run covers the same tests).
- Migration peer review: independent recheck passed.
- Parity ledger: 65 of 65 passed, recorded at 01:20, before the strict-record
  follow-up, in
  `/Volumes/ANNEX/build/simplify/scratch/CODEX-RECOVERY-2026-09-26/ledger.log`.
- Client, storage, service-worker and behavior browser gates: reported as
  passed; no log for them is in the recovery directory.
- Behavior runtime checks: 36 checks plus 9 additional checks reported as
  passed; no log is in the recovery directory.
- Schema contract and standalone runtime checks: reported as passed; no log is
  in the recovery directory.

The post-freeze Mac canonical workspace gate passed: formatting, all-target /
all-feature check and Clippy with warnings denied, tests, warning-denied
rustdoc, and dependency advisories/bans/licenses/sources. The complete test
run passed 1,280 tests across 47 targets, with zero failures and two explicitly
ignored live-database tests. Those require dedicated SQL and Redis services;
no live-database success is claimed. The corrected Markdown policy target
passed all 17 tests. Logs are the `qualified-*.log` files under the recovery
scratch directory.

The earlier red runs remain as evidence. They exposed stale routing fixtures,
ignored unsupported controller extensions, model diagnostic paths, open record
validation, and Markdown fence handling. The final run follows all source
repairs, including PHP promoted-property parsing. The final ten-language
validation guide, run after the PHP fix, passed 20 HTTP probes. The final
typed-path guide also passed 20 HTTP probes, but it ran two minutes before the
PHP fix.

Later changes are not covered by this qualification: Yon error handlers
(ADR 0051) and the review fixes to model records made after it (inherited
fields, open records, PHP constructor rules and field splitting).

## Independent Windows-host checks

The older Windows Dropbox checkout was left untouched. A hash-verified source
snapshot was copied into isolated Windows Temp and WSL directories; no secrets
or installed toolchains were copied or changed. Logs and source hashes are in
`/Volumes/ANNEX/build/simplify/scratch/CODEX-RECOVERY-2026-09-26/wsl/EVIDENCE.md`.

Linux ARM64 under WSL passed workspace formatting, all-target/all-feature
check and Clippy, and warning-denied rustdoc. The final two-file PHP parser
delta was hash-verified before the same gates were rerun. All six model unit
tests passed there, including the PHP regression.

Earlier WSL runtime subsets are intentionally not called fully qualified:
the route filter passed 51 tests and failed two; migration passed 40 and
failed one. These overlap. Kotlin was absent, and the installed Node 18
could not load the generated JavaScript module. The existing startup
readiness probe also refuses that Node capability; the direct unit test
bypasses startup readiness. Contracts and diagnostics passed all 13 tests.
No complete all-language WSL run or live-database coverage is claimed.

A native Windows ARM64 attempt used the installed pinned Rust 1.97.1 but
stopped compiling a dependency because `link.exe` could not be found. It ran
no tests and produced no native performance comparison. No x64-emulated
substitute or tool installation was used. WSL results are Linux evidence,
not native Windows qualification.

## Scope limits

- No commit, push, release or external issue changes were made.
- Workspace success does not replace native-platform or live-database gates.
- No mobile emulator or desktop native application was launched in this wave.
- Ruby runtime artifact qualification is a separate prerequisite recorded in
  ADR 0049. It is not a claim that Tac or Yon Ruby authoring support is complete.
- Later NEXT briefs remain queued. Routing recovery does not implement their
  new scheduling, error, argument or rule-table contracts implicitly.
