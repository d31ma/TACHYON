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
| Web (static site in `dist/web`) | native-tested (candidate) | `rust-ci.yml` runs the browser gates on every push, on Linux x64, in Playwright's browser builds: all of them in Chromium, and some (storage, the offline preview, Yon reads kept offline, web APIs and others) also in Firefox and WebKit. Installability (the web app manifest, its icons and the worker, `scripts/pwa-install-browser-gate.mjs`, 2026-10-25) is checked in Chromium only, through the DevTools Protocol; Firefox and Safari have no installability check to ask, and opening the installed app in its own window is not run. Since CI2 (2026-10-19, ADR 0022 amendment) its `browser-languages` job runs all seven Tac service-language browser gates (page programs, browser APIs, held clients, storage, cached clients, async listeners and window controls) in Chromium, Firefox and WebKit, with every language toolchain installed. `publish.yml` builds pages with the downloaded `ty` and runs the storage, runtime and website gates in Chromium, Firefox and WebKit. The v26.36.01 release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) ran that step in Chromium only, because the step installed only Chromium then | a release run of the three-browser step; branded Chrome and Edge on Windows. Branded Safari on macOS and the iOS simulator and Chrome on Android ran the web gates on 2026-10-25 (BR2, "Branded browsers" below); the gates WebDriver or one shared Android context cannot drive stay unrun there |
| macOS WKWebView | native-tested (candidate) | full arm64 application gate passed: controls, accessibility, typing, lifecycle and visual parity at 420x752 | exact release artifact and current-head CI |
| iOS WKWebView | simulator-tested | isolated iPhone SE (3rd generation), iOS 26.5. Passed: Swift calls, publish, input/focus, assets and route state `7 -> 9 -> 7`. On 2026-10-19 the same gate also linked a Rust service (`aarch64-apple-ios-sim` static library, beside the two Swift services): its start hook wrote a file from the app's own process and its rendered count doubled after a page click (MR1) | exact release artifact; support on physical devices is still not claimed |
| Android WebView | simulator-tested (emulator) | Android 15 emulator. Passed: Kotlin calls, publish, input/focus per character with a delayed setter, service-worker retirement, packaged dynamic routes, window controls (granted minimize, resize and title act; ungranted maximize, fullscreen and close are refused, 2026-10-18), and a `@LocalState` field set by a Kotlin method kept across navigation and reload. Browser APIs (clipboard, share, notifications through the host; geolocation through the web view with the host's location question, 2026-10-19): `scripts/android/browser-apis-test.mjs` passed end to end on 2026-10-24 (RM3), with geolocation fed by `fused`, `network` and `gps` test providers. RM3 also ran granted maximize, fullscreen and close, background and foreground, external links, offline Yon reads, preference strings after a relaunch and a Python service, and the Yon reach, linked Rust and release checks on a Google Play production image ("Android runs owed, RM3" below) | exact release artifact; support on physical devices is still not claimed |
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
WebView debugging off skipped there. On 2026-10-24 (RM3) the same gate ran on
the Google Play production image (`ro.debuggable=0`), where the check
asserted and passed: the release APK opened no WebView DevTools socket.

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
Play image (RM3). RM3 ran both later the same day: the R8-shrunk release APK
installed and ran on the emulator and on a Google Play image, where the
release DevTools check passed ("Android runs owed, RM3" below).

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
cloud container and on WSL (aarch64), started it from `PATH` and from its
desktop file, and purged it. Not yet run: an install on a physical Linux
desktop.

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
`--release` with an App Store profile, installing and launching on an iOS 17.4
or later device (the iOS floor since ADR 0108; the only registered device is an
iPod touch on iOS 15), and an App Store Connect upload.

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
| iOS WKWebView | route items yes (MN1a); action items and the application lifetime yes (LT1, MN1b); the application `OnStop` runs when a person ends the app from the app switcher (MA4), not when iOS ends a suspended app | `scripts/ios/menus-test.mjs` on this Mac (2026-10-19, MN1a, Xcode with iOS 26.5): on an iPhone 17 Pro simulator Command-K from a simulated hardware keyboard reached the route item through `keyCommands`, and on an iPad Pro 13-inch (M5) simulator through the main menu that `buildMenu(with:)` fills; a Swift service on the opened page recorded `/status` both times. On 2026-10-24 (LT1, MN1b) `scripts/ios/application-lifetime-test.mjs` passed on the same simulators (iOS 26.5, Xcode 27.1, commit `566c24e9`): one Rust application service started at launch and served two pages, Command-1 and Command-2 called it through the iPhone key commands and the iPad menu bar, and Command-1 did nothing on a page without a listener. iPadOS keeps Command-G for Find Next, so an item with that shortcut never fires on iPad. On 2026-10-25 (MA4, commit `e6c017f1`) the same script opened the iPad menu bar by touch, a swipe down from the top edge, and touches on View and What's Running called the application service. On iPhone and iPad, ending the app from the app switcher ran `applicationWillTerminate`: the open page's Rust service stopped first, then the application service, once. Not covered: the iPad menu bar opened by a pointer, a person's keyboard, and a stop when iOS ends a suspended app (iOS does not tell the app) |
| Android WebView | route items, action items and the application lifetime yes (MN1a, LT1, MN1b) | `scripts/android/menus-test.mjs` on this Mac's emulator-5554 (API 35 arm64, 2026-10-19, MN1a): the system keyboard shortcuts list (Meta+/) showed File > Status Ctrl+K, `input keycombination` Ctrl+K opened `/status`, and a tap on the app bar's More options and then Guide opened `/guide`, read through the WebView's DevTools socket. On 2026-10-24 (LT1, MN1b) `scripts/android/application-lifetime-test.mjs` passed there (commit `566c24e9`): one Rust application service started at launch, served two pages and a rotation, stopped once on Back, and started again on the next launch in the same process; Ctrl+1 and Ctrl+2 called it, the page showed the forwarded section without a reload, and the overflow item was disabled on a page without a listener |

Standard menus on Linux and Windows (2026-10-18, ADR 0097 amendment): both
hosts now always show a menu bar with File, Edit (Undo, Redo, Cut, Copy,
Paste, Select All) and Window (Minimize, Maximize), and a `control`
shortcut gets Ctrl. Linux: the same `linux_host_tests` run, extended to
check the five menus, the Copy and Redo accelerators and Edit > Select All
in the real web view, passed three of three runs under Xvfb in an x86_64
Ubuntu 24.04 cloud container (GTK 4.14.5, WebKitGTK 2.52.6), and the Linux
window-control run still passed with the always-on menu bar (xfwm4 under
Xvfb). Windows: generated-source tests and the MinGW-w64 compile, and on
2026-10-24 (SD1, below) `scripts/windows/menu-tray-test.ps1` ran the Edit
menu and Window Minimize and Maximize inside WebView2 on the signed-in
desktop.

Background and foreground (2026-10-18, ADR 0028 amendment): every host
sends `ApplicationStateChanged` to the page and to natively linked
services. Linux: `scripts/linux/application-state-test.sh` passed under
Xvfb and xfwm4 in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5,
WebKitGTK 2.52.6): a page service and a linked Rust service each heard the
app go to the background and come back when the focus moved to another
window, and go to the background when it was minimized. Android: on
2026-10-24 (RM3, below) a JavaScript page service and a linked Kotlin
service heard background after Home and foreground after the app came back
on the API 35 emulator. Windows: on 2026-10-24 (SD1, below)
`scripts/windows/application-state-test.mjs` passed inside WebView2 on the
signed-in desktop, where the page and a linked Rust service heard
background and foreground. On 2026-10-25 (SW1, below) a linked C# listener
heard them there too.
macOS (2026-10-24, RM1): `scripts/application-state-macos-test.mjs`
passed on a Mac. A page service and linked Rust and Swift services heard each
change when the app stopped and started being active, was hidden and shown,
and its window was minimized and restored. iOS: on 2026-10-24 (RM2, below)
the page and a linked Swift service each heard background and foreground on
the iOS 26.5 simulator.

Windows bundles accept `--arch arm64` or `--arch x64`. The default matches
an ARM64 build host and falls back to x64 on other architectures. This choice
sets the Rust companion target, WebView2 library folder, C# runtime identifier
and linker architecture together. The build refuses a developer prompt whose
`VSCMD_ARG_TGT_ARCH` disagrees with the selected architecture.

macOS bundles accept `--arch arm64`, `--arch x64` or `--arch universal`
(ADR 0097 amendment of 2026-10-18). The default is the build machine's
architecture. `universal` builds both architectures and joins them with
`lipo`. On 2026-10-24 (RM1) a universal build ran on an Apple silicon Mac:
`lipo -archs` gave `x86_64 arm64` for the app and its helper, and the x86_64
slice ran under Rosetta and answered from its linked Rust service ("macOS
runs owed, RM1" below). It needs the `x86_64-apple-darwin` Rust target. No
run on an Intel Mac yet. Linux and iOS builds use the build machine's
architecture.
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
the same app packaged both libraries in the APK and in the App Bundle. On
2026-10-24 (RM3) the gate builds that signed release itself, installs it and
taps through to the Rust service, on the `userdebug` emulator and on the
Google Play production image ("Android runs owed, RM3" below). Still owed: the
`x86_64` library on an x86_64 emulator or device, and a physical device.

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
read, and an unanswered dialog closed after its patience. The same day
(FP5, below) a JavaScript page service, a linked C# service and a
WebAssembly Kotlin service opened the picker from a WebView2 page on that
desktop. A WebAssembly Swift field did the same on 2026-10-25 (SW3, below).
A linked Rust field did the same on 2026-10-25 (SW1, below).

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
- WSLg from the signed-in desktop session (2026-10-24, SD1, WSLg 1.0.79 with
  Weston 9.0.0, on `172164f8`): `compositor-test.sh --wslg`, started on the
  Windows desktop through `scripts/windows/desktop-run.ps1`, passed eight
  host tests (the host lifetime, page calls, storage across relaunch, the
  WebKitGTK startup check, appearance, fullscreen and both window control
  tests) and the three link tests, and Weston did not crash. The two title
  bar tests are left out there: they press the pointer through Mutter's
  RemoteDesktop D-Bus API, which WSLg does not have, and they pass under
  mutter and xfwm4.
- WSL, round 4 on `fe191f12`, after Samuel installed mutter 46.2, xdg-utils
  1.1.3 and gcc-mingw-w64-x86-64 13.2.0: `compositor-test.sh` passed both
  halves (xfwm4 compositing two and two; headless mutter Wayland ten host
  tests and three link tests), and `desktop-links-test.sh` passed through GIO
  and through the OpenURI portal, with the installed `.desktop` file opening
  `linkgate://` links in one running app. The two Windows link tests now
  compile with mingw but need Wine to run the x86_64 `.exe`, which arm64 WSL
  cannot run without an x86 emulator; they belong on Windows or x86_64 Linux.
- Not run: a physical Linux desktop.
  GeoClue, a notification server and an apt install of the `.deb` as root
  ran in LX1 (below), in the cloud and on WSL.

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
  and C# and Kotlin `WindowControls` callers. On 2026-10-25 (SW3, below) a
  Swift page service, `WindowControls` caller and FilePicker field passed
  with Swift 6.3.3 for Windows ARM64;
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
- `scripts/windows/page-services-native-test.mjs` with
  `TAC_WINDOWS_PICKER_CALLERS=js,kt,cs` (FP5, Windows 11 25H2 build
  26200.9711, .NET 10.0.302, Kotlin 2.1.10): `FilePicker` fields in a
  JavaScript page service, a WebAssembly Kotlin service and a linked C#
  service each opened the file dialog from the page, took a path typed into
  its file name box, read the file back, and answered `cancelled` (C#
  `Cancelled`) when the next dialog was closed with Escape;
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
the installed package (no verified domain) and x64 on an x64 machine. The
first two ran later: FP5 opened the picker from a WebView2 page the same day
(above), and LINKS opened a `d31ma.github.io` host link in the installed
signed package on 2026-10-25 ("Deep links on native hosts", below). x64 on
an x64 machine is still owed. After a
restart of the WSL distribution, whose disk had gone read-only, the WSLg
Wayland tests ran from the same signed-in session (see the WSL list above).

### Linux runs under Wayland and on WSL (2026-10-25, LX2)

LX2 ran four owed Linux checks on branch `claude/project-thread-ye5nxq`
(`6d51eb78`, develop `4243f2b6`), each in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5, WebKitGTK
2.52.6, mutter 46.2) and on WSL Ubuntu 24.04.5 (aarch64) on the Surface (the same GTK, WebKitGTK and mutter).
This evidence does not promote any tier.

- `scripts/linux/compositor-test.sh` now runs
  `linux_browser_api_tests::linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view`
  and `linux_file_picker_tests` in its headless mutter Wayland session. When
  GDK runs on Wayland, the fixtures click and press keys through Mutter's
  RemoteDesktop session (`NotifyPointerButton`, `NotifyKeyboardKeysym`)
  instead of XTest, with the window maximized so that its position is known.
  Mutter gives the remote session a keyboard only after its first key, so
  the session presses Shift once at start; until then no window is active.
  All 12 host tests passed under Wayland in both places, among them the
  browser APIs granted and refused, the file dialog and the window
  appearance with `background_color`; the X11 runs of both tests still pass
  under Xvfb.
- `scripts/linux/page-services-native-test.mjs` built a C# Tally service
  with `tac.dotnet.aot: true` and drove it under Xvfb: it answered the
  click. `dotnet.native.wasm` was 5.79 MB, against 2.98 MB (cloud) and
  1.46 MB (WSL) for the interpreter build of the same service. The Linux
  build prints no AOT note; the runtime size shows the AOT build.
- The same script built a linked Rust service (`libtachyoncompanion.so`)
  whose `#[LocalState]` field is set by its own `visit` method. The page
  showed `Visits 1`, a page reload cleared a plain page field (`Page
  fresh`) and still showed `Visits 1`, and the next visit showed `Visits 2`
  (N3).

On WSL the first page answered about 52 s after launch (the AT-SPI join
waits on the portal), against under 2 s in the cloud.

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
- WSL Ubuntu 24.04.5 (aarch64) on `e847d3a1`, after `geoclue-2.0` 2.7.0 and
  `dunst` 1.9.2 were installed as root: `desktop-services-test.sh` passed
  (GPS fix from GeoClue at accuracy level 8, "Order / Shipped" shown by
  dunst); the Yon reach test passed under headless mutter Wayland with
  JavaScript and with linked Rust services, and under X11 as a control; and
  `deb-install-test.sh` as root installed `nativegate_0.0.1-7_arm64.deb`,
  started it both ways and purged it. Two earlier Wayland rounds timed out
  while the USB drive that holds the WSL disk was resetting; the round with no
  resets passed both.

#### Android runs owed, RM3 (2026-10-24)

RM3 ran the Android rows of the runs-owed list on this Mac (Apple silicon,
macOS 26.6.2) on 2026-10-24, from branch `claude/rm3-android-20261024`: first
on develop `4c052177`, then all nine Android scripts and the byte-identical
test again after a rebase onto develop `259818ee`, which adds R8 shrinking to
`--release` (PKG7), so the release runs below are of shrunk APKs. The nine
scripts passed once more on develop `5c8e46ca`, the commit this branch
landed on, on both AVDs. `ty` was
built from that branch (debug), with Node 24.19.0,
Playwright over the WebView DevTools socket, Gradle 8.14.3, JDK 21 and NDK
r30. The emulator was emulator-5554 with the `tachyon-gate` AVD: Android 15
(API 35), `google_apis` arm64-v8a, `userdebug` (`AE3A.240806.043`), WebView
124.0.6367.219. Each script was run alone with `ANDROID_SERIAL=emulator-5554`
and `TAC_BIN` set, as `node scripts/android/<script>.mjs`.

- Window controls (`window-controls-test.mjs`): after the refusal checks, a
  second app grants all six controls. A linked Kotlin service's `maximize()`
  brought a 1280 by 800 window back to the full page size (412 by 842), and
  its `fullscreen()` hid the system bars (the status bar's `InsetsSource` in
  `dumpsys window` turned `visible=false`). `window.fullscreen` from the page
  showed them again. `window.close` from the page answered `ok` and finished
  the activity, and a `window.maximize` sent in the same moment, which reached
  the host after close started, was refused with "Native window is
  unavailable."
- Background and foreground (`page-checks-test.mjs`, new): a JavaScript page
  service and a linked Kotlin service both heard `ApplicationStateChanged`:
  `foreground` (replayed), then `background` after Home (`onStop`) and
  `foreground` after the app came back (`onStart`).
- External links (`page-checks-test.mjs`): taps on `https`, `http` and
  `mailto` links (through `uiautomator` bounds and `input tap`) started
  `ACTION_VIEW` from the app's own uid, which `ActivityTaskManager` logged.
  Chrome opened the web links and Gmail the mail link, and the page stayed
  where it was. A link with a user name (`https://ada@example.net/...`)
  started nothing.
- Offline Yon reads (`page-checks-test.mjs`): with `tac.yon.origin` set to a
  local Yon stand-in (reached through `adb reverse`) and `tac.cache` rules for
  a network-first and a cache-first path, both reads were kept online. Then
  airplane mode went on (`cmd connectivity airplane-mode enable`) and the
  reverse route was removed, because loopback through adb does not go through
  the device network. A read with no rule failed, the network-first read and
  the cache-first read answered their kept copies from Cache Storage, and the
  Yon stand-in saw no request. Airplane mode was turned off after the check.
- `preferenceStrings` (`page-checks-test.mjs`): a write was read back after
  Home, `am force-stop` and a relaunch in a new process.
- Browser APIs (`browser-apis-test.mjs`): the whole gate passed for the first
  time. The geolocation step timed out before because the web view asks the
  platform `fused` provider at balanced power, which ignores a `gps` test
  provider. The gate now adds `fused`, `network` and `gps` test providers and
  refreshes their position while it waits. It also denies the first location
  question, and before the second question the app showed the manifest reason
  ("Shows stores near you") from `tac.application.permissions.geolocation`.
  The clipboard round trip, the share sheet, the `POST_NOTIFICATIONS` question,
  a posted notification and its channel name ("Order updates") passed too.
- Python (`python-service-test.mjs`, new): the website's Tally page with
  `TallyService.py` on Pyodide 314.0.7. The APK (14.4 MiB) carried the five
  runtime files in its web bundle, and the page loaded all of them from
  `https://appassets.tachyon.local/.tachyon/services/python/314.0.7/`
  (`pyodide.asm.wasm` by 454 to 631 ms after the page started). The first
  answer to a click came 2.9 to 3.7 s after `am start` in two runs.
  `dumpsys meminfo` showed 84 to 87 MiB PSS in the app process and 112 to
  117 MiB in its web view renderer.
- Byte-identical rebuild: `cargo nextest run -p tachyon-core --lib
  --run-ignored only -E
  'test(a_packaged_android_app_is_byte_identical_when_built_twice)'` passed
  with `ANDROID_HOME` set (31 s, and 83 s after the rebase): two fresh
  packaged Android builds and an incremental one gave the same
  `artifact-manifest.json`.
- The Yon reach gate needed a fix in `scripts/supervised-command.mjs`. After
  the rebase it failed three runs in a row with "Process identity inspection
  returned an unsupported record": the runner reads `ps` every 50 ms while
  `ty serve` runs, and on this busy Mac one `ps` answered with no output, no
  error and exit 0. A process table always holds at least `ps` and node, so
  the runner now asks `ps` again, up to three times, when its answer is
  empty. `scripts/supervised-command-test.mjs` still passes, including its
  check that a failing `ps` leaves cleanup unconfirmed.
- Signed release with linked Rust (`rust-services-test.mjs`): after the debug
  checks, the gate builds `ty bundle --target android --release` with a
  throwaway keystore. `apksigner` and `jarsigner` verified the APK and the App
  Bundle, and both carried `libtachyoncompanion.so` for `arm64-v8a` and
  `x86_64`. The release APK installed, was not debuggable, and a tap (through
  `uiautomator`, without DevTools) reached the linked Rust service, whose count
  the page showed.

Production-image stand-ins for a physical device: the Mac has no Android
device, so a second AVD, `tachyon-play`, runs the Google Play image
(`system-images;android-35;google_apis_playstore;arm64-v8a`, `user` build
`AE3A.240806.036`, `release-keys`, `ro.debuggable=0`), on the same port in
turn. On it `yon-origin-test.mjs` (Yon reach from the page and a linked Kotlin
client) and `rust-services-test.mjs` (linked Rust, and the signed release APK
installed and run) passed, and `native-test.mjs` passed with its release check
asserting, not skipping: the release APK opened no
`webview_devtools_remote_<pid>` socket. These are emulator runs on a
production image, not device runs. Still owed: the `x86_64` Rust library on
an x86_64 emulator or device (this Mac's emulator runs arm64 only), and runs
on a physical device.

#### macOS runs owed, RM1 (2026-10-24)

RM1 ran the macOS rows of the runs-owed list on a signed-in Mac: an Apple
M4 Mac (Mac16,10) with macOS 26.6.2 (25G83), Xcode 27.1 (27A9275), Swift
6.4, Rust 1.97.1 and Node 24.19.0, on develop `4c052177`, with debug `ty`
builds. Microsoft Edge 156 was the default browser and the default `mailto`
handler. Each item below names its command.

- Window controls: `scripts/window-grant-macos-test.mjs` passed (a granted
  resize from a service widened the window to 480; without the grant it
  stayed 420). A page granted `close`, `fullscreen`, `maximize` and
  `minimize` drove each from a click: minimize miniaturized the window,
  maximize zoomed it to 1920x960 and back to 640x512, fullscreen entered
  and left full screen (1920x1080), and close removed the window. A
  `title` grant renamed the window to `Quarterly report`. Without the
  `maximize` grant the click showed the readable refusal and the window kept
  its size.
- Minimum size: a window with `minWidth` 520 and `minHeight` 400, dragged
  400 points smaller from its corner, stopped at 520 by 432 (400 plus the
  32-point title bar).
- Background and foreground: the new
  `scripts/application-state-macos-test.mjs` builds an app with a page
  service, a linked Rust service and a linked Swift service. All three heard
  `background` and `foreground` when another app became active and the app
  came back, when the app was hidden and shown, and when its window was
  minimized and restored. The script drives the app through Accessibility.
- External links: a page's `http`, `https` and `mailto` links and a
  `location.assign` to an `http` address each went to Edge through
  `NSWorkspace`, and the app stayed on its page. A local server logged the
  `http` link and the `location.assign` with Edge's user agent, and Edge's
  address bar showed the `https` link. The unified log showed the `mailto`
  hand-off: "Opening URL ... with application", with Edge brought to the
  front. An `http://rm1user@127.0.0.1/...` link opened nothing: there was no
  launch request and the server logged no request.
- Browser APIs:
  `native_browser_api_host_tests::macos_page_calls_reach_the_pasteboard_through_the_browser_api_bridge`
  passed. A page opened the share picker
  (`NSSharingServicePicker`). Its Copy service put the shared URL on the
  pasteboard, and the page heard `ShareFinished`. A page request opened the
  notification permission prompt (“RmApis” Notifications). It was not
  allowed then: an unanswered prompt that closes with its app, or a banner
  closed with its X, records "allow: NO" for that app id, and the page then
  hears `denied` without a new prompt. On a retry the same app, copied under
  a new id, opened the prompt again; Samuel allowed it, the page heard
  `granted`, and a notification from the page was delivered and presented as
  a banner while the app was in front (`usernoted`: "Authorization set ...
  allow: YES", then "Presenting ... as banner"). A page location request
  opened the
  CoreLocation prompt, “"RmApis" would like to use your current location.”,
  with the manifest reason underneath: "RM1 checks that the location prompt
  shows this reason". The prompt was answered "Don’t Allow". macOS refuses
  notification permission to an app started from the temporary folder
  (`UNErrorDomain` 1, "Notifications are not allowed for this
  application"). A small ad hoc signed app gets the same error there, and
  the prompt appears once the app is copied elsewhere.
- Page checks: the new `scripts/native/page-checks-macos-test.mjs` passed in
  16 s with a packaged app. `connect-src` refused an undeclared host: the
  page's `securitypolicyviolation` event named `connect-src`, and the host
  saw no request. The Yon origin answered. With Yon running, a counting
  front in front of Yon saw both network-first reads and only the first
  cache-first read. With Yon and the front stopped, the network-first read
  answered its kept copy, the cache-first read answered from the cache, and
  a `no-store` read failed. A preference string survived a quit and
  restart.
- WebAssembly page services: the new
  `scripts/native/page-services-macos-test.mjs` passed. A packaged app's
  Tally page answered a click from Python (Pyodide in the web view, opened
  and answered in 2.1 s), Dart, Kotlin and Java services. The Dart, Kotlin
  and Java apps had `LSMinimumSystemVersion` 15.2, and the Python app 13.0.
- Byte-identical rebuild:
  `native_reproducible_build_tests::a_packaged_macos_app_is_byte_identical_when_built_twice`
  passed (64 s).
- Universal build:
  `native_rust_crate_tests::the_universal_macos_bundle_holds_both_architectures_and_its_helper_runs`
  passed once `rustup target add x86_64-apple-darwin` had installed the
  x86_64 standard library. Without it the build stops with `TY1605` and
  names that command. `ty bundle --target macos --arch universal` of the
  same Greeter project gave `lipo -archs` `x86_64 arm64` for the app and
  its `probe-helper`, and `codesign --verify --strict` passed. Started with
  `open`, the arm64 slice ran. Started with `arch -x86_64`, the x86_64 slice
  ran under Rosetta (`NSRunningApplication.executableArchitecture`
  x86_64). In both, the page's Greet button called the linked Rust service,
  which answered from its vendored crates (`hello from macos vendored`).
  The helper printed the same under both architectures.

After a rebase onto develop `c6ce557c`, the window-grant, application-state,
page-check and page-service scripts, the pasteboard host test, the
byte-identical rebuild test and the universal build test passed again on the
same Mac.

Not run in RM1: an ahead-of-time C# service and an Intel Mac. MA6 ran the
first one on 2026-10-25 (below). `tac.dotnet.aot` does not need `--release`:
every `ty bundle` compiles services with the release profile, so the setting
applies to an unsigned bundle too (the iOS simulator run used one).

### macOS universal release with an ahead-of-time C# service (2026-10-25, MA6)

On the Mac (Apple M4, macOS 26.6.2, Xcode 27.1, .NET SDK 10.0.402 with
`wasm-tools`, Node 26.12, branch `0f082432`),
`TAC_APPLE_TEAM_ID=JAU9V4WH99 TAC_APPLE_NOTARY_PROFILE=tachyon-notary
scripts/native/macos-release-test.mjs` passed:

- `ty bundle --target macos --arch universal --release` built the C# Tally
  app with `tac.dotnet.aot: true`, signed it with the Developer ID
  Application identity and notarized `TallyRelease.dmg` in 54 s (157 s with
  a cold ahead-of-time build). Apple accepted the submission in about 25 s.
- `dotnet.native.wasm` was 5.52 MB ahead of time against 1.39 MB for an
  interpreter build of the same service.
- `lipo -archs` gave `x86_64 arm64` for the app. `codesign --display` showed
  `Authority=Developer ID Application`, the team identifier, the hardened
  runtime and a secure timestamp, and `codesign --verify --strict --deep`
  passed.
- `xcrun stapler validate` passed on the 6.55 MB `.dmg`, and `spctl` read
  both the `.dmg` and the app inside the mounted image as `Notarized
  Developer ID`.
- Opened from the mounted image, the arm64 slice answered a click from its
  C# service in 3.7 s. Started with `arch -x86_64`, the x86_64 slice ran
  under Rosetta (`NSRunningApplication.executableArchitecture` x86_64) and
  answered in 3.9 s.

`cargo test -p tachyon-cli --test it macos_release -- --ignored` (the
notarized and stapled release of the Release Check fixture) and the two
other `macos_release` tests passed on the same Mac. The macOS `tac.dotnet`
cell is `honored`. Still owed: an Intel Mac.

### Native apps reach Yon through a declared origin (2026-10-18)

ADR 0103 adds `tac.yon.origin` in `client/manifest.json` and `server.origins`
in `server/application.json`. The contract is unit-tested in the cloud, and
`scripts/yon-origin-browser-test.mjs` runs a web page on one port against
Yon on another in Chromium, and `scripts/native/yon-reach-test.mjs --target=web`
runs the probe that the native apps run. On 2026-10-19 it ran the macOS app
and the iOS simulator on a Mac, the packaged Android app on the Android 15
emulator (NET2), and the packaged Linux app under Xvfb (NET4). The Windows
app ran `scripts/windows/yon-reach.ps1` inside WebView2 on the signed-in
desktop on 2026-10-24 (SD1, the Windows row below).

| Host | Yon calls from the packaged app | Evidence |
| --- | --- | --- |
| Web | implemented | page transports, CORS, preflight, socket origin and `connect-src` unit tests; Chromium cross-port run, and cross-port offline reads in Chromium, Firefox and WebKit (NET8, below) |
| macOS, iOS | run on the host | `scripts/native/yon-reach-test.mjs` passed on 2026-10-19 for the macOS app (macOS 26.6.2, Xcode 27.1) and the iOS 26.5 simulator (iPhone 17 Pro) over `http` and over `https` through a local TLS proxy. In all three runs Yon received `Origin: tachyon-app://bundle`, and the page reached it with HTTP, a CORS preflight, a stream, a socket, a topic, a 422 problem and a signed-in route. The linked Swift `@HTTPClient` read reached the origin through `URLSession` too; it is also unit-tested against a local server. On 2026-10-24 (LR1, branch `0ba1079e`, macOS 26.6.2, Xcode 27.1, Swift 6.4.1, iOS 26.5 simulator) the same script passed with `TAC_YON_SERVICES=rust` on macOS and on the simulator over `http` and `https`: a linked Rust `HTTPClient` read reached Yon through the host's `URLSession` sender, and linked Rust chat and stream services held their socket and stream through the page. The Swift services passed again on macOS and the simulator. `native_rust_apple_host_tests` links the Apple Rust shim with a Rust companion and passed on that Mac and in an x86_64 Linux container (Swift 6.2) |
| Android | run on the emulator; an `https` origin, or a local `http` origin with cleartext to that one host | `scripts/android/yon-origin-test.mjs` passed on 2026-10-19 on the Android 15 (API 35, arm64) emulator against `ty serve` on the Mac through `adb reverse`. The packaged app reached Yon from `https://appassets.tachyon.local` with an HTTP read, a cross-origin write, a stream, a socket, a topic, a `@Check` 422 and a signed-in route (401 without a token, then with an `Authorization` header). A linked Kotlin `HTTPClient` reached Yon through the native transport in the app's own process. The transport is also tested on the JVM against a real HTTP server (`android_yon_http_tests`). Not run: a physical device |
| Windows | run on the signed-in desktop; linked Rust and C# `HTTPClient` calls reach it too | 2026-10-19 cloud runs and the MSVC and NativeAOT build on Windows 11 ARM64 (below); on 2026-10-24 `scripts/windows/yon-reach.ps1` passed inside WebView2 on the signed-in desktop of that machine: HTTP, a stream, a socket, a topic, a `@Check` 422 and a signed-in route from the page, Rust and C# `HTTPClient` reads, and a linked C# service's stream and socket clients held by the page (NET3, LS1); on 2026-10-25 a linked Rust service's stream and socket clients held by the page and a topic heard by a linked Rust service (SW1) |
| Linux | run under Xvfb | `scripts/linux/yon-origin-native-test.mjs` built the packaged app with GTK 4.14.5, WebKitGTK 2.52.6 and libsoup 3.4.4 in an x86_64 Ubuntu 24.04 cloud container and drove it through AT-SPI. It reached Yon on `http://127.0.0.1` and `https://localhost`, once with JavaScript page services and once with linked Rust services: HTTP read and write, a preflighted `@SignedIn` call (401 without a token), a topic, a `@Check` 422, a stream and a socket. The Linux gate runs both. Both also passed on WSL Ubuntu 24.04.5 (aarch64) in RL1, on `19a95a65`. LX1 ran both again in a headless mutter 46.2 Wayland session (`TAC_LINUX_DISPLAY=wayland`), in the cloud and on WSL (aarch64), reading the page from the web process's own accessibility tree because GTK did not embed it in the window there. Not run: a Wayland desktop session |

On 2026-10-24 (LS1, ADR 0058 and 0060 amendments) linked Swift, Kotlin and
C# services started to hold stream and socket clients through the page, as
linked Rust services do. Their companions ran in an x86_64 Ubuntu 24.04
cloud container (Swift 6.3.3, Kotlin 2.2.20 on the JVM, .NET 9): each
described the `realtime` and `sockets` clients, heard a tick and a chat
message through `event`, and sent through the page. On 2026-10-24 (SD1)
the Windows app held a linked C# service's stream and socket clients from
the page in `scripts/windows/yon-reach.ps1` (the Windows row above). On
2026-10-25 (MA1, branch `100f335e`, macOS 26.6.2, Xcode 27.1, Swift 6.4,
iOS 26.5 simulator) `TAC_YON_SERVICES=swift scripts/native/yon-reach-test.mjs`
passed in the macOS app and on the simulator over `http` and `https` with
the `realtime` and `sockets` services linked as Swift (the script checks
`tachyon.host.json`): the page held their stream and socket and the
services showed the ticks and the chat echo. On 2026-10-25 (MA2, Android
15 emulator) `TAC_YON_SERVICES=kotlin scripts/native/yon-reach-test.mjs
--target=android` passed with the same services linked as Kotlin, after the
model order fix below ("Android service runs, MA2").

On 2026-10-24 (LS3, ADR 0105 amendment) `@OnEvent` listeners became allowed
to be async in Kotlin, C# and F#, and in linked services of every language.
In an x86_64 Ubuntu 24.04 cloud container the async listener browser gate
passed for Swift, Rust, Kotlin, C# and F# in Chromium, Firefox and WebKit,
and linked companions ran their listeners in source order: Swift 6.3.3,
Kotlin on the JVM (Android-style companion), C# under .NET 10, and Rust
through the host's task dispatch, where calls answer `BUSY` while a
listener holds the service.

The LS3 host runs followed on 2026-10-24. Each app check builds an app whose
linked service has an async listener that waits 400 ms and a plain listener
of the same event, presses "Send one" and then "Send two", and needs the page
to show `a1s1` and then `a1s1a2s2`, with the async listener running in the
app's own process:

- macOS 26.6.2 (Mac, Xcode 27.1, Swift 6.4, Rust 1.97.1, develop
  `cb18afdb8`): `swift_listener_tests` and `native_rust_task_tests` passed
  natively, and `scripts/native/async-listener-app-test.mjs --target=macos`
  passed with a linked Swift listener in the macOS app. Each press waited
  about 430 ms.
- iOS 26.5 simulator (iPhone 17 Pro, same Mac and commit): the same check
  with `--target=ios` passed in the simulator app. Each press waited about
  460 ms.
- Android 15 emulator (API 35, arm64, WebView 124, Gradle 8.14.3, JDK 21):
  `--target=android` passed with a linked Kotlin `suspend` listener in the
  packaged app on `cb18afdb8` plus the fix below. Before the fix, a
  coroutine that really suspended answered `HTTP_MISSING` at once and its
  work finished later without a render, so the page showed `s1`. Now the call
  waits until the coroutine resumes, as Swift does.
- Windows 11 Home ARM64 (build 26200, MSVC 19.44.35229 ARM64, Rust 1.97.1,
  .NET 10.0.302, Node 24.21.0, develop `519f3bee`, no local changes):
  `csharp_native_task_tests` and `native_rust_task_tests` passed over SSH
  (9 tests), and `scripts/windows/async-listener-native-test.mjs` passed
  with a linked C# `Task` listener in the WebView2 app on the signed-in
  desktop. The page showed `[a1s1]` with the app's process ID, then
  `[a1s1a2s2]`. The first run, on `ddf568e8`, found that a finished C#
  listener task did not signal the page, which kept showing `[]`. It now
  sends the owner's `changed` signal. On 2026-10-25 (SW1) the script ran a
  linked Rust `async fn` listener there as well, with the same result.
- macOS and the iOS 26.5 simulator with a linked Rust listener (MA1,
  2026-10-25, branch `100f335e`, same Mac and Xcode):
  `TAC_LISTENER_LANGUAGE=rs scripts/native/async-listener-app-test.mjs`
  passed in the macOS app and in the simulator app. The Rust `async fn`
  listener waits 400 ms on another thread; each press waited about 430 ms,
  and the listener ran in the app's process.
- Android 15 emulator with a linked Rust listener (MA2, 2026-10-25, API 35
  arm64, WebView 124):
  `TAC_ANDROID_LISTENER_LANGUAGE=rs scripts/native/async-listener-app-test.mjs
  --target=android` passed in the packaged app. Each press waited about 430
  ms (430 and 415), and the listener ran in the app's process.

- Linux (x86_64 Ubuntu 24.04 cloud container, GTK 4.14.5, WebKitGTK
  2.52.6, Xvfb, develop `dbcaf6d9` plus the fix below):
  `scripts/linux/async-listener-native-test.mjs` built the Linux app with a
  linked Rust service whose `async fn` listener waits 400 ms on another
  thread, pressed "Send one" and "Send two" through AT-SPI, and the page
  showed `[a1s1]` and then `[a1s1a2s2]`. With it, a finished Rust listener
  task sends the owner's `changed` signal too, as the C# one does.

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

`scripts/windows/yon-reach.ps1` runs every path from the packaged app on a
Windows desktop. It passed inside WebView2 on the signed-in desktop on
2026-10-24 (SD1, the Windows row above), and WebView2 showed no local
network access prompt before the page on `https://tachyon.local` called
`http://localhost`. On 2026-10-25 (SW1, below) it also held a linked Rust
service's stream and socket clients from the page, and a linked Rust
service heard a topic.

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
- The app checks for the listener when it starts. Without it, the app shows a
  refusal page instead of loading a page. The page names the installed System
  WebView and asks for an update from Google Play, the way the Windows app
  explains an old WebView2 Runtime. On 2026-10-25 (GP2, develop `750551be`)
  `scripts/android/web-view-refusal-test.mjs` ran on the Mac emulators. On an
  Android 8.0 (API 26) google_apis arm64 image with
  `com.google.android.webview` 58.0.3029.125, the app showed the refusal
  page with that package and version, held no web view, recorded
  `bridge.refused` and no `controller.mounted`, and did not crash. On the
  API 35 image with WebView 124.0.6367.219, the same app opened its page.

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

Native calls have a ten-second browser wait limit. An app that grants
`filePicker` waits 40 seconds instead for every native call except the
start-up request, because a picker can stay open for 30 seconds (ADR 0089).
This limit does not stop or
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
owed, RL1" above; ADR 0087 amendment). On the iOS 26.5 simulator in RM2
(below), with Yon stopped, a network-first and a cache-first read answered
from the kept copy, and again after a terminate and relaunch. On the
Android emulator, in airplane mode, a network-first and a cache-first Yon
read answered from Cache Storage ("Android runs owed, RM3" above). The packaged
macOS app ran it in RM1 (`scripts/native/page-checks-macos-test.mjs`):
with Yon stopped, a network-first read answered its kept copy and a
cache-first read answered from the cache ("macOS runs owed, RM1" above).
On the signed-in Windows desktop in SD1,
`scripts/windows/page-checks-native-test.mjs` stopped Yon and a
network-first Yon read answered from the cache inside WebView2 ("Windows on
the signed-in desktop, SD1" above).

How long the kept reads last follows the web storage of each host (below).
The per-target support table gives `tac.cache` no relaunch limit on macOS,
Windows, Linux and iOS, where the web view keeps its storage on disk, and
marks it adapted on Android:

- Linux: since ST1 (`499a6246`) the host keeps web storage on disk, in
  `$XDG_DATA_HOME/<application id>/webkit`, so kept reads stay after a
  relaunch, as on macOS. Before ST1 they lasted only while the app ran.
- iOS: kept reads stay after a relaunch (RM2, 2026-10-24). The earlier
  "only while the app runs" result came from Tac's start-up sweep deleting
  the check's raw cache entry, not from WebKit.
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
| macOS WKWebView | yes, `NSWorkspace` (before 2026-10-18 for `http` and `https`) | On 2026-10-24 (RM1, macOS 26.6.2) a page's `http`, `https` and `mailto` links and a `location.assign` reached the default browser and mail handler (Edge), and the page stayed in the app; a link with a user name opened nothing ("macOS runs owed, RM1" above) |
| iOS WKWebView | yes, `UIApplication` (before 2026-10-18 for `http` and `https`) | `scripts/ios/page-checks-test.mjs` on the iOS 26.5 simulator (RM2, 2026-10-24): `http`, `https` and a tapped `target="_blank"` link opened Safari; `mailto` reached the system, which has no Mail app on the simulator; links with a user name or a password, `ftp:` and an empty `mailto:` opened nothing and the app stayed in front |
| Android WebView | yes, `ACTION_VIEW` (before 2026-10-18 for `http` and `https`) | `scripts/android/page-checks-test.mjs` on the API 35 emulator (2026-10-24, RM3): taps on `https`, `http` and `mailto` links started `ACTION_VIEW` from the app, Chrome and Gmail opened them and the page stayed; a link with a user name started nothing. Not run: a physical device |

### Deep links on native hosts (2026-10-19)

ADR 0106 adds `tac.application.links`. A link with a listed scheme, or an
`https` link to a listed host, opens the app at its route. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: `.desktop` scheme handlers, `G_APPLICATION_HANDLES_OPEN` forwarding | `native_deep_link_tests::the_linux_app_opens_links_at_their_route_and_forwards_later_links_to_the_running_app` compiled the generated host with GTK 4 and WebKitGTK 6.0 (warnings as errors) and ran it with `dbus-run-session -- xvfb-run -a` in an x86_64 Ubuntu 24.04 cloud container: the launch link opened its route, and two later launches handed their links to the running window. In RL1 it passed on WSL (aarch64) and under headless Wayland, and `scripts/linux/desktop-links-test.sh` installed the `.deb`'s `.desktop` file and opened `linkgate://` links with `gio open` and `xdg-open` in the one running app. Not run: a desktop |
| Windows WebView2 | yes: per-user scheme registration (MSIX: `windows.protocol` extensions), single instance, `WM_COPYDATA` forwarding | `native_deep_link_tests::a_second_windows_instance_forwards_its_link_to_the_first` built the registration and forwarding code with MinGW-w64 and ran it under Wine 9.0: the schemes were registered under `HKCU\Software\Classes` and a second process's link reached the first. `windows_package_tests::the_staged_layout_packs_and_signs_with_the_msix_sdk` packed a package that declares two schemes with `makemsix`, which validated its `windows.protocol` extensions against Microsoft's schemas (and refused an unknown extension category). On the Surface desktop (Windows 11 arm64, 2026-10-19), a built app opened `lk2check://alpha?x=1` at `/alpha/?x=1` on launch; later links reached the running window and their second instance exited, and an unknown route opened `start_url`. An installed test-signed MSIX package opened `lk2check://beta?x=5` at its route, and Windows, not the app, registered the scheme. Host links (LK3, 2026-10-19, commit `ce1b47e5`, x86_64 Ubuntu 24.04 cloud container): the same `makemsix` test packed and signed a package whose manifest also declares a `uap3:AppUriHandler` host, and `makemsix` refused a misspelled `uap3` element against the schema; the `windows-app-web-link` family name matches the publisher ID Windows gives Microsoft's own subject (`8wekyb3d8bbwe`). On 2026-10-24 the package registered in Developer Mode handed later `sd1check` links to its one running process (SD1). Host links from a hosted domain (LINKS, 2026-10-25, Windows 11 Home 25H2 build 26200.9711 ARM64, develop `1f06e1f8`): `browser-apis-test.ps1 -SkipUnpackaged -InstallRoute Machine -LinkHost d31ma.github.io -RequireHostLink`, run in an elevated PowerShell on the signed-in desktop, installed the signed `.msix` with the throwaway gate certificate trusted in `Cert:\LocalMachine\TrustedPeople`; Windows gave it the family `dev.tachyon.browser-gate-package_8htbb8zekacvc` that `https://d31ma.github.io/.well-known/windows-app-web-link` names, answered `Available` for `https://d31ma.github.io/alpha?x=3`, and the shell opened that link in the running package at `/alpha/?x=3`. A package registered from a folder in Developer Mode is listed as the handler too, but Windows never verifies its hosts, so the same link opened in Edge. Not run: a link clicked in a browser, which stays in the browser by design |
| macOS, iOS WKWebView | yes: `CFBundleURLTypes`, `kAEGetURL` (macOS), launch URL, `open url` and universal-link activity (iOS); associated domains with `TAC_APPLE_TEAM_ID` | on the Mac (2026-10-19, Swift 6.4.1): `the_apple_link_rule_opens_declared_schemes_and_hosts_at_their_route` passed and the iOS host tests passed. A built macOS app opened `lk2check://alpha?x=1` at `/alpha?x=1` on launch; while it ran, later links moved the same window, and an unknown route opened `start_url`. The same app on an iOS 26.5 simulator (iPhone 17 Pro) did the same on a cold start and while running. macOS rewrites `#` to `%23` in a link without `//` (`lk2check:alpha#frag`), so such a link opens `start_url`. Host links on macOS (LK3, 2026-10-19, macOS 26.6.2, Xcode 27.1, Swift 6.4): `native_deep_link_tests::the_macos_app_delegate_opens_universal_links_to_listed_hosts_at_their_route` compiled the generated host and handed it browsing-web activities through the `NSApplicationDelegate` requirement: a listed host opened its route with its query, an unknown route opened `start_url`, and an unlisted host and a non-browsing activity were refused. An unsigned `ty build --target macos` printed the links note and opened `lk3check://alpha?x=1` at `/alpha?x=1`. The Developer ID profile check and the entitlements are unit-tested. Universal links from a hosted domain (LINKS, 2026-10-24, iOS 26.5 simulator, iPhone 17 Pro): `scripts/ios/universal-links-test.mjs` built the app with `TAC_APPLE_TEAM_ID`, found the entitlements in the executable and none in the ad hoc signature, and `https://d31ma.github.io/orders?x=1` opened the app at `/orders?x=1` on a cold start and `?x=2` in the running app. Owed: a signed device check of universal links on iOS, and a signed macOS release with a Developer ID profile that has Associated Domains (none exists for the team) opening a link from a hosted domain |
| Android WebView | yes: intent filters, `autoVerify` for hosts, `singleTask` with `onNewIntent` | the link rule ran with OpenJDK in the cloud. On emulator-5554 (2026-10-19), a launch link opened its route, later links reached the running activity through `onNewIntent`, and an unknown route opened `start_url`. Verified app links (LINKS, 2026-10-25, develop `1f06e1f8`, API 35 `google_apis` arm64 emulator): `scripts/android/verified-links-test.mjs` found the APK's certificate in `https://d31ma.github.io/.well-known/assetlinks.json`, Android verified `d31ma.github.io` for `dev.tachyon.link_gate`, and `https://d31ma.github.io/orders?x=1` opened the app at its route on a cold start without a chooser, as did `?x=2` in the running app. Owed: verified app links on a physical device |

### Browser APIs on native hosts (2026-10-19)

ADR 0065's amendments of 2026-10-18 and 2026-10-19 cover `clipboard`,
`share`, `notifications` and `geolocation` in native apps. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: the web view's clipboard and geolocation (GeoClue) behind one permission decision from the manifest grants; notifications bridged to `GNotification`; share copies to the clipboard | `linux_browser_api_tests::linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb with XTest clicks in an x86_64 Ubuntu 24.04 cloud container, with and without a window manager: granted, the page read another source's clipboard text, wrote its own, got a test position, was granted and showed a notification, and shared a url to the clipboard; with nothing granted, each was refused. The test also passed in the WSL (aarch64) run of `38eaed29` and again in RL1 on `d7110f74`. LX1 ran `linux_browser_apis_reach_geoclue_and_the_notification_server` through `scripts/linux/desktop-services-test.sh`: the page's position came from GeoClue 2.7.0 on a private system bus (a GPS fix from an NMEA socket, allowed by the GeoClue demo agent) and its notification was shown by dunst 1.9.2, in the cloud and on WSL (aarch64). On 2026-10-25 (LX2) the same test ran in a headless mutter 46.2 Wayland session through `scripts/linux/compositor-test.sh`, clicking through Mutter's RemoteDesktop interface instead of XTest (`linux_wayland_input.c`), granted and refused, in an x86_64 Ubuntu 24.04 cloud container and on WSL Ubuntu 24.04.5 (aarch64) on the Surface. Not run: a physical Linux desktop, and a Wayland desktop session with a person's clicks |
| macOS WKWebView | yes, all four: `NSPasteboard`, `NSSharingServicePicker`, `UNUserNotificationCenter`, CoreLocation | code level in a Linux cloud container (`native::browser_api_bridge`, `native::apple_browser_apis`). On a Mac (2026-10-24, RM1, macOS 26.6.2): `native_browser_api_host_tests::macos_page_calls_reach_the_pasteboard_through_the_browser_api_bridge` passed; a page opened the share picker and its Copy service finished the share (`ShareFinished`); a page opened the notification prompt; a page opened the location prompt, which showed the manifest reason ("macOS runs owed, RM1" above). On the retry, after the prompt was allowed, a notification from the page was presented as a banner while the app was in front |
| iOS WKWebView | yes, all four: `UIPasteboard`, `UIActivityViewController`, `UNUserNotificationCenter`, CoreLocation | code level in a Linux cloud container (same tests). The simulator gate `scripts/ios/native-test.mjs` passed with its clipboard step on develop `fd0384ad` (iPhone SE (3rd generation) simulator, iOS 26.5): the bridge listed all four APIs, the page wrote text that `simctl pbpaste` read from `UIPasteboard`, the page read it back, and a share without a grant was refused. On 2026-10-24 (RM2) `scripts/ios/browser-prompts-test.mjs` tapped real buttons on the iOS 26.5 simulator and read each prompt from a `simctl io` screenshot: the location prompt carried the manifest reason and the page got the simulated position; the notification prompt was allowed and a foreground banner appeared; the share sheet opened, and closing it reported a cancelled share |
| Android WebView | yes: clipboard, share and notifications through the host bridge; geolocation through the web view (BA2) | `scripts/android/browser-apis-test.mjs` passed on the API 35 emulator on 2026-10-24 (RM3): the clipboard round trip, the share sheet, the notification question, a posted notification and its channel name, a denied location question, the manifest reason before the second question, and a test-provider position after Allow. Not run: a physical device |
| Windows WebView2 | yes: clipboard bridged to the Windows clipboard; share through the Windows share sheet (`IDataTransferManagerInterop`); notifications as toasts for the installed MSIX package (an unpackaged build answers `denied`); geolocation through WebView2 behind one `PermissionRequested` decision from the manifest grants (BA3) | `windows_browser_api_tests::the_windows_browser_apis_follow_the_manifest_grants_under_wine` compiled the bridge with MinGW (warnings as errors) and ran it under Wine 9.0 in an x86_64 Ubuntu 24.04 cloud container (BA3 branch on develop d7110f74): granted, a real clipboard round trip, the toast XML, the permission answers and the share values with a fake notifier and share sheet, and the permission decision; with nothing granted, each was refused. The whole generated host compiled with MinGW against the WebView2 1.0.3485.44 headers. `the_windows_browser_apis_match_the_windows_sdk_and_run_on_windows` passed on the Surface (Windows 11 ARM64 over SSH, MSVC 19.44.35229 ARM64, Windows SDK 10.0.26100.0, BA3 commit 971c6559): every declared Windows Runtime slot matched the SDK headers under `cl /W4 /WX`, and the program ran granted and refused with the runtime loaded and the share sheet opening for a message-only window. On the same machine `ty build --target windows` compiled and linked the desktop fixture app with all four APIs granted (`/W4 /WX`, WebView2 SDK 1.0.3485.44). On 2026-10-24 `scripts/windows/browser-apis-test.ps1` passed inside WebView2 on the signed-in desktop (SD1): the clipboard both ways, the share sheet opened, geolocation granted with no prompt, every API refused when none is granted, and, from the MSIX package installed by Developer Mode registration, a toast in the notification history. Not checked: whether the user shared from the sheet, which Windows does not report |

### File picker on native hosts (2026-10-19)

ADR 0089 and its amendments cover `tac.nativeCapabilities: ["filePicker"]`.
This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: `GtkFileDialog`, or the desktop portal's chooser where one runs, for JavaScript, TypeScript and linked Rust services, and for Swift, C# and Kotlin services, which run as WebAssembly and reach the dialog through the page (FP5) | `linux_file_picker_tests::linux_generated_host_opens_the_gtk_file_dialog_for_a_page_and_a_rust_service` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and drove the real dialog with XTest key presses under Xvfb in an x86_64 Ubuntu 24.04 cloud container on 2026-10-19: three runs through the portal's chooser (xdg-desktop-portal-gtk, xfwm4) and one each through GTK's own dialog with and without a window manager. It also passed on WSL Ubuntu 24.04 (aarch64, same GTK and WebKitGTK, Xvfb with xfwm4), once through each dialog; one earlier WSL run with a broken portal backend hit the 30-second dialog limit and answered `cancelled`, and its rerun passed. A page and a Rust service chose, read and released files; cancel, the second dialog, the handle and owner limits, the lifetime, another owner, a large and a non-UTF-8 file, the missing grant and a new page were checked. On 2026-10-24 (FP5) `scripts/linux/page-services-native-test.mjs` ran the packaged `native-capabilities` app with a Swift (6.3.3), a C# (.NET 10.0.302) and a Kotlin (2.1.10) WebAssembly service under Xvfb with xfwm4 in the same kind of container: each chose a file in GTK's dialog, read it, and answered `cancelled` after Escape. The same script passed on develop `0727eb7e` on WSL Ubuntu 24.04.5 (aarch64, GTK 4.14.5, WebKitGTK 2.52.6, Xvfb with xfwm4) with Swift 6.3.3, C# (.NET 9.0.317) and Kotlin 2.1.10, with `GIO_USE_VFS=local` and `GIO_USE_VOLUME_MONITOR=unix`: without them GVfs waited 25 seconds for UDisks2 there and the dialog reached its 30-second limit and answered `cancelled`. On 2026-10-25 (LX2) the test drove GTK's own dialog with key presses through Mutter's RemoteDesktop interface in a headless mutter 46.2 Wayland session (`scripts/linux/compositor-test.sh`), in an x86_64 Ubuntu 24.04 cloud container and on WSL Ubuntu 24.04.5 (aarch64) on the Surface: the page and the Rust service chose, read and released files and every check above passed. Not run: a Linux desktop, and the portal's chooser under Wayland |
| macOS WKWebView | yes: `NSOpenPanel`, also for C# and Kotlin services, which run as WebAssembly and reach it through the page (FP5) | see ADR 0089 and its amendments. On 2026-10-24 C# and Kotlin fields chose, read and cancelled in the panel on macOS 26 (FP5) |
| Windows WebView2 | yes: `IFileOpenDialog` for JavaScript, TypeScript and linked Rust and C# services (FP1), and for Swift and Kotlin services, which run as WebAssembly and reach it through the page (FP5) | under Wine in the cloud and on the signed-in desktop; see the Windows paragraph above. On 2026-10-24 the whole generated host, with the page route, compiled with MinGW against the WebView2 1.0.3240.44 headers and the Wine picker checks passed. On 2026-10-24 the MSVC build selected a file in the Windows dialog and closed an unanswered one on the signed-in desktop (SD1). The same day (FP5) `scripts/windows/page-services-native-test.mjs` ran a JavaScript page service, a linked C# service and a WebAssembly Kotlin service from a WebView2 page on that desktop: each chose a typed file in the dialog, read it, and answered `cancelled` after Escape. On 2026-10-25 (SW3) a WebAssembly Swift field (Swift 6.3.3 for Windows ARM64) did the same from a WebView2 page on that desktop. On 2026-10-25 (SW1) a linked Rust field did the same; before that it had run only under Wine |
| iOS WKWebView | yes: `UIDocumentPickerViewController` for JavaScript, TypeScript and linked Swift and Rust services, and for C# and Kotlin services, which run as WebAssembly and reach it through the page (FP4) | code level in a Linux cloud container (`ios::tests`, `scripts/native/file-picker-runtime-test.mjs`). On 2026-10-24 (FP4) `scripts/ios/file-picker-test.mjs` passed on the Mac (macOS 26.6.2, Xcode 27.1) on the iOS 26.5 simulator (iPhone 17 Pro): a JavaScript page service, linked Swift and Rust services, and C# and Kotlin WebAssembly services each chose a file seeded into On My iPhone, read it, and answered `cancelled` when the next picker was closed. Not run: a physical device |
| Android WebView | yes: `ACTION_OPEN_DOCUMENT` for JavaScript, TypeScript and linked Kotlin and Rust services, and for Swift and C# services, which run as WebAssembly and reach it through the page (FP2) | code level in a Linux cloud container (`android::tests`, `android_file_picker::tests`); the generated host compiled with `javac` against the API 35 `android.jar`. On 2026-10-24 (FP2) `scripts/android/file-picker-test.mjs` passed on the Mac's API 35 arm64 emulator (`tachyon-gate`, Android System WebView 124): a JavaScript page service, linked Kotlin and Rust services, and Swift and C# WebAssembly services each chose a file in DocumentsUI's Downloads, read it, and answered `cancelled` after Back. Not run: a physical device |

### Window settings on iOS and Android (2026-10-19, WIN5)

`theme_color`, `background_color`, `display`, `orientation` and
`tac.application.window.titleBar: "integrated"` now reach the mobile apps
(ADR 0052 and ADR 0098 amendments). This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| iOS WKWebView | yes: the status bar strip and its text color, the web view and launch screen color (an asset catalog color), the hidden status bar, the supported interface orientations, edge to edge with the three `--tac-titlebar-*` properties from the safe-area insets, and `WindowControls` for linked Swift services | `scripts/ios/window-settings-test.mjs` on the Mac (macOS 26.6.2, Xcode 27.1 27A9275, iOS 26.5 simulators, WIN5 branch on develop eefb70b0) built three apps and read the running page, a linked Swift service and a screenshot. iPhone 17 Pro: a standard window with a dark `theme_color` showed it across the 62-point status bar with light text (`statusBarStyle` 1), started the page at the safe-area top and showed `background_color` through the transparent page; the bundle's `Assets.car` held the launch color. An integrated window with a light `theme_color` put the page at the top edge with dark text (3), set `--tac-titlebar-height` to `62px`, and the page's own bar filled the status bar in the screenshot. `display: "fullscreen"` with `landscape` hid the status bar and opened in landscape (750 by 382). The linked Swift service saw the `title` grant and no `minimize` grant and set the scene title to `Quarterly report` on the iPhone and on an iPad Pro 13-inch (M5). On 2026-10-25 (MA4, commit `e6c017f1`) the same script held the standard app before `main` with `simctl launch --wait-for-debugger`, and the screenshot showed the launch screen: `#ffaa00` behind the status bar and across the whole screen, before the page ran. Not checked: a device turned by hand, and a physical device |
| Android WebView | yes: the status and navigation bar colors and icon colors, the window, splash screen and web view color, hidden system bars, `screenOrientation`, and edge to edge with the three properties from the system-bar and cutout insets | `scripts/android/window-settings-test.mjs` on the Mac's API 35 arm64 emulator (`tachyon-gate`, 1080 by 2400 at 420 dpi) built three APKs and read the page over DevTools, `aapt2` for the manifest and `screencap` pixels. A standard window filled the status and navigation bars with `theme_color` and showed `background_color` through the page (412 by 842). An integrated window drew the page under the status bar, set `--tac-titlebar-height` to `49px`, and the page's bar reached the bottom of the status bar. `display: "fullscreen"` with `landscape` opened in landscape with no system bars (866 by 412, the 49 dp cutout still pads the leading side). On 2026-10-25 (MA4, commit `df1e8c26`) the same script also read the bar icons and the splash screen, on `tachyon-gate` (API 35, WebView 124.0.6367.219) and on `tachyon-api34` (API 34, WebView 113.0.5672.136). The dark `theme_color` gave light status and navigation icons (median luminance 215 to 255) and the light one dark icons (29 to 120, window flags `LIGHT_STATUS_BARS LIGHT_NAVIGATION_BARS`). With the app held before its first frame (`run-as` `kill -STOP`), `dumpsys` listed its splash window and the screen showed `background_color` `#ffaa00`. The integrated, standard and fullscreen checks passed on API 34 too. Not checked: Android below API 34, and a physical device |

### Desktop window colors and display (2026-10-24, WIN7)

`theme_color`, `background_color` and `display: "fullscreen"` now reach the
macOS window, and `background_color` the Windows and Linux windows (ADR 0052
amendment). This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| macOS WKWebView | yes: the title bar in `theme_color` with dark or light title text (standard title bar only), the window and the web view's layer in `background_color` before the first paint, and a full-screen start | on 2026-10-24 `native_window_appearance_host_tests::macos_window_takes_theme_color_background_color_and_fullscreen_display` passed on macOS 26.6.2 (Apple M4, Xcode 27.1 27A9275) on the WIN7 branch `55f114a4` (develop `4eac45df`): a standard window took `#1a237e` in the sampled title bar pixel with the dark appearance (white title text), `#fdf6e3` in the page area and on the web view's layer, and the page kept following the application's light and dark appearance; a `display: "fullscreen"` window started full screen over its `background_color`. Screenshot pixels are compared in the screenshot's own color space. Not run: an Intel Mac |
| Windows WebView2 | yes: the window class brush, `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` and the controller's `DefaultBackgroundColor` (only the last two with an integrated title bar) | on 2026-10-24 the whole generated host with a `background_color` compiled with MinGW against the WebView2 headers, standard and integrated, and `windows_window_background_tests::the_windows_window_background_paints_a_win32_window_under_wine` read the class brush, the painted client pixel and the WebView2 start color in an x86_64 Ubuntu 24.04 cloud container (Wine 9.0 under Xvfb). The same day `the_windows_window_background_paints_a_real_win32_window` passed with MSVC 14.44 (ARM64) on Windows 11 25H2 (build 26200, ARM64) in the signed-in desktop session, painting `RGB(26, 35, 126)` in the client area; over SSH (session 0) the brush and start color checks passed but `GetPixel` cannot see painted pixels. On 2026-10-25 (SW2, below) every sampled pixel below the content of a WebView2 page without its own background had the declared `#1565c0` on that desktop |
| Linux WebKitGTK | yes: the web view background color and the GTK window background | on 2026-10-24 `linux_window_appearance_tests::linux_generated_host_opens_the_declared_size_minimum_and_theme_color` read `background_color` back from the WebKitGTK view and sampled it in the rendered page area, with GTK 4.14.5 and WebKitGTK 2.52.6 under Xvfb with xfwm4 in an x86_64 Ubuntu 24.04 cloud container. On 2026-10-25 (LX2) the same test passed in a headless mutter 46.2 Wayland session (`scripts/linux/compositor-test.sh`), in an x86_64 Ubuntu 24.04 cloud container and on WSL Ubuntu 24.04.5 (aarch64) on the Surface. Not run: a Linux desktop |

### Native preview against the development server (2026-10-24, NET5)

`ty preview --target <native>` builds the app, opens it, and has its scheme
handler load each page from the development server, so edits reach the
running app as hot updates (ADR 0110). Release builds refuse the development
origin (TY1601). This evidence does not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: libsoup fetch in the `tachyon://app` scheme handler; the app starts as a child process | on 2026-10-24 `scripts/linux/dev-loop-native-test.mjs` passed with GTK 4.14.5, WebKitGTK 2.52.6 and libsoup 3.4.4 under Xvfb in an x86_64 Ubuntu 24.04 cloud container, driven through AT-SPI: a Yon call reached the development server, a component CSS edit arrived while the page kept its state, a component HTML edit appeared, a manifest edit rebuilt and reopened the app, and stopping `ty preview` closed it. The same day it passed on WSL Ubuntu 24.04.5 (aarch64, WebKitGTK 2.52.6) on the Surface. Not run: Wayland, a Linux desktop |
| macOS WKWebView | yes: `URLSession` fetch in the `tachyon-app://bundle` scheme handler; the app starts as a child process | on 2026-10-24 `scripts/native/dev-loop-macos-test.mjs` passed on macOS 26.6.2 (Apple M4, Xcode 27.1), reading the page through the Accessibility API: the same steps as Linux, with the app reopened after the manifest edit and closed when `ty preview` stopped. Not run: an Intel Mac |
| iOS WKWebView | yes: the same Swift fetch; the app is installed and launched with `xcrun simctl` | on 2026-10-24 `scripts/ios/dev-loop-test.mjs` passed twice on an iOS 26.5 simulator (Xcode 27.1), reading the page by OCR of simulator screenshots: the same steps, with the app reinstalled after the manifest edit and ended when `ty preview` stopped. Not run: a device |
| Android WebView | yes: `HttpURLConnection` fetch in `shouldInterceptRequest`, `adb reverse` for the port, `adb install -r` and `am start` | on 2026-10-24 the generated preview `MainActivity.java` compiled with `javac` against `android.jar` (API 35), AndroidX WebKit 1.14.0 and AndroidX Core in the cloud. The same day `scripts/android/dev-loop-test.mjs` passed on an API 35 emulator on the Mac, reading the page by OCR of screen captures: the same steps over `adb reverse`, with the app reinstalled after the manifest edit, and force-stopped with the port forward removed when `ty preview` stopped. Not run: a device |
| Windows WebView2 | yes: WinHTTP fetch in the WebResourceRequested handler; the app starts as a child process | on 2026-10-24 `native_development_preview_tests::the_preview_windows_host_compiles_against_the_win32_and_webview2_headers` compiled the preview host with MinGW against the Win32, WinHTTP and WebView2 headers in the cloud. On 2026-10-25 (SW2, below) `scripts/windows/dev-loop-test.mjs` passed with the app open on the signed-in Windows 11 ARM64 desktop |

### Web storage across relaunch (2026-10-18)

ADR 0059's amendment of 2026-10-18 covers this. Each host check writes an
IndexedDB row, a localStorage value and a Cache API entry, quits the
application so its process ends, starts it again and reads all three back.
This evidence does not promote any tier.

| Host | Kept across relaunch | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes | `linux_storage_tests::linux_web_storage_is_kept_after_the_application_quits_and_starts_again` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 and ran it twice under Xvfb on WSL Ubuntu 24.04 (aarch64). The Cache API entry went through the framework's `tac-storage.js`. The page writes all three about 20 ms before the quit; without the 750 ms settle at quit the localStorage value was lost in 4 of 10 runs, with it 10 of 10 kept it. With the old ephemeral session the same test fails |
| Android WebView | IndexedDB and localStorage yes; Cache API entries not always | `scripts/android/native-test.mjs` on an API 35 arm64 emulator (WebView Chrome 124), across `am force-stop` and a new process: a `@LocalState` field and a Store row saved and found by id were kept in every run. The `@Cached` entry, written about 6 s before the force-stop, was kept and served in 3 runs and missing from Cache Storage in 2 of the 3 runs that checked it before the read. A `preferenceStrings` write (localStorage) was read back after a relaunch in `scripts/android/page-checks-test.mjs` (2026-10-24, RM3) |
| macOS WKWebView | yes | `scripts/phase4-macos-test.mjs` on 2026-10-19 (macOS 26.6.2, Xcode 27.1) with Accessibility access: the first launch wrote all three, the app quit, and a new process read `local=kept,store=kept,cache=kept`. An earlier run looked like a frozen page after relaunch. The page was running, but the gate's check set its text after OnStart without asking for a render, and its raw Cache API entry had no `x-tachyon-expires-at` header, so Tac's start-up cache sweep deleted it. The check now calls `taskScheduler.render()` and writes the entry in Tac's format |
| iOS WKWebView (simulator) | yes | `scripts/ios/native-test.mjs` on an iPhone SE (3rd generation) simulator with iOS 26.5 (Xcode 27.1), RM2 on develop `4c052177`: a real terminate, a new process, and `local=kept,store=kept,cache=kept` in 2 of 2 runs. WebKit wrote the Tac-format entry as a Cache Storage record on the first launch, and the same file was read after the relaunch. An earlier run reported the entry missing because its raw entry had no `x-tachyon-expires-at` header and Tac's start-up sweep deleted it; the gate now requires `cache=kept` |
| Windows WebView2 | yes | WebView2 keeps its data in `<App>.exe.WebView2` (ADR 0097). On 2026-10-24 `scripts/windows/relaunch-storage-test.mjs` kept localStorage, IndexedDB and the Cache API across a relaunch on the signed-in Windows 11 ARM64 desktop (SD1) |

### iOS simulator runs (2026-10-24, RM2)

Run on the Mac (macOS 26.6.2, Xcode 27.1 27A9275) on the iOS 26.5 (23F77)
simulator, from develop `4c052177` with a debug `ty` of the RM2 branch. No
host code changed. Each script builds its own app, uses its own simulator
device set under `$TMPDIR` and deletes it after the run. This evidence does
not promote any tier.

| Run | Result | Evidence |
| --- | --- | --- |
| Cache API across a relaunch | passed | `scripts/ios/native-test.mjs`, 2 of 2 runs; see "Web storage across relaunch" above |
| Offline Yon reads (`tac.cache`) | passed | `scripts/ios/page-checks-test.mjs`: with Yon stopped, network-first and cache-first reads answered from the kept copy, and again after a terminate and relaunch; a read with no rule failed |
| `preferenceStrings` across a relaunch | passed | same script: `dark` written, read back by a new process |
| `ApplicationStateChanged` | passed | same script: the page and a linked Swift service heard background (another app launched) and foreground |
| External links | passed | same script; see "External links on native hosts" above |
| `connect-src` (`tac.outgoing`, Content-Security-Policy) | passed | same script: a fetch to an undeclared host raised a `connect-src` violation, and that host received no request |
| WebAssembly page services | passed | `scripts/ios/page-services-test.mjs`: one app each for Python (Pyodide), C# (`tac.dotnet.aot`, `dotnet.native.wasm` 5.78 MB against 2.98 MB without AOT), Kotlin, Dart, Java, Go, Ruby, Lua, C++, F# and a linked Rust service. Each answered a click, kept its scoped style inside the component and opened the dynamic route `/items/7/`. The Kotlin, Dart and Java apps were built at `MinimumOSVersion` 18.2 |
| Vendored Rust crate | passed | `scripts/ios/rust-crate-test.mjs`: a vendored dependency and a `platformDependencies.ios` crate were linked, and the page got `hello from ios` |
| Byte-identical rebuild | passed | `native_reproducible_build_tests::a_packaged_ios_app_is_byte_identical_when_built_twice` |
| Browser API prompts | passed | `scripts/ios/browser-prompts-test.mjs`, read from screenshots; see "Browser APIs on native hosts" above |

Not covered by these runs:

- Disposal on termination by `simctl terminate`: it kills the app, so neither
  `applicationWillTerminate` nor a Swift `@OnStop` runs. Ending the app from
  the app switcher does run them (MA4, below).
- iOS 18.2 or a 17.4 runtime: the 18.2 floor (Kotlin, Dart and, until
  2026-10-25, Java) and the new 17.4 floor ran on iOS 26.5 and 18.0 only.
  The 18.4 Java floor (JF1) has not run.
- iOS shows no app reason in the notification prompt, so a manifest reason
  for notifications is not displayed there.
- A physical device and a signed install stay owed. Universal links ran on the
  simulator from a hosted domain (LINKS, 2026-10-24); a signed device check
  of them stays owed.

Older runtimes (2026-10-24, RM2). Xcode 27.1 offers no iOS 17.x or 18.x
runtime, so the iOS 17.0.1 (21A342) and 18.0 (22A3351) images came from
developer.apple.com. Each image stayed on an external volume: mounted
read-only with `hdiutil`, linked into
`~/Library/Developer/CoreSimulator/Profiles/Runtimes`, with devices in an
APFS sparse bundle (`TAC_IOS_RUNTIME` picks the runtime in the scripts).
CoreSimulator still builds each runtime's dyld cache on the internal disk
(2.6 GB for 17.0.1, 3.4 GB for 18.0).

| Runtime | Result | Evidence |
| --- | --- | --- |
| iOS 17.0.1 | install, launch and page passed; component styles did not apply | `scripts/ios/native-test.mjs` (render, input, Swift calls, clipboard, a linked Rust sync member, `local=kept,store=kept,cache=kept` after a relaunch) and `scripts/ios/rust-crate-test.mjs` passed on develop `4eac45df`. In `scripts/ios/page-services-test.mjs` the Go and linked Rust apps answered clicks and opened `/items/7/`, but no component style applied: WebKit runs CSS `@scope` only from 17.4. The iOS floor is now 17.4 (ADR 0108), so 17.0.1 is below it |
| iOS 18.0 | passed | the same three scripts on develop `4889d1a5` (floor 17.4): the main gate, the vendored crate, and Go and linked Rust apps whose component style applied inside the component only. A Kotlin app (floor 18.2) was refused at install (`Have 18.0; need 18.2`), as the floor intends |

Async linked Rust members (LR1) were not exercised on either runtime.

### Page accessibility on iOS and Android (2026-10-24, AX1)

Run on the Mac (macOS 26.6.2, Xcode 27.1) from branch
`claude/project-thread-muo2f6` (`b5d609c9`, on develop `b51b6b91`). No host
code changed. Both gates build the page in
`scripts/native/mobile-accessibility-fixture.mjs`: a heading, an `svg` with
`role="img"`, a labelled text field, a button, a `role="status"` paragraph
and a link to a second page. They read it through the platform
accessibility API, not the DOM. This evidence does not promote any tier.

| Gate | Result | Evidence |
| --- | --- | --- |
| `scripts/ios/accessibility-audit-test.mjs` on an iPhone 17 Pro simulator, iOS 26.5 (23F77) | passed, 2 of 2 runs | An XCUITest bundle (`scripts/ios/AccessibilityAudit.swift`, in a project the gate writes) found the web view `tachyon-application`, the heading as static text, the image, the text field named by its label, the button and the link by name. It typed into the field and pressed the button, the status text it read changed to `Saved hello`, and the link opened the second page. `performAccessibilityAudit` reported 0 issues |
| `scripts/android/accessibility-test.mjs` on emulator-5554, Android 15 (API 35, `tachyon-gate`) | passed | `scripts/android/AccessibilityProbe.java`, run through `app_process` with UiAutomation, read the WebView's nodes: an `android.widget.EditText` named `Note` (the label is its hint text), an `android.widget.Button`, an `android.widget.Image`, a clickable link and the heading. ATF-style checks (speakable text, 48 dp touch targets, duplicate clickable bounds, content descriptions on editable fields, redundant role words) found nothing. Typing, the button and the link worked from their accessibility bounds |

Limits these runs found:

- On Xcode 27.1 and iOS 26.5, `performAccessibilityAudit` does not look
  inside `WKWebView` content. A throwaway run with text planted at about
  1.9:1 contrast returned 0 issues, and the audit did not flag the link's
  111x23 pt accessibility frame. The audit covers the host's own elements
  only, so page contrast and touch targets on iOS are the page's to check.
- `uiautomator dump` drops the hint text where Chromium puts a labelled
  field's name, which is why the Android gate reads the tree with its own
  UiAutomation probe.
- Chromium gives the Android heading the role description `heading`, but
  `isHeading()` is false, so TalkBack's heading navigation may skip it.
- No VoiceOver or TalkBack session ran: the Google APIs emulator image has no
  TalkBack, and the gates stop at what the accessibility APIs show. Android
  text contrast is not checked.

### System Back on native hosts (2026-10-24, SB1)

ADR 0109. Each host sends System Back to the web view's history, so Back
lands on the previous route as a normal page load. This evidence does not
promote any tier.

| Host | What goes back | Result | Gate |
| --- | --- | --- | --- |
| Linux (GTK 4.14.5, WebKitGTK 2.52.6, x86_64 cloud, Xvfb + xfwm4) | Alt+Left, Alt+Right, the Back and Forward keys, mouse buttons 8 and 9 (host binding; WebKitGTK alone ignores them) | passed | `linux_generated_host_goes_back_and_forward_through_page_history` |
| macOS 26.6.2, Xcode 27.1, arm64 | View > Back (Command-[), View > Forward (Command-]), trackpad swipes | passed (menu keys and items; trackpad swipes are not driven) | `macos_view_menu_goes_back_and_forward_through_the_web_view_history` |
| iOS 18.0 simulator (iPhone 16 Pro), Xcode 27.1 | edge swipes | passed | `TAC_IOS_RUNTIME=.iOS-18-0 scripts/ios/system-back-test.mjs` |
| iOS 26.5 simulator (iPhone 17 Pro), Xcode 27.1 | edge swipes | failed: the XCUITest edge drag does not move the web view back, and does not move Safari back either | `scripts/ios/system-back-test.mjs` |
| Android 15 (API 35) emulator, gesture navigation, WebView 124.0.6367.219 | Back key and the back gesture; Back on the first route leaves the app | passed | `scripts/android/system-back-test.mjs` |
| Windows 11 25H2 (26200.9711) arm64, WebView2 155.0.4283.62, signed-in desktop | WebView2's own Alt+Left, Alt+Right and mouse back and forward buttons | passed | `scripts/windows/system-back-test.mjs` |

On the iOS 26.5 simulator the same XCUITest edge drag pops a UIKit
navigation stack (Settings > General) but moves neither the Tachyon web view
nor Safari back a page. On iOS 18.0 the same app and drag go back and
forward. A real touch on iOS 26 is not checked by these runs. Two System Back
runs need Samuel: a real touch on an iOS 26 device, and a person at a Mac
trackpad for the swipes. The support table claims the iOS 18.0 run and the
macOS menu keys and items only.

### Component styles on WebKitGTK (2026-10-25, GP1)

ADR 0108 amendment. Component styles compile to CSS `@scope`. The Linux floor
stays WebKitGTK 2.40, and a Linux build of an app with a component
stylesheet notes that component styles need WebKitGTK 2.44 or newer. This
evidence does not promote any tier. All runs: x86_64 Ubuntu 24.04 cloud
container, GTK 4.14.5, Xvfb; the older releases were installed from the
Ubuntu and Debian archives in place of 2.52.6.

| WebKitGTK | `scripts/linux/webkit-scope-probe.py` | Packaged app (`scripts/linux/page-services-native-test.mjs`, scoped style step) |
| --- | --- | --- |
| 2.52.6 | the rule applies inside the component only | passed: the build printed the 2.44 note, and the probe read "Style stays inside its component" through AT-SPI |
| 2.44.0 | the rule applies inside the component only | the page showed "Style stays inside its component", `rgb(10, 20, 30)` on its own paragraph and `rgb(0, 0, 0)` on the nested and sibling paragraphs (read from a screenshot) |
| 2.42.5, 2.42.4, 2.42.1 | the `@scope` block is dropped | 2.42.5 and 2.42.1: the page ran with "Style dropped", every paragraph `rgb(0, 0, 0)` (screenshot) |
| 2.40.0 | the `@scope` block is dropped | before the fix the app stopped at start with `undefined symbol: webkit_clipboard_permission_request_get_type`; after it the page ran with "Style dropped" (screenshot) |

WebKitGTK 2.40.0 to 2.44.0 exposed no page content to AT-SPI under GTK 4.14,
so the probe saw only the window and its menus there; those rows were read
from screenshots of the same app. With the host fix,
`linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view` still
read the clipboard with the grant and was refused without it on 2.52.6.

### Java web services need exception references (2026-10-25, JF1)

ADR 0083 amendment. TeaVM writes a Java `catch` as the WebAssembly
`try_table` instruction, and every Java service has one (the prelude
catches), so a Java service needs exception references (`exnref`) as well as
WasmGC. BR2 found it: Chrome 124 on an Android emulator refused the module
with `Invalid opcode 0x1f`. TeaVM 0.15.0 has no legacy exception mode, and
Binaryen 133 cannot translate `exnref` back, so Java's floors moved. Dart
and Kotlin keep theirs. This evidence does not promote any tier.

| Where | Java floor before | Java floor now | How it was checked |
| --- | --- | --- | --- |
| Chrome and Edge | 119 | 137 | Chrome for Testing headless shell: 136.0.7103.113 refused a TeaVM module with one `catch`, 137.0.7151.119 loaded it |
| Firefox | 120 | 131 | SpiderMonkey shell: 130.0 refused it, 131.0 loaded it |
| Safari | 18.2 | 18.4 | MDN browser-compat-data (`webassembly.exceptionsFinal`); not run |
| WebKitGTK (Linux app) | 2.46 | 2.48 | JavaScriptCore from the Ubuntu and Debian archives: 2.46.0 and 2.46.6 refused it, 2.48.1, 2.48.6, 2.50.4 and 2.52.6 loaded it |
| iOS app | 18.2 | 18.4 | follows Safari; `system_floor` unit tests only |
| macOS app | 15.2 | 15.4 | follows Safari; `system_floor` unit tests only |
| Windows app (WebView2) | 119.0 | 137.0 | follows Chrome; `system_floor` unit tests only |
| Android app | API level 26, note names WebView 119 | API level 26, note names WebView 137 | follows Chrome; `system_floor` unit tests only |

All runs were in an x86_64 Ubuntu 24.04 cloud container. A browser without
`try_table` now gets the Java loader's own message, naming Chrome or Edge
137, Firefox 131 and Safari 18.4, instead of a compile error;
`a_java_service_runs_in_the_page_as_webassembly` checks it with a
`WebAssembly.validate` that refuses the probe, because Node 22 runs
`exnref`. A packaged app built at the new floors has not been run: the iOS
and macOS page-service gates now expect 18.4 and 15.4 for Java.

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

### Branded browsers (BR2, runner 2026-10-25)

`scripts/branded-browser-gate.mjs --browser=<name>` runs the web gates
unchanged in a branded browser on its own system. It starts each gate with
`node --import scripts/branded/preload.mjs`, which points the gate's
Playwright launcher at the branded browser and makes the other engines
refuse, so a gate passes only in that browser. Each gate's log, a
`<gate>.launches.jsonl` receipt naming the browser version and user agent
that ran, and a `summary.json` go to `--logs=`.

| `--browser=` | Browser | How the gate reaches it |
| --- | --- | --- |
| `safari` | Safari on macOS | `safaridriver` (W3C WebDriver) through `scripts/branded/webdriver-browser.mjs` |
| `ios-safari` | Safari on the iOS simulator | `safaridriver` with `safari:useSimulator` (`TAC_BRANDED_SIMULATOR_UDID` picks the device) |
| `chrome`, `msedge` | Google Chrome, Microsoft Edge | Playwright's `channel` launch of the installed browser |
| `android-chrome` | Chrome on an Android emulator or device | `adb forward` to `chrome_devtools_remote`, Playwright `connectOverCDP`, and `adb reverse` for each loopback port a gate serves |
| `webkitgtk-webdriver` | WebKitGTK MiniBrowser | `WebKitWebDriver`; a stand-in that checks the Safari path off a Mac, not a branded run |

The WebDriver path covers the calls the gates make (navigation, `evaluate`,
`waitForFunction`, role, label, text and CSS locators through open shadow
roots, typing, clicks, element handles, several tabs, page errors heard after
the load). A call WebDriver has no equivalent for (`page.route`,
`addInitScript`, `setOffline`, a second open context, a persistent context)
throws `branded browser cannot ...`, and the runner reports that gate as
unsupported, never as passed. So Safari runs leave out `window-controls`,
`storage`, `svg-namespace`, `authored-node` and `preview-offline`, and list
them in the summary. Page errors thrown before the load event and console
messages logged before it are not heard over WebDriver.

Cloud check (2026-10-25, x86_64 Ubuntu 24.04 container, `ty` from develop
`2f86e866`, JavaScript, TypeScript, Rust, Go and Dart services): in Google
Chrome 156.0.8078.47 the adapter check, `page-program`, `async-listener`,
`held-clients`, `cached-client`, `window-controls`, `component-scope`,
`logic-controls`, `svg-namespace`, `authored-node`, `yon-reads` and
`preview-offline` passed. `scaffold` failed: Chrome itself asks for
`/favicon.ico`, a fresh `ty init` app serves none, and Chrome logs the 404
as a console error, which Playwright's Chromium build never does. Through
the WebKitGTK stand-in (MiniBrowser 2.52.6) the adapter check,
`page-program`, `async-listener`, `held-clients`, `cached-client`,
`component-scope`, `logic-controls`, `scaffold` and `yon-reads` passed.
These container runs check the runner.

Safari on the iOS simulator (2026-10-25, macOS 26.6.2, Xcode 27.1, iPhone 17
on iOS 26.5, Safari 26.5, user agent `iPhone OS 18_7 ... Version/26.5`,
branch `f1f46a00`, release `ty`): the adapter check, `page-program` (C++,
C#, Dart, F#, Go, Java, JavaScript, Kotlin, Rust and Swift services),
`async-listener`, `held-clients`, `cached-client`, `component-scope`,
`logic-controls`, `scaffold` and `yon-reads` passed. `held-clients` failed
once in the full list (WebDriver "no such window" right after a new tab
opened) and passed in both runs alone. On that simulator a safaridriver tap
reports success, but WebKit gets only `pointerdown` and `touchstart`. The
touch-up waits for the next tap, so the target never gets a click, and the
held touch can later click the previous target. WebDriver typing fires
`keydown` without changing the value. So on the simulator the adapter clicks
through `element.click()` and fills the way Playwright's `fill` does, and
logs each one. These runs check the pages in Safari, not touch input. An
iOS 18.0 simulator refused the session ("Could not find any session
hosts").

Chrome on the Android emulator (2026-10-25, `tachyon-gate`, the google_apis
userdebug arm64 image of Android 15, not the Google Play image; Chrome
124.0.6367.219, user agent `Android 10; K ... Chrome/124.0.0.0 Mobile`).
Chrome on Android gives no new browser context over CDP, so each context a
gate opens is a view of Chrome's one context: its own pages and
`newContext` options (media, locale, viewport and the rest applied per
page), with storage cleared when the last view closes. The runner gives a
loopback port its `adb reverse` route on the first request to it, and
passes a gate's `--host-resolver-rules` to Chrome's command line. With
every language but Java (the `*_LANGUAGES` lists of `page-program`,
`held-clients`, `window-controls` and `cached-client`), the adapter check,
`page-program`, `async-listener`, `held-clients`, `cached-client`,
`window-controls`, `logic-controls`, `svg-namespace`, `scaffold`,
`authored-node`, `yon-reads` and `runtime` passed. The rest:

- Java services: Chrome 124 has no Wasm `exnref`, which the TeaVM Java
  build uses ("Invalid opcode 0x1f"), so a page with a Java service never
  renders.
- `web-apis` fails: `navigator.share()` opens Android's share sheet and
  answers only after the user picks a target or dismisses it, so the gate's
  5-second wait for `ShareFinished` or `BrowserCallFailed` runs out.
- `component-scope` fails: a `3px` border computes as `2.66667px` at the
  emulator's fractional device pixel ratio.
- `website` is unsupported: at 1440 px it asks for `hasTouch: false`, and a
  phone is always a touch device.
- `storage` is left out: it keeps several contexts open, each with its own
  storage. `preview-offline` is unsupported (a persistent context).

On the Google Play image (2026-10-25, `tachyon-play`: Android 15, API 35,
`user` build, release keys, `ro.debuggable=0`), with release `ty` from
develop `86680eaa` and the same language lists, the result was the same.
Its Chrome is also 124.0.6367.219, because the Play Store has not updated
the system image's Chrome, so Java was not run there either. The preload's
first-run bypass (`am set-debug-app --persistent` and the flags file
`/data/local/tmp/chrome-command-line`) worked on this `user` build: Chrome
skipped its first-run screens and its DevTools socket answered. Chrome's
"Chrome notifications make things easier" prompt stayed over the page, but
it did not block the socket or any gate, so no taps were needed. The
adapter check, `page-program`,
`async-listener`, `held-clients`, `cached-client`, `window-controls`,
`logic-controls`, `svg-namespace`, `scaffold`, `authored-node`, `yon-reads`
and `runtime` passed. `web-apis` (the share sheet) and `component-scope`
(`2.66667px`) failed for the same causes, `website` and `preview-offline`
were unsupported, and `storage` was left out.

Safari on macOS (2026-10-25, macOS 26.6.2, Safari 27.0 (21625.1.29.18.28)
through `safaridriver`, user agent `Macintosh; Intel Mac OS X 10_15_7 ...
Version/27.0`, release `ty` from develop `86680eaa`, every language
toolchain on PATH, Java included): the adapter check, `page-program` (C++,
C#, Dart, F#, Go, Java, JavaScript, Kotlin, Rust and Swift services),
`async-listener`, `held-clients`, `cached-client`, `component-scope`,
`logic-controls`, `scaffold` and `yon-reads` passed, 9 of 9. WebDriver
clicks and typing reached the pages directly; the adapter never fell back
to `element.click()` or a scripted fill. `window-controls`, `storage`,
`svg-namespace`, `authored-node` and `preview-offline` were left out (calls
WebDriver cannot make, above).

Windows desktop (2026-10-25, IYOR-Surface, Windows 11 ARM64, in Samuel's
signed-in session through `scripts/windows/desktop-run.ps1`; release `ty`
from develop `ad27e53b`, runner `f58b814b`; Node 24.21.0, JDK 21 with TeaVM
0.15.0, Swift 6.3.3, .NET SDK 10.0.302, Go 1.27.1, Dart, Kotlin 2.1.10 and
Ruby 4.0.6). In Microsoft Edge 155.0.4283.62 every gate of the default list
ran (none is left out for Chromium browsers), with every language:

- Passed (14): the adapter check, `page-program`, `async-listener`,
  `held-clients`, `cached-client`, `storage`, `component-scope`,
  `logic-controls`, `svg-namespace`, `authored-node`, `yon-reads`,
  `preview-offline`, `website` and `runtime`.
- `web-apis` fails: Edge on Windows has `navigator.share()`, which opens the
  Windows share dialog and answers only after the user picks a target or
  closes it, so the gate's 5-second wait runs out (as on Android).
- `scaffold` fails: Edge asks for `/favicon.ico` itself and logs the 404 as a
  console error, as Chrome does.
- `window-controls` fails to build its fixture on this machine: with
  every language its route loads 4070 KB of WebAssembly against the
  3584 KB budget (TY1337). Without the Swift service it builds and passes
  in Edge, and on the Mac (Swift 6.4.1) the same route with every language
  stays under the budget. So the Windows Swift 6.3.3 build is larger; this
  is a toolchain cause, not Edge.

The first run found three runner and gate problems, all fixed: the preload
must be passed to `node --import` as a `file:` URL on Windows; the
supervisor could judge a gate's exit against a process snapshot from
before the exit (on Windows a snapshot takes seconds), and so reported
"live owned descendants" for five gates whose checks had passed; and Edge's
own background connections through the `preview-offline` CONNECT proxy
were refused, then reset, with no error listener on the socket. `website`
also hit its 120-second build limit once under load and passed alone.
Google Chrome on the Windows desktop is still owed (row "Web" above).

### Linked services share one program (2026-10-24, LS2)

`native/linked_page_program_tests.rs` ran in an x86_64 Ubuntu 24.04 cloud
container: two linked services share one Setup configuration in Swift
(Swift 6.2 on Linux), Kotlin (Kotlin 2.1.10 on the JVM) and C# (.NET
10.0.302, net9.0), and the C# native task fixture's two services both reach
Yon. These run the generated companion outside its app (ADR 0022 amendment of
2026-10-24). On 2026-10-25 (MA1, branch `100f335e`, macOS 26.6.2, Xcode
27.1, Swift 6.4) `scripts/native/async-listener-app-test.mjs
--check=shared-setup` built the same two linked Swift services into the
macOS app and the iOS 26.5 simulator app: the writer incremented, and the
reader read the writer's Setup identity and count 1 in both apps. On the
Android 15 emulator (MA2, 2026-10-25) the same check with
`--target=android` passed with two linked Kotlin services after the model
order fix ("Android service runs, MA2"). On 2026-10-24 the C# test passed on Windows 11 ARM64 too, with
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
native gate installs PHP, Groovy, Kotlin, Dart and Lua. Its first run with
them (39253757046) failed 44 tests; WG1 fixed their causes.

### Windows native gate (2026-10-25, WG2)

On Windows 11 ARM64 (IYOR-Surface, over SSH, commit `a798b43f`) the whole
native gate passed: `cargo nextest run --workspace --all-targets
--all-features --locked --no-tests=fail` ran 4580 tests, all passed (one
`relay_boundaries` deadline test on its retry), 77 skipped, in 946 s. The
tools were Rust 1.97.1, cargo-nextest 0.9.146, MSVC 14.44 (`vcvarsall
arm64`), clang++ 21.1.8, Node 24.21.0, bun 1.4.3, Python 3.14.7, Ruby 4.0.6,
Go 1.27.1, JDK 21.0.12, PHP 8.4.20, Dart 3.13.4, Lua 5.4.6, Groovy 5.1.3,
Kotlin 2.1.10 and .NET SDK 10.0.302, with `CI=1`. The runs before it found
and fixed:

- C++ pages and Yon programs with a `Behaviors::create` context or a Store
  crashed at exit under the MSVC library: their setup and store registries
  were destroyed before the services that release into them. The registries
  now live until the process ends.
- The Yon Topic tests and the test unit marker proofs looked for runtimes by
  bare name, so every `.exe` runtime (and `groovyc.bat`, `kotlinc.bat`)
  skipped. All 15 Topic tests now run on Windows.
- The scheduled-worker tests wrote the temp path with backslashes into
  Groovy, Lua and F# strings.
- The tachyon-cli tests did not compile on Windows (a Unix-only Gradle test).

`scripts/test-managed-storage.mjs --without-docker` passed on the same
machine (commit `661753db`): the storage contracts, the FYLO route phase with
the x64 FYLO under emulation, and the Redis contract against Garnet 2.2.4.
This is an ARM64 run over SSH; WebView2 runs are recorded with the desktop
gates above.

### Support table narrowed to its runs (2026-10-25, JS1)

The platform survey of develop `4243f2b6` compared 26 cells of
`target_support.json` with the runs on record. Following the ADR 0102 rule
(a native cell is `honored` only with a recorded run there), the cells whose
text named callers that never ran on that target are now `runOwed`, each
naming the run that would move it back:

| Cell | What ran | Still to run (thread) |
| --- | --- | --- |
| `tac.application.window.controls`, macOS | a page and a page service (RM1) | none: C# and Kotlin WebAssembly callers ran in the app (MA1, below), so the cell is `honored` again |
| `tac.application.window.controls`, Windows | a page, a linked C# service and a WebAssembly Kotlin service in WebView2 (SD1); a real Win32 window over SSH (RW1) | none: the WebAssembly Swift caller (SW3) and a linked Rust caller (SW1) ran in WebView2 (below), so the cell is `honored` again |
| `tac.nativeCapabilities` filePicker, macOS | C# and Kotlin fields in `NSOpenPanel` (FP5) | none: a linked Rust field ran in `NSOpenPanel` (MA1, below), so the cell is `honored` again |
| `tac.nativeCapabilities` filePicker, Windows | the MSVC dialog test (SD1); page, linked C# and WebAssembly Kotlin fields from a WebView2 page (FP5) | none: the WebAssembly Swift field (SW3) and a linked Rust field (SW1) ran from a WebView2 page (below), so the cell is `honored` again |
| `tac.go`, macOS and Android | a Go service in the macOS app (MA1) and on the emulator (MA2, below), so both are `honored` again | none |
| `tac.browserApis` geolocation, macOS | the location prompt with the manifest reason, answered Don't Allow (RM1) | none: the prompt was allowed and a position reached the page (MA3, below), so the cell is `honored` again |
| background and foreground signal, Windows | the page and a linked Rust service (SD1) | none: a linked C# listener heard it (SW1, below), so the cell is `honored` again |
| reaching Yon, Windows | HTTP from the page and linked Rust and C# services, and a linked C# service's held stream and socket clients (NET3, LS1) | none: a linked Rust service's held stream and socket clients ran in `scripts/windows/yon-reach.ps1` (SW1, below), so the cell is `honored` again |

The Android window controls stay `adapted`; their note now says that Swift
and C# WebAssembly callers have not run on the emulator; MA2 (below) ran
them. Text fixes
with no change of state: iOS release packaging names an iOS 17.4 device (the
floor since ADR 0108); the Windows integrated title bar note names the SD1
desktop run instead of saying it has not run; macOS System Back no longer
names the trackpad swipes, which no run drives. The three `tac.outgoing`
cells on macOS, iOS and Windows already agreed with their runs.

### macOS callers that moved cells back (2026-10-25, MA1)

On the Mac (macOS 26.6.2, Xcode 27.1, Swift 6.4, Node 26.12.0, a 1920 by
1080 screen, release `ty` from branch `100f335e`),
`TAC_MACOS_SERVICE_LANGUAGES=go TAC_MACOS_WINDOW_CALLERS=cs,kt
TAC_MACOS_PICKER_CALLERS=rs scripts/native/page-services-macos-test.mjs`
passed in 2 minutes 11 seconds:

- A Go page service in the packaged Tally app answered a click
  (`LSMinimumSystemVersion` 13.0, built in 13 s, opened and answered in
  3.4 s).
- C# and Kotlin `ReportService` callers of the `window-controls` feature,
  running as WebAssembly, renamed the window to `Quarterly report` and
  resized it to 1280 wide (read from the window server).
- The `native-capabilities` feature with its linked Rust `PickerService`
  opened `NSOpenPanel`, took a typed path through
  `scripts/native/MacOsOpenPanelKeys.applescript`, read the file
  (`read`), and answered `cancelled` for an Escaped panel.

No permission prompt appeared. The macOS window controls, file picker and
`tac.go` cells are `honored` again.

### Android service runs (2026-10-25, MA2)

On the Mac's emulator-5554 (`tachyon-gate` AVD: Android 15, API 35,
`google_apis` arm64-v8a, `userdebug`, WebView 124.0.6367.219), with a debug
`ty` from branch `claude/project-thread-03rpod`, Node 24.19.0, Go 1.27.2,
.NET SDK 10.0.402 with `wasm-tools` and `wasm-tools-net9`, and Swift 6.4.1
with its WebAssembly SDK:

- `scripts/android/rust-crate-test.mjs`: a Rust service using a vendored
  crate, and a second one listed only under
  `tac.application.rust.platformDependencies.android`, answered
  `hello from android` in the app's process. The APK carries the Rust
  library for arm64-v8a and x86_64.
- `scripts/android/page-services-test.mjs`: a Go page service and a C#
  page service with `tac.dotnet.aot` answered a click and a method with an
  argument (`2 in the basket`, `2 items`), each loading its WebAssembly from
  `https://appassets.tachyon.local`. The ahead-of-time `dotnet.native.wasm`
  was 5,784,383 bytes against 1,454,619 bytes for the same service built
  with the trimmed interpreter. Swift and C# `ReportService` callers of the
  `window-controls` feature, running as WebAssembly, set the window title to
  `Quarterly report`, resized the window to 1280 by 800 and minimized the
  app.
- `TAC_ANDROID_LISTENER_LANGUAGE=rs scripts/native/async-listener-app-test.mjs
  --target=android`: the linked Rust async listener (above).
- `TAC_YON_SERVICES=kotlin scripts/native/yon-reach-test.mjs
  --target=android` and `scripts/native/async-listener-app-test.mjs
  --check=shared-setup --target=android` failed on the first run. The
  linked Kotlin companion filled a model's constructor in the order
  `getDeclaredFields` lists the fields, and ART lists them alphabetically:
  `Tick(sku, level)` got `sku = "3"` and `level = 0`, and
  `ProgramState(identity, count)` got its values swapped. The JVM lists
  fields in declaration order, so the off-device tests did not see it. The
  Android companion now carries `tacModelOrder`, generated from the client's
  Kotlin models and the built-in event models, and both gates passed three
  times in a row: the page held the linked Kotlin `realtime` and `sockets`
  services' stream and socket clients and delivered their messages, a linked
  Kotlin HTTP client reached Yon, and two linked Kotlin services read one
  Setup identity and count 1. The Kotlin `suspend` listener passed again.

The Android `tac.application.rust`, `tac.dotnet` and `tac.go` cells are
`honored`, and the window controls and reaching Yon notes name these runs.
An app whose only HTTP client uses a page adapter still needs a
`tac.yon.origin` to build (TY1601), so the shared Setup fixture declares one
it never calls.

### Page checks on macOS, iOS and Android (2026-10-25, MA3)

The page-check gates gained the steps the survey of `4243f2b6` found
missing. They ran on the Mac (macOS 26.6.2 25G83, Xcode 27.1 27A9275, iOS
26.5 simulator runtime 23F77 on an iPhone 17 Pro, the `tachyon-gate`
emulator image `sdk_gphone64_arm64` Android 15 API 35 AE3A.240806.043 with
Android System WebView 124.0.6367.219), on branch `b70dcc31`:

- `scripts/native/page-checks-macos-test.mjs` passed in 17 s. A
  `@SessionState` field was kept across a navigation to another page and
  back, and was gone after a quit and restart. The `_param` route
  `/items/7/` opened in the packaged app with its own linked Swift service:
  the root count was set to 7 (doubled 14 by Swift), the item route started
  at 0 and was set to 9 (doubled 18), and back on the root a fresh owner
  started at 0 and was set to 7 again. A `@LocalState` field set by a Swift
  method was read after `location.reload()` and after a quit and restart.
- `scripts/native/geolocation-macos-test.mjs` (new) passed in 8 s. The app's
  `Info.plist` carried the manifest location reason, the CoreLocation prompt
  (shown by CoreLocationAgent) was allowed by the gate through the
  Accessibility API, and a position with latitude and longitude in range
  reached the page through `PositionFound`. The gate does not print the
  position. Each run uses a new bundle id, so System Settings keeps one
  Location Services entry per run.
- `scripts/ios/page-checks-test.mjs` passed in 149 s. A `@SessionState`
  field was kept across a navigation to `/other/` and back, and was gone
  after a terminate and relaunch. With the simulator device set inside an
  APFS disk image on an external volume, the gate failed at its first
  external link ("Tac native call timed out" after Safari opened), the same
  way the script from before this change did; with the device set on the
  internal disk, as in RM2, it passed.
- `scripts/android/page-checks-test.mjs` passed in 46 s. `connect-src`
  refused a page fetch to `http://localhost:<port>`, a host outside
  `tac.outgoing.hosts` that `adb reverse` would have reached, with a
  `securitypolicyviolation`, and that host saw no request while the Yon
  origin answered. A component `tac.css` rule coloured an output inside its
  component and not an output in another component, and the build printed
  the WebView 118 style note (ADR 0108). A `@SessionState` field was kept
  across a navigation and was gone after a relaunch. External links must
  stay the last page step: the host cancels a tapped external link's
  navigation, and Playwright then waits on it before every later click.

The macOS geolocation cell is `honored` again, and the Android
`tac.outgoing` note records the refusal run.

### Swift on the Windows desktop (2026-10-25, SW3)

The Surface (Windows 11 ARM64, WebView2 Runtime 155.0.4283.62, MSVC ARM64)
now has Swift 6.3.3 for Windows ARM64
(`swift-6.3.3-RELEASE-windows10-arm64.exe` from swift.org, installed
without a UAC prompt) and the matching Swift SDK for WebAssembly
(`swift-6.3.3-RELEASE_wasm.artifactbundle`), with `TAC_SWIFT_WASM_SDK`
set to its `wasm32-unknown-wasip1` folder. With a debug `ty` from develop
`2aeb2de6` and `TAC_WINDOWS_SERVICE_LANGUAGES`,
`TAC_WINDOWS_WINDOW_CALLERS` and `TAC_WINDOWS_PICKER_CALLERS` set to
`swift`, `scripts/windows/page-services-native-test.mjs` passed in 114 s in
the signed-in desktop session, started through
`scripts/windows/desktop-run.ps1`:

- The Swift Tally page service, compiled to WebAssembly in 32 s, answered
  `2 in the basket` and `2 items` after a click (loaded in 2.9 s).
- The Swift `ReportService` caller of the `window-controls` feature
  renamed the window to `Quarterly report` and resized it to 1280 by 800.
- The Swift FilePicker field of the `native-capabilities` feature opened
  `IFileOpenDialog`, read the typed file (`read`), and answered
  `cancelled` when the next dialog was closed with Escape.

The Windows window controls and file picker cells then owed only their
linked Rust callers, which ran in SW1 (below).

### Linked services in WebView2 (2026-10-25, SW1)

On the Surface (Windows 11 25H2 build 26200 ARM64, WebView2 Runtime
155.0.4283.62, MSVC 14.44 ARM64, Rust 1.97.1, .NET 10.0.302, Node 24.21.0)
a debug `ty` from branch `f1f39fa8` ran four gates in the signed-in desktop
session, each started through `scripts/windows/desktop-run.ps1` and
`scripts/windows/developer-run.ps1`:

- `scripts/windows/page-services-native-test.mjs` with
  `TAC_WINDOWS_SERVICE_LANGUAGES=,` (Windows drops a variable set to an
  empty string, and the list reads `,` as empty) and
  `TAC_WINDOWS_WINDOW_CALLERS` and `TAC_WINDOWS_PICKER_CALLERS` set to `rs`
  passed in 32 s. The linked Rust `ReportService` of the `window-controls`
  feature renamed the window to `Quarterly report` and resized it to 1280
  by 800, and the linked Rust FilePicker field of the `native-capabilities`
  feature opened `IFileOpenDialog`, read the typed file (`read`) and
  answered `cancelled` after Escape. Each app's `tachyon.host.json` listed
  the Rust companion.
- `scripts/windows/application-state-test.mjs` passed in 47 s. A linked C#
  service heard `foreground` (replayed), `background` after the focus moved
  away, `foreground` again and `background` after a minimize, beside the
  page and the linked Rust service.
- `scripts/windows/yon-reach.ps1` passed in 61 s. Besides the earlier page,
  HTTP and C# paths, the page held a linked Rust service's stream (`Rust
  stream ok 3`) and socket (`Rust socket rust-echo`) clients, and a linked
  Rust service heard the `ReachNews` topic (`Rust topic heard 1`). The same
  Rust services passed on the Linux app under Xvfb in an x86_64 cloud
  container first.
- `scripts/windows/async-listener-native-test.mjs` passed in 41 s with a
  linked C# `Task` listener and then a linked Rust `async fn` listener of
  400 ms: each showed `[a1s1]` with the app's process ID, then
  `[a1s1a2s2]`.

The Windows window controls, file picker, background and foreground, and
reaching Yon cells are `honored` again.

### Native shell runs (2026-10-25, MA4)

On the Mac (macOS 26.6.2, Apple M4, Xcode 27.1, iOS 26.5 simulators), with a
debug `ty` from branch `claude/project-thread-t2wmoo` (commit `e6c017f1`):

- `scripts/ios/window-settings-test.mjs` (iPhone 17 Pro and iPad Pro 13-inch
  (M5)): the WIN5 checks passed again. The standard app, held before `main`
  with `simctl launch --wait-for-debugger`, showed its launch screen:
  `background_color` `#ffaa00` filled the status bar row and rows at 30, 60
  and 90 percent of the screen, and the page had not run. A JavaScript
  `WindowControls` caller set titles of 16, 2,000 and 70,000 characters and
  heard two `BrowserCallFailed` events: `windowControls setTitle invalid`
  (the host refuses a title over 1,024 bytes) and `windowControls setTitle
  invalid or oversized host call` (the host refuses a request over 64 KiB).
  The valid title raised no event.
- `scripts/native/page-services-macos-test.mjs` with only the refusal check:
  the same caller in the packaged macOS app heard one `BrowserCallFailed`,
  `windowControls setTitle invalid or oversized host call`. The macOS host
  accepts a 2,000-character title and keeps its first 256 characters. A call
  after the window closed is also refused (`Native window unavailable.`), but
  no page is left to hear it.
- `scripts/ios/application-lifetime-test.mjs`: on iPhone and iPad, a
  double press of Home opened the app switcher and a swipe on the app's card
  ended the app. `applicationWillTerminate` ran: the open page's Rust
  `SectionService` ran its `OnStop` before the application service's
  `OnStop`, which ran once. On iPad a touch swipe down from the top edge
  opened the menu bar, and touches on View and What's Running forwarded
  `SectionChosen`. The touches come from `scripts/ios/simulator-tap.m`
  (`home` and `drag` modes).

On the Mac's emulator-5554, with the same branch at commit `df1e8c26`:

- `scripts/android/window-settings-test.mjs` passed on `tachyon-gate`
  (Android 15, API 35, WebView 124.0.6367.219) and on a new `tachyon-api34`
  AVD (Android 14, API 34, `google_apis` arm64-v8a, WebView
  113.0.5672.136). The standard app's dark `theme_color` gave light status
  and navigation icons (median luminance 215 to 255 in the bar frames that
  `dumpsys window` lists), and the integrated app's light `theme_color` gave
  dark icons (29 to 120; the gesture handle is translucent black). The
  standard app, stopped with `run-as` `kill -STOP` right after `am start`,
  kept its splash window on screen: `dumpsys` listed `Splash Screen` for the
  package and the screen showed `#ffaa00`. On WebView 113 a component's
  `tac.css` does not apply (it needs `@scope`, WebView 118, ADR 0108), so the
  gate now sizes the probe's bar with an inline style.
- `scripts/android/application-lifetime-test.mjs` passed on `tachyon-gate`:
  on Back, `onDestroy` disposed the open page's Rust `SectionService` (its
  `OnStop` ran) before the application service stopped once.

The first API 34 boot needed several tries: Android killed the app while
Play Services restarted, and a "System UI isn't responding" dialog dimmed the
screen. Gradle 8.14 does not run on the Mac's default JDK 25, so the runs
used JDK 21. Evidence on the Mac: `/Volumes/FORGE/Logs/TACHYON/ma4-20261025/`.

### Page and shell runs in WebView2 (2026-10-25, SW2)

On the same Surface (Windows 11 ARM64, WebView2 Runtime 155.0.4283.62,
WebView2 SDK 1.0.3485.44, MSVC ARM64, Python 3.14.7) a debug `ty` from
branch `97f3fc29` ran three gates in the signed-in desktop session, each
started through `scripts/windows/desktop-run.ps1` and
`scripts/windows/developer-run.ps1`:

- `scripts/windows/dev-loop-test.mjs` passed in 39 s. `ty preview --target
  windows` opened the app against the development server. Through UI
  Automation, a Yon call answered from the development server, a component
  CSS edit arrived while the page kept its count (no reload), a component
  HTML edit appeared, and a manifest edit rebuilt and reopened the app.
  `ty preview` runs in its own console process group
  (`scripts/windows/process-group.py`), and a `CTRL_BREAK` to it closed the
  app; `ty preview` exited with code 0.
- `scripts/windows/window-shell-test.ps1` passed in 53 s, with a new Shell
  Page app beside the earlier ones. Its `background_color` is `#1565c0` and
  its page has no background of its own: all 43,078 sampled pixels below
  the page content had `#1565c0`. A component `tac.css` rule gave its own
  `.mark` the background `rgb(200, 30, 120)` (computed style, and 74% of
  the element's pixels), while a `.mark` in another component kept
  `rgba(0, 0, 0, 0)` (none of its pixels), on WebView2 155. A page service
  called the granted `setTitle` with a 600-character title. The page check
  let it through, the host refused it (a title holds at most 512 UTF-16
  units), and the service heard `BrowserCallFailed` with `api`
  `windowControls`, `call` `setTitle` and the message `Invalid window
  title.`; the title did not change.
- `scripts/windows/page-services-native-test.mjs` with the three
  `TAC_WINDOWS_*` lists set to `,` passed in 19 s. A linked Rust service's
  `#[LocalState]` field, set by its own method, still read `Visits 1` after
  a page reload, while a JavaScript field went back to `fresh`.

The Windows development preview and `background_color` cells are
`honored`.

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
| Rust workspace and Phase 4 CLI on Windows x86_64 | native-tested development milestone | updated 2026-10-18. The release builds `ty-windows-x64.exe` for `x86_64-pc-windows-msvc` on `windows-2025`. The release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) (v26.36.01, 2026-08-31) clean-installed and checked that executable on `windows-2025`, and `rust-ci.yml` runs the locked workspace suite on `windows-latest` (its run 39253757046 of 2026-10-19, the first with every Yon language installed, failed 44 tests; WG1 fixed their causes, and on 2026-10-25 the whole suite passed on Windows 11 ARM64, see Windows native gate). Before, this row said `buildable` (only the GNU target compiled locally). No Windows ARM64 `ty` is released. This row is about the CLI only; for the WebView2 host see the tables above |
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
