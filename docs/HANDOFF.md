# Handoff — Tachyon Rust Rewrite

## Current continuation — 2026-09-26

[Routing recovery](ROUTING_RECOVERY_2026_09_26.md) records the continuation of
Claude's interrupted coordinated routing wave. ADR 0048 replaces Yon folder
routes with RequestRoute controllers and replaces Client/WebCall with
HTTPClient and request method annotations. The website, editor declarations
and current references use this contract. The evidence record distinguishes
completed checks, the native HTTP service qualification fix and remaining gates.
Later NEXT phases are not complete. Ruby runtime artifact qualification is
not yet Ruby language support.

The final Mac canonical gate passes: 1,280 tests, no failures, two explicitly
ignored live SQL/Redis tests; formatting, check, Clippy, rustdoc and dependency
policy also pass. Isolated WSL Linux gates and six model tests pass. Native
Windows ARM64 compilation stops at missing `link.exe`; no native test success
or speed comparison is claimed. See the recovery record for source identities,
website probes, remaining NEXT phases and platform limits.

## Previous continuation — 2026-09-25

[WebAssembly events](WASM_EVENTS_2026_09_25.md) records continuation of
Claude's latest interrupted session after Go support. Rust, C# and Go web
services now join the page event bus for both incoming and self-published
events. The evidence record tracks ordering, async delivery, replay, cleanup
and final verification. The entries below are previous milestones.

## Previous feature — 2026-09-25

[Authored single-class files](SINGLE_CLASS_LAYOUT_2026_09_25.md) records the
ADR 0044 implementation and its verification status. An empty or
comment-only route marker is allowed only when a controller for that route
exists; without one it fails with TY2015. Root middleware is a `Middleware`
class with `before(request)` and `after(request)`. The Gateway review below
is the completed preceding milestone.

## Previous continuation — 2026-09-25

See [Gateway review and recovery](GATEWAY_REVIEW_2026_09_25.md) for the prior milestone. The
newer Claude session implemented Gateway, relay hardening and models across
the service languages. Its independent review was interrupted. This
continuation repairs confirmed boundary defects and records fresh evidence.
ADR 0044, including plain handlers and middleware, was selected next and is
covered by the current feature above. The earlier note saying Gateway is
unimplemented is historical.

## Latest recovery — 2026-09-24

Read [Simplification recovery](SIMPLIFICATION_RECOVERY.md) first. It records
the latest Claude session, repaired interrupted edits, completed R5/R6/X3
cleanup, fresh verification, and the separate accepted Gateway feature.
The sections below are historical and may describe superseded APIs.

## Realms mirror each other — added 2026-09-15

ADR 0031 and ADR 0032 change every record below that describes Yon role
folders per route or the `EventPublisher` annotation.

What changed:

- **Yon role folders are shared per realm.** `server/services`,
  `server/repositories`, `server/clients`, `server/configurations`,
  `server/entities` and `server/delegates` are used by every route. They mirror
  `client/<role>` exactly.
  - A route directory holds only its `yon.<ext>` entry and its `controllers/`
    folder.
  - A role file beside a route fails with `TY1310`, "@X type 'Y' sits outside
    its realm's shared role folder". The message names the shared destination.
  - `ty migrate` reports it as `role.route` Changed, with the same destination.
- **Markers are mirrored.** Only `Controller` and `Stream` are Yon-only now.
  - Tac gained `@Component()`, bare `@Repository()` and `@Inject(A, B)`
    (provider types bound to lower-camel fields).
  - Yon gained `@Resource()` over `dataSource`, `redis`,
    `applicationEventPublisher` and, in JS/TS only, `signal`. The field must
    have exactly the same name as the resource.
  - The reserved Yon provider tokens are `DataSource`, `Redis` and
    `ApplicationEventPublisher`.
- **Events follow Spring.** `@EventPublisher` is removed.
  - A class declares an `applicationEventPublisher` field without an initial
    value. In Python this is
    `application_event_publisher: ApplicationEventPublisher`. Typed languages
    use a field of that type, which `RequiredArgsConstructor` infers like
    `dataSource`.
  - The class calls `publishEvent(event, value)` or
    `publishEvent(event, value, retain)`.
  - `@EventListener` takes `event` or `events`, `condition`, `id` and `replay`.
  - `@Order(n)` orders listeners, lowest first. Ties keep declaration order.
  - `@Async()` delays delivery until after the current pass.
  - A listener's return value is published again when it is `{event, value}`
    (with optional `retain`), or an array of at most 32 of them. This never
    happens for `Async`.
- **`yon.datasource.driver` accepts `fylo`** (ADR 0032):
  - the root is given as `fylo:///absolute/root`;
  - one `fylo` subprocess, owned by the server, speaks the bounded NDJSON
    machine protocol;
  - CrudRepository commands map onto `putData`/`patchDoc`/`delDoc`/`findDocs`;
  - raw `dataSource.query` goes through FYLO's SQL subset.

Traps:

- **Shared folders versus routed controllers.** Only `controllers/` stays
  beside a route. Moving a controller into `server/controllers` fails, in the
  same way as leaving a service in `server/routes/<route>/services`. A shared
  service is one class. Each invocation still owns its own graph of instances,
  so "shared" does not mean state shared across requests.
- **`EventPublisher` is gone, not deprecated.** Any remaining use fails with
  "EventPublisher is removed (ADR 0031)". A class that only injects the
  publisher still gets an event scope. So a file with no marker at all can now
  carry event metadata.
- **Condition semantics are exact** and must stay the same in every runtime:
  - Paths walk objects by key and arrays by integer index.
  - A missing path is absent.
  - `eq`/`ne` are strict: same JSON type and same value.
  - Ordering comparisons need two numbers or two strings. Otherwise they are
    false.
  - Any evaluation failure simply means no match.
  - Conditions are compiled once into a JSON matcher. No runtime may add an
    expression evaluator.
- **Order and Async have language limits.**
  - `@Order(n)` takes an integer literal. Listeners without an order sit at 0,
    in declaration order.
  - `@Async()` is a compile error in Yon php, java, kt, cs and rs: "Async
    listeners require a Yon language with asynchronous methods (js, ts, py)".
  - An async listener never publishes its return value again.
- **FYLO assigns identifiers.**
  - A `save` without an identifier returns the entity with its TTID.
  - A `save` that names an identifier FYLO does not know fails.
  - FYLO rejects a root path that is not canonical. So the driver makes the
    configured root canonical once, and startup fails when the root does not
    exist.
  - Ordering and paging happen over the bounded set of matches, at most 10,000
    documents.

Verified by the lead so far:

- The shared-folder rule and its `TY1310`, with a JavaScript route served.
- The injected publisher with `Order`, `condition`, `events`, publishing
  returned events again, and `Async`: in the Tac browser, in Yon JavaScript
  and in Yon Python.
- The FYLO datasource unit test, and a Yon route that runs save, patch, find,
  count, exists, `in` predicates, raw SQL and delete over FYLO.
- `ty doctor` reports the `fylo` requirement when the driver is configured.

**Pending**, with other agents working on them now:

- the five compiled Yon languages (php, java, kt, cs, rs);
- native Tac services;
- the full gate and website suites.

Their sources are changing under this ADR while this is written. The event
runtimes of the compiled languages already take the compiled listener options.
Native lowering already emits entries with several events, conditions, order
and async, while the native preludes still declare the single-event listener.
So none of it is qualified. Do not claim support for them until their own
evidence is recorded here.

### Qualification, 2026-09-16

Everything below ran on this Mac against the pinned toolchains.

- `cargo fmt --all --check` is clean. `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` is clean. tachyon-core, tachyon-cli and
  tachyon-contracts all pass.
- All eight Yon languages pass the local event contract, including the five
  compiled ones. The contract covers: an injected `ApplicationEventPublisher`,
  `events` lists, compiled conditions, `Order`, publishing returned events
  again, and `Async` refused where the language has no asynchronous method.
- These browser and native gates pass:
  - tac-client (component and repository providers bound with Inject);
  - behavior, events, tac-events, storage, service-worker, runtime,
    hot-update, phase 3, expressions, application events;
  - schema contract, feature smoke;
  - the Rust and Swift native event gates, the native ABI gate, the Apple
    protocol gate;
  - the compatibility ledger (61 claims) and standalone Rust.
- Website: 46 tests, the snippet verifier (70 bundles, 89 HTTP probes) and the
  browser input gate pass. The type scale is one step smaller: body text is
  14px and the hero is 72px, checked on desktop and mobile.

Qualification found four implementation gaps. All are fixed:

- The marker scanner dropped a bare Tac `@Repository()`, so it could never be
  injected. The check now tells Yon's stereotype without parentheses apart
  from the Tac role. Yon's own bare `@Repository()` and `@Component` are
  filtered out of the Tac pass instead.
- `@Resource` on a field was parsed only in JavaScript and TypeScript.
  Compiled languages now find the field's owner from the enclosing
  declaration, which a Rust struct needs.
- Java's condition matcher did not compile: a wildcard `Map` cannot take a
  typed default from `getOrDefault`.
- One method entry was recorded twice when `Order` or `Async` named a listener
  before the listener's own annotation.

### Native gates, 2026-09-16

Three of the five native gates ran here on real devices, and they pass:

- **iOS** on a booted iOS 26.5 simulator: WKWebView render, input and focus,
  the Swift ABI, OS access, publication, dynamic routes and route isolation.
- **Android** on the emulator from the ANNEX SDK: WebView and Kotlin evidence,
  transport failure paths, sixteen Java JSON-budget cases, and the frame,
  origin, depth, byte and navigation boundary probes.
- **Linux** in the pinned Debian container: both routes rendered through
  WebKitGTK, native Rust calls, shared assets, route isolation, and the
  publication arriving in the page.

Running them found three defects. All are fixed:

- `scripts/native/desktop-fixture.py` and both mobile gates still used the
  removed `@EventPublisher`. Each now declares a publisher member and calls
  `publishEvent`.
- The iOS gate took its native identity from every `init` it saw, including
  its own adversarial probes, which leave out a service on purpose. So it
  overwrote the real identity with `undefined`. It now takes only the
  runtime's first activation.
- The pinned Linux image had no OpenSSL headers, so building the compiler
  inside it failed before the gate could start. `libssl-dev` is now in the
  Dockerfile that CI builds from.

**macOS** needs Accessibility permission for the process that launches the
gate. Without it, the accessibility probe refuses and the gate cannot run.

**Windows.** The gate host here is ARM64 Windows with only the x64 MSVC tools
installed, so nothing links for `aarch64-pc-windows-msvc`. Building everything
as x64 under emulation gets further:

- the WebView2 host, the NativeAOT C# companion and the Rust companion all
  compile;
- the application launches and records `companion.loaded`;
- then the WebView2 controller refuses with `bridge.unavailable`, and no
  top-level window appears, even though the Evergreen runtime is installed.

An emulated x64 process does not get a WebView2 environment on that machine.
The fix is the "MSVC v143 ARM64 build tools" component in the Visual Studio
Installer. Then the host builds natively and uses the ARM64 runtime that is
already installed.

Two more notes for whoever sets up that machine:

- `Enable-Msvc` in `scripts/windows/native-test.ps1` hard-codes
  `-arch=x64 -host_arch=x64`. It would link wrongly on an ARM64 runner even
  with the right tools.
- An old machine-level `TAC_WEBVIEW2_SDK` there points at a directory without
  `WebView2.h`. This stops the gate from downloading its own pinned SDK.

Still not run here: the legacy differential, which needs the frozen legacy
binary.

### Annotation reference and two codegen fixes — added 2026-09-16

The website documents every annotation in every language that accepts it. A
new `annotations` docs topic has 447 example files in eighteen sections. They
are grouped by family: roles, injection, data access, caching, outbound HTTP,
configuration binding, validation, lifecycle and events, and streaming. Each
family appears once per realm, across eight Yon languages and six Tac
languages. Every example is a complete project that the published-snippet gate
builds or serves.

Before this, the audit found 358 missing pairs of annotation and language.
Seven annotations had no example anywhere: Cache, Cacheable, CachePut,
CacheEvict, Inject, Stream and Scope.

The combinations that are still missing are refusals. A final section names
them with the reason:

- A native Tac service is a plain type. It rejects Service, Component, Inject,
  Scope and Resource with TY1308. It rejects Client with the browser-only
  message of ADR 0027. It reaches the network through HttpExchange and a
  configured adapter.
- Async needs a language with asynchronous methods, so five Yon languages
  reject it.
- Stream needs generator syntax, which Java and Rust do not have.

Writing those examples found two native code generation defects. Both are
fixed in `crates/tachyon-core/src/companion.rs`:

- A Rust struct that wrote several fields on one line lost all fields except
  the first. It also inserted the rest of the text into the remaining field's
  setter cast, so the generated companion did not compile. Field declarations
  are now split on commas at depth zero, as well as on newlines.
- A Swift service with no members emitted `return []`, an empty *array*, where
  the declared return type is a dictionary. It now emits `[:]`.

In both cases, `ty bundle --native` reported success, because that command
only stages native sources and does not compile them. Remember this staging
boundary: a native gate or a real Xcode build is what catches this kind of
defect.

## Owned local events and lifecycle — 2026-09-15

ADR 0028 and APPLICATION_EVENTS.md replace the local-event proposal below.
ADR 0031 replaces the publisher annotation named in this section.

- The public `this.tac` object, the `TacRuntime` type and the automatic
  mount/hydrate hooks are removed.
- Browser JS/TS uses an injected `applicationEventPublisher` and
  EventListener, PostConstruct, PreDestroy and Resource(element, signal,
  renderScheduler, cacheManager, resourceLoader).
- Owners in the managed provider graph take part. Arbitrary instances created
  by hand do not.
- Client/HttpExchange stays the HTTP surface.

**Yon events** run inside one cached invocation graph. Tachyon adds no bus
across requests, no remote bus and no background bus.

- JS/TS/Python async methods are supported. Compiled languages and PHP use
  synchronous methods.
- Streaming containers and transient event providers fail closed (they are
  refused).
- Four native adapters use synchronous hooks and events for the page only,
  new activation IDs, rejection of stale calls, and queued publication when a
  publish happens during another publish (reentrant publication).
- Native host navigation and shutdown dispose owned graphs through an internal
  operation. Bridges that face the browser reject that operation.

**Evidence so far:**

- All eight real Yon language invocations and an adversarial JS case pass
  (9/9, strict gate with no skips).
- All four generated native language runtime gates pass.
- These pass: six browser gates, the standalone event-runtime and
  compatibility smoke tests, native Rust ABI, Android JSON boundary tests and
  cross-platform host generator tests.
- These are not full emulator qualification, and not UI qualification on five
  operating systems.

**Final checks:**

- Formatting, the locked all-target/all-feature workspace check, workspace
  Clippy and rustdoc with warnings denied, and the dependency policy pass.
- Core unit tests pass when run one at a time (serially): 439 passed, 2
  external storage contracts explicitly ignored.
- The parallel core run had four failures that depend on deadlines. Exact
  serial retests and the full serial core run passed, without raising any
  limits.
- Website tests pass 45/45 (1,091 assertions). Both TypeScript decorator modes
  pass. The real desktop/mobile website input gate passes.
- The exact published event examples pass all eight Yon HTTP invocations plus
  four native staging checks.
- The compiler classification ledger passes 39/39.
- The older Phase 3 browser gate and semantic HMR (hot module replacement)
  gate pass after their fixtures moved to owned hooks.

Those gates found two problems, and the fixes are verified: attribute
reconciliation on a stable root, and initialization that fails for good. Now
PostConstruct attributes survive new renders, and failed owners stay inactive.
They are not initialized twice and not destroyed twice.

The full workspace run reached an obsolete Phase 4 Swift probe that used the
old ABI without owners. The migrated exact test now passes, with assertions for
fresh and stale owners and route isolation. The run did not reach later
workspace targets, so a clean full-workspace test result is not claimed.

The earlier iOS SDK/Xcode-license limits on qualification have not changed.

Graphify refreshed 48 focused files to 8,915 nodes, 20,655 edges and 618
communities. Semantic token usage is unknown. HTML output was left out,
because the graph is above the 5,000-node limit. Its retrieval benchmark is an
estimate. It does not measure billing or runtime performance.

**New user feedback.** Spring Data repositories usually expose generic
CrudRepository<Entity, Id>/JpaRepository<Entity, Id>, not raw DataSource
fields.

- Yon DataSource injection today is lower-level infrastructure. It is not yet
  that generic CRUD contract.
- Do not simply change the examples to generics that are not supported.
- Recommendation: a separate slice for a generic repository compiler and
  runtime, with automatic datasource binding, inherited CRUD, entity mapping,
  derived methods and Query. Keep direct DataSource access as an advanced way
  out.
- Tac must still not receive server database credentials or connections.
- The user explicitly chose the Tachyon spelling CRUDRepository<Entity, ID>,
  not CrudRepository. Use CrudRepository only when you refer to Spring's
  actual API.

## Scoped discovery, managed storage and declarative clients — 2026-09-15

ADR 0027 changes the explicit-manifest and manually initialized resource
boundaries that the older record below describes.

- Tac/Yon entry points now find annotated files in the same language in role
  folders, from captured source. Since ADR 0031, these folders are shared per
  realm.
- Legacy module manifests fail with migration guidance.
- Yon startup resolves configured datasource/Redis resources.
  RequiredArgsConstructor injects the reserved DataSource/Redis providers into
  bare repositories, without user constructors or Inject lists.
- Credentials stay in the private root `.env` snapshot. Tac stays client-only.
- MANAGED_STORAGE.md lists the supported dialects, TLS, bounded operation
  semantics and limits.

**Client** is a Tachyon role, not a Spring annotation. HttpExchange describes
the request. Client supplies layout, discovery and injection.

- The user asked that this replace the public `this.tac.fetch` API.
- Browser JS/TS construction is automatic.
- Native HttpExchange adapters stay manually configured, because the user
  chose this explicitly.
- The public fetch migration and browser qualification are complete.
- The bounded anonymous JSON cache policies stay available through Client.

**Local events.** The user also asked for a replacement of local pub/sub in
the style of EventPublisher/EventListener. The user explicitly chose local
application events only.

- LOCAL_EVENTS_PROPOSAL.md records the contract. No executable event markers
  exist yet.
- Keep the existing Yon topic/SSE network subsystem.
- Implement all language and lifecycle slices before removing the old local
  APIs.

Current implementation evidence (this is not a release claim):

- **Managed repositories** passed in all eight Yon languages: inferred
  repository/resource injection, bound SQLite values, and server-owned storage
  shared across separate handler processes (8/8 tests).
- **Compiled resource transport tests** passed 88 assertions across Java,
  Kotlin, C# and PHP. They include malformed framing, redirects,
  authentication, body limits and deadlines. Discovery and layout integration
  tests passed.
- **Browser Client:**
  - Compiler tests passed for JS/TS and for explicit native rejection. Real
    HTTP transport tests passed.
  - All three browser gates passed against the latest CLI: declarative Clients
    and owner disposal, IndexedDB/cache policies, and service-worker
    offline/privacy behavior.
  - Both TypeScript decorator modes passed, including the check that public
    `this.tac.fetch` is gone.
- **Real containers:** contracts passed for PostgreSQL 18, MySQL 8.4 and
  MariaDB 11.8 with verified TLS and a private fixture CA, and for Redis 8. SQL
  Server has not run on this ARM host. This matrix does not qualify its
  support level.
- **Storage:** all seven storage tests passed. They include interrupting an
  unbounded SQLite query at the existing five-second limit, and successful
  queries after that. The durable container harness is
  scripts/test-managed-storage.mjs. Its disposable containers and fixture
  certificates were cleaned up.
- **Website:** tests passed 44/44 with 1,014 assertions. They include
  compiling and importing complete JS/TS multi-file Client recipes. The owned
  session5901 serves the rebuilt website at 127.0.0.1:3000. The storage page
  was checked after reload: all eight Yon language choices and the new Client
  examples appear.
- **Discovery:** tests passed 21/21.
  - The earlier broad core run had 424 passes, four diagnostic regressions and
    two ignored tests.
  - All four regressions were fixed and passed exact retests. New regression
    tests for unsupported entries, type imports and reserved resource names
    also passed.
  - This is not a later clean run of every core test together.
- **Dependency policy:** the advisory/license/source policy passed, with
  scoped, reviewed license exceptions: foldhash Zlib, xxhash-rust BSL-1.0 and
  webpki-roots CDLA-Permissive-2.0. NOTICE includes the distributed license
  texts.
- **Workspace:** formatting, the locked all-target/all-feature workspace check,
  Clippy with warnings denied and rustdoc passed with the new dependencies.
  - The full workspace test run stopped at two Phase 5 iOS packaging tests,
    because the selected Command Line Tools do not have the iphonesimulator
    SDK. That command did not run all later targets.
  - Accepting the Xcode license is still a user action.
  - Neither the iOS packaging gate nor SQL Server qualification is claimed as
    passing.
- **Graphify** was refreshed for scoped discovery, managed storage, the Client
  migration and the local-events proposal, which is explicitly not
  implemented. The earlier sections below are historical evidence. They do not
  replace the current status in this section.

## JSON returns and separated configuration — 2026-09-15

**JSON returns.** ADR 0026 implements normal JSON-compatible Yon return values
in all eight languages.

- Objects/maps, arrays/lists, scalars and null become one JSON response.
- Strings that look like JSON stay strings.
- Explicit HTTP status, headers, raw HTML and relay answers use each language's
  own response type (ADR 0062 removed YonResponse).
- Each language's standard JSON API is preferred. Java/Kotlin and standalone
  Rust have bounded strict codecs.
- Rust parsing and stringification can fail and return an error.
- No SSR (server-side rendering) and no implicit HTML rendering were added.
  Ordinary arrays do not select streaming.

**Application Configuration v2** accepts only nested `tac`/`yon` namespaces.

- Each underscore in a prefixed application name starts another path segment.
  Value uses canonical dotted paths.
- Configuration that is not secret lives in `application.json` and optional
  Yon profiles.
- Only the project-root `.env` supplies private `YON_` secrets. Overrides from
  the parent application environment and JSON placeholders are rejected.
- These fail closed without showing values: recognized credential fields and
  URLs, duplicates, collisions, unsafe names, inputs that are too large, and
  unsafe filesystem inputs.
- Secrets with arbitrary, unusual names still depend on developers classifying
  their inputs correctly.
- Environment variables for runtime and isolation policy stay separate from
  application configuration.

**Snapshots.**

- Discovery captures the public base `tac` namespace without opening `.env` or
  Yon profiles.
- Bundles publish only that snapshot at `/.tachyon/application.json`,
  including staged native output.
- Yon captures a separate, private, unchangeable snapshot at startup. Changes
  to secrets or configuration need a restart.
- Binding is explicit in every language. It is not automatic datasource setup
  and not a Tac container.
- Role annotations already validate suffixes, exact file names and plural
  folders before build and execution (also for interpreted languages). TY1310
  does not depend on running the application.
- Manifests stay explicit inputs for selecting and assembling files. They are
  not validators and not hidden provider registries.

**Examples.** Website and Markdown examples were migrated, including all eight
Yon language alternatives.

- QA fixed Rust map-key inference, Java nullable JSON echo, Kotlin typed
  configuration keys, and a PHP overview that still wrapped plain data.
- Strict verification keeps checking other languages after a required tool
  probe is missing, but it still fails the gate. No deadline was raised.
- Scaffolds now contain `application.json` and a secret-only `.env.example`
  instead of `.env.test`.
- Compatibility scripts explicitly normalize this intentional difference from
  the oracle. The whole legacy oracle matrix was not run again.

Verification evidence for this change:

- **JSON runtime:** all eight direct-JSON runtime tests passed, with 48
  supervised invocations, coverage of malformed input, and explicit HTTP
  metadata. The Kotlin/C# generator streaming regression test also passed.
- **Core and contracts:** the final core library run passed 407/407. Contract
  library and repository policy: 7 + 12 passed. Four public annotation-layout
  tests passed, including failure before runtime selection. The public Tac
  snapshot/artifact regression test passed.
- **Configuration:** configuration units 16/16, scaffolds 4/4, selected
  configuration integrations 9/9 and CLI configuration 2/2 passed. Separate
  binding/JSON tests in all languages also cover typed runtime behavior. Do
  not suggest that all 14 configuration integrations were run again in one
  final command.
- **Behavior:** behavior units 57/57 have passing evidence across runs. These
  passed: JS runtime25 plus extensions9, the JS/TS/Python/PHP/Rust
  integrations, and the Java/Kotlin/C#/Swift standalone binding checks. Both
  TypeScript decorator modes passed.
- **Website:** tests 42/42, 856 assertions. All eight configuration tabs were
  checked in the browser, with no warnings or errors. The existing owned
  session90159 serves the updated examples at 127.0.0.1:3000.
- **Workspace checks:** formatting, the locked workspace check, Clippy and
  rustdoc with warnings denied, and cargo-deny passed. The existing duplicate
  dependency warnings remain.
- **Full workspace test run:** it stopped at two iOS CLI tests, because the
  selected Command Line Tools do not have iphonesimulator. macOS CLI tests
  passed. The workspace targets after that stop were not all run again. No
  clean full canonical test gate is claimed.
- **First strict snippet runs:**
  - They first selected the system Node23.5, which cannot pass the authored
    TypeScript capability probe. Bun and the newly installed Node24 pass those
    examples.
  - After the Kotlin example fix, the configuration run with the ANNEX
    toolchain passed the other seven Yon languages, the complete TypeScript
    worked example (8 HTTP probes), 2 bundles and 2 JSON checks.
  - C# failed its bounded readiness probe, although the SDK was installed and
    earlier C# runtime and example runs passed.
  - This strict gate is still not passing. No compiler, readiness or request
    deadline and no assertion was weakened.
- **Later strict-layout run** with ANNEX Node/Kotlin:
  - It passed without interruption: 6 bundles (4 of them only native source
    staging), 9 HTTP probes (all eight Yon languages plus the complete
    TypeScript example), 2 JSON checks and no failures.
  - A separate retry of only the configuration run still hit Kotlin's
    readiness deadline, although the run before it passed.
  - Installing the missing tools did not remove all local failures that depend
    on timing. Do not report the strict configuration gate as passing.

ANNEX tool follow-up, which the user explicitly asked for:

- Installed Node24.21.0 LTS (macOS arm64) and the pinned Kotlin JVM
  compiler2.1.10 from official distributions. Both match the published SHA-256
  checksums.
- Added an opt-in `tachyon-env.sh` in the external toolchain folder. It sources the
  existing ANNEX settings and selects the new Node/Kotlin plus the existing
  ANNEX JDK17.
- Found the existing .NET9.0.317, Android SDK and Swift toolchains. Rust1.97.1
  stays managed by rustup.
- The installation record is kept in the external toolchain folder, outside
  the repository.
- Xcode27 and the iPhoneSimulator27.0 SDK already exist in
  `/Applications/Xcode.app`. The user explicitly chose to keep that
  installation. Apple's license is not accepted, and this blocks `xcrun`. The
  license must be reviewed and accepted before iOS checks.
- Nobody accepted the license, changed the global `xcode-select` or edited a
  shell startup file. macOS-only checks set
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for that command only.

No commit, push, release or secret file was made. Keep the large dirty
worktree that already existed. Graphify gets a scoped refresh for this
feature. The older handoff sections below keep their historical evidence.

## RequiredArgsConstructor dependency initialization — 2026-09-12

ADR 0025 makes `RequiredArgsConstructor()` enough for constructor generation
and dependency selection on managed Yon roles, in all eight languages.

- Direct fields without an initial value and not static are selected, in
  declaration order.
- An explicit quoted list of fields can select them or change their order.
- Typed fields resolve concrete providers. Rust uses `Arc<Provider>`.
  JavaScript uses exact lowerCamel names that handle acronyms
  (MoneyService -> moneyService, HTTPClient -> httpClient).
- Missing or ambiguous providers and unsupported types fail discovery.
- Fields with default values and static state are kept as they are.
- Explicit Inject still works but is not required.
- These keep their existing boundaries: Tac, unmanaged behavior construction,
  configuration/Value binding, scope lifetimes and explicit module manifests.

**Implementation.** The pre-pass that reads only source is in
`yon/required_args.rs` and `required_args/fields.rs`.

- Convention validation, stream discovery and executable Yon lowering all use
  it.
- The existing behavior frontend generates constructors.
- The existing provider graph validates scopes, cycles and limits.
- Kotlin secondary constructors, and Rust constructors spread across several
  inherent impl blocks, are counted correctly.
- No runtime dependency, global container, reflection or folder scanning was
  added.

Verification for this change:

- 34/34 focused Yon unit tests pass, including 11 inference regression tests.
- **All eight Yon languages** have passing real-runtime evidence for automatic
  and explicit selection, reversed field order, shared and transient
  identities, and two invocations per variant. Seven passed in one run. Kotlin
  hit the existing compiler deadline, then passed an unchanged focused retry
  (106.68 seconds). So this is not an uninterrupted test run of all eight
  languages.
- Four public annotation-layout tests pass. They include inferred
  dependencies, exclusion of static and default fields, unchangeable
  execution, and rejection of invalid graphs.
- **All 408 core unit tests** have passing evidence across batches. The broad
  run passed 404 and failed four process/deadline checks in CHEX/isolation.
  Exact retries of the unchanged binary passed all five CHEX tests and the
  isolation test.
- **The full workspace run** stopped at a CLI application-configuration
  readiness timeout. Its unchanged focused retry passed. The remaining
  workspace targets were not all run again after that stop. Do not claim a
  clean canonical test gate.
- Contract tests pass: 7 library checks + 12 repository-policy checks,
  including ADR25. Formatting, the locked all-target/all-feature check, Clippy
  with warnings denied, rustdoc with warnings denied, the dependency audit and
  both TypeScript decorator modes pass. The audit keeps the existing
  duplicate-crate warnings.
- **Website:** tests pass 41/41 with 843 assertions. All eight Yon language
  tabs were clicked in the browser. They show matching examples without
  constructors. No browser warnings or errors appeared. The owned terminal
  session 6922 serves the updated site at 127.0.0.1:3000. The earlier session
  7356 was stopped cleanly.
- **Strict website snippet verification** with Node:
  - The final run passes six Tac bundles (four of them only native source
    staging), eight HTTP method probes (seven Yon languages plus the Markdown
    configuration example) and two JSON checks.
  - C# fails its unchanged startup probe with TY2101 dotnet not usable.
  - An earlier Bun run also failed Kotlin, C# and Rust. The final run passes
    Kotlin and Rust.
  - The strict snippet gate is still not passing, even though there is runtime
    evidence for all eight languages.

Nobody changed production readiness or compiler deadlines, test assertions,
host security settings or unrelated host processes. No commit, push or release
was made. Graphify gets a scoped refresh for this feature and its evidence.
The historical qualification sections below describe their own earlier runs.

## Strict role modules and polyglot website examples — 2026-09-12

ADR 0024 adds three required rules for Controller, Service, Repository,
Client, Component, Configuration and Entity:

- a primary-role suffix;
- a file name with exactly matching case;
- a plural folder directly above the file.

Modifiers add no extra role layout. Rust impl targets and both the `yon::` and
`tac::` annotation namespaces are validated. TY1310 reports an invalid layout.
This is a Tachyon convention, not a Spring requirement.

**Module manifests.** Sibling files `yon.modules.json`/`tac.modules.json`
select ordered modules in the same language, with paths relative to the
project, from the unchangeable captured snapshot.

- Nothing else is added: no global registry, package scanning, recursive
  imports, SSR, or new language or target support.
- The selected sources and their metadata are part of the generated output
  and of the cache inputs.
- The annotation-modules/v1 schema is registered with valid and invalid
  fixtures.

**Website guide.** The website's `/docs/strict-annotation-layout` guide has
eight complete Yon alternatives and six Tac alternatives, with role files,
entry points and manifests.

- All 14 tabs were tested in the browser, and they show the matching source
  files.
- The root native Tac examples stage for macOS, Android, Windows and Linux. The
  generated Swift host and companion also pass type checking.
- The existing guides with inline roles, and the behavior and Yon integration
  fixtures, were migrated without weakening their semantic assertions.

**Snippet run.** The final strict-layout snippet run passes six bundles, seven
HTTP probes (six Yon languages plus the Markdown configuration example) and
two JSON checks.

- Kotlin and C# fail the unchanged 30-second readiness deadline. The run
  before that passed C# and failed only Kotlin.
- All eight languages pass the separate supervised behavior integrations. Do
  not describe the strict website snippet gate as passing.
- The final website tests pass 40/40 with 794 assertions. Both TypeScript
  decorator modes pass.
- The rebuilt CLI serves the updated site at 127.0.0.1:3000.

**Workspace run.**

- The workspace run passed 557 tests. Then one process-cleanup test failed,
  because a descendant PID file was missing. Its exact unchanged retry passed.
- The remaining 18 tests passed in a separate batch.
- So there is passing evidence for all 576 tests across 27 targets, but it is
  not a clean uninterrupted workspace run.
- No production deadline or cleanup assertion was weakened.
- The fixture for the unchangeable annotation snapshot now turns off watching
  explicitly and passes in 0.44 seconds. Its original run with the watcher on
  also passed, but spent 134 seconds in startup.
- All 23 dedicated dispatch tests passed.
- A local sample showed that the long watcher delay was in macOS
  FSEventStreamStart registration, not in source assembly.

Final formatting, Clippy with warnings denied and rustdoc pass. All cargo-deny
checks pass, with the existing duplicate-version warnings. The scoped Graphify
update keeps the existing history and records that actual semantic usage is
not available. No commit, release, timeout increase or host security change
was made.

## Resumed verification after ANNEX reconnection — 2026-09-12

ANNEX mounted again at 13:34 EDT with the same volume UUID. Its toolchain
configuration and build directory are available again. macOS logs show that
the earlier incident was USB timeouts followed by a failed recovery of the
storage driver. It was not a permission change in the source tree. No disk
repair was done, and no host security setting was changed.

The complete canonical gate and the strict website snippet suite were started
again against the pinned toolchain and a freshly built
`<cargo-target>/debug/ty`.

**Scheduler-sensitive JavaScript test.** The resumed run reproduced a
JavaScript regression test that depended on the scheduler.

- The test assumed that authorization and lookup delays of 15 ms would fall on
  both sides of a 25 ms budget. But load on the host could expire the
  operation before lookup started.
- The test now uses a controlled monotonic clock that is local to the runtime.
  It proves expiry separately in authorization, lookup and storage. It keeps
  the shared signal/deadline checks and the existing cancellation tests with
  real timers.
- Production code and deadlines did not change.
- The complete JavaScript behavior gate passed five runs in a row (each with 25
  runtime checks plus nine extension checks). The repository type check passed
  in both decorator modes. The real-HTTP schema smoke test passed.

**First strict full snippet run.** It failed 12 startup/request deadlines,
although 58 bundles, 42 HTTP probes, 13 syntax checks and three JSON checks
passed. It also reported 98 descriptive or native-fragment examples that it
did not run.

- Website tests with a fresh CLI passed 38/38 with 699 assertions.
- The Rust run with the old binary passed all CLI suites and repository
  contracts, then reproduced the C# behavior failure. It was stopped once the
  focused fix was ready. Only its verified owned process groups were ended, and
  their exit was checked.

**C# compiler-server stall.** An instrumented cold C# fixture found a real
stall in the persistent compiler server.

- A normal `dotnet build` hit the existing 60-second compiler deadline. The
  same sources compiled in 12.4 seconds with `--disable-build-servers`.
- The framework's C# handler build and the standalone C# behavior gate now
  turn off build-server reuse. The existing readiness probe already did this.
- A compiler spy at the CLI boundary rejects the old command and requires
  owned compilation before artifacts are published. The old binary was seen
  failing this regression test. The rebuilt CLI passes it.
- The standalone C# behavior gate passes after the fix.
- No existing compiler server was killed, no timeout was raised, and the
  Tachyon artifact cache stays on.
- A second review found no problem in this scoped change.

**HTTP dispatch delay.** A later HTTP dispatch delay was sampled on macOS.

- The blocked thread was inside `SourceWatcher::start` /
  `FSEventStreamStart` registration, before the request started. It was not
  inside stream delivery.
- Ten dispatch fixtures that only test HTTP now turn off watching explicitly.
  Three fixtures that test the watcher or lifecycle on purpose keep it on.
- Website HTTP examples also use `serve --no-watch`. This keeps project
  discovery, compilation, readiness and real HTTP dispatch.
- The checked test environment leaves `NODE_ENV=production` and
  `YON_SKIP_BUNDLE` unset, so the initial build stays on.
- Production watcher behavior did not change.

**Final website and language results.**

- Final website tests pass 38/38 with 699 assertions. Type checking passes in
  both decorator modes.
- The final strict language retry after watcher isolation still exits 1:
  three bundles and six HTTP probes pass, but C# reports TY2101 during SDK
  readiness, and Kotlin goes over the unchanged 30-second readiness deadline.
- Five native/style fragments are explicitly outside what that verifier runs.
- The full all-suite was not run again after these failures. Do not claim a
  passing website snippet gate.
- Heavy, unrelated VS Code JVM processes were not touched.
- SDK and runtime versions match. The redacted diagnostic did not show the
  exact cause of the remaining C# readiness failure.

**Rust verification.** After the change to dispatch-only fixtures, Rust
verification finished in explicit batches, not in one uninterrupted canonical
run.

- The first final-product run passed every CLI suite, the repository
  contracts, all 376 core unit tests, 14 configuration integrations and all 11
  behavior integrations.
- Its old dispatch binary was stopped while watcher registration delayed it.
  Only its verified owned process group was ended, and its exit was checked.
- The updated dispatch binary passed all 23 tests, including the intentional
  watcher tests.
- All remaining handler, annotation and diagnostic targets passed as well.
- A check against Cargo's list of targets confirms 26 host test targets, 554
  passed, zero failed and zero ignored across the two batches.
- Final formatting, the locked all-target/all-feature workspace check, Clippy
  with warnings denied, rustdoc with warnings denied and all cargo-deny checks
  passed. The existing duplicate-version warnings remain.
- This does not make the separate strict website failures pass, and it does
  not establish cross-platform release qualification.
- The local logs are `/tmp/tachyon-resume-gates.ONdcQB/canonical.log` and
  `remaining-final.log`. `dispatch-sample.txt` contains the watcher diagnosis.
  These are local diagnostic evidence, not portable release artifacts.

## Review of the Claude `Value` continuation — 2026-09-12

This review looked at the September 12 continuation in the local Claude
project session.

**What it implemented.** The contract is `Behaviors.values(Type,
application)`: an explicit JSON/map factory. It is not automatic injection
into instance fields.

- `Bean` is still not implemented. Do not describe the decorator family that
  was requested earlier as fully delivered.
- The existing Configuration/ConfigurationProperties/Profile/
  ConditionalOnProperty behavior has not changed.

**What this review fixed.** It reproduced and fixed these problems:

- a compiler panic on a missing path;
- `type` options that were not strings being set to a default without any
  error;
- TypeScript definite-assignment and optional field declarations being
  rejected;
- the standalone JVM gate leaving the new `__VALUES__` template token
  unresolved.

It also aligned limits: C#/Swift Value field names now share the frontend's
128-character limit, and selected strings share the 4 KiB UTF-8 limit in all
runtimes.

**Tests and docs.**

- Regression tests cover all nine source spellings and the framework
  environments where they apply, both TypeScript decorator modes, and explicit
  JavaScript assignment after successful binding.
- The documentation and website now show that assignment to a real instance.
- The snippet verifier now runs the website configuration example and the
  complete TypeScript worked example over HTTP. Before, it skipped the website
  configuration example. Running it found and fixed three problems in the
  examples: marker syntax over several lines, an undefined cache factory, and
  use of an unsupported `request.signal` property.
- A malformed Value fuzz seed keeps the panic reproducer.
- This review adds no new dependency, no implicit injection or container, no
  commit, no branch reconciliation and no publication.

**Review verification:**

- Formatting, the locked all-target/all-feature workspace check, Clippy with
  warnings denied, rustdoc with warnings denied and all cargo-deny checks
  passed. The existing duplicate-version warnings remain.
- Both TypeScript decorator modes and the repository type check passed.
- The final focused run passed 55 behavior tests, including native
  C#/Swift/Rust execution. All 11 supervised behavior integration tests passed
  with all features.
- The standalone Java/Kotlin/native-Kotlin checks and 25 JavaScript runtime
  checks plus nine extension checks passed.
- Website tests passed 38/38 with 699 assertions. Guide verification passed
  six bundles and three HTTP probes, which now include both application
  configuration examples.

**Do not describe the broad verification as a clean first pass:**

- A workspace run that started before the final parity edits failed its C#
  behavior fixture. That target passed after a rebuild from the final source.
- The broader website snippet run reported seven failures in compiler
  startup, readiness or requests. Strict retries stopped at Kotlin preflight
  failures that came and went. A direct Kotlin recheck passed, but this does
  not make the broad snippet suite pass.
- No deadline, toolchain requirement or security setting was weakened to get
  a pass.
- The broad workspace run was finally stopped during the routing/dispatch
  suite, after several minutes without progress. It is incomplete, not a
  passing canonical gate.
- Only the identified validation process groups owned by the review were
  ended, and their exit was verified. Unrelated host processes were not
  touched.
- Run the complete workspace and full snippet suites again on a responsive
  host before making any release-qualification claim.

At the final environment check, the external toolchain folder and the
external Cargo target folder were not available. Bring back the external
toolchain/build volume before retrying those commands. This review did not
mount anything and did not change any host security setting.

## Private application configuration — 2026-09-12

ADR 0023 adds an optional root `application.json`, plus one
`application-<profile>.json` that is required if a profile is selected
(`YON_PROFILE`). No dependency was added. See `APPLICATION_CONFIGURATION.md`
and the versioned `api/application-config/v1` authoring schema.

**Order of sources:** the CLI host/port override environment, the
selected profile, the base file, then defaults. File structure
is validated before merging. Only the winning values go through placeholder
expansion and endpoint validation.

**The core loader** is `ApplicationConfig`.

- It rejects duplicate or unknown keys, unsafe files, invalid profiles,
  unresolved placeholders and inputs over the limits. Errors and Debug output
  leave out values.
- Limits: files 64 KiB, resolved child JSON 16 KiB, 128 keys in total, nesting
  depth 16.
- Strings cannot contain control characters.
- Configuration loading and source loading share the kept project directory
  capabilities, including for explicit handler invocation.
- Resolved snapshots do not change for the lifetime of the server. Restart
  after configuration changes.

**What Yon receives.**

- Yon receives the reserved, output-only `YON_APPLICATION_JSON` on both
  invocation paths. This includes middleware and workers that share the
  supervisor.
- Datasource and Redis settings go to adapters that the application owns.
  Tachyon provides no automatic driver, pool or connection.
- `properties` stays an explicit input to behavior binding. File profiles do
  not automatically select `@Profile` configurations.
- Tac builds do not load, resolve or publish these files.
- Isolation, runtimes and ordinary environment inheritance stay separate
  deployment policy. A Firecracker driver must explicitly forward application
  settings into its guest.

**Tests.**

- The focused loader corpus passed 18 tests.
- The integration corpus passed 14 tests. Every supported Yon language
  actually ran. The corpus also covers streaming, typed binding, root
  replacement, HTTP exclusion, and checks that nothing leaks into the build.
- The new parser fuzz target runs in scheduled fuzz CI.
- New scaffolds and the website include a safe starter configuration with only
  host and port.
- The verification counts in the older behavior section below describe its
  earlier run, not this configuration change.

**Final local macOS qualification** passed formatting, the locked
all-target/all-feature workspace check, Clippy with warnings denied, **550
tests across 26 suites** (zero failed or ignored), rustdoc with warnings denied
and all four cargo-deny checks. The existing duplicate-version warnings remain.

- Website tests passed **38 tests / 698 assertions** with
  `TAC_BIN=<cargo-target>/debug/ty`. The repository-local
  `target/debug/ty` is not a usable executable on this machine.
- Both changed CI workflows passed `actionlint`.
- The new fuzz target passed locked compilation. No local instrumented fuzz
  campaign and no remote CI run is claimed.
- Scheduled fuzzing now uses separate jobs per target and a corpus cache per
  target, so the sixth target does not use up the old shared time budget.
- The final run includes the restored early CLI rejection of non-loopback
  addresses and the updated policy assertion for 23 ADRs.
- No release, push or publication was made.

> Current behavior override (2026-09-12): ADR 0022 adds cache, persistence and
> outbound HTTP adapters that the application selects, with behavior
> annotations in all supported Tac and Yon languages. See
> `BEHAVIOR_ANNOTATIONS.md`.
>
> - This replaces Tac's blanket ban on decorators. It does not replace Tac's
>   client-only rendering or its explicit mount/signal APIs.
> - All eight Yon languages, Tac JS/TS, and native Swift/Kotlin/C#/Rust page
>   companions have execution adapters.
> - Native lifetimes are explicit. Synchronous deadline checks are
>   cooperative.
> - Shared extensions now include cache mutations, explicit typed
>   properties and profile conditions, DTO validation, and authorized
>   structured and derived repository queries. See
>   `BEHAVIOR_EXTENSION_SPEC.md` and the optional
>   `BEHAVIOR_PROJECT_STRUCTURE.md`.
> - Transactions and discovery of annotations across files are still
>   deferred.
> - No SSR, and no server secrets in the browser. The older overrides below
>   are history.

## Shared behavior extension verification — 2026-09-12

The final local macOS run passed the complete canonical gate: formatting,
all-target/all-feature checking, Clippy with warnings denied, **515 tests
across 24 suites**, rustdoc with warnings denied, and all four cargo-deny
checks. Cargo-deny kept the existing duplicate-version warnings. It reported no
failing advisory, ban, license or source check. On this machine, use the
pinned Rust 1.97.1 toolchain and
an external `CARGO_TARGET_DIR`.

Other gates that ran and passed:

- All 52 focused behavior tests, including native C#/Swift/Rust execution and
  the frontend matrix for nine languages. The canonical run also ran real
  supervised endpoint fixtures for JS/TS, Python/PHP, Java/Kotlin, C# and
  Rust.
- Java/Kotlin runtime boundary scripts (including nested Tac Kotlin), C#/Swift
  runtime scripts, Python/PHP boundary scripts, and 31 JS runtime checks.
- Both TypeScript decorator modes and the repository type check command.
- All 38 website tests (697 assertions), the emitted JS/TS behavior browser
  gate, and desktop/mobile website interaction checks with the rebuilt CLI.
- Compilation of the annotation fuzz target and formatting of the fuzz
  workspace. This covers compilation and seeds only. It does not claim a
  completed fuzz campaign.

Regression tests cover:

- qualified native query return types;
- Python query declarations where derived and explicit queries are next to
  each other;
- text that looks like a decorator inside strings;
- annotated constructor fields with inline comments;
- the strict PHP DTO object shape;
- duplicate nested policy keys;
- late cache writes after a timer that was blocked delivers.

Some fresh executable launches stalled long enough to time out local checks.
Unchanged reruns, one after another, passed. No runtime deadline or host
security setting was relaxed. The local results do not claim a new remote CI
run, a release, or Windows/Linux GUI qualification. No commit or publication
was made.

> Current HTTP-method override (2026-09-12): annotated controllers expose
> instance methods named after the uppercase verbs `GET`/`POST`/other
> supported verbs directly. HTTP method decorators are removed, with `TY2015`
> migration guidance. The optional `Stream` stays on methods named after
> verbs. Markers for cache, pagination, entity, configuration, constructor and
> HTTP client are proposed in `YON_ANNOTATION_PROPOSAL.md`. They are not
> implemented. The choice of who owns the supporting adapters and backends is
> still open.

> Current Yon override (2026-09-11): ADR 0021 adds optional compile-time
> controller/provider annotations and constructor injection within one source
> file, in the eight owned languages. Plain Handler endpoints stay valid.
> Providers never live longer than the supervised invocation. This slice does
> not implement discovery of annotated providers across files, or
> guard/interceptor/filter APIs. See `YON_ANNOTATIONS.md`. Tac stays without
> decorators and rendered on the client.

> Current authoring override (2026-09-10): ADR 0020 replaces every Tachyon
> decorator convention. Tac uses a plain default-exported presentation model,
> an optional `mount()`, and explicit runtime signals. Yon routes expose a
> plain `Handler`. Application modules have no framework layer markers,
> generators stream, and `relay(...)` is explicit. The historical notes about
> decorators and layers below are evidence only. They do not define the
> implementation.

> Current reconciliation override (2026-08-30): the phase notes and the notes
> specific to one machine in this document are history. Work on
> `codex/reconcile-local-features` in the workspace that the user selected.
> Never switch repositories because of the old path below. Keep Cargo output
> outside Dropbox with `CARGO_TARGET_DIR`. ADRs 0018/0019 replace native widget
> planning and WebAssembly companions. Browser components use JS/TS. Page
> companions compile for native targets. Do not install the obsolete Wasm
> toolchains. See `RECONCILIATION.md` for the current inventory and the
> qualification that is still open. The first signed and attested release has
> already happened. New behavior still needs a new, unchangeable release.

> Current architecture override (2026-08-10): ADR 0015 makes Tac rendered only
> on the client. The historical notes about SSR islands below describe evidence
> that is no longer current. Do not use them to decide the implementation.
> ADR 0016 makes Yon REST-only. It has no templates and no server renderer.

You are taking over the Rust rewrite of Tachyon. This is the whole briefing:
what is true today, what is left, and where the traps are. Everything here was
verified by running it, not by reading it. Where something is not verified,
the text says so.

## Your role

You are the framework engineer for the compiler, the server runtime, the
native hosts, packaging and documentation. `AGENTS.md` defines the canonical
gate and the boundary of the rewrite. `docs/ENGINEERING_STANDARDS.md` defines
the quality bar. Read `CONTEXT.md`, `docs/PROJECT_PLAN.md`,
`docs/PARITY_LEDGER.md` and `docs/CUTOVER.md` before you change anything.

## The one rule that matters here

**Verify by running things. Never by reading code, and never by just saying
so.**

This repository's documentation is unusual: every claim includes the command
that produced it. This is not decoration. It is the reason the parity ledger
can be trusted.

- If you cannot run something, say so and record it as unknown.
- An unknown is not a pass.
- Do not write "met", "verified" or "passing" next to anything that you did
  not see succeed.

I made two mistakes here, and you can make them too:

- `cargo test --workspace 2>&1 | grep -E "FAILED|panicked"` returns
  **nothing** when the test binaries fail to *compile*. I reported a passing
  suite twice because of this, and I was wrong both times. Count the result
  lines instead: `cargo test --workspace 2>&1 | grep -cE "^test result"`
  should be 17. Also check separately for `^error(\[|:)`.
- A gate script that is not executable in git fails only on a clean checkout.
  All of `scripts/**/*.sh` had mode 644, so the release lifecycle drill could
  never have run in CI. This is fixed, but look for this kind of problem.

## Environment

Work in `~/dev/TACHYON`, **not** in the Dropbox copy. Dropbox syncs the build
cache, and that makes git operations take minutes.

Why each setting below is needed:

- `$HOME/.cargo/bin` comes first because a Homebrew rustc 1.95 would
  otherwise hide rustup 1.97.1.
- `$HOME/dev/TACHYON/node_modules/.bin` gives tsc 7.0.2. The tsc on PATH is
  5.1.3.
- `$HOME/.dotnet` is a user-local .NET 9 with wasm-tools. The system .NET has
  no workload.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export PATH="$HOME/dev/TACHYON/node_modules/.bin:$PATH"
export PATH="$HOME/.dotnet:$PATH"
export TAC_KOTLIN_WASM_STDLIB="$HOME/.local/share/tachyon/kotlin-stdlib-wasm-js-2.1.10.klib"
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
export PATH="$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$PATH"
```

Toolchains installed on this machine, all verified to work:

| Language | What it needs | Where it is |
| --- | --- | --- |
| Rust | `wasm32-unknown-unknown` | rustup 1.97.1 |
| Dart | nothing extra | 3.6.0 |
| Kotlin | `kotlin-stdlib-wasm-js.klib`, which `kotlinc` does not ship | `TAC_KOTLIN_WASM_STDLIB` above |
| Swift | a swift.org toolchain **and** the wasm SDK; Xcode's `swiftc` cannot cross-compile | Swift 6.3.3 installed user-locally through Swiftly and exposed through `~/Library/Developer/Toolchains/swift-6.3.3-RELEASE.xctoolchain`, plus `~/.swiftpm/swift-sdks` |
| C# | `wasm-tools` workload | `~/.dotnet` (system .NET needs root, so it is unpatched) |

`ty doctor <project>` reports all five. It tests what each toolchain can
actually do, not just `--version`. Trust it more than your own assumptions:
four of these looked ready when none of them could build.

## State of the repository

The rewrite and the website migration are being prepared on
`codex/stable-rust-rewrite` as one cutover release candidate. Review
`git status` before you commit. It is not one small patch. It contains the new
Rust rewrite, earlier product changes, the website cutover and release
hardening.

### What is done and proven

- **Phases 0–7 of the engineering work.** The evidence for each phase is in
  `docs/PHASE_*_EVIDENCE.md`.
- **Browser companions in five languages**: Rust, Dart, Kotlin, Swift, C#.
  Each is compiled by its own real compiler, not by a transpiler for a subset
  of the language.
  - One JSON protocol.
  - Two module shapes: bare wasm, or a JS module that exports `tacInvoke`
    when the toolchain emits only WasmGC or a whole runtime.
  - See ADR 0011 and its amendment.
  - Gate: `node scripts/wasm/companion-browser-test.mjs` runs all five on one
    page in Chromium.
- **Native apps for iOS, Android and Linux**, verified again by running them
  on 2026-07-31. `scripts/android/native-test.sh` and
  `scripts/linux/native-test.sh` both PASS. iOS was tested by hand on iPhone 17
  Pro. The macOS evidence block is named below.
- **Windows native execution** in CI run 30714886154. The generated Win32 app
  compiled and launched. It exposed its UIA names and its backing `Button`
  HWND, accepted `BM_CLICK`, changed bound state `0 → 1`, recorded its
  lifecycle, and closed.
- **Compatibility differential** 3/3 corpus projects
  (`TAC_LEGACY_BIN=/path/to/released/ty TAC_BIN=/path/to/rewrite/ty node
  scripts/compat/differential.mjs`). The environment variables tie the gate to
  unchangeable binaries, even after the legacy runtime is removed from the
  tree.
- **Release lifecycle drill** PASS, including a bit-identical rebuild.

### What is open

`docs/CUTOVER.md` decides. Five of six gate conditions are met, and one is
still open. Automated security qualification is required for each release. An
independent human review is optional and does not block a release.

1. **Release provenance (condition 4).** The workflow and the local
   reproducible release drill pass. But one real workflow started by a tag
   must publish an archive and an attestation before external verification can
   be recorded.

Separate gaps for promoting targets remain:

- macOS Accessibility permission is still needed for the local probe.
- Windows semantic UIA roles/`InvokePattern` are still not proven, even though
  native execution passed.

The stable workflow runs only on tags and cannot create its own tag. In order,
it:

1. builds five native CLI assets with auditable dependency metadata;
2. publishes a CycloneDX SBOM (software bill of materials);
3. attests and keyless-signs every distributed input;
4. stages a private draft;
5. verifies the assets and installers on their native runners;
6. only then makes the release public.

`install.sh` and `install.ps1` now fail closed when a checksum is missing or
does not match. They no longer install the removed FYLO tooling.

## Website migration — completed 2026-08-01

`website/` now builds and tests through the Rust implementation. It emits 11
routes and compiles the five real-language browser companions with their own
toolchains.

The migration check reports **128 supported, 1 changed, 1 unsupported**:

- The five ADR 0011 companion migrations are supported, because each legacy
  companion is paired with a sidecar built by a real compiler.
- Navigation between documents is the one changed finding.
- The one unsupported finding is telemetry. The parity ledger records it as an
  explicit product boundary.

The migration sets these authoring rules:

- Structural control flow is written in Tac HTML, and the client renderer owns
  it. The compiler serializes bounded expressions and control plans. Mounted
  component companions supply state and methods. The legacy `hydrate` spelling
  is only another name for a mount schedule. It is not a way to escape into
  imperative structure.
- Shared browser assets under `client/shared/` publish at the stable
  `/shared/` path. The compiler enforces limits on file count and bytes, and
  refuses links and entries that are not files.
- The Rust compiler does not automatically combine ancestor page layouts. So
  the website uses explicit `<atlas-layout>` and `<docs-layout>` components.
- The five polyglot examples are normal Rust, Dart, Kotlin, Swift and C# in
  the ADR 0011 ABI shape. They are migration findings on purpose, because the
  legacy subset transpiler cannot read the same sources.
- Reused incremental routes keep their event descriptors, so a second build
  cannot delete `.tachyon/events.js`.

The build, companion and legacy-oracle evidence below was recorded on
2026-08-01. The website suite was run again on 2026-08-11:

```text
ty build website
  Built 11 routes ... compiled=0 reused=10
bun run test
  24 pass, 0 fail, 353 assertions
node scripts/wasm/companion-browser-test.mjs
  wasm companion gate passed for rs, dart, kt, swift, cs

YON_DIST_PATH=<temporary-output> /path/to/26.30.04/ty bundle
  Bundle completed ... routes=66 targets=web
  11 authored route HTML files emitted beneath web/
```

The last command ran from `website/` with the archived 26.30.04 executable.
The legacy log shows a higher route count because it includes its internal
page and component entries. The published set of authored routes is the same
11-route set that the rewrite produces. A separate `YON_DIST_PATH` kept the two
outputs apart.

The event, island-expression, service-worker and Phase 3 Chromium gates also
pass. Repeated reused builds keep byte-identical `.tachyon/events.js` and
`shared/scripts/imports.js`.

## Released-surface parity hardening — completed 2026-08-01

The public workflow is now tested directly against the Rust executable. It is
not assumed from command aliases. `scripts/compat/standalone-rust.mjs`:

- creates a project and checks every scaffold file;
- bundles and serves web output;
- checks the released cache lifecycle and the `spa-renderer.js` entry;
- builds a source-only native host;
- drives literal page state through a real Chromium click;
- runs its native controller through state, shortcut and content-surface
  calls.

```text
TAC_BIN=target/debug/ty node scripts/compat/standalone-rust.mjs
  PASS: released standalone workflow matches Rust ty (macos)

TAC_LEGACY_BIN=/tmp/.../26.30.04/ty TAC_BIN=target/debug/ty \
  node scripts/compat/differential.mjs
  scaffold: 18 generated files match after declared legacy scaffold removals
  3/3 corpus projects match across implementations
```

The CLI now keeps these released behaviors: the version output, root help,
target aliases, comma-separated and `all` targets, the target/host/port
environment variables, an absolute `TAC_DIST_PATH`, `serve --no-bundle`,
source-only native packaging, and the cache shape.

Two real-binary regression tests:

- One proves that `serve --no-bundle` does not compile changed source.
- One proves that a single command with several targets publishes exactly one
  level at `dist/web`, `dist/macos`, `dist/ios` and `dist/android`. Before this
  test existed, native targets were published by mistake as
  `dist/macos/macos`.

**Scaffold.** The 18 scaffold files that parity covers stay byte-identical,
including the root Yon handler. The declared differences:

- remove the obsolete database layout and environment entries;
- normalize the installer sentence that only the standalone release had;
- leave the new standalone ambient type contract out of the byte comparison.
  Rust scaffold tests still cover how it is generated.

**Page-class workflow.** The released bounded page-class workflow is still
kept: literal page state, class fields, the `mount()` lifecycle, assignment
events and native controller capabilities.

- Inline state can only be literal, on purpose. Executable initializers fail
  with `TY1306` and are never emitted as authored source.
- The current browser gate requires a render plan with a schema version and
  `/.tachyon/tac-client.js` for every Tac route. Tests at the compiler level
  cover emission, and repeated tests at the browser level cover updates.

The migrated website was then rebuilt through the same public commands:

| Command | Result |
| --- | --- |
| `bun run bundle` | Rust ty: 11 routes |
| `bun run test` | 24 pass, 353 assertions |
| `YON_DIST_PATH=<temp> /path/to/26.30.04/ty bundle` | 11 authored route documents |
| `ty bundle --target macos,ios,android` | three packaged 11-route apps |

**Native results.**

- The combined native build reported 296 native nodes and 39 bounded surface
  payloads per target.
- The corrected artifacts installed and launched on macOS, the booted iPhone
  17 Pro simulator and the Android emulator.
- On Android, CDP (the Chrome DevTools Protocol) proved that the mobile menu
  opens and closes, and that the saved theme changes from light to dark. Logs
  for the app's process had no fatal exception.
- The native Android gate also passed platform-widget accessibility, the
  increment interaction, isolated fallback and lifecycle logging.
- iOS and macOS rendered the same responsive view in screenshots. On this
  simulator, iOS needed about 5–15 seconds for its first local WebKit
  surfaces.
- Automated clicks on those two hosts are still blocked by the macOS
  assistive-access permission named elsewhere in this file. So the
  screenshots do not prove interaction.

The in-tree JavaScript framework and its test suite were removed on
2026-08-09. These are now the unchangeable oracles for public behavior: the
archived 26.30.04 executable, the standalone workflow, the neutral
differential corpus and the migrated website. A repository policy test stops
the deleted implementation paths from coming back.

## Recently fixed handoff defects

- Native-planning diagnostics now point to the authored `tac.html` input. Yon
  does not take part in native view planning.
- Same-origin links inside a local WebSurface now pass navigation to the
  native route stack. Taps on the Android emulator went `/` → `/docs` →
  `/docs/introduction` on the migrated website.
- Native planning no longer hard-codes `w-app-bar` or `w-button`. Custom
  design systems reach adapters through explicit semantic roles. The Phase 4
  fixture declares `role="banner"` and still honestly records its
  `layout.app_bar` evidence.

## Traps in the code

- **`n_%06d` node ids are a public contract** (`api/native-ui/v1/schema.json`,
  `^n_[a-z0-9_]{1,126}$`). I broke it when I scoped ids per route. The schema
  test caught it. Scope the *payload path*, not the id.
- **SwiftUI `@StateObject` makes a view's memberwise init isolated to the main
  actor.** This breaks the nonisolated node builder in the generated host. Use
  `sizeThatFits` instead of an observable height.
- **The generated Swift, Java and C hosts are Rust string literals.** They
  compile only when you build for that target, so `cargo test` does not show a
  typo. Build the real app for every host you change.
- **Android emulator:** a killed emulator leaves `*.lock` files in
  `~/.android/avd/<name>.avd/`. The next boot then fails with "a snapshot
  operation is pending". Delete the files and boot with `-no-snapshot`.
- **Docker on this machine** cannot use buildx, because root owns
  `~/.docker/buildx/refs`. Build the Linux gate image with
  `DOCKER_BUILDKIT=0`.
- **Disk space is low.** Emulators refuse to boot with less than about 3 GB
  free. `target/` is a cache and is safe to delete. Nothing unique is in it.

## How the maintainer works

- Some decisions belong to the maintainer: hardware, credentials, security
  review, and accepting simulator evidence. Record these as **their** signed,
  dated decision, not as a claim by the implementation. See the amendments in
  `docs/CUTOVER.md` and `docs/SUPPORT_TIERS.md` for the format.
- Write plain, specific text that admits what it does not know. Match that
  style.
- Do not invent evidence to close a gate. The gate exists to be honest about
  what is not proven yet.

## Suggested next moves

1. Complete the pull-request matrix and get a review for the cutover commit.
2. Create the annotated `v26.35.07` tag only after the reviewed commit is on
   `main`. The tag workflow will stage, verify and publish it.
3. Grant macOS Accessibility permission and run the macOS evidence script
   again.
4. When useful, schedule an independent human security review for major
   changes to trust boundaries. It is not required before publication.

## Semantic hot updates — completed 2026-08-09

The development loop no longer fingerprints the source tree every 400 ms.

- `notify` feeds a bounded queue and a coalescer with a 75 ms quiet period.
- Successful builds publish Hot Update Protocol v1 at `/.tachyon/hot`.
- CSS is swapped in place.
- Supported Tac companion edits replace island boundaries that the compiler
  owns.
- All structural or uncertain edits reload the page.
- A failed build leaves the previous output and DOM running and shows a
  text-only structured diagnostic overlay.

**State transfer.** Island instances can implement `hotState()`,
`restoreHotState(state)` and `hotDispose()`.

- Without explicit state hooks, the enumerable fields that are not functions
  are copied with `structuredClone`. If the default clone fails, the
  transferred state is an empty object.
- An explicit `hotState()` result is also cloned before restore.
- No JSON or depth limit is claimed.
- Disposal always aborts the signal that the runtime owns.
- Missed event sequences, broadcast lag, changed boundary identity and client
  errors lead to a full reload instead of an unsafe patch.

**Evidence** is in `crates/tachyon-core/tests/it/dispatch.rs`, the compiler and
watcher unit tests, and `scripts/hot-update-browser-test.mjs`. The browser
scenario was also run by hand against `ty dev`:

- CSS and island changes kept a counter value.
- The old island was disposed and aborted.
- An invalid template showed diagnostics without replacing the page.
- Restoring the structural template caused the expected full reload, with no
  console errors.

**Follow-up on the real Tachyon website.** It found and closed two gaps:

- Static generated assets now take priority over a dynamic page fallback. So
  `/docs/client.js` is served as JavaScript instead of being rewritten to
  `/docs/_topic`. `/docs` again redirects and renders Introduction.
- Island replacement now also keeps a bounded set of native browser state, in
  addition to companion state: form values and checked state for inputs,
  `<details>` disclosure, nonzero scroll offsets, and focus by element id. The
  snapshot looks at most at the first 2,048 elements with an `id`. It does not
  keep text selection or the selected `<option>`.

Verification in a real browser kept an open quickstart disclosure and a
focused, edited input through a live companion replacement, with no console
errors.

## Environment-selected Yon isolation — added 2026-08-09

ADR 0014 adds a provider boundary without putting deployment policy in
`tachyon.json`.

- `YON_ISOLATION=process` is the default, for compatibility.
- `firecracker` requires a control program that is an absolute path, a regular
  file and not a symlink. Tachyon passes it bounded pool, vCPU, memory and
  deny-egress arguments.
- Every Yon entry point reads the same parent environment and uses Handler
  Protocol v1. This includes middleware, workers, HTTP dispatch and explicit
  CLI invocation.
- Invalid or partial configuration fails as `TY2010` before any handler runs.

Do not describe this as qualified microVM isolation yet. These still need their
own vertical slice: the first-party control program, the Linux/KVM host
profile, the jailer configuration, the guest image, the warm-pool and snapshot
lifecycle, and native adversarial evidence.

## Pages compose components — added 2026-09-15

ADR 0029 makes the page directory pure composition. `client/pages/**` holds
only `tac.html`, and that template contains only component tags, whitespace
and comments.

- Anything else fails as `TY1309`:
  - a companion file beside a page (checked in `project.rs`);
  - a plain element, text, slot, control flow or state script in a page
    template (checked in `compiler.rs`).
- Site-wide assets are declared once in `tac.config.js` through
  `static scripts` and `static styles` on its default-exported class
  (ADR 0040).
- For this rule, custom elements such as `<w-container>` are plain elements.
  They belong inside a component.

**Native companions** (`tac.swift`, `tac.kt`, `tac.cs`, `tac.rs`) belong to
components. They are activated for each component instance.

- Native requests carry `{op, route, component, owner, name, args, value}`.
- Every prelude keeps a map of activations keyed by owner (at most 256). It
  answers `TY_NATIVE_COMPONENT` for an unknown component and
  `TY_NATIVE_OWNER` for a stale owner. It disposes everything on
  `host-dispose`.
- The browser runtime binds a native companion when it creates a component
  instance whose plan node has `native: true`. It disposes the companion
  together with the instance.
- A component with only native companions and no browser module still
  compiles for the web (`TY1010` names the missing browser target). It may
  hydrate, because a host answers for it.

**Evidence on this Mac:** core lib; CLI phase 4 (Swift and Rust companions,
and the probe for several instances side by side); the Rust ABI gate; the
tac-client, events, behavior, storage, service-worker and runtime browser
gates; the website tests; and the snippet verifier. The Android, iOS, Windows
and Linux native gates were migrated to requests addressed by component, but
they could not run here. Run them in CI before you trust the per-instance path
on those hosts.

## Components use services — added 2026-09-15

ADR 0030 moves behavior out of the component directory.

**Component directory.** `client/components/<name>/` holds `tac.html` and,
optionally, `tac.css`. A `tac.js`, `tac.ts`, `tac.swift`, `tac.kt`, `tac.cs`,
`tac.rs` or `tachyon-island.*` beside a component view fails as `TY1310`
(`template/component.rs`). The message names the service file to move it
into.

**Service files.** Behavior lives in `client/services/<Name>Service.<ext>`.

- The class has exactly the same name as the file stem, is annotated
  `@Service()` and is the default export. `template/service.rs` discovers it.
- These are also `TY1310`: a bad name, a subdirectory, a missing default
  export, or a service with both a `js` and a `ts` file.
- `js`/`ts` reach the browser. `swift`/`kt`/`cs`/`rs` are one native
  implementation per language.

**Views.** A view addresses a service by instance name: `{basket.count}`,
`on:click="basket.add()"`, `on:input="basket.query = $event.target.value"`.

- The grammar for handlers and assignments is `name` or `service.member`,
  with one dot.
- The old message "must name one exported identifier" is now "must name one
  handler, as save() or basket.add()".
- A component whose view names a service defaults to the mount policy `load`.
  A mount policy on a component that names no service is `TY1405`
  (`template/client.rs`).

**Output.**

- Browser modules are emitted at `dist/.tachyon/services/<Class>.js`.
- The hot boundary is `service:<Class>` (`hot_update.rs`).
- The page plan carries `services: { basket: {module, scope, native} }`, and
  each component node lists the services it names.
- `Service` and `Scope` are global Tac markers (ADR 0046).
  `behavior/mod.rs` keeps them for `environment == "tac"`. Yon lowers its own.

**Tools.** `ty init` writes `client/services/ShellService.js`. For any
`tac.js`/`tac.ts` elsewhere, `ty migrate` reports:

- `companion.component` (Changed, with the destination
  `client/services/<PascalCase>Service`);
- `service.tac` (Supported);
- `service.native` (Changed);
- `companion.controller` (Changed).

Traps:

- **Instance names are derived mechanically:** remove the `Service` suffix and
  make only the first letter lowercase. `UserListService` becomes `userList`,
  but `HTTPService` becomes `hTTP`. Choose service names so that the derived
  name reads well. The view cannot rename a service.
- **Singleton versus prototype.**
  - By default, a service is a singleton for the page. `tac-client.js`
    constructs it when the first component that names it renders. It
    initializes it (PostConstruct) when that component activates. It disposes
    it on a `pagehide` that is not for the bfcache (back/forward cache), or on
    a runtime failure. Two instances of one component share it.
  - `@Scope({"value":"prototype"})` constructs one service for each component
    instance, with that instance's properties, and disposes it together with
    the instance.
  - Persisted keys differ. Singleton:
    `tac:service:<instance>:<pathname>:$field`. Prototype:
    `tac:<component>.<instance>:<persist-id|pathname>:$field`.
  - Discovery reads the scope from the text of the `@Scope(` marker, not from
    a parsed annotation. The behavior lowering validates the literal.
- **Service-to-service injection uses undefined fields.**
  - A field that is declared, left `undefined` and named after another service
    (`catalog;`) receives that singleton after construction. So you cannot use
    it in the constructor or in a field initializer.
  - A field with a value, or one named after nothing in the plan's service
    catalog, is left alone.
  - A cycle throws an error.
  - Collaborators in `client/clients`, `client/repositories`,
    `client/configurations`, `client/entities` and `client/delegates` still
    go through `@RequiredArgsConstructor`.
- **The native `service` field.**
  - Host requests are `{op, route, service, owner, name, args, value}`, where
    `service` is the instance name (`"basket"`), not the class.
  - An unknown or missing service answers `TY_NATIVE_SERVICE`. A stale owner
    answers `TY_NATIVE_OWNER`. Event payloads and evidence JSON carry
    `service`.
  - The browser binds one native activation for each browser-side service
    instance (`bindNativeInstance(instance, name)`). So a singleton is
    activated once per page, and a service with prototype scope once per
    component instance.
  - Dispatch tables: `tacServiceKnown` / `tacServiceMembers` (Swift),
    `tacServiceMembers` (Kotlin), `TacServices.Members` (C#) and
    `tac_service_members` (Rust).
  - `NativeCompanionInput` carries `{snapshot_root, language, source, service,
    class}`.
  - The bridge verb is still `companion.invoke`.

**Evidence on this Mac:** the core implementation was verified in a real
browser before this documentation pass. The Android, iOS, Windows and Linux
native gates were migrated to requests addressed by service, but they were not
run here. Run them in CI before you trust the per-instance native path on
those hosts.

Migrating the gates found two runtime traps. Both are fixed:

- A `$field` on a service broke dependency inference (`TY2015`) whenever the
  project also had a `client/clients/*` role file. The reason: the client
  planner ran `required_fields` over the service class. Inference now skips
  persisted fields, in `yon/required_args/fields.rs`.
- A service hot update rebuilt the component subtree twice: once for the swap,
  and once after the new owner's `PostConstruct`. The second render came after
  `restoreMutableDom`, so an open `<details>` and a typed draft were lost. The
  activation render is now tracked in `inflightActivations`, and `hotUpdate`
  waits for it before restoring. The render itself still does not block,
  because a mount triggered by an interaction replays its event on the element
  that received it.

**Corpus changes.** The compatibility corpus (`corpus/*`) and the phase 3 and
hot-update browser fixtures now put their markup inside a component
(`<home-view />`, `<demo-shell>`, `<hot-shell>`). Document titles are in
`tac.config.js`. The differential's semantic-DOM extractor unwraps chains of
single children, so the new wrapper adds no difference to `parity.json`.
Confirm this on the next CI run of `scripts/compat/differential.mjs`, which
needs the frozen legacy binary.
