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

These results qualify only the working candidate. The Tier column names the
tier that this evidence reaches. "(candidate)" means that the working
candidate ran, not the published release download. A release has the tier
only after the same gate passes on its download. No target is `preview` or
`supported`.

| Target host | Tier | Observed execution | Remaining qualification |
| --- | --- | --- | --- |
| Web (static site in `dist/web`) | native-tested (candidate) | `rust-ci.yml` runs the browser gates on every push, on Linux x64, in Playwright's browser builds: all of them in Chromium, and some (storage, the offline preview, Yon reads kept offline, web APIs and others) also in Firefox and WebKit. Since CI2 (2026-10-19, ADR 0022 amendment) its `browser-languages` job runs all seven Tac service-language browser gates (page programs, browser APIs, held clients, storage, cached clients, async listeners and window controls) in Chromium, Firefox and WebKit, with every language toolchain installed. `publish.yml` builds pages with the downloaded `ty` and runs the storage, runtime and website gates in Chromium, Firefox and WebKit. The v26.36.01 release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) ran that step in Chromium only, because the step installed only Chromium then | a release run of the three-browser step; branded browsers on their own systems (Safari on macOS and iOS, Chrome and Edge on Windows, Chrome on Android) |
| macOS WKWebView | native-tested (candidate) | full arm64 application gate passed: controls, accessibility, typing, lifecycle and visual parity at 420x752 | exact release artifact and current-head CI |
| iOS WKWebView | simulator-tested | isolated iPhone SE (3rd generation), iOS 26.5. Passed: Swift calls, publish, input/focus, assets and route state `7 -> 9 -> 7`. On 2026-10-19 the same gate also linked a Rust service (`aarch64-apple-ios-sim` static library, beside the two Swift services): its start hook wrote a file from the app's own process and its rendered count doubled after a page click (MR1) | exact release artifact; support on physical devices is still not claimed |
| Android WebView | simulator-tested (emulator) | Android 15 emulator. Passed: Kotlin calls, publish, input/focus per character with a delayed setter, service-worker retirement, packaged dynamic routes, window controls (granted minimize, resize and title act; ungranted maximize, fullscreen and close are refused, 2026-10-18), and a `@LocalState` field set by a Kotlin method kept across navigation and reload. Browser APIs (clipboard, share, notifications through the host; geolocation through the web view with the host's location question, 2026-10-19): the debug APK compiles with Gradle on Linux. The emulator gate `scripts/android/browser-apis-test.mjs` ran on the emulator, but it timed out at the geolocation step every time: the web view's location request never turned on GPS, so `adb emu geo fix` never reached it. `36ea6ac5` feeds the check from a mock gps provider instead; no run has passed since that change | exact release artifact; a passing browser-API emulator run; support on physical devices is still not claimed |
| Linux WebKitGTK | buildable, with VM, container and WSL runs (none of them is a Linux desktop) | Rust native service compiles. The shared route/security probe passes. On 2026-10-18 `scripts/linux/native-test.sh` ran the GTK 4 / WebKitGTK host under Xvfb with an AT-SPI (Linux accessibility bus) session, in a cloud Linux VM (Ubuntu 24.04, GTK 4.14.5) and in the pinned Debian trixie image (GTK 4.18.6). Passed: the window name, the menu bar and its item names, page roles and names (landmark, heading, button, link), AT-SPI activation of Rust calls and route changes, and the lifecycle log. RL1 (2026-10-19) added headless Wayland (mutter), a compositing window manager and WSL (aarch64) runs | exact release artifact; Linux native GUI CI on the current head; a run on a Linux desktop, because a container or VM does not prove desktop behavior |
| Windows WebView2 | native-tested (candidate), from signed-in arm64 desktop runs: UI Automation exposed semantic roles of page controls (SD1), as the Phase 5 rule in `PROJECT_PLAN.md` requires | generated C compiles with the official software development kit (SDK). Real C# protocol/OS execution. On 2026-10-19, on the signed-in desktop of a Windows 11 ARM64 Surface, `scripts/windows/native-test.ps1 -Msix` built a release MSIX package, installed it and launched it (PKG4). WebView2 rendered the page, UI Automation found the page controls by name and invoked one, the native Rust, OS and publish calls answered, and the WebView2 data was in the package's LocalState folder. On the same desktop, links opened the built app and the installed package at their routes (LK2). On 2026-10-24 the SD1 gates ran there from SSH through `scripts/windows/desktop-run.ps1`: both routes of `scripts/windows/native-test.ps1`, `scripts/windows/yon-reach.ps1` (NET3, LS1), the menus, Edit commands and Minimize/Maximize, the tray, the caption color, page window controls, background and foreground, web storage across relaunch, external links, the browser APIs, page services and an x64 build under emulation (see Windows on the signed-in desktop below) | exact release artifact; Windows native GUI CI; x64 on an x64 machine |

Android release packages (2026-10-18, ADR 0104): `ty bundle --target android
--release` was built in an x86_64 Linux cloud container (Android SDK platform
35, build-tools 35.0.0, Gradle 8.11.1, AGP 8.7.3, OpenJDK 21) with a throwaway
PKCS12 keystore. `apksigner verify` and `jarsigner -verify` showed both the APK
and the `.aab` signed with that key, `aapt2` showed the declared version code
and name and no `debuggable` flag, and no generated file held the password.
On 2026-10-19 `scripts/android/native-test.mjs` passed on the Mac's Android
35 emulator (develop `26d76ebf`): after the debug checks it built a release
with a throwaway keystore, verified both signatures, and installed and
launched the release APK, which kept the declared version code and was not
debuggable. The emulator image itself is debuggable (`ro.debuggable=1`), and
there WebView opens DevTools for every app, so the check that a release keeps
WebView debugging off still needs a production image or a physical device.

R8, ABIs and offline Gradle (PKG7, 2026-10-24, ADR 0104 amendment): in an
x86_64 Linux cloud container (Android SDK platform 35, build-tools 35.0.0, NDK
r30 30.0.16248370, Gradle 8.14.3, AGP 8.7.3, OpenJDK 21), the Rust and Kotlin
probe of `scripts/android/rust-services-test.mjs` was built with `ty bundle
--target android`. `--arch arm64` gave an APK with only
`lib/arm64-v8a/libtachyoncompanion.so` and `aarch64` in the artifact manifest.
`--release` with a throwaway keystore gave an R8-shrunk APK and AAB: `apksigner
verify` showed the signature, the APK held both libraries in one
`classes.dex`, `dexdump` found `TachyonRust` with its native and JNI callback
methods, `MainActivity`, the Kotlin service with its methods and the Kotlin
prelude under their own names, and 11 AndroidX classes were left. With the
network cut (Java's proxy set to a closed port), `--release --arch x64` built
from the filled Gradle cache with `--offline`, gave only the `x86_64` library,
and a reused route kept its native services. A first build with an empty
Gradle cache printed the note naming the modules it downloads. Not run: the
shrunk APK on an emulator (RM3), and the release DevTools check on a Google
Play image (RM3).

macOS release packages (2026-10-19, ADR 0104 amendment): `ty bundle --target
macos --release` signs the app and its helpers with the hardened runtime,
builds a `.dmg`, then signs, notarizes, staples and assesses it. Checked in an
x86_64 Linux cloud container: the signing variables are checked before the
build (`TY1605`), the certificate is chosen from `security find-identity`
output, the codesign, hdiutil, notarytool, stapler and spctl arguments, the
entitlements from the manifest grants, and the notarization answer. On
2026-10-19 the macOS tests in `crates/tachyon-cli/tests/it/macos_release_cli.rs`
passed on an Apple silicon Mac: an ad hoc release with the hardened runtime
flag, the location entitlement and the expected `.dmg` contents. On the same
day the ignored Developer ID test passed on that Mac (develop `fe191f12`),
with team JAU9V4WH99's Developer ID Application certificate and a notarytool
keychain profile: Apple answered `Accepted`, `xcrun stapler validate`
accepted the stapled ticket, `spctl --assess --type open` answered
`source=Notarized Developer ID`, and the app carries `TeamIdentifier=JAU9V4WH99`
and the hardened runtime flag. The location prompt that the entitlement
allows is still checked by hand (the `geolocation` row).

Linux release packages (2026-10-19, ADR 0104 amendment): `ty bundle --target
linux --release` was built in an x86_64 Ubuntu 24.04 cloud container (GTK
4.14.5, WebKitGTK 2.52.6, dpkg 1.22.6, squashfs-tools 4.6.1, AppImage runtime
20251108). `scripts/linux/native-test.sh` extracted the `.deb`, checked its
command link and desktop file (`desktop-file-validate`) and its AppStream
metainfo (`appstreamcli validate`, no errors), then under Xvfb started both the
installed command and the AppImage (`APPIMAGE_EXTRACT_AND_RUN=1`, since the
container has no FUSE) until each showed its first page and loaded its Rust
library. `apt-get install` of the `.deb` resolved its GTK 4 and WebKitGTK
dependencies and installed `/usr/bin/nativegate`. `linux_package_tests` built
two releases that gave the same `.deb` and AppImage bytes. RL1 (below) ran
these package checks on WSL Ubuntu 24.04 (aarch64) and started the AppImage
through FUSE there. LX1 (below) installed the `.deb` as root with apt in the
cloud container, started it from `PATH` and from its desktop file, and purged
it. Not yet run: `dpkg -i` or `apt install` of the `.deb` on WSL, which needs
administrator rights, and an install on a Linux desktop.

Windows release packages (2026-10-19, ADR 0104): `ty bundle --target windows
--release` builds an MSIX package with `makeappx` and signs it with
`signtool`. In an x86_64 Linux cloud container, a staged package layout was
packed with Microsoft's MSIX SDK (`makemsix`, which validates
`AppxManifest.xml` against the Windows schemas), signed with a self-signed
certificate by `osslsigncode`, verified and unpacked again; the packaged-host
data-folder helper ran under Wine. On Windows 11 ARM64 (2026-10-19),
`scripts/windows/native-test.ps1 -Msix` built an arm64 package with
`makeappx`, signed it with `signtool` and a self-signed certificate, and,
from an elevated prompt on the signed-in desktop, installed it, launched it
and found its WebView2 data in the package's LocalState folder. Distribution
needs a trusted code-signing certificate.

iOS device and App Store packages (2026-10-19, ADR 0104 amendment): in an
x86_64 Linux cloud container, unit tests covered the profile checks (team, app
ID, kind), the certificate choice, the entitlements, the Xcode build keys and
the device `Info.plist`, and CLI tests covered the `TY1001` and `TY1605`
refusals. On the Mac (macOS 26.6, Xcode 27.1 27A9275, iphoneos SDK 27.1, develop
fd0384ad), `scripts/ios/native-test.mjs` passed without signing variables
(iPhone SE simulator, iOS 26.5) and again with `TAC_APPLE_TEAM_ID` and a
wildcard development profile: `ty bundle --target ios --device` chose the
Apple Development certificate from the profile's SHA-1, and the app passed
`codesign --verify --strict` with `iPhoneOS`, `embedded.mobileprovision`,
`PrivacyInfo.xcprivacy` and the `.ipa` `Payload` layout. Not yet run:
`--release` with an App Store profile, installing and launching on an iOS 17
or later device (the only registered device is an iPod touch on iOS 15), and
an App Store Connect upload.

### Application services and declared menus on desktop hosts (2026-10-04)

ADR 0097 and its amendment cover an application-lifetime Rust service and
`tac.application.menus`. The service lives inside the app process only; no
host creates an operating system service, daemon, login item or autostart
entry. This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| macOS WKWebView | yes | the generated AppDelegate is compiled and run with a real Rust library (ignored SDK-lane tests in `native_application_host_tests` and `native_menu_host_tests`) |
| Linux WebKitGTK | yes | `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and a real Rust library, and ran it under Xvfb on WSL Ubuntu 24.04 (aarch64), passing three of three runs: the service started once at launch, a menu action reached the page service without a reload, the item was disabled with no recipient, Help and Home kept the service, and Close Window stopped it once. Not covered: a physical Linux desktop, Wayland, x86_64, and clicks or keys by a person. WSLg's display did not accept connections from the remote test lane, and with WebKit's sandbox on, the first page load took up to about 80 seconds on that machine. This run predates `594a5833` (2026-10-18). Before that fix, GTK 4 did not draw the menu bar, and the popover menu items had no accessible name. The test activates the menu actions through the application's action group, so it passed without a visible menu bar. Since the fix the test also checks that the window shows its menu bar, the AT-SPI probe of `scripts/linux/native-test.sh` reads the drawn menu items by name, and the test passed again in the WSL run of `38eaed29` |
| Windows WebView2 (x64) | yes | built and run on Windows 11 ARM64 on 2026-10-04 with Visual Studio Build Tools 2022 17.14 (MSVC 14.44, ARM64-hosted x64 tools), Windows SDK 10.0.26100.0 and WebView2 SDK 1.0.3485.44. `ty`, built natively for `aarch64-pc-windows-msvc`, ran `ty bundle --target windows` for the `menu-actions` and `menu-items-that-open-a-page` examples: the whole host compiled with `cl /W4 /WX`, linked `WebView2LoaderStatic.lib`, and `TachyonRustCompanion.dll` was built (after the C4100 fix of 2026-10-04). The WebView2 handler interface IDs match `WebView2.h`. The apps ran from an SSH session and were driven with Win32 messages: the application service started once before the window, the File, View and Help menus showed the declared items and shortcuts, a menu action ran, an item whose event had no recipient stayed disabled and did not run, and Close Window ended the process with exit code 0 after exactly one stop (also after `WM_ENDSESSION`). This SSH run did not reach WebView2: outside a signed-in desktop session `CreateCoreWebView2Controller` fails with `0x80070578` (invalid window handle). WebView2 pages ran later on the signed-in desktop (2026-10-19): the MSIX gate rendered pages and made native calls (PKG4), and links opened the app at their routes (LK2); see the Windows row and the Windows release packages above. On 2026-10-24 route items, the `AcceleratorKeyPressed` handler (Ctrl+K) and `scripts/windows/native-test.ps1` without `-Msix` passed there (SD1). On the same day View > Show Running and Ctrl+2 sent their events to the page, which now shows them (SD1) |
| iOS WKWebView | route items yes (MN1a); action items refused (`TY1601`), and the application lifetime refused (`TY1406`) until LT1 and MN1b | `scripts/ios/menus-test.mjs` on this Mac (2026-10-19, MN1a, Xcode with iOS 26.5): on an iPhone 17 Pro simulator Command-K from a simulated hardware keyboard reached the route item through `keyCommands`, and on an iPad Pro 13-inch (M5) simulator through the main menu that `buildMenu(with:)` fills; a Swift service on the opened page recorded `/status` both times. Not covered: the iPad menu bar opened by a pointer or touch, and a person's keyboard |
| Android WebView | route items yes (MN1a); action items refused (`TY1601`), and the application lifetime refused (`TY1406`) until LT1 and MN1b | `scripts/android/menus-test.mjs` on this Mac's emulator-5554 (API 35 arm64, 2026-10-19, MN1a): the system keyboard shortcuts list (Meta+/) showed File > Status Ctrl+K, `input keycombination` Ctrl+K opened `/status`, and a tap on the app bar's More options and then Guide opened `/guide`, read through the WebView's DevTools socket |

Standard menus on Linux and Windows (2026-10-18, ADR 0097 amendment): both
hosts now always show a menu bar with File, Edit (Undo, Redo, Cut, Copy,
Paste, Select All) and Window (Minimize, Maximize), and a `control`
shortcut gets Ctrl. Linux: the same `linux_host_tests` run, extended to
check the five menus, the Copy and Redo accelerators and Edit > Select All
in the real web view, passed three of three runs under Xvfb in an x86_64
Ubuntu 24.04 cloud container (GTK 4.14.5, WebKitGTK 2.52.6), and the Linux
window-control run still passed with the always-on menu bar (xfwm4 under
Xvfb). Windows: generated-source tests and the MinGW-w64 compile only; the
Edit commands inside WebView2 (sent through the DevTools protocol) and
Minimize/Maximize still need the interactive desktop run listed above.

Background and foreground (2026-10-18, ADR 0028 amendment): every host
sends `ApplicationStateChanged` to the page and to natively linked
services. Linux: `scripts/linux/application-state-test.sh` passed under
Xvfb and xfwm4 in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5,
WebKitGTK 2.52.6): a page service and a linked Rust service each heard the
app go to the background and come back when the focus moved to another
window, and go to the background when it was minimized. Android: the host
code and a Kotlin companion compile against Android platform 35; no
emulator run yet. Windows: generated-source tests and a MinGW-w64 compile of
the state code only; the run inside WebView2 waits for the interactive
desktop run. macOS and iOS: generated-source tests only; the Swift was not
compiled in that run.

Windows bundles accept `--arch arm64` or `--arch x64`. The default matches
an ARM64 build host and falls back to x64 on other architectures. This choice
sets the Rust companion target, WebView2 library folder, C# runtime identifier
and linker architecture together. The build refuses a developer prompt whose
`VSCMD_ARG_TGT_ARCH` disagrees with the selected architecture.

macOS bundles accept `--arch arm64`, `--arch x64` or `--arch universal`
(ADR 0097 amendment of 2026-10-18). The default is the build machine's
architecture. `universal` builds both architectures and joins them with
`lipo`. Only the staging and the artifact manifest are tested so far; the
universal build, its `lipo -archs` check and a run on an Intel Mac are not
yet verified. Linux and iOS builds use the build machine's architecture.
Android takes `--arch arm64`, `x64` or `universal` (PKG7, 2026-10-24, ADR 0104
amendment): the Rust library is built for `arm64-v8a`, `x86_64` or both (MR2),
and Gradle's `abiFilters` name the same ABIs. `universal` is the default. An
app without Rust services holds no native code.

Rust on Android (2026-10-19, ADR 0019 amendment): Rust services are linked
into the Android app through JNI. `scripts/android/rust-services-test.mjs`
passed on the Mac's Android 15 (API 35, arm64) emulator with NDK r30: the APK
held `libtachyoncompanion.so` for `arm64-v8a` and `x86_64` and no WebAssembly,
and a Rust service ran in the app's own process next to a Kotlin service.
Calls, an awaited method, an event, the granted and refused window controls
and the window title, an `HTTPClient` call to `tac.yon.origin` (and its
`HTTP_LIMIT` refusal), a `@LocalState` field across a reload and
`tac_set_local` across a relaunch all passed. A signed `--release` build of
the same app packaged both libraries in the APK and in the App Bundle; it was
not installed. Not run: the `x86_64` library on an x86_64 emulator, and a
physical device.

On 2026-10-04, commit `2be294ba` built the Observer fixture through
`ty bundle --target windows` for both architectures on Windows 11 ARM64.
Each bundle included its host executable, Rust companion DLL and
`observer-hook` helper beside the host in `bin`. The helpers ran successfully,
and `dumpbin` confirmed ARM64 or x64 for all three artifacts. The fixture used
vendored Rust crates; bundle compilation ran offline. An x64 request from an
ARM64 developer prompt was refused with the required prompt and architecture
choices. These build and helper results do not qualify WebView2 page rendering
or interaction; those gates still need an interactive desktop session.

On 2026-10-18 the Windows host started to open its window from the
manifest: `tac.application.window` size and minimum size in
device-independent pixels, a hex `theme_color` as the Windows 11 caption
color, and `"display": "fullscreen"` as a borderless fullscreen start (ADR
0052 amendment). Validated so far: unit tests of the generated host, a C test
of the frame code against Win32 stand-ins, and a MinGW compile against the
Win32 headers. On 2026-10-19 (RW1) the real-window check
`the_windows_window_frame_drives_a_real_win32_window` passed with MSVC on
Windows 11 ARM64 over SSH: the window opened at the declared size and
minimum size, and borderless full screen for `"display": "fullscreen"`.
`native_windows_controls_drive_a_real_win32_window` passed there too. The
caption color ran on the signed-in desktop in SD1 (below).

Windows over SSH (2026-10-19, RW1): on Windows 11 ARM64 (build 26200,
WebView2 Runtime 155.0.4283.62, MSVC 14.44 ARM64 tools, WebView2 SDK
1.0.3485.44), from an MSVC developer environment over SSH:

- `native_reproducible_build_tests::staged_native_apps_are_byte_identical_when_built_twice`
  and `a_packaged_windows_app_is_byte_identical_when_built_twice` passed;
- `windows_window_frame_tests::the_windows_window_frame_drives_a_real_win32_window`
  and `native_window_control_tests::native_windows_controls_drive_a_real_win32_window`
  passed, so a Win32 window opens without a signed-in desktop;
- `windows_web_engine_check::tests::the_webview2_check_refuses_an_old_or_missing_runtime_with_the_install_message`
  built the startup check with `cl` and `WebView2LoaderStatic.lib` and ran
  it against the installed runtime, a floor of 9999.0 and a missing runtime
  (`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` set to an empty folder). The message
  box is replaced by a capture, so the run shows the message text but not
  the box itself.

On 2026-10-19 the Windows host started to open the file dialog
(`IFileOpenDialog`) for `FilePicker` (FP1, ADR 0089 amendment). The picker
code, built with MinGW-w64, ran under Wine 9.0 and Xvfb in an x86_64 cloud
container: `windows_file_picker_tests` checked the whole contract with a
test chooser and drove Wine's real dialog to select and read a file and to
cancel an unanswered one. The generated host passed a MinGW-w64 syntax
check against the WebView2 headers. On 2026-10-24 (SD1) the MSVC build
passed `the_windows_file_picker_selects_a_file_in_the_windows_dialog` on
the signed-in Windows 11 ARM64 desktop: the typed file was selected and
read, and an unanswered dialog closed after its patience. A picker opened
from a WebView2 page on a signed-in desktop is still owed.

On 2026-10-19 the Windows host started to build `"titleBar": "integrated"`
(ADR 0098 amendment): the page reaches the top edge under the system caption
buttons, gets the three inset properties, and WebView2 moves the window from
`data-tac-drag-region` elements through CSS `app-region`; the menu bar moves
into the window menu. Validated in an x86_64 Ubuntu 24.04 cloud container:
generated-host unit tests, a MinGW-w64 13 compile of the whole generated
host (standard and integrated) against the Win32 headers and the WebView2
SDK 1.0.4258.31 headers with warnings as errors, and
`the_windows_title_bar_drives_a_win32_window_under_wine` on Wine 9.0 under
Xvfb: the client area started at the window's top edge at the manifest size,
the top border answered `HTTOP` and `HTTOPLEFT`, the menus moved into the
window menu, the insets held the caption height and button width and were
`0px` in full screen, and the web view's region left out only the caption
buttons. Wine has no DWM, so this used the fallback button size. The page
script, run in Chromium with Playwright, turned a drag region into
`app-region: drag` and its button into `no-drag`, and sent no
`window.dragRegions`. On the Surface (Windows 11 ARM64, over SSH, cl
19.44.35229 for ARM64, WebView2 SDK 1.0.3485.44) the same fixture compiled
with `cl /W4 /WX` and passed on a real HWND
(`the_windows_title_bar_drives_a_real_win32_window`), and `ty build --target
windows` built the integrated-title-bar docs example and a Yon Reach app
with C# and Rust companions with integrated and standard title bars,
printing the two Windows notes only when they applied. Launched from SSH,
the integrated app started and recorded `bridge.unavailable`, as every
WebView2 app does outside a signed-in desktop. The look, Snap Layouts and a
drag ran in WebView2 on the signed-in desktop on 2026-10-24 (SD1, below).

On 2026-10-18 the Linux host started to open its window from the manifest
too: the page at `tac.application.window` size and minimum size, a hex
`theme_color` as a GTK header bar color, and `"display": "fullscreen"` as a
fullscreen start (ADR 0052 amendment). `linux_window_appearance_tests`
compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings
as errors) and ran it under Xvfb with xfwm4 in an x86_64 Ubuntu 24.04 cloud
container, three of three runs: the page opened at 900x640, a resize below
the 720x520 minimum stopped at 720x520, the header bar rendered `#1a237e`
with white text, and `fullscreen` reached the GTK window and the X11
toplevel. Both tests also passed in the WSL (aarch64) run of `38eaed29`
(below), and under headless Wayland in RL1 (below). Not run: a physical
desktop.

On 2026-10-19 the Linux host started to build `"titleBar": "integrated"`
(ADR 0098 amendment): the page reaches the top edge under GTK's window
buttons, gets the three inset properties, and moves the window from
`data-tac-drag-region` elements; the menu bar becomes a main menu button.
`linux_title_bar_tests` compiled the generated host with GTK 4.14.5 and
WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb with xfwm4 in an
x86_64 Ubuntu 24.04 cloud container, three of three runs, with real XTest
pointer events: the insets were 46/46/135 px with a `close:minimize,maximize`
layout, presses on a button in the drag region and below it reached the page
while a press in the region did not, a 120x72 px drag moved the window by
that much, a double-click maximized and a second one restored, full screen
zeroed the insets, and a new page started with them. The same run on WSL
(Ubuntu 24.04.5, aarch64, GTK 4.14.5, WebKitGTK 2.52.6, Xvfb with xfwm4) on
develop `38eaed29` passed all eleven ignored Linux host tests, and the title
bar test three of three times. RL1 (below) ran it under a compositing window
manager and under headless Wayland. Not run: a physical desktop. The eleven tests are the ignored Linux tests that
existed before `38eaed29`; the three `linux_package_tests` that `38eaed29`
added were not part of the run:

- `linux_browser_api_tests::linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view`
- `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime`
- `linux_page_call_tests::linux_page_calls_page_large_results_through_the_real_webkit_bridge`
- `linux_title_bar_tests::linux_generated_host_integrates_the_title_bar_with_insets_and_drag_regions`
- `linux_web_engine_check::an_old_webkitgtk_shows_a_message_instead_of_a_blank_window`
- `linux_window_appearance_tests::linux_generated_host_opens_the_declared_size_minimum_and_theme_color`
- `linux_window_appearance_tests::linux_generated_host_starts_fullscreen_when_the_manifest_display_is_fullscreen`
- `linux_window_control_tests::linux_generated_host_drives_granted_window_controls_and_refuses_a_denied_one`
- `native_external_link_tests::the_linux_host_hands_external_links_to_the_launcher_and_keeps_the_page`
- `native_reproducible_build_tests::a_packaged_linux_app_is_byte_identical_when_built_twice`
- `native_rust_crate_tests::the_linux_bundle_places_the_helper_beside_the_host_and_it_runs`

#### Linux runs owed, RL1 (2026-10-19)

RL1 ran the Linux rows of the runs-owed list that need no physical desktop.
Cloud: an x86_64 Ubuntu 24.04 container with GTK 4.14.5, WebKitGTK 2.52.6,
Xvfb 21.1.12, xfwm4 4.18.0 and mutter 46.2. WSL: Ubuntu 24.04.5 aarch64 on
WSL2 (kernel 6.18.54.1), GTK 4.14.5, WebKitGTK 2.52.6, node 24.21.0, bun
1.4.3, libfuse2t64 2.9.9, with no administrator rights.

- Wayland (D10): `scripts/linux/compositor-test.sh` starts headless mutter
  (`--headless --wayland --virtual-monitor 1280x900`) and drives the pointer
  through mutter's RemoteDesktop D-Bus interface. With `GDK_BACKEND=wayland`
  ten `native::linux_` host tests passed, and the three `the_linux_` link
  tests. It skips the package tests, and `linux_browser_api_tests` and
  `linux_file_picker_tests`, whose fixtures drive input through X11's XTest. The title bar test maximizes first,
  because a Wayland client only knows its position at (0, 0): the insets,
  presses in and below the drag region, a double-click unmaximize, and a drag
  that unmaximizes through the compositor all passed. xdg-shell has no
  minimized state, so the window control test skips the minimized wait under
  Wayland and says so. Weston was not used; mutter answered every check.
- Compositing: the same script runs xfwm4 `--compositor=on` under Xvfb with
  the Composite extension and checks that xfwm4 owns `_NET_WM_CM_S0`. The
  title bar and window control tests passed, two and two. The new test
  `linux_generated_host_places_the_window_buttons_at_the_declared_inset`
  passed under X11 and Wayland: with `titleBarButtonInset` 20/14 the start
  and end button groups sat at the declared top and side.
- Granted window controls: the new test
  `linux_generated_host_maximizes_fills_the_screen_and_closes_when_granted`
  maximized, entered and left full screen, and closed the window.
- WebAssembly page services in WebKitGTK, under Xvfb in the cloud
  (`scripts/linux/page-services-native-test.mjs`): the Tally page answered a
  click with Go, C#, F#, Swift, Kotlin, Java, Dart, Lua, C++ and Python
  (Pyodide in the web view) services (first page in 1.2 to 1.9 s, Python 4.0 s; 636 to 765 MiB
  resident for the app and its web processes). Swift, C# and Kotlin
  WindowControls callers renamed the window and resized it to 1280x800 page
  pixels. The Ruby service could not build in the cloud, whose proxy refused
  its mruby/prism source download; it answered on WSL (below).
- Page checks (`scripts/linux/page-checks-native-test.mjs`): typing kept the
  focus and caret; `connect-src` refused an undeclared host, which saw no
  request, while the Yon origin answered; a network-first Yon read came from
  the cache with Yon stopped (NET6); a `@SessionState` field survived
  navigation; a preference string survived quit and restart. The offline
  read found a bug: a JavaScript
  `@HTTPClient` read skipped the read cache (fixed, ADR 0087 amendment).
- External and scheme links (`scripts/linux/desktop-links-test.sh`): the
  real `GtkUriLauncher` handed `https` and `mailto` links to a recorder set
  as their default handler, through the OpenURI portal and through GIO with
  the portal off. With the `.deb`'s `.desktop` file installed under
  `XDG_DATA_HOME`, `gio open` and `xdg-open` of `linkgate://` links reached
  the one running app.
- WSL, round 1 on `d7110f74`: `linux_browser_api_tests`,
  `linux_window_appearance_tests` (xfwm4), the Linux external link and deep
  link tests and `an_old_webkitgtk_shows_a_message_instead_of_a_blank_window`
  passed. `scripts/linux/native-test.sh` passed its build, AT-SPI probe,
  release packages, the extracted `.deb` and the AppImage. The two Windows
  tests in those files need `x86_64-w64-mingw32-gcc`, which WSL does not
  have.
- WSL, round 2 on `da32d264`: `native-test.sh` started the AppImage through
  a real FUSE mount and left no document portal mount behind. Its Yon reach
  step failed only in cleanup: WSL's clock drift moved `ps` start times by up
  to two seconds, so `scripts/supervised-command.mjs` saw a changed process.
  It now reads the kernel start time from `/proc/<pid>/stat` on Linux.
- WSL, round 3 on `19a95a65`: `native-test.sh` passed in full, including the
  AppImage through FUSE and Yon reach with JavaScript and with linked Rust
  services. `page-checks-native-test.mjs` passed, and so did the Ruby
  WebAssembly page service. Under xfwm4 with its compositor on, the title bar
  and window control tests passed two of two each; one earlier title bar run
  timed out reaching the accessibility bus under load and passed on rerun.
- WSLg: all eight host tests run under `GDK_BACKEND=wayland` failed to open
  the display. WSLg's own Weston crashed (signal 11) as the first GTK window
  opened, in a session started over SSH; a ten-line GTK 4 window crashed it
  the same way. WSLg is not counted as a Wayland run.
- WSL, round 4 on `fe191f12`, after Samuel installed mutter 46.2, xdg-utils
  1.1.3 and gcc-mingw-w64-x86-64 13.2.0: `compositor-test.sh` passed both
  halves (xfwm4 compositing two and two; headless mutter Wayland ten host
  tests and three link tests), and `desktop-links-test.sh` passed through GIO
  and through the OpenURI portal, with the installed `.desktop` file opening
  `linkgate://` links in one running app. The two Windows link tests now
  compile with mingw but need Wine to run the x86_64 `.exe`, which arm64 WSL
  cannot run without an x86 emulator; they belong on Windows or x86_64 Linux.
- Not run: `dpkg -i` or `apt install` of the `.deb` on WSL, because both need
  administrator rights (`native-test.sh` unpacks and starts it without them);
  WSLg from a signed-in desktop session; a physical Linux desktop. GeoClue
  and a notification server ran in LX1 (below).

### Windows on the signed-in desktop (2026-10-24, SD1)

WebView2 does not start from an SSH session (`0x80070578`). On 2026-10-24
`scripts/windows/desktop-run.ps1` ran each gate from SSH in the desktop
session of the same signed-in user, through a one-time interactive
scheduled task, and read its output from a file. The machine was the
Windows 11 ARM64 Surface (build 26200, WebView2 Runtime 155.0.4283.62,
MSVC 14.44 ARM64 tools, Windows SDK 10.0.26100.0, WebView2 SDK
1.0.3485.44), signed in and unlocked. Passed there:

- `scripts/windows/native-test.ps1`, both routes, with the native Rust and
  C# calls and publish;
- `scripts/windows/yon-reach.ps1` (NET3, LS1): HTTP, a stream, a socket, a
  topic, a `@Check` 422 and a signed-in route from the page, Rust and C#
  `HTTPClient` reads, and a linked C# service's stream (three ticks) and
  socket (an echo) clients held by the page. No local network prompt
  appeared;
- `scripts/windows/window-shell-test.ps1`: the integrated title bar
  started the client area at the window's top edge, left the caption
  buttons to `WM_NCHITTEST` with no web view over them, and gave the page
  30px, 0px and 145px insets; a real click in the drag region reached the
  page, a `SendInput` drag moved the window, a double-click maximized and
  restored it, a right-click opened the window menu, and hovering the
  maximize button showed the Snap Layouts flyout. The caption took the
  `theme_color` (all 760 sampled caption pixels matched `#b03a2e`), the
  standard File, Edit and Window menus with the Edit commands, page window
  controls (a resize to 640 device-independent pixels, maximize, minimize,
  fullscreen, `setTitle` and close with exit code 0) and a fullscreen start;
- `scripts/windows/menu-tray-test.ps1`: route menu items, menu actions
  and their Ctrl+2 shortcut, sent while WebView2 had the focus, whose events
  reached the page, the Ctrl+K accelerator, the Edit menu, Window Minimize
  and Maximize, and the tray
  icon, whose menu held Open, the declared items and Exit, and whose
  declared item's event reached the page; closing the
  window hid it, a click on the icon showed it again, Exit ended the app
  and removed the icon;
- `scripts/windows/page-checks-native-test.mjs`: typing kept focus and caret
  through reactive writes, `connect-src` refused an undeclared host (it saw
  no request), a network-first Yon read answered from the cache with Yon
  stopped, a `@SessionState` field survived a navigation, a preference
  string survived a quit and a relaunch, and UI Automation exposed Edit,
  Button, Hyperlink, Text and Heading roles (AX2);
- `scripts/windows/application-state-test.mjs`: the page and linked Rust
  services heard background and foreground;
- `scripts/windows/relaunch-storage-test.mjs`: localStorage, IndexedDB and
  the Cache API survived a relaunch;
- `scripts/windows/page-services-native-test.mjs`: WebAssembly page
  services in Python, Go, C#, F#, Kotlin, Java, Dart, Ruby, Lua and C++,
  and C# and Kotlin `WindowControls` callers. Swift has no toolchain on that
  machine;
- `scripts/windows/browser-apis-test.ps1` (BA3): the clipboard both ways,
  the share sheet opened, geolocation granted with no prompt, every API
  refused with nothing granted, and, from the MSIX package registered in
  Developer Mode, a toast in the notification history and later deep links
  handed to the one running process. A link to a host (LK3) was not
  checked, because no domain is verified for the package;
- `scripts/windows/external-links-test.mjs`: http, https and mailto links
  opened in the default browser and mail app;
- an x64 build run under emulation on ARM64 (PE machine `0x8664`) rendered
  its page, and its `setTitle` reached the window;
- `the_windows_file_picker_selects_a_file_in_the_windows_dialog` (FP1),
  built with MSVC: the typed file was selected and read, and an unanswered
  dialog closed after its patience;
- the real-window title bar test, the WebView2 startup check with the
  current, an old and a missing runtime, the window frame and native
  controls tests, and
  `the_windows_browser_apis_match_the_windows_sdk_and_run_on_windows`;
- `two_linked_csharp_services_share_one_setup_configuration_when_published_with_native_aot`
  (LS2) for `win-arm64`: both NativeAOT-published services read the same
  identity, and the shared setup ran once.

The runs fixed five things. An unanswered Windows file dialog whose
patience ended while it was still starting ignored `IFileDialog::Close` and
stayed open; the host now repeats the close and sends Cancel until the
dialog returns. The title bar test now allows the DWM caption
bounds to reach the window border (801 against a client edge of 800). The
WebView2 startup check fixture no longer names its program `installed.exe`,
which Windows treats as an installer that needs elevation (error 740). The
tray gate reads popup menu items through `MN_GETHMENU`, because UI
Automation shows the tray menu empty. A menu or tray action that forwards
an event with `CurrentPageEvents` now tells the page to read its fields
again (see the CHANGELOG).

Not run in SD1: a picker opened from a WebView2 page, a host link opening
the installed package (no verified domain), x64 on an x64 machine, and the
Wayland tests from the signed-in session under WSLg
(`scripts/linux/compositor-test.sh --wslg`), because the WSL distribution's
disk was read-only with I/O errors.

### Linux desktop stand-ins (2026-10-24, LX1)

LX1 ran the Linux desktop services, a Wayland session and a root package
install in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5, WebKitGTK
2.52.6, mutter 46.2, GeoClue 2.7.0, dunst 1.9.2):

- `scripts/linux/desktop-services-test.sh` starts a private system bus,
  GeoClue with a GPS fix from an NMEA socket (`scripts/linux/nmea-feed.py`),
  the GeoClue demo agent and dunst, then runs
  `linux_browser_api_tests::linux_browser_apis_reach_geoclue_and_the_notification_server`.
  The page got the position 51.5, -0.125 from GeoClue (GeoClue logged
  `org.tachyon.linux_host_check` started, allowed by the agent at accuracy
  level 8), and dunst showed the page's notification "Order / Shipped" from
  `LinuxHostCheck`. It needs no root.
- `TAC_LINUX_DISPLAY=wayland node scripts/linux/yon-origin-native-test.mjs`
  ran the packaged app in a headless mutter Wayland session and passed with
  JavaScript and with linked Rust services (HTTP, a preflighted signed-in
  request, a topic, a `@Check` 422, a stream and a socket). Under Wayland,
  GTK never embedded WebKit's accessibility socket in the window, so the
  window's tree showed an empty web view; the web process's own tree held the
  page, and `scripts/linux/yon-origin-probe.py` reads and clicks it there.
  A screen reader in such a session would see the same empty web view; a
  Wayland desktop session has not been checked.
- `scripts/linux/deb-install-test.sh`, as root: `apt-get install` of the
  release `.deb` resolved its dependencies and installed it, the app started
  from `nativegate` on `PATH` and from its installed `.desktop` file through
  `gio launch`, and `apt-get purge` removed the command, the library folder
  and the desktop file. apt reported 0 B installed, because the control file
  has no `Installed-Size`.

### Native apps reach Yon through a declared origin (2026-10-18)

ADR 0103 adds `tac.yon.origin` in `client/manifest.json` and `server.origins`
in `server/application.json`. The contract is unit-tested in the cloud, and
`scripts/yon-origin-browser-test.mjs` runs a web page on one port against
Yon on another in Chromium, and `scripts/native/yon-reach-test.mjs --target=web`
runs the probe that the native apps run. On 2026-10-19 it ran the macOS app
and the iOS simulator on a Mac, the packaged Android app on the Android 15
emulator (NET2), and the packaged Linux app under Xvfb (NET4); the Windows
app has not run it yet.

| Host | Yon calls from the packaged app | Evidence |
| --- | --- | --- |
| Web | implemented | page transports, CORS, preflight, socket origin and `connect-src` unit tests; Chromium cross-port run, and cross-port offline reads in Chromium, Firefox and WebKit (NET8, below) |
| macOS, iOS | run on the host | `scripts/native/yon-reach-test.mjs` passed on 2026-10-19 for the macOS app (macOS 26.6.2, Xcode 27.1) and the iOS 26.5 simulator (iPhone 17 Pro) over `http` and over `https` through a local TLS proxy. In all three runs Yon received `Origin: tachyon-app://bundle`, and the page reached it with HTTP, a CORS preflight, a stream, a socket, a topic, a 422 problem and a signed-in route. The linked Swift `@HTTPClient` read reached the origin through `URLSession` too; it is also unit-tested against a local server |
| Android | run on the emulator; an `https` origin, or a local `http` origin with cleartext to that one host | `scripts/android/yon-origin-test.mjs` passed on 2026-10-19 on the Android 15 (API 35, arm64) emulator against `ty serve` on the Mac through `adb reverse`. The packaged app reached Yon from `https://appassets.tachyon.local` with an HTTP read, a cross-origin write, a stream, a socket, a topic, a `@Check` 422 and a signed-in route (401 without a token, then with an `Authorization` header). A linked Kotlin `HTTPClient` reached Yon through the native transport in the app's own process. The transport is also tested on the JVM against a real HTTP server (`android_yon_http_tests`). Not run: a physical device |
| Windows | run on the signed-in desktop; linked Rust and C# `HTTPClient` calls reach it too | 2026-10-19 cloud runs and the MSVC and NativeAOT build on Windows 11 ARM64 (below); on 2026-10-24 `scripts/windows/yon-reach.ps1` passed inside WebView2 on the signed-in desktop of that machine: HTTP, a stream, a socket, a topic, a `@Check` 422 and a signed-in route from the page, Rust and C# `HTTPClient` reads, and a linked C# service's stream and socket clients held by the page (NET3, LS1) |
| Linux | run under Xvfb | `scripts/linux/yon-origin-native-test.mjs` built the packaged app with GTK 4.14.5, WebKitGTK 2.52.6 and libsoup 3.4.4 in an x86_64 Ubuntu 24.04 cloud container and drove it through AT-SPI. It reached Yon on `http://127.0.0.1` and `https://localhost`, once with JavaScript page services and once with linked Rust services: HTTP read and write, a preflighted `@SignedIn` call (401 without a token), a topic, a `@Check` 422, a stream and a socket. The Linux gate runs both. Both also passed on WSL Ubuntu 24.04.5 (aarch64) in RL1, on `19a95a65`. LX1 ran both again in a headless mutter 46.2 Wayland session (`TAC_LINUX_DISPLAY=wayland`), reading the page from the web process's own accessibility tree because GTK did not embed it in the window there. Not run: a Wayland desktop session |

On 2026-10-24 (LS1, ADR 0058 and 0060 amendments) linked Swift, Kotlin and
C# services started to hold stream and socket clients through the page, as
linked Rust services do. Their companions ran in an x86_64 Ubuntu 24.04
cloud container (Swift 6.3.3, Kotlin 2.2.20 on the JVM, .NET 9): each
described the `realtime` and `sockets` clients, heard a tick and a chat
message through `event`, and sent through the page. Not run: a linked
stream or socket inside the macOS, iOS, Android or Windows app.

On 2026-10-19 (NET3, ADR 0103 amendment) linked services started to answer
the page later through a task, and linked Rust and C# `HTTPClient` calls
started to reach `tac.yon.origin`. The cloud evidence:

- a Rust companion built for `x86_64-pc-windows-gnu` reached a local server
  through WinHTTP under wine 9.0 (`200` and `422`, no cookies);
- the Windows resume message code was built with MinGW and run under wine;
- a C# companion was built with .NET 10 and reached a local server
  (statuses, limits, deadline, cancel and the refusals);
- `scripts/yon-reach-fixture-test.mjs` passed in Chromium against Yon on
  another origin: HTTP, a stream, a socket, a topic, a `@Check` 422 and a
  `SignedIn` route.

On 2026-10-19 (RW1) the reach fixture with linked Rust and C# services was
built for Windows on Windows 11 ARM64 over SSH (MSVC 14.44, .NET 10.0.302):
`scripts/native/yon-reach-fixture.py` and `ty build --target windows` gave
an ARM64 `YonReach.exe`, a NativeAOT `TachyonCompanion.dll` (no CLR header;
it exports the `tac_native_*` entry points) and `TachyonRustCompanion.dll`.
The app was not launched. The run found two faults, now fixed: the
fixture's linked services broke the Tac method rule (`TY1314`), and the
build left the Rust companion's `.dll.lib` and `.dll.exp` in `bin`.

Not run: WebView2 itself.
`scripts/windows/yon-reach.ps1` runs every path from the packaged app on a
Windows desktop. It also shows whether WebView2 asks for local network
access before a page on `https://tachyon.local` may call
`http://localhost`.

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

Desktop test host limits:

- The macOS and Windows gates need a signed-in graphical session. They do
  not run headless, and they do not run from an SSH session alone.
- macOS: the gates open windows, read them through Accessibility, take
  screenshots and use the iOS simulator, so the runner is a launch agent in
  the signed-in `runner` session (see
  [`SELF_HOSTED_RUNNERS.md`](SELF_HOSTED_RUNNERS.md)).
- Windows: a network logon such as SSH has no desktop session. There
  `CreateCoreWebView2Controller` fails with `0x80070578` (invalid window
  handle), and `Add-AppxPackage` fails with `0x80070005`, so the WebView2
  and MSIX gates run from a prompt on the signed-in desktop.

Native calls have a ten-second browser wait limit. This limit does not stop or
roll back native service code. Services must stay responsive. A hung
application may need to be relaunched.

### Web pages keep cross-origin Yon reads offline (2026-10-19)

A web build with `tac.yon.origin` and `tac.cache` ships the same page script
as a native build, because the service worker leaves other origins alone
(ADR 0087 amendment). `scripts/yon-reads-web-browser-test.mjs` passed in
Chromium, Firefox and WebKit in the cloud: a bundled web app on one port
read Yon on another, went offline while Yon still ran, and answered the
public read from its kept copy.

### Native apps keep Yon reads offline (2026-10-19)

A native build with `tac.yon.origin` and `tac.cache` applies the rules to
Yon reads in the page's Cache Storage (ADR 0087 amendment). It is tested in
Node and by `scripts/yon-reads-browser-test.mjs` in Chromium, Firefox and
WebKit in the cloud. The packaged Linux app ran it under Xvfb in RL1: a
network-first Yon read came from the cache with Yon stopped ("Linux runs
owed, RL1" above; ADR 0087 amendment). The airplane-mode runs on the iOS simulator and the
Android emulator, and offline reads in the macOS and Windows apps, are owed.

How long the kept reads last follows the web storage of each host (below).
The per-target support table gives `tac.cache` no relaunch limit on macOS,
Windows and Linux, where the web view keeps its storage on disk, and marks it
adapted on iOS and Android:

- Linux: since ST1 (`499a6246`) the host keeps web storage on disk, in
  `$XDG_DATA_HOME/<application id>/webkit`, so kept reads stay after a
  relaunch, as on macOS. Before ST1 they lasted only while the app ran.
- iOS: kept reads last only while the app runs. This is not proven yet: the
  iOS relaunch run used a cache entry that Tac's start-up sweep deletes, so
  the limit waits on a rerun on the simulator (plan item RM2).
- Android: a read kept shortly before the system stops the app is not always
  there after a relaunch.

### External links on native hosts (2026-10-18)

ADR 0018's amendment of 2026-10-18 covers links that leave the app. A link to
an `http`, `https` or `mailto` address opens in the default browser or mail
app; every other scheme opens nothing. This evidence does not promote any
tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes, `GtkUriLauncher` | `native_external_link_tests::the_linux_host_hands_external_links_to_the_launcher_and_keeps_the_page` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb in an x86_64 Ubuntu 24.04 cloud container, four of four runs: a page script's `location` changes, `window.open` calls and a `target="_blank"` link reached a fake launcher only for the `http`, `https` and `mailto` links, and the page stayed on `tachyon://app/`. The test also passed in the WSL (aarch64) run of `38eaed29` and under headless Wayland in RL1, and RL1's `scripts/linux/desktop-links-test.sh` handed `https` and `mailto` links from the real `GtkUriLauncher` to a default handler, with and without the OpenURI portal. Not run: a desktop |
| Windows WebView2 | yes, `ShellExecuteW` | the opener and the shared check, built with MinGW-w64 and run under Wine 9.0, open only the external links. On 2026-10-24 `scripts/windows/external-links-test.mjs` opened http, https and mailto links from a WebView2 page on the signed-in Windows 11 ARM64 desktop (SD1) |
| macOS, iOS WKWebView | yes, `NSWorkspace` and `UIApplication` (before 2026-10-18 for `http` and `https`) | source checks only for the `mailto` and no-user-name rule; not run on the Mac |
| Android WebView | yes, `ACTION_VIEW` (before 2026-10-18 for `http` and `https`) | source checks only for the `mailto` and no-user-name rule; not run on an emulator |

### Deep links on native hosts (2026-10-19)

ADR 0106 adds `tac.application.links`. A link with a listed scheme, or an
`https` link to a listed host, opens the app at its route. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: `.desktop` scheme handlers, `G_APPLICATION_HANDLES_OPEN` forwarding | `native_deep_link_tests::the_linux_app_opens_links_at_their_route_and_forwards_later_links_to_the_running_app` compiled the generated host with GTK 4 and WebKitGTK 6.0 (warnings as errors) and ran it with `dbus-run-session -- xvfb-run -a` in an x86_64 Ubuntu 24.04 cloud container: the launch link opened its route, and two later launches handed their links to the running window. In RL1 it passed on WSL (aarch64) and under headless Wayland, and `scripts/linux/desktop-links-test.sh` installed the `.deb`'s `.desktop` file and opened `linkgate://` links with `gio open` and `xdg-open` in the one running app. Not run: a desktop |
| Windows WebView2 | yes: per-user scheme registration (MSIX: `windows.protocol` extensions), single instance, `WM_COPYDATA` forwarding | `native_deep_link_tests::a_second_windows_instance_forwards_its_link_to_the_first` built the registration and forwarding code with MinGW-w64 and ran it under Wine 9.0: the schemes were registered under `HKCU\Software\Classes` and a second process's link reached the first. `windows_package_tests::the_staged_layout_packs_and_signs_with_the_msix_sdk` packed a package that declares two schemes with `makemsix`, which validated its `windows.protocol` extensions against Microsoft's schemas (and refused an unknown extension category). On the Surface desktop (Windows 11 arm64, 2026-10-19), a built app opened `lk2check://alpha?x=1` at `/alpha/?x=1` on launch; later links reached the running window and their second instance exited, and an unknown route opened `start_url`. An installed test-signed MSIX package opened `lk2check://beta?x=5` at its route, and Windows, not the app, registered the scheme. Host links (LK3, 2026-10-19, commit `ce1b47e5`, x86_64 Ubuntu 24.04 cloud container): the same `makemsix` test packed and signed a package whose manifest also declares a `uap3:AppUriHandler` host, and `makemsix` refused a misspelled `uap3` element against the schema; the `windows-app-web-link` family name matches the publisher ID Windows gives Microsoft's own subject (`8wekyb3d8bbwe`). On 2026-10-24 the package registered in Developer Mode handed later `sd1check` links to its one running process (SD1). Not run: a host link opening the installed package, which needs a public domain serving `windows-app-web-link` |
| macOS, iOS WKWebView | yes: `CFBundleURLTypes`, `kAEGetURL` (macOS), launch URL, `open url` and universal-link activity (iOS); associated domains with `TAC_APPLE_TEAM_ID` | on the Mac (2026-10-19, Swift 6.4.1): `the_apple_link_rule_opens_declared_schemes_and_hosts_at_their_route` passed and the iOS host tests passed. A built macOS app opened `lk2check://alpha?x=1` at `/alpha?x=1` on launch; while it ran, later links moved the same window, and an unknown route opened `start_url`. The same app on an iOS 26.5 simulator (iPhone 17 Pro) did the same on a cold start and while running. macOS rewrites `#` to `%23` in a link without `//` (`lk2check:alpha#frag`), so such a link opens `start_url`. Host links on macOS (LK3, 2026-10-19, macOS 26.6.2, Xcode 27.1, Swift 6.4): `native_deep_link_tests::the_macos_app_delegate_opens_universal_links_to_listed_hosts_at_their_route` compiled the generated host and handed it browsing-web activities through the `NSApplicationDelegate` requirement: a listed host opened its route with its query, an unknown route opened `start_url`, and an unlisted host and a non-browsing activity were refused. An unsigned `ty build --target macos` printed the links note and opened `lk3check://alpha?x=1` at `/alpha?x=1`. The Developer ID profile check and the entitlements are unit-tested. Owed: a signed device check of universal links on iOS, and a signed macOS release with a Developer ID profile that has Associated Domains opening a link from a hosted domain |
| Android WebView | yes: intent filters, `autoVerify` for hosts, `singleTask` with `onNewIntent` | the link rule ran with OpenJDK in the cloud. On emulator-5554 (2026-10-19), a launch link opened its route, later links reached the running activity through `onNewIntent`, and an unknown route opened `start_url`. Owed: verified app links for hosts on a device |

### Browser APIs on native hosts (2026-10-19)

ADR 0065's amendments of 2026-10-18 and 2026-10-19 cover `clipboard`,
`share`, `notifications` and `geolocation` in native apps. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: the web view's clipboard and geolocation (GeoClue) behind one permission decision from the manifest grants; notifications bridged to `GNotification`; share copies to the clipboard | `linux_browser_api_tests::linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb with XTest clicks in an x86_64 Ubuntu 24.04 cloud container, with and without a window manager: granted, the page read another source's clipboard text, wrote its own, got a test position, was granted and showed a notification, and shared a url to the clipboard; with nothing granted, each was refused. The test also passed in the WSL (aarch64) run of `38eaed29` and again in RL1 on `d7110f74`. LX1 ran `linux_browser_apis_reach_geoclue_and_the_notification_server` through `scripts/linux/desktop-services-test.sh`: the page's position came from GeoClue 2.7.0 on a private system bus (a GPS fix from an NMEA socket, allowed by the GeoClue demo agent) and its notification was shown by dunst 1.9.2. Not run: a physical Linux desktop; Wayland, because the test clicks through X11's XTest |
| macOS WKWebView | yes, all four: `NSPasteboard`, `NSSharingServicePicker`, `UNUserNotificationCenter`, CoreLocation | code level only, in a Linux cloud container (`native::browser_api_bridge`, `native::apple_browser_apis`). Owed: the compile and host test on a Mac, and a manual check of the share, notification and location prompts |
| iOS WKWebView | yes, all four: `UIPasteboard`, `UIActivityViewController`, `UNUserNotificationCenter`, CoreLocation | code level in a Linux cloud container (same tests). The simulator gate `scripts/ios/native-test.mjs` passed with its clipboard step on develop `fd0384ad` (iPhone SE (3rd generation) simulator, iOS 26.5): the bridge listed all four APIs, the page wrote text that `simctl pbpaste` read from `UIPasteboard`, the page read it back, and a share without a grant was refused. Owed: share, notifications and geolocation with a grant on the simulator, and a manual check of the system prompts |
| Android WebView | yes: clipboard, share and notifications through the host bridge; geolocation through the web view (BA2) | see the Android WebView row above |
| Windows WebView2 | yes: clipboard bridged to the Windows clipboard; share through the Windows share sheet (`IDataTransferManagerInterop`); notifications as toasts for the installed MSIX package (an unpackaged build answers `denied`); geolocation through WebView2 behind one `PermissionRequested` decision from the manifest grants (BA3) | `windows_browser_api_tests::the_windows_browser_apis_follow_the_manifest_grants_under_wine` compiled the bridge with MinGW (warnings as errors) and ran it under Wine 9.0 in an x86_64 Ubuntu 24.04 cloud container (BA3 branch on develop d7110f74): granted, a real clipboard round trip, the toast XML, the permission answers and the share values with a fake notifier and share sheet, and the permission decision; with nothing granted, each was refused. The whole generated host compiled with MinGW against the WebView2 1.0.3485.44 headers. `the_windows_browser_apis_match_the_windows_sdk_and_run_on_windows` passed on the Surface (Windows 11 ARM64 over SSH, MSVC 19.44.35229 ARM64, Windows SDK 10.0.26100.0, BA3 commit 971c6559): every declared Windows Runtime slot matched the SDK headers under `cl /W4 /WX`, and the program ran granted and refused with the runtime loaded and the share sheet opening for a message-only window. On the same machine `ty build --target windows` compiled and linked the desktop fixture app with all four APIs granted (`/W4 /WX`, WebView2 SDK 1.0.3485.44). On 2026-10-24 `scripts/windows/browser-apis-test.ps1` passed inside WebView2 on the signed-in desktop (SD1): the clipboard both ways, the share sheet opened, geolocation granted with no prompt, every API refused when none is granted, and, from the MSIX package installed by Developer Mode registration, a toast in the notification history. Not checked: whether the user shared from the sheet, which Windows does not report |

### File picker on native hosts (2026-10-19)

ADR 0089 and its amendments cover `tac.nativeCapabilities: ["filePicker"]`.
This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: `GtkFileDialog`, or the desktop portal's chooser where one runs, for JavaScript, TypeScript and linked Rust services, and for Swift, C# and Kotlin services, which run as WebAssembly and reach the dialog through the page (FP5) | `linux_file_picker_tests::linux_generated_host_opens_the_gtk_file_dialog_for_a_page_and_a_rust_service` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and drove the real dialog with XTest key presses under Xvfb in an x86_64 Ubuntu 24.04 cloud container on 2026-10-19: three runs through the portal's chooser (xdg-desktop-portal-gtk, xfwm4) and one each through GTK's own dialog with and without a window manager. It also passed on WSL Ubuntu 24.04 (aarch64, same GTK and WebKitGTK, Xvfb with xfwm4), once through each dialog; one earlier WSL run with a broken portal backend hit the 30-second dialog limit and answered `cancelled`, and its rerun passed. A page and a Rust service chose, read and released files; cancel, the second dialog, the handle and owner limits, the lifetime, another owner, a large and a non-UTF-8 file, the missing grant and a new page were checked. On 2026-10-24 (FP5) `scripts/linux/page-services-native-test.mjs` ran the packaged `native-capabilities` app with a Swift (6.3.3), a C# (.NET 10.0.302) and a Kotlin (2.1.10) WebAssembly service under Xvfb with xfwm4 in the same kind of container: each chose a file in GTK's dialog, read it, and answered `cancelled` after Escape. Not run: a Linux desktop, Wayland |
| macOS WKWebView | yes: `NSOpenPanel`, also for C# and Kotlin services, which run as WebAssembly and reach it through the page (FP5) | see ADR 0089 and its amendments. Owed: a C# and a Kotlin field choosing a file in the panel on a Mac |
| Windows WebView2 | yes: `IFileOpenDialog` for JavaScript, TypeScript and linked Rust and C# services (FP1), and for Swift and Kotlin services, which run as WebAssembly and reach it through the page (FP5) | under Wine in the cloud only; see the Windows paragraph above. On 2026-10-24 the whole generated host, with the page route, compiled with MinGW against the WebView2 1.0.3240.44 headers and the Wine picker checks passed. On 2026-10-24 the MSVC build selected a file in the Windows dialog and closed an unanswered one on the signed-in desktop (SD1). Owed: a page, a Swift and a Kotlin picker from a WebView2 page on a signed-in desktop |
| iOS, Android | no: `unsupported` (FP4, FP2) | see the per-target support table |

### Window settings on iOS and Android (2026-10-19, WIN5)

`theme_color`, `background_color`, `display`, `orientation` and
`tac.application.window.titleBar: "integrated"` now reach the mobile apps
(ADR 0052 and ADR 0098 amendments). This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| iOS WKWebView | yes: the status bar strip and its text color, the web view and launch screen color (an asset catalog color), the hidden status bar, the supported interface orientations, edge to edge with the three `--tac-titlebar-*` properties from the safe-area insets, and `WindowControls` for linked Swift services | `scripts/ios/window-settings-test.mjs` on the Mac (macOS 26.6.2, Xcode 27.1 27A9275, iOS 26.5 simulators, WIN5 branch on develop eefb70b0) built three apps and read the running page, a linked Swift service and a screenshot. iPhone 17 Pro: a standard window with a dark `theme_color` showed it across the 62-point status bar with light text (`statusBarStyle` 1), started the page at the safe-area top and showed `background_color` through the transparent page; the bundle's `Assets.car` held the launch color. An integrated window with a light `theme_color` put the page at the top edge with dark text (3), set `--tac-titlebar-height` to `62px`, and the page's own bar filled the status bar in the screenshot. `display: "fullscreen"` with `landscape` hid the status bar and opened in landscape (750 by 382). The linked Swift service saw the `title` grant and no `minimize` grant and set the scene title to `Quarterly report` on the iPhone and on an iPad Pro 13-inch (M5). Not checked: the launch screen on screen, a device turned by hand, and a physical device |
| Android WebView | yes: the status and navigation bar colors and icon colors, the window, splash screen and web view color, hidden system bars, `screenOrientation`, and edge to edge with the three properties from the system-bar and cutout insets | `scripts/android/window-settings-test.mjs` on the Mac's API 35 arm64 emulator (`tachyon-gate`, 1080 by 2400 at 420 dpi) built three APKs and read the page over DevTools, `aapt2` for the manifest and `screencap` pixels. A standard window filled the status and navigation bars with `theme_color` and showed `background_color` through the page (412 by 842). An integrated window drew the page under the status bar, set `--tac-titlebar-height` to `49px`, and the page's bar reached the bottom of the status bar. `display: "fullscreen"` with `landscape` opened in landscape with no system bars (866 by 412, the 49 dp cutout still pads the leading side). Not checked by the run: the bar icon colors, the splash screen, Android below API 35, and a physical device |

### Web storage across relaunch (2026-10-18)

ADR 0059's amendment of 2026-10-18 covers this. Each host check writes an
IndexedDB row, a localStorage value and a Cache API entry, quits the
application so its process ends, starts it again and reads all three back.
This evidence does not promote any tier.

| Host | Kept across relaunch | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes | `linux_storage_tests::linux_web_storage_is_kept_after_the_application_quits_and_starts_again` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 and ran it twice under Xvfb on WSL Ubuntu 24.04 (aarch64). The Cache API entry went through the framework's `tac-storage.js`. The page writes all three about 20 ms before the quit; without the 750 ms settle at quit the localStorage value was lost in 4 of 10 runs, with it 10 of 10 kept it. With the old ephemeral session the same test fails |
| Android WebView | IndexedDB and localStorage yes; Cache API entries not always | `scripts/android/native-test.mjs` on an API 35 arm64 emulator (WebView Chrome 124), across `am force-stop` and a new process: a `@LocalState` field and a Store row saved and found by id were kept in every run. The `@Cached` entry, written about 6 s before the force-stop, was kept and served in 3 runs and missing from Cache Storage in 2 of the 3 runs that checked it before the read |
| macOS WKWebView | yes | `scripts/phase4-macos-test.mjs` on 2026-10-19 (macOS 26.6.2, Xcode 27.1) with Accessibility access: the first launch wrote all three, the app quit, and a new process read `local=kept,store=kept,cache=kept`. An earlier run looked like a frozen page after relaunch. The page was running, but the gate's check set its text after OnStart without asking for a render, and its raw Cache API entry had no `x-tachyon-expires-at` header, so Tac's start-up cache sweep deleted it. The check now calls `taskScheduler.render()` and writes the entry in Tac's format |
| iOS WKWebView (simulator) | IndexedDB and localStorage yes; Cache API entries no | `scripts/ios/native-test.mjs` on an iPhone SE (3rd generation) simulator with iOS 26.5 (Xcode 27.1): a real terminate, a new process, and the row and value read back. The cache name came back with no entries, and WebKit wrote no record file for it before the quit. That run used the check's old raw cache entry, which Tac's start-up cache sweep deletes, so the iOS cache result is owed a rerun |
| Windows WebView2 | yes | WebView2 keeps its data in `<App>.exe.WebView2` (ADR 0097). On 2026-10-24 `scripts/windows/relaunch-storage-test.mjs` kept localStorage, IndexedDB and the Cache API across a relaunch on the signed-in Windows 11 ARM64 desktop (SD1) |

### Browser engines for the web target (2026-10-19)

On 2026-10-19 (BR1) these browser gates ran in Chromium 141, Firefox 153 and
WebKit 26.5 (Playwright 1.62.1) in an x86_64 Ubuntu 24.04 cloud container,
with `ty` built from develop `d7110f74`, and all passed (the browser-API,
held-client and storage gates passed again on `162064a0`):

- `scripts/page-program-browser-gate.mjs`: one page program for C++, C#,
  Dart, F#, Go, Java, JavaScript, Kotlin, Rust and Swift (ADR 0022
  amendment).
- `scripts/web-apis-browser-test.mjs`: browser APIs for JavaScript, Rust, Go,
  Swift, Java, F#, C# and Kotlin services, and a `tac.dotnet.aot` release
  build (ADR 0065 amendment).
- `scripts/held-clients-browser-gate.mjs`: socket and stream clients held by
  Rust, Go, Dart, Swift, Kotlin, Java, C#, F# and C++ services against
  `ty serve` (ADR 0060 amendment).
- `scripts/storage-browser-test.mjs`: Stores, caches and stored fields with
  every service language, Ruby in WebKit included (ADR 0059 amendment).

The Python Tac gate already ran in all three engines (ADR 0093). A container
run is not a desktop browser install, and Safari itself was not run; WebKit
stands in for it.

### Linked services share one program (2026-10-24, LS2)

`native/linked_page_program_tests.rs` ran in an x86_64 Ubuntu 24.04 cloud
container: two linked services share one Setup configuration in Swift
(Swift 6.2 on Linux), Kotlin (Kotlin 2.1.10 on the JVM) and C# (.NET
10.0.302, net9.0), and the C# native task fixture's two services both reach
Yon. These run the generated companion outside its app; the macOS, iOS
simulator and Android emulator hosts are not run yet (ADR 0022 amendment of
2026-10-24). On 2026-10-24 the C# test passed on Windows 11 ARM64 too, with
`dotnet run` and published with NativeAOT for `win-arm64`
(`two_linked_csharp_services_share_one_setup_configuration_when_published_with_native_aot`):
both services read the same identity, and the shared setup ran once.

### Yon handlers on Windows (2026-10-19, WY1)

On Windows 11 ARM64 (IYOR-Surface, over SSH, commit 3d264ff3), with clang++
21.1.8, Temurin 21, Kotlin 2.1.10, Groovy 5.1.3, Dart 3.13.4, PHP 8.4.20,
Lua 5.4.6 and Go 1.27.1 on PATH:

- `yon_cpp*` (C++ Yon handlers: typed routes, settings, services and stores,
  sockets, streams, a scheduled worker, answers, errors, the HTTP client): 21
  of 22 passed, the same as develop d7110f74. C++ and Rust handlers are now
  compiled to a `.exe`. The one failure is
  `yon_cpp_outbound::a_cpp_gateway_calls_a_command_and_sees_only_fixed_failures`:
  its gateway runs `python3`, which on that machine is the Microsoft Store
  alias (exit 9009), not Tachyon.
- `controller_languages`: Groovy, Kotlin, PHP, Lua, Dart, Java, C#, F#, Ruby,
  Rust and Go controllers bound arguments and models and answered. Python
  skipped for the same `python3` alias.
- `outgoing_loopback`: Java, Kotlin, Dart, Go and JavaScript handlers got
  `403` from the outgoing guard for an unlisted loopback port, and the Java,
  Kotlin and Dart handlers reached a listed `api.localhost` through it (ADR
  0094 amendment of 2026-10-19). Before that change Java and Kotlin skipped
  the guard.
- `handler::source` unit tests: 60 passed.

Kotlin and Groovy ship only `.bat` launchers on Windows; Tachyon's runtime
already ran them through the JVM, but the tests probed for them by name and
skipped. The probes now ask `cmd`. Since CI1 (2026-10-19) the windows-latest
native gate installs PHP, Groovy, Kotlin, Dart and Lua, but its first run
with them (39253757046) failed 44 tests, so the Windows handler evidence is
still the Surface run above (WG1).

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
| Linux GTK4 | buildable with container execution evidence | pinned Debian trixie container compiled the host with warnings denied and launched it headlessly. It read back over AT-SPI (the Linux accessibility bus) seven declared names, native roles, and an `Atspi.Action` activation |
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
| Rust workspace and Phase 4 CLI on Linux x86_64 | native-tested development milestone | updated 2026-10-18. `rust-ci.yml` runs the locked workspace suite on `ubuntu-latest`. The release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) (v26.36.01, 2026-08-31) clean-installed and checked the released `ty-linux-x64` on an `ubuntu-24.04` runner. Before, this row said the suite ran only in a pinned Linux container. macOS packaging still requires a macOS host |
| Rust workspace and Phase 4 CLI on Windows x86_64 | native-tested development milestone | updated 2026-10-18. The release builds `ty-windows-x64.exe` for `x86_64-pc-windows-msvc` on `windows-2025`. The release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) (v26.36.01, 2026-08-31) clean-installed and checked that executable on `windows-2025`, and `rust-ci.yml` runs the locked workspace suite on `windows-latest` (its run 39253757046 of 2026-10-19, the first with every Yon language installed, failed 44 tests; see Yon handlers on Windows). Before, this row said `buildable` (only the GNU target compiled locally). No Windows ARM64 `ty` is released. This row is about the CLI only; for the WebView2 host see the tables above |
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
