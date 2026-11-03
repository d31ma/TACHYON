# Tachyon Rust Rewrite Project Plan

## Executive Decision

Rebuild Tachyon in Rust as a modular monolith: one program made of separate
internal modules. Do the work on a long-lived rewrite branch in the existing
repository.

Keep the current implementation runnable. It is the compatibility oracle: the
reference that shows the expected behavior. Keep it until the Rust
implementation meets the cutover gate.

The rewrite does not copy the old code line by line. It deliberately keeps
stable behavior, source conventions, contracts, and developer workflows. The
internal structure is new code, written from scratch (greenfield).

## Delivery Principles

The post-cutover local-event and lifecycle slice follows ADR (architecture
decision record) 0028, as amended by ADR 0031. The slice contains:

- shared validation;
- Resource ownership in both realms;
- an injected ApplicationEventPublisher instead of a publisher annotation;
- Yon invocation events in all eight languages;
- four native owner adapters;
- executable negative tests;
- migrated website examples.

ADR 0031 also shares role folders per realm. ADR 0032 adds the `fylo`
datasource driver. docs/SUPPORT_TIERS.md records the current qualification
and platform limits. This slice does not establish an unqualified stable release.

The principles:

- Deliver small vertical slices. Each slice delivers the implementation,
  negative tests, documentation, observability, and compatibility evidence
  together.
- Add a crate only when it owns production behavior and tests.
- Make boundaries stable before you optimize implementations.
- Treat "it builds" and "it is supported" as separate claims.
- Prefer evidence from native execution over claims based on
  cross-compilation.
- Do not publish an incomplete implementation under the stable Tachyon name.

## Phase 0: Foundation

### Scope

- A Rust workspace with an exact supported toolchain.
- Repository policies for lint, dependencies, licensing, and sources.
- Product context: terms, relationships, invariants, non-goals, and deferred
  decisions.
- Five accepted architecture decisions.
- An initial threat model.
- Versioned canonical schemas, each with accepted and rejected examples.
- Cross-platform Rust CI.
- Policies for release, support, security, governance, and contribution.
- A documented boundary between the archived release (the oracle) and the new
  greenfield code.

### Exit Gate

- [x] The exact Rust toolchain installs from `rust-toolchain.toml`.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace --all-targets --all-features --locked` passes.
- [x] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
      passes.
- [x] `cargo test --workspace --all-targets --all-features --locked` passes.
- [x] `cargo doc --workspace --all-features --no-deps --locked` passes with
      warnings denied.
- [x] Every canonical schema passes Draft 2020-12 meta-validation.
- [x] Every canonical schema accepts its positive example and rejects its
      negative example.
- [x] Dependency advisories, licenses, sources, and banned dependencies pass
      policy.
- [x] Linux, macOS, and Windows CI use the same exact toolchain and commands.
- [x] All GitHub Actions are pinned to immutable commits.
- [x] A new contributor can explain the system and its trust boundaries without
      reading implementation source.
- [x] Phase 0 evidence is recorded in `docs/PHASE_0_EVIDENCE.md`.

Phase 0 does not implement a compiler, server, handler runtime, or renderer.

## Phase 1: Web Vertical Slice

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_1_SPEC.md`](PHASE_1_SPEC.md), and the validation record is in
[`PHASE_1_EVIDENCE.md`](PHASE_1_EVIDENCE.md).

### Scope

- Project discovery, and deterministic route graphs for Tac views and Yon REST
  handlers.
- Bounded, source-aware HTML tokenization and Phase 1 feature validation.
- Stable failure messages, both for humans and in Diagnostics v1 format.
- Deterministic staged web output and Route Manifest v1.
- `ty init`, `ty build`, and a loopback-safe `ty dev`.
- Acceptance tests that run the real binary on several platforms.

### Exit Gate

- [x] The real `ty` binary initializes, builds, and serves one generated
      project.
- [x] Repeated builds are byte-identical and emit Route Manifest v1.
- [x] Static `tac.html` routes and `yon.*` REST handlers have canonical ordering.
- [x] Unsupported Phase 2/3 syntax fails with stable diagnostics.
- [x] A failed build keeps the last known-good output.
- [x] Tests exercise HTTP GET, HEAD, 404, path traversal resistance, and
      defensive headers against the real server.
- [x] macOS runs the native acceptance suite.
- [x] Linux runs the same suite in a pinned official container.
- [x] Windows compiles locally. The native `windows-latest` CI job is the
      authoritative gate for execution.
- [x] These pass: formatting, check, Clippy, tests, rustdoc, coverage,
      supply-chain policy, and the released-binary compatibility suite.

## Phase 2: Yon Handler Boundary

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_2_SPEC.md`](PHASE_2_SPEC.md), and the validation record is in
[`PHASE_2_EVIDENCE.md`](PHASE_2_EVIDENCE.md).

### Scope

- Typed Handler Protocol v1 envelopes for request, response, error, and
  cancellation.
- Deterministic discovery of `yon.js` and `yon.py`, and their manifest
  metadata.
- Bounded supervision that spawns one process directly for each request.
- JavaScript and Python adapters, both sync and async.
- These remain: the eight-language boundary, the explicit supervised relay,
  and multi-frame generator delivery. ADR 0020 replaced the layer and method
  decorators with plain conventions: `Handler`, `relay(...)`, and native-yield.
- Deadlines, cancellation, concurrency admission, forced termination, reaping,
  and crash isolation.
- A child process inherits no environment variables by default. It gets only
  the ones on an explicit allowlist.
- Contract tests that use real runtimes and the compiled binary.

### Exit Gate

- [x] JavaScript and Python pass the same request/response corpus against real
      child processes.
- [x] Missing layer annotations and removed interpreter registrations fail
      closed: Tachyon rejects them instead of continuing. Relay stderr is
      redacted. A streaming timeout or disconnect reaps inheriting
      descendants.
- [x] The compiled `ty handler invoke` path exercises both runtimes and every
      Handler Protocol v1 HTTP method.
- [x] Handler-only and composed routes emit deterministic Route Manifest v1
      metadata.
- [x] These fail closed: framing, stdout/stderr separation, frame size,
      request IDs, and response validation.
- [x] Deadline time includes concurrency admission and process lifetime.
- [x] Cancellation, timeout, crash, malformed output, and forced termination
      all reap the child. A later invocation then starts cleanly.
- [x] Child environments contain only a minimum runtime baseline plus the
      explicit allowlist entries.
- [x] Tests cover paths with spaces and Unicode, exceptions, missing
      classes/methods, syntax errors, non-serializable results, and missing
      runtimes.
- [x] These gates pass: formatting, check, Clippy, tests, rustdoc, coverage,
      dependency policy, action pinning, cross-platform buildability,
      Linux-container execution, and released-binary compatibility.

## Phase 3: Tac View Semantics

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_3_SPEC.md`](PHASE_3_SPEC.md), and the validation record is in
[`PHASE_3_EVIDENCE.md`](PHASE_3_EVIDENCE.md).

### Scope

- Safe, bounded binding expressions and contextual escaping.
- Conditionals and iterations, in canonical and legacy-compatible forms.
- Recursive Tac components, properties, slots, and cycle detection.
- Yon handler discovery. The build does not execute handlers and gives them no
  view context.
- Tac rendering owned by the browser. Component mount schedules are load,
  idle, visible, interaction, and never. (Superseded design recorded by ADR
  0015.)
- View IR (intermediate representation) v1, View Source Map v1, verified
  incremental builds, and diagnostic recovery across multiple sources.

### Exit Gate

- [x] Golden tests on the compiled binary cover controls, bindings,
      components, slots, Tac render plans, source maps, Yon non-execution, and
      incremental reuse.
- [x] These fail closed: malformed expressions, orphan controls, component
      cycles, invalid islands, cache corruption, and failed-build rollback.
- [x] Tests in real Chromium prove client initial render, automatic component
      mounting, rerendering, bounded failure marking, and interaction replay.
- [x] The Phase 1 and Phase 2 compatibility suites still pass.
- [x] These pass: formatting, check, Clippy, tests, rustdoc, coverage,
      dependency policy, action pinning, Linux-container execution, and
      Windows buildability.

## Phase 4: Native Vertical Slice

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_4_SPEC.md`](PHASE_4_SPEC.md), the implementation boundary in
[`PHASE_4_IMPLEMENTATION_PLAN.md`](PHASE_4_IMPLEMENTATION_PLAN.md), and the
validation record in [`PHASE_4_EVIDENCE.md`](PHASE_4_EVIDENCE.md).

### Scope

- Native UI v1 planning from the fully resolved Phase 3 output.
- Fixed semantic adapters from HTML to SwiftUI, with deterministic node
  identities.
- Scalar state authored in HTML, bindings, increment/toggle actions, and
  lifecycle logging.
- Local fallback for the smallest possible subtree, and HTTPS WebSurfaces with
  no bridge.
- Capability Manifest v1, Artifact Manifest v1, generated Swift source, an
  ad-hoc-signed `.app` bundle, and atomic publication.
- Semantic and visual comparison between real macOS Accessibility and mobile
  Chromium.

### Exit Gate

- [x] Compiled-binary and schema tests cover adapters, evaluated views,
      accessibility, declarative state, manifests, fallback, and rollback.
- [x] These fail closed: invalid configuration, state, accessibility, URL,
      nesting, surface count, symlink, and output shapes.
- [x] A real SwiftUI `.app` builds, signs, and launches. It handles native
      button, keyboard input, and disclosure interactions. It then tears down
      its lifecycle.
- [x] Required roles and names match the mobile-web reference. Native and web
      screenshots pass the documented coarse visual-layout budget.
- [x] These gates pass: formatting, check, Clippy, tests, rustdoc, coverage,
      dependency policy, action pinning, Linux-container execution, Windows
      buildability, released-binary compatibility, and release smoke.

## Phase 5: Platform Expansion

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_5_SPEC.md`](PHASE_5_SPEC.md), and the validation record is in
[`PHASE_5_EVIDENCE.md`](PHASE_5_EVIDENCE.md).

Add Android, iOS, Windows, and Linux one at a time. A platform moves forward
only when evidence exists for native compilation, execution, packaging,
interaction, accessibility, upgrade, and rollback.

### Scope

- A `NativeTarget` selection that passes through the CLI, planner, staging, and
  Artifact Manifest v1. Each target's output is isolated from the others.
- Platform-neutral staging that every host generator shares.
- Hosts: iOS `UIKit`-backed `SwiftUI`, Linux GTK4 with `WebKitGTK`, Windows
  Win32 common controls, and Android platform views.
- Reproducible gates for each platform, and the CI jobs that run them.

### Exit Gate

- [x] One project lowers to byte-identical Native UI v1 for every target. The
      only differences are the target tag and the fallback reason that names
      the platform.
- [x] Each target publishes only under its own directory. A failed build
      publishes nothing there.
- [x] iOS builds a signed simulator `.app`. The app launches and responds to
      native button, keyboard, and disclosure interaction.
- [x] Android assembles an APK. The APK launches on an emulator and exposes
      native `Button` and `EditText` widgets with their declared content
      descriptions.
- [x] Linux compiles with warnings denied and launches headlessly. Over AT-SPI
      (Assistive Technology Service Provider Interface), it returns the
      declared names, native roles, and an assistive-technology activation.
- [x] Windows cross-compiles to a `PE32+` GUI binary with warnings denied.
- [x] Every declared platform reduction has a named viability gate. The gate is
      recorded in `PHASE_5_SPEC.md` and reflected in `SUPPORT_TIERS.md`.
- [x] Formatting, check, Clippy, tests, and rustdoc pass.

The `windows-native` CI job owns the Windows execution evidence. It recorded a
native run on 2026-08-01. Windows stays below `native-tested` until there is
evidence for semantic UI Automation roles and accessibility activation.

## Phase 6: Compatibility and Migration

Status: complete on 2026-07-26. The normative behavior is recorded in
[`PHASE_6_SPEC.md`](PHASE_6_SPEC.md), the validation record is in
[`PHASE_6_EVIDENCE.md`](PHASE_6_EVIDENCE.md), and the resulting statement is
[`PARITY_LEDGER.md`](PARITY_LEDGER.md).

Run the archived v26.30.04 release and the Rust implementation over the shared
application corpus. Compare route graphs, rendered HTML, HTTP behavior, events,
diagnostics, and artifacts. Deliver `ty migrate check` and a feature-parity
ledger.

### Scope

- A shared corpus. Every project in it builds under both implementations.
- A browser-based differential test. It compares route graphs, semantic DOM,
  and HTTP status. Declared divergences stay visible. An undeclared divergence
  is fatal.
- `ty migrate check`, which classifies a project without executing it.
- A ledger that covers the view, server, build, and native areas.

### Exit Gate

- [x] Every corpus project builds under both implementations.
- [x] Every corpus route renders an identical semantic DOM, or its divergence
      is declared and has a ledger row.
- [x] `ty migrate check` classifies the archived migration fixture. It attaches
      an action to every non-supported finding. Its output is
      byte-deterministic.
- [x] The ledger distinguishes `identical`, `equivalent`, `changed`,
      `unsupported`, and `rust-only`. It also states what it does not claim.
- [x] The differential runs in CI.

The differential deliberately does not compare artifacts. The two
implementations emit different output by design. Observable behavior is the
contract.

## Phase 7: Enterprise Qualification

Status: implementation complete on 2026-07-26. Release-candidate packaging,
fail-closed installers, signing, attestation, native post-publication
verification, and draft-to-public promotion were completed on 2026-08-01.
Automated security qualification is the pre-tag security gate; independent
human review is optional. The normative behavior is in
[`PHASE_7_SPEC.md`](PHASE_7_SPEC.md), and the validation record is in
[`PHASE_7_EVIDENCE.md`](PHASE_7_EVIDENCE.md).

Complete these: fuzzing, sanitizers, supply-chain evidence, performance
budgets, soak tests, recovery drills, automated security qualification, and
supported-target promotion.

### Exit Gate

- [x] Every trust boundary has a fuzz target. 7,197,692 executions under
      `AddressSanitizer` produced zero crashes, timeouts, or OOMs (out of
      memory errors).
- [x] The native planner's cross-target validity invariant holds under
      fuzzing.
- [x] The library suite passes under `AddressSanitizer`.
- [x] Recovery drills pass against the real executable for interruption,
      corruption, contention, read-only output, and failed builds.
- [x] The soak test keeps the determinism, descriptor, and latency properties.
- [x] Performance budgets are published and met.
- [x] Supply-chain policy passes. The SBOM (software bill of materials) and
      auditable-build jobs exist.
- [x] **Automated security qualification is complete.** No unowned critical or
      high finding remains; see
      [`SECURITY_REVIEW_PACKAGE.md`](SECURITY_REVIEW_PACKAGE.md).
- [x] **Install, upgrade, rollback, and uninstall exercises are recorded for
      the `ty` artifact.** The release lifecycle drill installs, upgrades,
      rolls back, verifies, and uninstalls a deterministic archive.

Independent human review is still welcome as an extra layer of defense. It
does not block CI, packaging, support-tier claims, tagging, or publication.

## Cutover Gate

[`CUTOVER.md`](CUTOVER.md) records the status of each condition.
**All six conditions are met.** Release provenance was the last one. It closed
when the tag workflow published v26.35.07 and v26.36.01, and their assets were
verified again outside the workflow on 2026-10-18.

This gate does not make a new version release-ready. Each new version must
also pass the reconciliation gate in [`CUTOVER.md`](CUTOVER.md) and
[`RECONCILIATION.md`](RECONCILIATION.md), on its own downloaded release assets.

The Rust implementation became Tachyon's default only when all of these were
true:

- every stable behavior has passing compatibility evidence or a documented
  migration;
- supported platforms pass their native evidence profiles;
- install, upgrade, rollback, and uninstall have been exercised;
- release artifacts are signed, attested, and independently verifiable;
- no unowned critical threat-model finding remains;
- stable documentation describes the Rust implementation, not plans.

## Post-qualification Capability: Semantic Hot Updates

ADR 0026 specifies direct JSON results and Application Configuration v2.
Application Configuration v2 has public tac configuration, private yon
configuration, and separate .env secrets. docs/APPLICATION_CONFIGURATION.md
describes the implementation. It is separate from release qualification and
from native SDK availability.

Status: complete on 2026-08-09. The decision and safety boundary are recorded
in [`adr/0013-semantic-hot-updates.md`](adr/0013-semantic-hot-updates.md) and
superseded for current renderer ownership by ADR 0015.

In development, bounded operating-system file events start rebuilds. A
rebuild publishes Hot Update Protocol v1 over a same-origin event stream.

- CSS updates keep the document.
- Tac component boundaries that the renderer owns transfer state in a
  lifecycle-aware way.
- A compiler failure keeps the last good page and shows structured
  diagnostics.
- Every ambiguous or structural change reloads the page safely.

`scripts/hot-update-browser-test.mjs` is the repeatable browser gate.

## Post-qualification Capability: Yon Isolation Backends

[ADR 0021](adr/0021-yon-annotation-frontend.md) and
[Yon annotations](YON_ANNOTATIONS.md) track the later annotation and controller
work:

- The first slice implements source-local provider graphs and direct-call
  constructor injection across the eight owned languages.
- ADR 0024 adds explicit, bounded cross-file source modules and strict rules
  for role naming and placement.
- ADR 0025 adds required-field inference through RequiredArgsConstructor.
  Authors write no constructors and no Inject lists.
- Execution hooks are still deferred.

Feature implementation, per-language runtime evidence, and release
qualification are separate things.

Status: transport boundary complete on 2026-08-09. The decision and security
limits are recorded in
[`adr/0014-environment-selected-yon-isolation.md`](adr/0014-environment-selected-yon-isolation.md).

Operators select the backend only through the parent environment. The choices
are the default process backend or a Firecracker control driver. HTTP
handlers, middleware, workers, and explicit invocation use that policy. Builds
never execute Yon.

Both backends keep Handler Protocol v1 framing, process-group cleanup,
deadlines, cancellation, concurrency, bounded output, and diagnostics. The
Firecracker path also passes a policy to the driver: bounded pool, CPU, and
memory values, and denied egress (no outgoing network traffic).

The current driver transport accepts only JavaScript and Python. TypeScript
and the prepared Java, PHP, Kotlin, C#, and Rust paths fail closed before the
driver process starts. This stays true until a bounded, authenticated
artifact-transfer contract exists.

This work does not yet claim a first-party Firecracker runtime or support for
hardware isolation. These remain future vertical slices:

- a production-qualified control program;
- a jailer/host profile;
- a guest image;
- a warm-pool lifecycle;
- snapshot lineage;
- native Linux evidence.
