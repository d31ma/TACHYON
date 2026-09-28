# Phase 5 Evidence

This document records how [`PHASE_5_SPEC.md`](PHASE_5_SPEC.md) was validated.

- Every claim below names the command that produced it and the kind of machine
  it ran on.
- A claim without recorded evidence is listed in §7 as an open gap. It is not
  counted as done.

The build machine for the evidence run locally:

- macOS 26 (Darwin 25.5.0), Apple silicon
- Xcode 26.6, Swift 6.3.3
- Rust 1.97.1
- Docker 29.6.2
- `x86_64-w64-mingw32-gcc` 16.1.0

## 0. Re-verification, 2026-07-31

On that date every runnable profile below ran again. Each one gave the same
result as recorded:

- iOS on iPhone 17 Pro / iOS 26.5. The app launched as process ID (pid) 13540.
  The count went `0 → 2`. There were 12 bound input events and an isolated
  `WebSurface`. The lifecycle log ended with `controller.suspended`.
- Android on the `tachyon-gate` emulator.
- Linux in the pinned container.

macOS is blocked. The process that runs the probe needs Accessibility
permission, and that permission was not granted. Windows native execution was
recorded later, on 2026-08-01, as described in §4. The gate status that uses
these results is in [`CUTOVER.md`](CUTOVER.md).

## 1. Automated Gate

`cargo test --workspace --all-targets --all-features --locked`

| Suite | Coverage |
| --- | --- |
| `native::host` | literal escaping for Swift, Kotlin, C, and XML; bounded toolchain version banners |
| `native::ios` | UIKit-only host source, lifecycle events, no script message handler, launchable simulator plist, build-machine triple |
| `native::linux` | GTK4 adapters, ephemeral `WebKitGTK` session, `gtk_accessible_update_property` usage, no script message handler, valid `GApplication` id, desktop entry |
| `native::windows` | deterministic repeated lowering, embedded state and action tables, common-controls and per-monitor DPI manifest, host-appropriate compiler selection |
| `native::android` | valid Java packages for hyphenated, numeric-leading, and reserved segments; platform-view host with no `addJavascriptInterface`; pinned Gradle and manifest surface |
| `phase5_cli` | per-target output isolation, iOS bundle shape and signature, Apple-target semantic parity, identical planning diagnostics across all five targets |

The Apple-parity test enforces spec §1.2 automatically. Two targets must lower
one project to the same Native UI v1 document. Only the target tag and the
fallback reason that names the platform may differ. If anything else differs,
the test fails.

## 2. iOS — simulator-tested

Toolchain: `xcrun --sdk iphonesimulator swiftc`, target
`arm64-apple-ios17.0-simulator`. Device: iPhone 17 Pro, iOS 26.5
(`063A637F-1E5B-4C4C-BC21-9FF8206D7E4C`).

```bash
ty build <project> --target ios
xcrun simctl install <udid> dist/ios/PhaseFive.app
xcrun simctl launch <udid> dev.tachyon.phase-five
```

Result: `Built ios app with 1 routes (native_nodes=24 web_surfaces=1)`.

| Claim | Observation |
| --- | --- |
| The bundle installs and launches | `simctl launch` returned pid 78562 |
| Native adapters render | Screenshot shows a native large-title heading, body text, `UIButton`, bound output, `UITextField`, and `DisclosureGroup` |
| Button dispatches `increment` | Two taps on "Add one" moved the bound `count` output from `0` to `2` |
| Text field binds input | Typing `Ada Lovelace` into the native field populated the bound `name` state |
| Disclosure toggles | Tapping "More detail" revealed "Disclosure content." |
| Fallback is a subtree, not the page | `<x-chart>` rendered as an isolated local-bundle `WKWebView` showing "Chart fallback" while every sibling stayed native |
| Lifecycle is recorded | `Library/Logs/Tachyon/dev.tachyon.phase-five.jsonl` contained `controller.created`, `route.opened`, `websurface.created`, `websurface.attached`, `controller.mounted`, `controller.active`, and `controller.suspended` after backgrounding |

The simulator can stop the process without a graceful notification. In that
case `controller.destroyed` is not recorded. Spec §3.1 says this is expected
iOS behavior. It does not count as evidence for or against.

Tier: **simulator-tested**. It is not `native-tested`, for two reasons:

- the artifact is built against the simulator software development kit (SDK);
- no run on physical hardware has been recorded.

## 3. Linux — container-tested

Environment: `scripts/linux/Dockerfile` (Debian trixie, GTK4, `WebKitGTK` 6.0,
json-glib, at-spi2-core, Rust 1.97.1).

```bash
docker build -f scripts/linux/Dockerfile -t tachyon-linux-gate:1.97.1 scripts/linux
docker run --rm --cap-add SYS_ADMIN --cap-add NET_ADMIN \
  --security-opt seccomp=unconfined --security-opt apparmor=unconfined \
  -v "$PWD":/workspace -w /workspace tachyon-linux-gate:1.97.1 \
  ./scripts/linux/native-test.sh
```

Result: `PASS: Linux native gate`.

| Claim | Observation |
| --- | --- |
| The generated host compiles | `cc -std=gnu17 -O2 -Wall -Wextra -Werror` succeeded against GTK4, `WebKitGTK` 6.0, and json-glib |
| Packaging is complete | `bin/PhaseFive`, `resources/NativeIndex.json`, `resources/NativeUI/root.json`, and `dev.tachyon.phase-five.desktop` were published |
| The application runs headlessly | Launched under `Xvfb` and `dbus-run-session`; the window presented and the lifecycle log recorded `controller.created`, `route.opened`, `controller.mounted`, and `controller.active` |
| Accessible names reach the platform | `scripts/linux/a11y-probe.py` read back all seven declared names over AT-SPI (the Linux accessibility bus): `Phase Five demo`, `Phase Five`, `Increase count`, `Count`, `Your name`, `More detail`, `Sales chart` |
| Semantics map to native roles | `Increase count` → `button`; `Your name` → `text` |
| Interaction works through assistive technology | `Atspi.Action.do_action` on the button moved the bound state to `1`. The tree showed this as a new accessible name. |
| The surface is isolated | `WebKitGTK` ran with a temporary (ephemeral) network session. The fallback document had `default-src 'none'`. |

This evidence run found two defects and fixed them. They were not ignored:

1. Accessible names set on plain GTK containers never reached AT-SPI. GTK4
   treats a generic container as presentational only. Named containers are now
   wrapped in a frame with the group role.
2. A button built with its own label took its accessible name from that label.
   It ignored the declared name. The text is now given as a child that is
   hidden from accessibility, so the declared name wins.

Tier: **container-tested**. `SUPPORT_TIERS.md` says a container does not
replace evidence from a native host. So this is not `native-tested`.

## 4. Windows — native execution recorded, promotion blocked

```bash
ty build <project> --target windows
file dist/windows/PhaseFive/bin/PhaseFive.exe
```

| Claim | Observation |
| --- | --- |
| The generated host cross-compiles | `x86_64-w64-mingw32-gcc (GCC) 16.1.0` with `-Wall -Wextra -Werror -municode -mwindows` |
| The artifact is a real Windows binary | `PE32+ executable (GUI) x86-64, for MS Windows` |
| The manifest is accurate | `target` = `{os: windows, architecture: x86_64, abi: win32}`; toolchain recorded as `mingw-w64-gcc (GCC) 16.1.0` |
| Packaging is complete | `bin/PhaseFive.exe`, `bin/PhaseFive.exe.manifest`, and the full `resources/` tree were published |
| The generated host executes on Windows | The `windows-native` job in [run 30714886154](https://github.com/d31ma/Tachyon/actions/runs/30714886154) built, launched, inspected, controlled, and closed the generated application on `windows-latest` |
| UI Automation reaches the generated HWNDs (window handles) | UI Automation (UIA) returned `Phase Five`, `Add one`, live output `0`, and `Sales chart`. The HWND class behind the button was `Button`. |
| Native interaction and state work | A native `BM_CLICK` moved the bound output from `0` to `1` and recorded `state.increment` |
| Lifecycle is recorded | The log contained `controller.created`, `controller.active`, and `controller.destroyed` |

This is real native execution evidence. It is not a promotion to
`native-tested`.

- The hosted runner's managed UI Automation client reports every standard child
  HWND as `ControlType.Pane`.
- So the run cannot prove UIA roles or `InvokePattern` activation.
- The gate therefore proves these as separate facts: the UIA names, the real
  native HWND class, native Win32 interaction, state binding, and lifecycle.

The accessibility-role gap stays listed in §7.

## 5. Android — emulator-tested

Toolchain: Gradle 8.14.5, JDK 21, Android Gradle Plugin 8.7.3, build-tools
35.0.0, compileSdk 35, minSdk 26. Device: headless `pixel_6` emulator,
`system-images;android-36;google_apis;arm64-v8a`, Android 16.

```bash
ANDROID_HOME=<sdk> ./scripts/android/native-test.sh
```

Result: `PASS: Android native gate`.

| Claim | Observation |
| --- | --- |
| The APK (Android package) assembles | `gradle assembleDebug` produced `PhaseFive.apk`. It uses no Kotlin plugin and no third-party runtime dependency. |
| It installs and launches | `adb install` succeeded. `am start -W` reported `Status: ok`, `LaunchState: COLD`, `TotalTime: 924`. |
| Adapters map to platform widgets | `uiautomator` reported `Increase count` → `android.widget.Button` and `Your name` → `android.widget.EditText` |
| Accessible names reach the platform | `Phase Five demo`, `Increase count`, `Count`, `Your name`, and `More detail` were present as `content-desc` on native nodes |
| Button dispatches `increment` | Native taps moved the bound `count` output from `0` to `2`, with one `state.increment` recorded per tap |
| Text field binds input | Typing `Ada` into the native `EditText` produced `state.input` events and showed the bound value |
| Disclosure toggles | Tapping `More detail` revealed "Disclosure content." and recorded `state.disclosure` |
| Fallback is a subtree | `<x-chart>` rendered inside an isolated `WebView` showing "Chart fallback". Every sibling stayed native. |
| Lifecycle is recorded | `files/tachyon/dev.tachyon.phase-five.jsonl` contained `controller.created`, `route.opened`, `websurface.attached`, `controller.mounted`, and `controller.active` |

On 2026-08-01 the migrated 11-route website was rebuilt and tested on the
same emulator:

- The generated APK launched without an application exception.
- The mobile menu opened and closed.
- The light/dark theme changed.
- Physical taps navigated from `/` to `/docs`, and then to the dynamic
  `/docs/introduction` route.
- Dark system-bar icons stayed visible on the light status bar.
- The other Chromium log entries were WebView environment warnings (DNS and
  configuration, and cache setup). They were not process failures.

Tier: **simulator-tested** (emulator). It is not `native-tested`, for two
reasons:

- no run on physical hardware is recorded;
- the APK is a debug build signed with the debug key.

## 6. Continuous Gates

| Job | Platform | Proves |
| --- | --- | --- |
| `native` | Linux, macOS, Windows | format, check, Clippy, tests, rustdoc on the exact toolchain |
| `linux-native` | Linux container | GTK4 compilation, headless launch, AT-SPI names, roles, and activation |
| `windows-native` | `windows-latest` | Win32 compilation, launch, UI Automation names, native HWND classes, `BM_CLICK` activation, bound state, lifecycle |
| `android-native` | Linux + emulator | APK assembly, install, launch, `uiautomator` names, native tap, lifecycle |
| `macos-native` | `macos-latest` | Phase 4 macOS launch, accessibility, lifecycle, and visual parity |

## 7. Open Gaps

| Gap | What closes it |
| --- | --- |
| iOS in CI | A `macos-latest` job booting a simulator and running the iOS gate |
| iOS and Android on physical hardware | Device provisioning, distribution signing, and a release keystore |
| Linux on a host rather than a container | A native Linux runner executing `scripts/linux/native-test.sh` |
| Windows UI Automation roles and accessibility activation | A client or generated provider that reports the standard controls with their semantic UIA roles and exposes `InvokePattern`. The current hosted client reports them all as panes. |
| Windows accessible names distinct from visible text | A UI Automation provider for the generated controls, per spec §6 |
| Embedded `WebSurface` on Windows | The `WebView2` viability gate, per spec §6 |
| Any platform at `supported` | Install, upgrade, rollback, and uninstall exercises plus a published support window, per `SUPPORT_TIERS.md` |

This document claims no platform as `supported`. No Phase 5 artifact is a
Tachyon preview or release.
