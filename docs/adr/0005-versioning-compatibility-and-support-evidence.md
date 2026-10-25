# ADR 0005: Versioning, Compatibility, and Support Evidence

- Status: Accepted
- Date: 2026-07-26

## Context

The rewrite must:

- let machine contracts change over time;
- keep the current developer workflow where that is intended;
- later replace a released implementation.

It must do this without treating build output as proof of platform support.

Tachyon already uses a UTC CalVer (calendar versioning) product version. Cargo
packages need package metadata that is compatible with SemVer (semantic
versioning). Public schemas need their own major versions.

## Decision

- Keep Tachyon's UTC CalVer identity for product releases.
- Use internal Cargo package versions only as implementation metadata.
- Record these values in Artifact Manifest v1: the exact product version, the
  commit, the toolchains, the contracts, the target, the inputs, the outputs
  and the digests.
- Give each public machine contract its own version, stored in a directory per
  major version.
- Within one major contract version, prefer changes that only add things.
- A breaking contract change requires an RFC (request for comments), a
  migration, compatibility tests and a deprecation window.
- Keep the current public behavior with neutral compatibility fixtures. Do not
  import legacy internals for this.
- Use the support vocabulary in `docs/SUPPORT_TIERS.md`.
- Never promote a platform only because cross-compilation works.

During Phase 0, the rewrite version is `0.0.0-phase0`. Tachyon must not publish
this version as a preview or stable release.

## Consequences

- Product versions and internal package versions can change separately, and
  nobody mixes them up.
- Each artifact describes itself, and anyone can verify it on its own.
- Marketing claims about a platform need evidence. Release engineering owns
  that evidence.
- Every difference from the legacy implementation must be intentional,
  documented and testable.
- Contract consumers can negotiate compatibility without parsing text written
  for humans.

## Rejected Alternatives

- **One version for every schema and package**: unrelated compatibility
  changes would have to happen at the same time.
- **Semantic versioning for the public product immediately**: it would change
  the current Tachyon release identity for no need.
- **Support on successful cross-build**: a cross-build gives no evidence that
  the native runtime works.
- **Big-bang replacement without dual-run fixtures**: replacing everything at
  once, without fixtures that run on both implementations, hides regressions
  until cutover.

## Acceptance Gate

- Artifact Manifest v1 records the required provenance fields;
- the release and support documents use the same vocabulary;
- the rewrite documentation clearly separates the legacy baseline, preview and
  stable cutover;
- no Phase 0 workflow publishes a Tachyon release.

## Amendment (2026-10-18): native builds are incremental and reproducible

- A native build reuses the web stage of the previous build of the same
  target. Before it compiles the web stage, it copies the published app's
  `web/` folder (which holds `.tachyon/build-state.json`) into its temporary
  stage. The web compiler then reuses only routes whose inputs and artifact
  digests still match, exactly as a web build does. `--no-incremental` skips
  the copy and compiles every route. A missing, symlinked or unreadable
  previous folder means a full build, never a failure.
- Every native tool runs with `SOURCE_DATE_EPOCH` (0 when unset, the same
  value the Artifact Manifest records) and `ZERO_AR_DATE=1`.
  - `cc` (Linux) gets `-ffile-prefix-map=<source folder>=.` and `rustc`
    gets `--remap-path-prefix=<source folder>=.`, so the random staging
    folder never reaches a binary.
  - `swiftc` (macOS, iOS) runs in the source folder with relative source
    paths and `-Xfrontend -disable-incremental-llvm-codegen`. Its
    prefix-map flags do not remap `#filePath`, and incremental LLVM code
    generation hashes a random temporary object path into the binary.
  - MSVC compiles and links with `/Brepro`. The C# companion project sets
    `Deterministic`, `ContinuousIntegrationBuild` and a `PathMap`.
  - Gradle archives use `isPreserveFileTimestamps = false` and
    `isReproducibleFileOrder = true`.
- Build intermediates are not outputs: the Windows host's `.obj` file, the C#
  companion's `bin` and `obj` folders and Gradle's `build`, `.gradle` and
  `.kotlin` folders are removed after the app is copied out of them.
- `native_reproducible_build_tests.rs` builds the same project twice into two
  folders and compares the `artifact-manifest.json` bytes, then builds a third
  time with reuse and compares again. Staged apps for all five targets run
  everywhere. The packaged Linux test ran in a cloud container with GTK 4 and
  WebKitGTK 6.0 and matched. The packaged macOS, iOS, Windows and Android
  tests are `#[ignore]`d lane tests; until they run on those machines, the
  support table lists those targets as `adapted` with a note. Android debug
  APKs are signed with the machine's own debug key, so they match only on the
  same machine.
- Cargo-built Rust crates (`tac.application.rust` crates) keep cargo's own
  path handling. Cargo passes the crate's own files as relative paths, and a
  remap flag in `RUSTFLAGS` would rebuild every vendored dependency on each
  build.

## Amendment (2026-10-19): web builds write an Artifact Manifest too

- A web build (`ty build`, `ty bundle --target web`) writes
  `artifact-manifest.json` at the root of its output, after the precompressed
  files. It lists every other output file with its SHA-256 and size, sorted by
  path, with the same product version, commit (`GITHUB_SHA`) and
  `SOURCE_DATE_EPOCH` as a native build. Its target is `os` `web`,
  `architecture` `universal` and `abi` `browser`; its toolchain is `rust`, and
  it names no `native_host` contract. Artifact Manifest v1 accepts `web` as an
  `os` value; nothing else in the contract changes.
- The web stage of a native build writes none: the native build's own
  manifest already lists every file under `web/`, and a nested manifest would
  turn a style-only change into a restart for native UI updates.
- Two web builds of the same sources write the same `artifact-manifest.json`
  bytes, so byte-identical checks cover the web the way they cover native
  builds (`web_artifact_manifest.rs`).
- The file is served with the rest of the site, like `route-manifest.json`.
  It names only files that are already public.
- The commit, `SOURCE_DATE_EPOCH` and Rust version now come from one place
  (`compiler/artifact_provenance.rs`) for web and native builds.

## Amendment (2026-10-19): the Windows repeat build ran

This amendment supersedes one sentence of the 2026-10-18 amendment above,
which stays as the history of this decision: "The packaged macOS, iOS,
Windows and Android tests are `#[ignore]`d lane tests; until they run on
those machines, the support table lists those targets as `adapted` with a
note."

- `a_packaged_windows_app_is_byte_identical_when_built_twice` passed with
  MSVC on Windows 11 ARM64 over SSH (RW1, `docs/SUPPORT_TIERS.md`, Windows
  over SSH), so the support table marks Windows byte-identical rebuilds
  `honored`.
- The macOS, iOS and Android tests have not run on their machines yet. The
  table marks those targets `runOwed` (ADR 0102 amendment), not `adapted`,
  and names each test as the owed run.

## Amendment (2026-10-24): the Android repeat build ran

This amendment supersedes "The macOS, iOS and Android tests have not run on
their machines yet" in the 2026-10-19 amendment above, for Android only; that
sentence stays as the history of this decision.

`a_packaged_android_app_is_byte_identical_when_built_twice` passed on this
Mac with `ANDROID_HOME`, Gradle 8.14.3 and JDK 21 (RM3, `docs/SUPPORT_TIERS.md`, "Android runs owed, RM3"): two fresh
packaged builds and an incremental one gave the same
`artifact-manifest.json`. The support table marks Android byte-identical
rebuilds `honored`. The macOS and iOS tests are still owed.

## Amendment (2026-10-25): the macOS and iOS repeat builds ran

This amendment supersedes "The macOS and iOS tests are still owed" in the
2026-10-24 amendment above, which stays as the history of this decision.

On 2026-10-24, on a Mac (macOS 26.6.2, Xcode 27.1),
`native_reproducible_build_tests::a_packaged_macos_app_is_byte_identical_when_built_twice`
passed in RM1, and `a_packaged_ios_app_is_byte_identical_when_built_twice`
passed in RM2 (`docs/SUPPORT_TIERS.md`, "macOS runs owed, RM1" and "iOS
simulator runs"). With the Linux, Windows and Android runs above, every
native target has a repeat-build run, and the support table marks
byte-identical rebuilds `honored` on each. A signed `--device` or `--release`
iOS `.ipa` is still never byte-identical, because its signature differs.
