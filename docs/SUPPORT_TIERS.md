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
| Android WebView | Android 15 emulator. Passed: Kotlin calls, publish, input/focus per character with a delayed setter, service-worker retirement, packaged dynamic routes, window controls (granted minimize, resize and title act; ungranted maximize, fullscreen and close are refused, 2026-10-18), and a `@LocalState` field set by a Kotlin method kept across navigation and reload. Browser APIs (clipboard, share, notifications through the host; geolocation through the web view with the host's location question, 2026-10-19): the debug APK compiles with Gradle on Linux; the emulator gate `scripts/android/browser-apis-test.mjs` has not run yet | exact release artifact; the browser-API emulator run; support on physical devices is still not claimed |
| Linux WebKitGTK | Rust native service compiles. The shared route/security probe passes. On 2026-10-18 `scripts/linux/native-test.sh` ran the GTK 4 / WebKitGTK host under Xvfb with an AT-SPI (Linux accessibility bus) session, in a cloud Linux VM (Ubuntu 24.04, GTK 4.14.5) and in the pinned Debian trixie image (GTK 4.18.6). Passed: the window name, the menu bar and its item names, page roles and names (landmark, heading, button, link), AT-SPI activation of Rust calls and route changes, and the lifecycle log | exact release artifact; Linux native GUI CI on the current head; a run on a Linux desktop, because a container or VM does not prove desktop behavior |
| Windows WebView2 | generated C compiles with the official software development kit (SDK). Real C# protocol/OS execution. | Windows native GUI CI execution |

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

macOS release packages (2026-10-19, ADR 0104 amendment): `ty bundle --target
macos --release` signs the app and its helpers with the hardened runtime,
builds a `.dmg`, then signs, notarizes, staples and assesses it. Checked in an
x86_64 Linux cloud container: the signing variables are checked before the
build (`TY1605`), the certificate is chosen from `security find-identity`
output, the codesign, hdiutil, notarytool, stapler and spctl arguments, the
entitlements from the manifest grants, and the notarization answer. On
2026-10-19 the macOS tests in `crates/tachyon-cli/tests/it/macos_release_cli.rs`
passed on an Apple silicon Mac: an ad hoc release with the hardened runtime
flag, the location entitlement and the expected `.dmg` contents. Not yet run:
signing with the team's Developer ID certificate and notarization (the ignored
Developer ID test). The team's Developer ID Application certificate is in the
Mac keychain, but `xcrun notarytool store-credentials` answers HTTP 403 ("a
required agreement is missing or has expired") for team JAU9V4WH99, so no
notary profile exists yet.

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
two releases that gave the same `.deb` and AppImage bytes. Not yet run: WSL
Ubuntu 24.04 (aarch64), an AppImage started through FUSE, and an install on a
Linux desktop.

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
| Linux WebKitGTK | yes | `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and a real Rust library, and ran it under Xvfb on WSL Ubuntu 24.04 (aarch64), passing three of three runs: the service started once at launch, a menu action reached the page service without a reload, the item was disabled with no recipient, Help and Home kept the service, and Close Window stopped it once. Not covered: a physical Linux desktop, Wayland, x86_64, and clicks or keys by a person. WSLg's display did not accept connections from the remote test lane, and with WebKit's sandbox on, the first page load took up to about 80 seconds on that machine |
| Windows WebView2 (x64) | yes | built and run on Windows 11 ARM64 on 2026-10-04 with Visual Studio Build Tools 2022 17.14 (MSVC 14.44, ARM64-hosted x64 tools), Windows SDK 10.0.26100.0 and WebView2 SDK 1.0.3485.44. `ty`, built natively for `aarch64-pc-windows-msvc`, ran `ty bundle --target windows` for the `menu-actions` and `menu-items-that-open-a-page` examples: the whole host compiled with `cl /W4 /WX`, linked `WebView2LoaderStatic.lib`, and `TachyonRustCompanion.dll` was built (after the C4100 fix of 2026-10-04). The WebView2 handler interface IDs match `WebView2.h`. The apps ran from an SSH session and were driven with Win32 messages: the application service started once before the window, the File, View and Help menus showed the declared items and shortcuts, a menu action ran, an item whose event had no recipient stayed disabled and did not run, and Close Window ended the process with exit code 0 after exactly one stop (also after `WM_ENDSESSION`). Not run: anything inside WebView2. Outside a signed-in desktop session `CreateCoreWebView2Controller` fails with `0x80070578` (invalid window handle), so page rendering, page events from a menu action, route items, the `AcceleratorKeyPressed` handler and `scripts/windows/native-test.ps1` still need a run on an interactive desktop |
| iOS, Android | refused at build time | `TY1601` for menus, `TY1406` for the application lifetime |

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
yet verified. Linux and iOS builds use the build machine's architecture, and
Android has no `--arch` until MR2.

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
Win32 headers. The MSVC build and the real-window check
`the_windows_window_frame_drives_a_real_win32_window` still need a run on
Windows; the caption color also needs a look on an interactive desktop.

On 2026-10-18 the Linux host started to open its window from the manifest
too: the page at `tac.application.window` size and minimum size, a hex
`theme_color` as a GTK header bar color, and `"display": "fullscreen"` as a
fullscreen start (ADR 0052 amendment). `linux_window_appearance_tests`
compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings
as errors) and ran it under Xvfb with xfwm4 in an x86_64 Ubuntu 24.04 cloud
container, three of three runs: the page opened at 900x640, a resize below
the 720x520 minimum stopped at 720x520, the header bar rendered `#1a237e`
with white text, and `fullscreen` reached the GTK window and the X11
toplevel. Not run: WSL (aarch64), a physical desktop, Wayland.

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
bar test three of three times. Not run: a physical desktop, a compositing
window manager, Wayland.

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
| Web | implemented | page transports, CORS, preflight, socket origin and `connect-src` unit tests; Chromium cross-port run |
| macOS, iOS | run on the host | `scripts/native/yon-reach-test.mjs` passed on 2026-10-19 for the macOS app (macOS 26.6.2, Xcode 27.1) and the iOS 26.5 simulator (iPhone 17 Pro) over `http` and over `https` through a local TLS proxy. In all three runs Yon received `Origin: tachyon-app://bundle`, and the page reached it with HTTP, a CORS preflight, a stream, a socket, a topic, a 422 problem and a signed-in route. The linked Swift `@HTTPClient` read reached the origin through `URLSession` too; it is also unit-tested against a local server |
| Android | run on the emulator; an `https` origin, or a local `http` origin with cleartext to that one host | `scripts/android/yon-origin-test.mjs` passed on 2026-10-19 on the Android 15 (API 35, arm64) emulator against `ty serve` on the Mac through `adb reverse`. The packaged app reached Yon from `https://appassets.tachyon.local` with an HTTP read, a cross-origin write, a stream, a socket, a topic, a `@Check` 422 and a signed-in route (401 without a token, then with an `Authorization` header). A linked Kotlin `HTTPClient` reached Yon through the native transport in the app's own process. The transport is also tested on the JVM against a real HTTP server (`android_yon_http_tests`). Not run: a physical device |
| Windows | implemented; linked Rust and C# `HTTPClient` calls reach it too; not run on the host | 2026-10-19 cloud runs (below); `scripts/windows/yon-reach.ps1` on a signed-in desktop is owed (NET3, with N1) |
| Linux | run under Xvfb | `scripts/linux/yon-origin-native-test.mjs` built the packaged app with GTK 4.14.5, WebKitGTK 2.52.6 and libsoup 3.4.4 in an x86_64 Ubuntu 24.04 cloud container and drove it through AT-SPI. It reached Yon on `http://127.0.0.1` and `https://localhost`, once with JavaScript page services and once with linked Rust services: HTTP read and write, a preflighted `@SignedIn` call (401 without a token), a topic, a `@Check` 422, a stream and a socket. The Linux gate runs both. Not run: WSL, a Wayland desktop |

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

Not run: the MSVC and NativeAOT Windows build, and WebView2 itself.
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

Native calls have a ten-second browser wait limit. This limit does not stop or
roll back native service code. Services must stay responsive. A hung
application may need to be relaunched.

### Native apps keep Yon reads offline (2026-10-19)

A native build with `tac.yon.origin` and `tac.cache` applies the rules to
Yon reads in the page's Cache Storage (ADR 0087 amendment). It is tested in
Node and by `scripts/yon-reads-browser-test.mjs` in Chromium, Firefox and
WebKit in the cloud. No native host has run it yet: the airplane-mode runs
on the iOS simulator and the Android emulator are owed, and on Linux the
kept reads last only while the app runs (ST1).

### External links on native hosts (2026-10-18)

ADR 0018's amendment of 2026-10-18 covers links that leave the app. A link to
an `http`, `https` or `mailto` address opens in the default browser or mail
app; every other scheme opens nothing. This evidence does not promote any
tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes, `GtkUriLauncher` | `native_external_link_tests::the_linux_host_hands_external_links_to_the_launcher_and_keeps_the_page` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb in an x86_64 Ubuntu 24.04 cloud container, four of four runs: a page script's `location` changes, `window.open` calls and a `target="_blank"` link reached a fake launcher only for the `http`, `https` and `mailto` links, and the page stayed on `tachyon://app/`. Not run: the real launcher on a desktop, WSL, Wayland |
| Windows WebView2 | yes, `ShellExecuteW` | the opener and the shared check, built with MinGW-w64 and run under Wine 9.0, open only the external links. Not run: WebView2 on an interactive desktop |
| macOS, iOS WKWebView | yes, `NSWorkspace` and `UIApplication` (before 2026-10-18 for `http` and `https`) | source checks only for the `mailto` and no-user-name rule; not run on the Mac |
| Android WebView | yes, `ACTION_VIEW` (before 2026-10-18 for `http` and `https`) | source checks only for the `mailto` and no-user-name rule; not run on an emulator |

### Deep links on native hosts (2026-10-19)

ADR 0106 adds `tac.application.links`. A link with a listed scheme, or an
`https` link to a listed host, opens the app at its route. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: `.desktop` scheme handlers, `G_APPLICATION_HANDLES_OPEN` forwarding | `native_deep_link_tests::the_linux_app_opens_links_at_their_route_and_forwards_later_links_to_the_running_app` compiled the generated host with GTK 4 and WebKitGTK 6.0 (warnings as errors) and ran it with `dbus-run-session -- xvfb-run -a` in an x86_64 Ubuntu 24.04 cloud container: the launch link opened its route, and two later launches handed their links to the running window. Not run: WSL, a desktop with the `.desktop` file installed |
| Windows WebView2 | yes: per-user scheme registration (MSIX: `windows.protocol` extensions), single instance, `WM_COPYDATA` forwarding | `native_deep_link_tests::a_second_windows_instance_forwards_its_link_to_the_first` built the registration and forwarding code with MinGW-w64 and ran it under Wine 9.0: the schemes were registered under `HKCU\Software\Classes` and a second process's link reached the first. `windows_package_tests::the_staged_layout_packs_and_signs_with_the_msix_sdk` packed a package that declares two schemes with `makemsix`, which validated its `windows.protocol` extensions against Microsoft's schemas (and refused an unknown extension category). On the Surface desktop (Windows 11 arm64, 2026-10-19), a built app opened `lk2check://alpha?x=1` at `/alpha/?x=1` on launch; later links reached the running window and their second instance exited, and an unknown route opened `start_url`. An installed test-signed MSIX package opened `lk2check://beta?x=5` at its route, and Windows, not the app, registered the scheme. Not run: later links into the installed package |
| macOS, iOS WKWebView | yes: `CFBundleURLTypes`, `kAEGetURL` (macOS), launch URL, `open url` and universal-link activity (iOS); associated domains with `TAC_APPLE_TEAM_ID` | on the Mac (2026-10-19, Swift 6.4.1): `the_apple_link_rule_opens_declared_schemes_and_hosts_at_their_route` passed and the iOS host tests passed. A built macOS app opened `lk2check://alpha?x=1` at `/alpha?x=1` on launch; while it ran, later links moved the same window, and an unknown route opened `start_url`. The same app on an iOS 26.5 simulator (iPhone 17 Pro) did the same on a cold start and while running. macOS rewrites `#` to `%23` in a link without `//` (`lk2check:alpha#frag`), so such a link opens `start_url`. Owed: a signed device check of universal links |
| Android WebView | yes: intent filters, `autoVerify` for hosts, `singleTask` with `onNewIntent` | the link rule ran with OpenJDK in the cloud. On emulator-5554 (2026-10-19), a launch link opened its route, later links reached the running activity through `onNewIntent`, and an unknown route opened `start_url`. Owed: verified app links for hosts on a device |

### Browser APIs on native hosts (2026-10-19)

ADR 0065's amendments of 2026-10-18 and 2026-10-19 cover `clipboard`,
`share`, `notifications` and `geolocation` in native apps. This evidence does
not promote any tier.

| Host | Implemented | Validated |
| --- | --- | --- |
| Linux WebKitGTK | yes: the web view's clipboard and geolocation (GeoClue) behind one permission decision from the manifest grants; notifications bridged to `GNotification`; share copies to the clipboard | `linux_browser_api_tests::linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view` compiled the generated host with GTK 4.14.5 and WebKitGTK 2.52.6 (warnings as errors) and ran it under Xvfb with XTest clicks in an x86_64 Ubuntu 24.04 cloud container, with and without a window manager: granted, the page read another source's clipboard text, wrote its own, got a test position, was granted and showed a notification, and shared a url to the clipboard; with nothing granted, each was refused. Not run: WSL, a Linux desktop with a notification server and GeoClue, Wayland |
| macOS WKWebView | yes, all four: `NSPasteboard`, `NSSharingServicePicker`, `UNUserNotificationCenter`, CoreLocation | code level only, in a Linux cloud container (`native::browser_api_bridge`, `native::apple_browser_apis`). Owed: the compile and host test on a Mac, and a manual check of the share, notification and location prompts |
| iOS WKWebView | yes, all four: `UIPasteboard`, `UIActivityViewController`, `UNUserNotificationCenter`, CoreLocation | code level only, in a Linux cloud container (same tests). Owed: the simulator gate (`scripts/ios/native-test.mjs`) and a manual check of the system prompts |
| Android WebView | yes: clipboard, share and notifications through the host bridge; geolocation through the web view (BA2) | see the Android WebView row above |
| Windows WebView2 | no host bridge yet (BA3) | see the per-target support table |

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
| Windows WebView2 | not run | WebView2 keeps its data in `<App>.exe.WebView2` (ADR 0097); pages need an interactive desktop session |

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
| Rust workspace and Phase 4 CLI on Windows x86_64 | native-tested development milestone | updated 2026-10-18. The release builds `ty-windows-x64.exe` for `x86_64-pc-windows-msvc` on `windows-2025`. The release run [33363597481](https://github.com/d31ma/Tachyon/actions/runs/33363597481) (v26.36.01, 2026-08-31) clean-installed and checked that executable on `windows-2025`, and `rust-ci.yml` runs the locked workspace suite on `windows-latest`. Before, this row said `buildable` (only the GNU target compiled locally). No Windows ARM64 `ty` is released. This row is about the CLI only; for the WebView2 host see the tables above |
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
