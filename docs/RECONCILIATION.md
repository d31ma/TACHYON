# Local feature reconciliation

This document tracks how features from an older local tree return to main.
"Reconciliation" means checking each local feature against the released code
and deciding to keep, restore, replace or reject it.

## Release acceptance boundary

- The starting points are the released `v26.35.07` and main at commit `963031f`.
- The feature list comes from the preserved local tree `9b1ff17`. It is not
  limited to the changes that pull request (PR) 172 selected.
- The old tag stays immutable.
- A new version may publish only when both conditions hold:
  - every row in the table below has behavioral evidence or an explicit
    compatibility decision;
  - the downloadable binaries pass the same gates.

Graphify gives dependency context. Source snapshots and executable tests
decide what is included. Existing recovery branches and release branches stay
as permanent references.

## Feature inventory

ADR means architecture decision record (see `docs/adr/`).

| Area | Acceptance criterion | Disposition |
| --- | --- | --- |
| Yon handlers/relays/streams | Eight owned languages. Plain Handler entry points. Six languages can stream with generators. No fallback to the legacy interpreter. | Keep the process hardening. Adopt the ADR 0020 authoring conventions. |
| Handler/source/process lifetime | Non-following owned snapshot, bounded processes, cache quota, runtime readiness, cancellation, first and second signals | Keep the released behavior. Do not replace it with older local implementations. |
| Browser expressions | Await, assignment, `$event`, switch, arithmetic, nested lexical scope, scoped CSS, component naming | Keep the paths that already shipped. Add regression tests. |
| Editing focus | Inputs without IDs, caret and selection, textarea, and custom-element slot ancestry stay intact through reactive updates | Keep the connected editing control. Verify native hosts and the real website. |
| Browser helpers | `this.tac` fetch/invalidate/precache/clearAssetCache/render and bounded retained publish/subscribe | Replaced by ADR 0028: owner-bound `@Resource` fields (`cacheManager`, `resourceLoader`, `renderScheduler`, `element`, `signal`). They keep the same bounded storage and lifecycle ownership. `this.tac` is removed, not kept as an alias. |
| Persistence/lifecycle/signals | `$` per-tab and `$$` cross-session persistence; `mount()`; explicit publish/subscribe | Persistence is kept. `mount()` and publish/subscribe are replaced by `@PostConstruct`/`@PreDestroy`, an injected `ApplicationEventPublisher` and `@EventListener` (ADR 0028, ADR 0031). The reload, instance isolation and teardown gates stay. |
| Counted/declaration loops | Bounded counted loops that increase or decrease, const/let iterable bindings, nested scope | Restore. Reject malformed literal updates and literal updates that never end the loop. |
| HTML literals/bootstrap | Decode text references once. Never read them again as markup or expressions. UTF-8 byte order mark (BOM). Mobile viewport. | Restore. Keep the attributes the tokenizer decoded, without decoding them twice. |
| Native targets | Trusted local WebView application, companions compiled for the native target, deterministic target precedence, dispatch per route | Behaviour is a service in `client/services/`. Native services are activated per browser-side service instance and addressed by instance name (ADR 0030). Restore the accepted local architecture with origin checks and the owned process and snapshot guarantees. |
| Application/browser metadata | Captured manifest and config, validated metadata, icons and window settings, deterministic native and browser output | Restore. Do not let overlays skip validation. |
| Request contracts | CHEX validates parameters, headers and body before the handler runs. An invalid schema or runtime fails before readiness. | Restore with bounded, supervised external validation and immutable schema capture. |
| API documentation | Deterministic api.json and schema viewer show the captured contracts. Dynamic OPTIONS uses the matched route. | Restore together with the request-contract gate. |
| CLI | preview builds and watches Tac+Yon. start runs existing output without a rebuild or hot module replacement (HMR). bundle --native stages one target. | Restore. Keep the compatible legacy commands and structured shutdown. |
| Tests/support/release | Native execution per platform, security, compatibility, signed provenance, acceptance of the installed artifact | Update the gates to match the real architecture. Do not weaken tests into checks of source shape. |

## Explicit compatibility decisions

- Keep `serve`, `dev`, `native-bundle`, legacy `<if>/<else>/<for>` and bare
  iteration bindings as compatibility spellings. The new capabilities do not
  require removing a working spelling. New examples use preview/start,
  bundle --native, `<logic :if>`, `<logic :else-if>`, `<logic else>`, and
  `<loop :for>` with declared loop bindings. The fallback `else` is a bare
  flag. These controls belong inside components; pages compose components.
- `preview --static` still lets you inspect static artifacts without running
  Yon or rebuilding. Plain preview is the development command.
- Keep these released safeguards:
  - readiness checks before the server binds;
  - source snapshots;
  - the supervised command runner.

  Node is the default under ADR 0020. Overrides are still capability-probed.
  Do not restore readiness that only warns, ambient source reads,
  unbounded command capture or detached watchers.
- Keep the released environment variable namespace:
  - `TAC_DIST_PATH` selects the built output.
  - `YON_*` selects serving and handler runtime behavior.

  Do not bring back `YON_DIST_PATH` or `TACHYON_*` variables from the old local
  tree. They were removed.
- Native services replace browser WASM companions on purpose (ADR0019).
  Reject unsupported combinations of service language and target, and give
  migration guidance. Never drop authored behavior without an error.
- ADR 0020 replaces the decorator authoring model of ADR 0017. It keeps the
  bounded process, relay, and streaming hardening of ADR 0017.
  - The recovered native decisions get ADR0018 and ADR0019.
  - The old local streaming ADR is already covered by the current Yon decision.
    It is not a competing new contract.
- Keep compatibility with full HTML documents. New templates should be
  fragments, and metadata should come from captured configuration. Do not bring
  back the older parser's panic on malformed leaf children.
- Persisted fetch caching is opt-in. The rules are:
  - Only eligible same-origin GET responses are cached.
  - Credentials are omitted.
  - Responses marked authorization, private or no-store are not cached.
  - Entry count, response size and age are bounded.
  - Unavailable storage must not crash views.

  The unpublished local implementation cached broadly by default. That default
  is not safe.

## Completion evidence

Local functional qualification is complete. Publication is still pending.
This document is not a release sign-off.

The list below records the first local qualification on 2026-08-31 UTC.
The integration qualification in the next section replaces its open
Linux/Windows checks and its local binary identity.

- The full canonical Rust gate passes. It covers formatting, checks of all
  targets and all features, Clippy with warnings denied, 449 tests across
  16 suites, documentation with warnings denied, and the dependency, license
  and source policy. The dependency check still reports its allowed
  duplicate-version warnings. CLI phases 1-7 and all seven signal-lifecycle
  gates include production shutdown and a 60-second server soak test.
- A fresh coverage run passes its 80% minimums: 86.53% lines, 85.84% functions
  and 86.96% regions. All 442 instrumented tests pass. Seven existing component
  tests are excluded only under `cfg(coverage)`. They run in the canonical suite.
- Seven template regression tests prove:
  - counted and declaration loops;
  - invalid updates;
  - decoding entities only once;
  - BOM handling;
  - mobile bootstrap metadata.
- The real CHEX integration and the compiled-CLI schema probe reject an invalid
  body, headers, parameters and JSON. The server answers dynamic OPTIONS and
  serves the API viewer.
- The browser storage and runtime gates pass. They include counted-loop DOM
  behavior, aggregate bounds on empty loops, forwarding of native arguments,
  and cancelling a delayed mount. Service-worker privacy tests cover cache hits
  and eviction, credentials, authorization, no-store/private/Vary and verified
  static assets.
- Input regression:
  - The first reproduction kept one character (`C`) instead of `Customer`.
  - The fixed runtime keeps the original connected input through typing, edits
    in the middle, backward selection, textarea input, component ancestry and
    custom-element slots.
  - The real website gate covers the home, docs and feature pages at 1440px and
    390px. On touch it uses the supported keyboard shortcut. It includes
    asynchronous search results, spaces, fast typing and Escape.
- A second regression reproduced late native acknowledgements overwriting newer
  typed characters.
  - Each field now has a revision number, so optimistic input is kept.
  - Deterministic browser ordering passes, and so does the real Android app.
  - On Android the test types at 40 ms per character while a Kotlin setter waits
    100 ms. It then confirms the complete native field value, focus and DOM
    identity.
- Rust native-service application binary interface (ABI) execution proves
  isolated state per instance, native OS access and publish callbacks.
- These gates run real native hosts and native services:
  - macOS accessibility and visual;
  - Android 15 emulator;
  - iOS 26.5 / iPhone SE simulator.

  They verify rendered interactions, kept input focus, native OS calls, publish
  callbacks and route isolation. Qualification on Linux/Windows native GUI
  runners is still open. The generated Windows C and Linux Rust compile locally.
- The website build-output and compiled-example suite has 36 tests,
  663 assertions and zero failures. The JavaScript semantic type checks for the
  website and the release probe pass.
- Verification of published snippets:
  - It runs 56 bundles and 53 HTTP method probes. These include all eight server
    languages and five server-sent events (SSE) examples.
  - It also runs 13 syntax checks and three JSON checks.
  - Fragments that cannot run, and missing execution coverage, are listed
    separately. They are never counted as compiled tests.
  - Java/Kotlin/C#/Rust examples keep their helper types in the same Yon source.
    The project does not claim cross-file compilation.
  - Regression tests make sure those helpers do not become Handler endpoints and
    do not change its stream protocol.
- `scripts/release/feature-smoke.mjs` passes against the reconciliation CLI. It
  proves client plans and production startup without recompiling or HMR.
- The exact CHEX v26.32.02 artifact is pinned by digest for all five release
  architectures. Its real validation protocol was probed locally.
- Eight browser gates and all 39 compatibility-ledger claims pass. Workflow
  syntax passes `actionlint -shellcheck=''`. Plain actionlint still reports
  five ShellCheck findings. The base commit shows the same five findings.
- GitHub had no open issues when checked on 2026-08-31 UTC. Existing unrelated
  dependency PRs were not changed.

All final executable, browser and native probes used the same frozen local
development CLI. Its SHA-256 hash is
`0a08ee6bedc01e6e0bbe63938a2f0745400d61b06db9a0055a3331e574dd17fb`.
The hash was the same before and after native qualification.

- Its embedded version is still `26.35.07`. That marks the development build
  with an unchanged version number. It does not replace the immutable published
  release.
- The canonical gate and the coverage gate each built the same source tree on
  their own.
- Detailed local reports are kept under the ignored `target/` directory.
  Published artifacts need new evidence.

## Integration qualification (2026-08-31 UTC)

PR [179](https://github.com/d31ma/Tachyon/pull/179) merged into main at
`8a3016f5079bd45c6e56c85d3f40c235a9b1b0f9`. Its final head
`8a6e86bd11bc7e43065ecd2f0633d57f6180dbe5` passed all 18 jobs in
[the complete CI run](https://github.com/d31ma/Tachyon/actions/runs/33359303631).
The run includes:

- real Linux GTK4 and Windows WebView2 navigation, native calls, stylesheets
  and cleanup;
- macOS/iOS and Android UI;
- website typing;
- Rust gates on all three desktop systems;
- compatibility, coverage, fuzzing and sanitizers.

No review conversations or open issues were left at the merge check.

The final local canonical gate passes 450 tests across 16 suites. No test
failed or was ignored. Formatting, check, Clippy, rustdoc and dependency policy
also pass. Production-source coverage passes the same 80% minimums: 86.51%
lines, 85.85% functions and 86.95% regions. The final frozen development CLI has
SHA-256 `d04a19eed574449cc1671adc8f391254864413f9d7ff63f176fcda34fd9414d9`.
That CLI includes the fixes for Linux content-type and Windows local-response
routing. The later CI fixes change only test harnesses.

- The Linux headless gate keeps WebKit compositing. It uses Mesa software
  rendering and shared-memory buffers.
- Windows cleanup waits for owned WebView processes within bounded deadlines.
- The scheduled-worker snapshot test now waits until a marker write completes,
  not until the file exists. A deliberately delayed write reproduces the
  original race.

The existing behavioral assertions and deadlines are kept. These changes are
not platform exclusions or waived CI gates.

Release `26.36.01` is prepared from this merged main. It needs a new immutable
tag. These steps are still required:

- publication;
- verification of the downloaded artifacts;
- a matching website deployment;
- verification of the installed CLI.

Integration CI alone does not prove those results. The separate
cybersecurity-review waiver below is still explicit. It does not change the
automated release qualification.

## Review disposition and operational limits

- The Codebase Steward review and the focused native and website user
  experience (UX) reviews passed. The final full run found that one standalone
  Swift test did not include the shared JSON helper. Its compile inputs were
  fixed without changing the production binary. The targeted test and the
  complete suite then passed.
- The user waived the separate cybersecurity review for this run. Completed
  fixes and the existing automated regression and CI gates stay in place. This
  waiver is not a cybersecurity sign-off. It is not a permanent change to the
  release CI policy.
- The site reliability engineering (SRE) review found no blocking operational
  regression. This result depends on remote native CI and qualification of the
  downloaded artifacts.
  - Native-call timeouts limit how long the caller waits. They do not limit
    execution or rollback owned by the application.
  - Companions must stay responsive. Hung native work can require the user to
    relaunch the application.
- Some failures passed on a later run without any change:
  - Earlier local CHEX startup and Apple helper timeout failures passed on later
    runs, including the final full run. Their causes are not confirmed.
  - One Android emulator installation failed during boot readiness. It passed
    on a retry with no change.

  These are recorded as observed temporary failures. They are not claimed as
  root-cause fixes.
- The existing problem that the search button is hard to find with a coarse
  pointer (touch) is deferred to a separate UX change. The website design is
  kept. Simulator and emulator evidence does not prove support on physical
  devices.

The release workflow now runs these gates on the downloaded staged
executables:

- schema, native ABI, and template/CLI gates;
- storage and runtime browser gates on the downloaded Linux executable.

These are still required:

- checks on the current-head release PR, including Linux/Windows native GUI;
- review feedback;
- a new immutable version;
- publication evidence.

Before claiming completion, record the release SHA, the artifact digests, the
matching website deployment and the installed CLI.
