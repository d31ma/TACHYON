# Tachyon Support Tiers

## Vocabulary

Each target has one support tier. The tiers are:

- **Buildable**: the toolchain emits an artifact for the target.
- **Simulator-tested**: behavior passes in an emulator or simulator.
- **Native-tested**: the released artifact executes on the target operating
  system and architecture.
- **Preview**: the artifact is distributed for evaluation. The production
  gates it is missing are documented.
- **Supported**: these gates pass on native infrastructure: installation,
  contracts, security, crash behavior, upgrade, rollback, accessibility,
  packaging, and soak (long-running load).

One label never replaces another. Containers do not replace filesystem and
process tests on a native host. Emulation does not replace evidence from a
device or the native architecture when a target claims `supported`.

## Reconciliation candidate evidence (unpublished)

These results qualify only the working candidate. They do not promote a
support tier. They do not replace verification of the real release download.

| Target host | Observed execution | Remaining qualification |
| --- | --- | --- |
| macOS WKWebView | full arm64 application gate passed: controls, accessibility, typing, lifecycle and visual parity at 420x752 | exact release artifact and current-head CI |
| iOS WKWebView | isolated iPhone SE (3rd generation), iOS 26.5. Passed: Swift calls, publish, input/focus, assets and route state `7 -> 9 -> 7` | exact release artifact; support on physical devices is still not claimed |
| Android WebView | Android 15 emulator. Passed: Kotlin calls, publish, input/focus per character with a delayed setter, service-worker retirement and packaged dynamic routes | exact release artifact; support on physical devices is still not claimed |
| Linux WebKitGTK | Rust native service compiles. The shared route/security probe passes. | Linux native GUI CI execution |
| Windows WebView2 | generated C compiles with the official software development kit (SDK). Real C# protocol/OS execution. | Windows native GUI CI execution |

### Application services and declared menus on desktop hosts (2026-10-04)

ADR 0097 and its amendment cover an application-lifetime Rust service and
`tac.application.menus`. The service lives inside the app process only; no
host creates an operating system service, daemon, login item or autostart
entry. This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| macOS WKWebView | yes | the generated AppDelegate is compiled and run with a real Rust library (ignored SDK-lane tests in `native_application_host_tests` and `native_menu_host_tests`) |
| Linux WebKitGTK | yes | `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and a real Rust library, and ran it under Xvfb on WSL Ubuntu 24.04 (aarch64), passing three of three runs: the service started once at launch, a menu action reached the page service without a reload, the item was disabled with no recipient, Help and Home kept the service, and Close Window stopped it once. Not covered: a physical Linux desktop, Wayland, x86_64, and clicks or keys by a person. WSLg's display did not accept connections from the remote test lane, and with WebKit's sandbox on, the first page load took up to about 80 seconds on that machine |
| Windows WebView2 (x64) | yes, generated only | generated-source tests, and the Win32 menu code compiled and linked with MinGW-w64 GCC (`-Wall -Wextra -Werror`) on macOS. Not run: the MSVC `/W4 /WX` build of the whole host, the WebView2 `AcceleratorKeyPressed` handler, and any run on Windows. These need Visual Studio Build Tools (MSVC), the Windows SDK and the WebView2 SDK 1.0.3485.44 (`TAC_WEBVIEW2_SDK`) on a Windows machine |
| iOS, Android | refused at build time | `TY1601` for menus, `TY1406` for the application lifetime |

Windows on ARM64 was evaluated, not implemented. The WebView2 runtime ships
for ARM64 (the test machine has 154.0.4258.53), the WebView2 SDK package has
an `arm64` library folder, Rust has the `aarch64-pc-windows-msvc` target, and
.NET publishes `win-arm64`. The generated C has no x64-only code. The build
fixes x64 in four places: the Rust companion target `x86_64-pc-windows-msvc`,
the WebView2 library folder `build/native/x64`, the C# runtime identifier
`win-x64`, and the `cl` of an x64 developer prompt. An ARM64 build needs a
target choice for those four values and the MSVC ARM64 tools. Until it is
built and run, x64 stays the only tested Windows target; an x64 app on
Windows 11 ARM64 runs under emulation, which is also not tested.

Use `scripts/phase4-macos-test.mjs`, `scripts/ios/native-test.mjs`,
`scripts/android/native-test.sh`, `scripts/linux/native-test.sh` and
`scripts/windows/native-test.ps1` to reproduce each platform gate.

The final macOS/iOS/Android gates used the same frozen candidate executable.

- Linux and Windows GUI CI on the current head is still pending.
- Compilation and protocol execution do not prove GUI qualification.
- The API 36/Android 16 results below are historical. They do not describe the
  final Android run.

Some failures passed later without any change:

- An earlier Apple helper run, with no change, timed out in three cases. Later
  runs passed.
- Android installation failed once on boot readiness. It passed on a retry with
  no change.

The causes of both are still not confirmed. Passing reruns do not prove that
either temporary failure is fixed.

Android runtime limits:

- Android requires a System WebView that supports `WEB_MESSAGE_LISTENER`.
- The AndroidX WebKit `1.14.0` bridge has no legacy fallback.
- An unsupported runtime reports that the bridge is unavailable. It needs a
  compatible WebView.

Native calls have a ten-second browser wait limit. This limit does not stop or
roll back native service code. Services must stay responsive. A hung
application may need to be relaunched.

## Historical Phase 5 Native Target Status

**Reconciliation notice (2026-08-30):**

- ADRs (architecture decision records) 0018 and 0019 replace the widget planner
  and WebAssembly companions. The replacements are platform web-view hosts and
  native page companions.
- The tables below are historical evidence for the previous architecture. They
  do not qualify the new hosts.
- The reconciliation release needs new platform evidence before it is
  published. See [`RECONCILIATION.md`](RECONCILIATION.md).
- The architecture change by itself promotes no support tier.

Evidence and reproduction commands are in `docs/PHASE_5_EVIDENCE.md`.

| Target | Status | Evidence |
| --- | --- | --- |
| macOS `SwiftUI` | native-tested development milestone | real `.app` launch, Accessibility, lifecycle, and screenshot parity |
| iOS `SwiftUI` | simulator-tested | signed simulator `.app` launched on iPhone 17 Pro / iOS 26.5; observed native button, text field, disclosure, isolated `WebSurface`, and lifecycle |
| Android views | simulator-tested (emulator) | debug APK launched on a headless `pixel_6` / Android 16 emulator; `uiautomator` confirmed native `Button` and `EditText`, declared content descriptions, native tap increment, and lifecycle |
| Linux GTK4 | container-tested | pinned Debian trixie container compiled the host with warnings denied and launched it headlessly. It read back over AT-SPI (the Linux accessibility bus) seven declared names, native roles, and an `Atspi.Action` activation |
| Windows Win32 | buildable with native execution evidence | `windows-latest` compiled, launched, inspected, controlled, and closed the generated `PE32+` app. Passed: UI Automation (UIA) names, a `Button` HWND (window handle) behind the button, bound state, and lifecycle. Semantic UIA roles are still an explicit gap that blocks promotion |

No Phase 5 target is `supported` in the vocabulary above. The vocabulary has
not changed:

- Simulator and emulator evidence still does not prove behavior on physical
  devices.
- A container still does not prove behavior on a Linux host.

**Amendment, 2026-07-31 (Samuel Ezenma, maintainer):** the *cutover gate*
accepts simulator, emulator, and pinned-container evidence for its native
evidence condition. This decision is about what blocks cutover. It is not a
promotion. A target reaches `supported` only through the promotion evidence
below. See [`CUTOVER.md`](CUTOVER.md). It records the same amendment and what
the amendment does not prove.

These targets were verified again by execution on 2026-07-31:

- iOS on iPhone 17 Pro / iOS 26.5;
- Android on the `tachyon-gate` emulator (API 36);
- Linux in the pinned `debian:trixie-slim` image.

macOS is blocked. The process that runs the probe needs Accessibility
permission, and it was not granted. The `windows-native` job in
[run 30714886154](https://github.com/d31ma/Tachyon/actions/runs/30714886154)
recorded Windows execution. The target is not promoted because semantic UIA
roles are still not proven.

On 2026-08-01 the migrated 11-route website was also rebuilt and launched:

- The iOS simulator and the macOS application rendered the complete homepage
  after their isolated surfaces finished loading.
- The Android emulator rendered the same mobile layout. Physical emulator taps
  navigated from `/` to `/docs`. They then resolved the dynamic
  `/docs/introduction` route through the native route stack.
- On the rebuilt APK, the mobile menu, theme control, and light system-bar
  appearance were also tested. No fatal application log entry appeared.

This adds evidence at the application level. It does not promote any tier.

The same website source also bundled successfully with the archived legacy
26.30.04 binary. It used the ordinary `ty bundle` command and an isolated
`YON_DIST_PATH`. Both implementations published the same 11 authored HTML
routes. The differential corpus still decides browser semantic parity.

## Phase 4 Status

| Surface | Status | Reason |
| --- | --- | --- |
| Rust workspace and Phase 4 CLI on macOS arm64 | native-tested development milestone | real binary, HTTP, Node.js, CPython, Chromium, SwiftUI application, Accessibility, lifecycle, and screenshot evidence executed locally |
| Rust workspace and Phase 4 CLI on Linux x86_64 | container-tested development milestone | the locked, platform-neutral Rust/process suite runs in a pinned official Linux container; macOS packaging correctly still requires a macOS host |
| Rust workspace and Phase 4 CLI on Windows x86_64 | buildable | the GNU target compiles locally; `windows-latest` CI still enforces real Node.js and CPython execution |
| Static web compiler and development server | implemented milestone | evidence recorded in `docs/PHASE_1_EVIDENCE.md`; not published |
| Eight-language Yon handler boundary | implemented milestone | plain `Handler` convention; explicit `relay(...)`; native generator streaming in JavaScript, TypeScript, Python, PHP, Kotlin, and C#; Java and Rust return one response |
| Tac client rendering, components, mount schedules, and incremental builds | implemented milestone | evidence recorded in `docs/PHASE_3_EVIDENCE.md`; Yon remains a separate REST boundary |
| HTTP handler dispatch | implemented milestone | exact and dynamic routes, methods, parameters, bodies, middleware, and bounded topic streams execute through the supervised handler boundary |
| Environment-selected Yon isolation transport | implemented milestone | tested: process compatibility mode, and a fail-closed Firecracker control-driver transport for JavaScript and Python. TypeScript and prepared direct-language artifacts are rejected before the driver starts, until an artifact-transfer contract exists. The driver and the Linux/KVM host are not qualified yet, so this is not a claim of hardware-isolation support |
| macOS SwiftUI native vertical slice | native-tested development milestone | evidence recorded in `docs/PHASE_4_EVIDENCE.md`; ad-hoc development signing only |
| Android, iOS, Windows, and Linux native renderers | implemented | see the Phase 5 table above |

No Phase 4 artifact is a Tachyon preview or supported release.

- In the default process mode, the handler child process is supervised and gets
  a restricted environment. It still has the filesystem and network access of
  the developer's account.
- The Firecracker driver transport does not meet the gate for OS-level resource
  and capability isolation. It will meet it only when its control program and
  host have native qualification evidence.
- The Firecracker driver transport also does not carry prepared TypeScript,
  Java, PHP, Kotlin, C#, or Rust artifact sets. For now, those languages need
  process mode.

## Promotion Evidence

A target cannot become supported without:

- clean installation from the release archive;
- verification of the artifact digest, signature, software bill of materials
  (SBOM), and provenance;
- CLI and contract corpus execution;
- paths containing spaces, Unicode, and platform separators;
- filesystem permission and symlink behavior;
- child-process cancellation, crash, signal, and reaping behavior;
- native UI interaction and accessibility evidence where applicable;
- upgrade from every supported release line;
- safe rollback or an explicit irreversible-migration boundary;
- bounded load and soak results on named reference hardware.
