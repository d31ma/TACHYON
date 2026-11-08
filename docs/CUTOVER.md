# Cutover Gate Status

"Cutover" means making the Rust implementation Tachyon's default. The cutover
gate is the list of conditions that must hold first.

## Current status: release candidate (2026-11-08)

Maintainer decision D1 (2026-10-31): the next version published from
`develop` is a **release candidate**. It is not a supported release and not a
preview. No target is promoted to `supported` or `preview`. Each target keeps
the tier that [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md) records for it, and a
promotion still needs the full promotion evidence listed there.

The candidate is published only after every gate in the table below has a
passing result on the release commit. These gates replace the open
reconciliation gate of 2026-08-30 (below) for this version.

- Release commit: the version commit `a17a1c79f` on `release/26.45.07`, on
  top of `develop` at `66ae003b2` with `main` (the 26.45.01 release commit)
  merged in. `develop` at `eaa638cbc` was merged after it as `8fa6cd034`:
  it pins the hosted runner images of `rust-ci.yml` and
  `fuzz-scheduled.yml` and changes no runtime code.
- Version: **26.45.07**
- 26.45.01 passed these gates on `5a6f31450` (2026-11-02), but it was never
  published: its publish run failed in the native artifact jobs, whose
  install, upgrade and rollback drill ran without Bun. Its results are in the
  history of this file on `release/26.45.01`.

A **pending** cell is filled with the date, the commit and the run (a CI run
link, or the name of the local run) when the gate runs on the release commit.

| Gate | Command | Result on the release commit |
| --- | --- | --- |
| Rust suite on macOS, Linux and Windows | `tachyon-ci --vms develop`: fmt, clippy `-D warnings`, the full nextest suite and rustdoc `-D warnings` in every lane | **Passed** 2026-11-08 on `a17a1c79f`: macOS 5,600 of 5,600 tests (local run `ci-20261108-0701`), Linux 5,578 of 5,578 and Windows 5,454 of 5,454 (`ci-20261108-0638`). On `8fa6cd034`, `cargo nextest run -p tachyon-contracts --test repository_policy --locked`: 40 of 40 |
| Website typing | `ty types website`, `bun run typecheck` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`, website group) |
| Website content, styles and language coverage | `website/scripts/build-content.mjs --check`, `build-styles.mjs --check`, `language-coverage.mjs --strict` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`) |
| Page size budget (TY1337) and responsive widths | `ty build website`, `ty preview --static`, `website/scripts/check-responsive.mjs` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`) |
| Browser gates | the `bun run test:*-browser` steps of the `rust-ci.yml` browser job, and `bun run test:website-browser` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`, browser group). The five `package.json` gates that release-extras does not run (native-bridge-bounds, python-tac-public-assets, server-probe, remotes-socket-browser, go-page-provider-identity-browser) passed in `release-26.45.07-drill-fu11`. Branded Safari 27.2: 9 of 9 (`release-26.45.07-safari-diag2`); Safari must not be already open, because it rejects automation sessions it did not launch; branded iOS Safari: 9 of 9 (`ci-20261108-1135`) |
| Website acceptance | `bun run test:website` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`) |
| Published snippets | `website/scripts/verify-snippets.mjs --no-cache` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`) |
| macOS native | `bun run test:phase4-macos` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-1555`, run from Terminal.app). Every other macOS group gate passed except the macOS release app (see below). `page-services-macos` is **pending**: every page service and WindowControls caller passed, but the FilePicker callers wait for a rerun with the fixed open-panel test script (the old script's final Return did not press Open on macOS 27.2) |
| iOS simulator | `scripts/ios/native-test.mjs` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-1135`) |
| Android emulator | `scripts/android/native-test.sh` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-1328`, with `ADB_MDNS=0`) |
| Linux GTK4 | `scripts/linux/native-test.sh`, `scripts/linux/application-state-test.sh` and the icon gate, in the pinned container | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-1453`, 14 of 14 linux steps) |
| Windows WebView2 | `scripts/windows/native-test.ps1` | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-1135`, 16 of 16 windows steps) |
| Native GUI tests that are ignored by default | `scripts/linux/compositor-test.sh`, `scripts/linux/wine-tests.sh`, the macOS GUI tests | **Passed** 2026-11-08 on `a17a1c79f`: compositor and Wine (20 of 20) in `ci-20261108-1453`, the macOS GUI tests in `ci-20261108-1555` |
| Fuzzing | the six `rust-ci.yml` fuzz targets, 300 seconds each | **Passed** 2026-11-08 on `a17a1c79f` (`ci-20261108-0718`) |
| Install, upgrade, rollback and uninstall | `scripts/release/lifecycle-drill.sh`, from v26.36.01 to the candidate and back to v26.36.01 | **Passed** 2026-11-08 on `a17a1c79f` (`release-26.45.07-drill-fu11`): every stage, and two packagings were bit-identical |
| Full CI on the `develop` to `main` pull request | `.github/workflows/rust-ci.yml` | **pending** |
| Signed, attested and verified assets | the sign, stage and verify jobs of `.github/workflows/publish.yml` | **pending** |
| macOS release app (Developer ID signing and notarization) | the ignored test `a_developer_id_macos_release_is_notarized_and_stapled` in `crates/tachyon-cli/tests/it/macos_release_cli.rs` | **Accepted exception** (2026-10-31): this machine has no notarization profile. v26.36.01 shipped no notarized macOS app either, only the `ty` executable with Sigstore attestations |

Flaky on 26.45.07's release commit (`a17a1c79f`): each failed once or
twice and then passed. The maintainer accepts or rejects each one before
the merge to `main`:

- `windows-page-checks`: in `ci-20261108-0718` every page step passed, then
  the UI Automation probe ran past its 600 second limit. It passed in
  `ci-20261108-1135`.
- `scripts/android/native-test.sh`: the system back check "Back goes to the
  previous route" failed in `ci-20261108-1251` and passed in
  `ci-20261108-1328`.
- The Linux GUI test
  `linux_generated_host_places_the_window_buttons_at_the_declared_inset`:
  it failed at different steps in `ci-20261108-0718` and
  `ci-20261108-1401`, and passed in `ci-20261108-1453`.

Passed before the release commit was chosen, on `develop` at `e5c2fc8d4`
(2026-10-31):

- The full nextest suite in the Linux lane (a linux/arm64 container): 5,103
  of 5,103 tests passed.
- The release candidate blockers of 2026-10-11 that needed code are closed:
  - the gateway runtime text test passes in that run;
  - `ty test` reports a unit that does not answer as failed and runs the other
    units;
  - a C# contract interface that is not public is refused (TY1308);
  - the website lists Java and Ruby on the browser HTTP transport.

Not passed, and not claimed by this release candidate:

- Before 2026-10-31 these gates had never passed on `develop`: published
  snippets, website acceptance, website browser typing, the page size budget,
  responsive widths, macOS native and iOS. The table above is their first
  run.
- No run used a physical iOS or Android device. iOS runs use the simulator
  without an Apple team ID.
- macOS app bundles that `ty bundle --release` builds are not notarized in
  this release candidate. The Developer ID and notarization gate is an
  accepted exception (see the table above).
- These browser and native UI gates are limited by the test devices, not
  passed (decided 2026-11-01):
  - Java web services on Android: the page-program, held-clients,
    cached-client, window-controls and web-apis cells of
    `branded-android-chrome`, and the Java cell of `wasm-features-app`.
    They need Android System WebView (Chrome) 137, and the test emulator
    (android-35 `google_apis`) has 124 and cannot update. Every other
    language passes in the same gates (16 of 17 cells in
    `wasm-features-app`).
  - The `android-web-view-refusal` refusal page: it needs WebView 83 or
    older to appear. The normal path passes.
  - `ios-system-back` on the iOS 26.5 simulator: an injected edge swipe
    starts WebKit's back gesture but does not complete it in three tries.
    `history.back()` works, a manual check with the simulator's own edge
    gesture went back, and the gate passed on iOS 18.0 (ADR 0109).
- The Linux desktop evidence comes from linux/arm64 containers, not from a
  physical x86_64 desktop.
- The Windows desktop gates other than `native-test.ps1` ran on ARM64 only.
- The Android gates ran with `ADB_MDNS=0`: adb 37.0.1 aborts in its mDNS
  code on macOS 27.2 when the emulator starts.
- A hung native service is not stopped by the native call time limit (see
  [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md), "Native apps reach Yon through a
  declared origin").

Human security review is still optional. Automated qualification of the exact
release commit is still required.

## Reconciliation gate (2026-08-30)

Release v26.35.07 completed the real
[Stable Release run 33301688118](https://github.com/d31ma/Tachyon/actions/runs/33301688118).
During the reconciliation audit, its downloadable artifacts passed checksum and
provenance verification. So the original provenance condition no longer waits
for a first release.

That result does not qualify the newer native architecture:

- The reconciliation in [`RECONCILIATION.md`](RECONCILIATION.md) must pass new
  gates before a new immutable version is published. These gates cover browser,
  schema, native-host, security, and downloaded-artifact behavior. For the
  release candidate, the gate table above is that check.
- Old widget and WebAssembly results cannot replace the host and companion
  gates of ADR 0018/0019 (ADR means architecture decision record).

## Historical cutover assessment (2026-08-01)

The cutover gate in [`PROJECT_PLAN.md`](PROJECT_PLAN.md) lists six conditions
that must all hold before the Rust implementation becomes Tachyon's default.

**All six conditions are met.** Condition 4 was the last one. It closed when
the tag workflow published v26.35.07 and v26.36.01; see section 4.

- Automated security qualification is the security gate before tagging.
- An independent human review is optional. It does not block merge, tagging,
  or publication.
- Meeting this historical gate does not qualify a new version. The
  release-candidate gates above decide that.

Every claim below is one of two things:

- an execution result, with the command that produced it;
- a maintainer decision, with the name of the person who made it.

Nothing here is a claim that the implementation makes about itself.

## Status

| # | Condition | Status | Basis |
| --- | --- | --- | --- |
| 1 | Every stable behavior has passing compatibility evidence or a documented migration | **Met** | Execution, re-verified 2026-07-31 |
| 2 | Supported platforms pass their native evidence profiles | **Met**, under the evidence standard amended below | Execution on three platforms, 2026-07-31; macOS and Windows named below |
| 3 | Install, upgrade, rollback, and uninstall have been exercised | **Met** for the `ty` artifact | Execution, re-verified 2026-07-31 |
| 4 | Release artifacts are signed, attested, and independently verifiable | **Met** | Execution: Stable Release runs 33301688118 (v26.35.07) and 33363597481 (v26.36.01); assets verified again outside the workflow, 2026-10-18 |
| 5 | No unowned critical threat-model finding remains | **Met** | Automated technical audit, remediation, and exact-head enterprise qualification; no critical or high blocker remains |
| 6 | Stable documentation describes the Rust implementation rather than plans | **Met** | Repository and website documentation rewritten and checked against executable behavior, 2026-08-01 |

## Evidence Standard — amended 2026-07-31

The original gate required:

- runs on physical devices for iOS and Android;
- a Linux runner on a native host;
- a recorded `windows-native` CI run.

It stated that simulators, emulators, and containers cannot replace any of
them.

**Amended by the maintainer (Samuel Ezenma, 2026-07-31):** simulator, emulator,
and pinned-container evidence satisfies conditions 2 and 3 for cutover.

What this changes and what it does not:

- It **does** accept that the generated hosts compile, launch, render native
  widgets, give accessible names to each platform's real accessibility API,
  and drive bound state. All of that was actually executed.
- It **does not** prove device-specific behavior. Real GPU drivers, vendor
  accessibility services, heat and memory pressure, and physical input were not
  tested. No run recorded here would catch a defect that only appears on
  hardware.
- [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md) has the same amendment. So a tier
  claim and this gate cannot disagree.

## 1. Compatibility Evidence — Met

`node scripts/compat/differential.mjs`, 2026-07-31:

```
3/3 corpus projects match across implementations
```

Both implementations render in a real browser. The script compares their route
graphs, semantic DOM, and HTTP status. These browser gates also pass:

- events;
- island expressions;
- offline service worker;
- the wasm application binary interface (ABI) probe;
- the five-language wasm companion gate.

`ty migrate check` reports `changed` and `unsupported` rows. Each finding has a
required action.

| Checked project | Supported | Changed | Unsupported |
| --- | --- | --- | --- |
| Archived migration fixture | 18 | 6 | 20 |
| This repository's migrated website, 2026-08-01 | 128 | 1 | 1 |

For the website, the changed finding is cross-document navigation. The
unsupported finding is telemetry. Telemetry is the remaining unsupported
product boundary recorded in [`PARITY_LEDGER.md`](PARITY_LEDGER.md).

On 2026-08-01, after work on dynamic routes and post-bundle compatibility, the
migrated website's public command ran again:

```text
bun run test
  24 pass, 0 fail, 160 expectations
```

The same Rust release binary produced the 11 web routes and the Android, iOS,
and macOS application bundles. Both `serve` and `preview` returned the concrete
dynamic `/docs/introduction` route, not a 404.

## 2. Native Evidence Profiles — Met, with macOS and Windows Named

Executed 2026-07-31 on a macOS arm64 test machine:

| Target | Command | Result |
| --- | --- | --- |
| iOS | `ty build --target ios`, `simctl install`, `simctl launch` on iPhone 17 Pro / iOS 26.5 | Launched as process ID (pid) 13540. Native large title, body, `UIButton`, bound output, `UITextField`, and disclosure rendered. Two taps moved the bound count `0 → 2`. Typing `Ada Lovelace` produced 12 `state.input` events. `<x-chart>` rendered as an isolated `WKWebView` subtree showing "Chart fallback". Every sibling stayed native. The lifecycle log recorded `controller.created → route.opened → websurface.created → websurface.attached → controller.mounted → controller.active → state.increment → state.input → controller.suspended`. |
| Android | `./scripts/android/native-test.sh` on the `tachyon-gate` emulator (API 36, `google_apis`, arm64) | `PASS: Android native gate`. 5 accessible names on native widgets. `Increase count` is a `Button` and `Your name` is an `EditText`. Activation through accessibility moved the bound output to `1`. The lifecycle log is complete. |
| Linux | `./scripts/linux/native-test.sh` inside the pinned `debian:trixie-slim` image | `PASS: Linux native gate`. The generated GTK4 host compiled under `-Werror` and launched under `Xvfb` and `dbus-run-session`. 7 declared accessible names reached AT-SPI (the Linux accessibility bus) with correct platform roles. AT-SPI activation incremented the bound state. |
| macOS | `node scripts/phase4-macos-test.mjs` | **Blocked, not failed.** The process that runs the probe needs macOS Accessibility permission. Without it the probe cannot read the window and reports: `phase4 accessibility probe: macOS Accessibility permission is not enabled for this process`. Grant it in System Settings → Privacy & Security → Accessibility and run again. |
| Windows | `scripts/windows/native-test.ps1` in [CI run 30714886154](https://github.com/d31ma/Tachyon/actions/runs/30714886154) | `PASS: Windows native gate`. The generated `PE32+` app compiled and launched. It showed its visible names through UI Automation (UIA) and a real `Button` HWND (window handle). It processed `BM_CLICK`, changed bound state `0 → 1`, recorded lifecycle, and closed. The managed UIA client reports only pane roles. This is still a named gap that blocks promotion. |

On 2026-08-01 the migrated website ran again:

- All 11 routes were built into iOS, Android, and macOS artifacts.
- iOS and macOS launched and rendered the homepage.
- On Android, real emulator taps navigated through `/docs` to the dynamic
  `/docs/introduction` route. The generated host recorded no fatal runtime
  exception.

This evidence tests same-origin route handoff from a WebSurface to native. The
earlier phase fixture did not contain that case.

## 3. Install, Upgrade, Rollback, Uninstall — Met

`./scripts/release/lifecycle-drill.sh`, 2026-07-31:

```
ok   the binary is bit-identical across builds
ok   every published file matches SHA256SUMS
ok   a modified artifact fails verification
ok   installed ty 26.33.01
ok   the installed tool built and published a project
ok   upgraded in place and still builds
ok   rolled back to ty 26.33.01 and still builds
ok   uninstall left nothing behind
PASS: release lifecycle drill
```

Before today the drill could not run from a clean checkout. Every gate script
in `scripts/` was committed without its executable bit. So the drill failed at
`build-artifact.sh: Permission denied`. CI's `./scripts/linux/native-test.sh`
would have failed the same way. The bits are set now.

The amended evidence standard covers the same lifecycle for generated native
application artifacts. Each platform's gate installs, launches, and drives the
generated application. It then removes the application together with the
emulator or container it ran in.

## 4. Signed, Attested, Independently Verifiable — Met

The tag workflow (`.github/workflows/publish.yml`) has published two stable
releases:

| Release | Run | Tag commit | Published |
| --- | --- | --- | --- |
| v26.35.07 | [33301688118](https://github.com/d31ma/Tachyon/actions/runs/33301688118) | `4a2e41a9` | 2026-08-30 |
| v26.36.01 | [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) (attempt 2) | `61589a7c` | 2026-08-31 |

Both runs finished with `success`. In each run, these jobs passed in order:

1. Five native release artifact jobs and the CycloneDX SBOM job attested what
   they built (`actions/attest-build-provenance`).
2. "Sign and stage immutable draft release" attested every distributed input,
   gave every asset a keyless Sigstore signature bound to the workflow
   identity, and created a non-public draft.
3. "Verify staged release" ran on five runners (macos-arm64, macos-x64,
   linux-x64, linux-arm64, windows-x64). Each one downloaded the draft and
   verified checksums, attestations, and workflow-identity signatures.
4. "Publish verified stable release" made the release public.

**Signed: met.** Every asset has a `.sigstore.json` bundle. In both runs,
`cosign verify-blob` checked every bundle on all five verify runners against
`publish.yml@refs/tags/v*` and the GitHub Actions OIDC issuer.

**Attested and independently verifiable: met.** On 2026-10-18 the assets of
both public releases were downloaded from GitHub into a clean Linux container
that had no part in the build, and checked there:

```bash
gh api repos/d31ma/Tachyon/releases/tags/<tag>   # download every asset
sha256sum --check --strict SHA256SUMS
for file in *; do
  [[ "${file}" == *.sigstore.json ]] && continue
  gh attestation verify "${file}" --repo d31ma/Tachyon
done
```

```
v26.35.07: 15 of 15 checksums OK, 16 of 16 attestations verified
v26.36.01: 15 of 15 checksums OK, 16 of 16 attestations verified
```

Each attestation names the signing workflow
`d31ma/Tachyon/.github/workflows/publish.yml@refs/tags/<tag>`, the tag commit,
and the run that built it. That container did not re-run `cosign verify-blob`
on the signature bundles. The verify runners in each release run did that.

**Reproducible: met.**

- Each artifact has `SHA256SUMS` over every published file.
- Each artifact has a `manifest.json` that records the release version, commit,
  `SOURCE_DATE_EPOCH`, target, and toolchain.
- Builds are reproducible. The bit-identical rebuild above proved this again
  on 2026-07-31. So a third party can rebuild from the recorded commit and compare
  digests without trusting the publisher.
- `rust-ci.yml` also publishes a CycloneDX software bill of materials (SBOM)
  and builds with `cargo auditable`.

These results prove the published v26.35.07 and v26.36.01 assets only. Each
new version needs its own tag run. Its draft assets must pass the same
verification before publication.

## 5. Threat-Model Findings — Met by Automated Qualification

On 2026-08-01 an independent automated technical audit found two
high-severity issues:

- Descendant handler processes could escape lifecycle control.
- Linux WebSurfaces accepted any `file://` navigation.

Both were fixed. The technical re-review found no remaining critical or high
blocker.

What exists instead, from [`PHASE_7_EVIDENCE.md`](PHASE_7_EVIDENCE.md):

- 7,197,692 fuzz executions across four trust boundaries under
  `AddressSanitizer`, with zero crashes;
- a clean sanitizer run;
- five recovery drills.

[`SECURITY_REVIEW_PACKAGE.md`](SECURITY_REVIEW_PACKAGE.md) contains the
technical audit record and the automated qualification criteria.

The stable tag requires two things:

- a clean security matrix on the exact head commit;
- an explicit owner and decision for every critical or high finding.

Independent human review is still welcome. It is not a release gate.

## 6. Stable Documentation — Met

These documents now describe the Rust implementation: `README.md`,
`CONTEXT.md`, the architecture overview, support tiers, cutover status, and the
migrated website.

Legacy claims about OpenAPI, telemetry, the client renderer, the runtime cache,
and native hosts were handled in one of two ways. They were removed from the
stable README, or they were named explicitly as compatibility decisions.

These items were checked against the executable implementation and its recorded
evidence:

- commands, ports, and output paths;
- Tac client-render and component behavior;
- Yon's REST-only boundary and the empty Route Manifest v1 compatibility field;
- real-language Wasm companions;
- native-first rendering and per-subtree fallback;
- security boundaries;
- support vocabulary.

## What Happens Next

1. Run the release-candidate gates on the release commit and record each
   result in the table above.
2. Open the `develop` to `main` pull request and complete its CI matrix.
3. Run the release workflow from the qualified immutable tag.
4. Verify its staged assets.

The macOS Accessibility automation is still a named local evidence gap. Do not
present it as completed platform promotion work.

[`RELEASE_ENGINEERING.md`](RELEASE_ENGINEERING.md) sets two rules. `main` stays
the released implementation until that pull request is merged. History is never
rewritten.
