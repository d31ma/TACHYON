# ADR 0106: Deep Links Come From the Manifest

- Status: Accepted; host runs on a Windows desktop are pending
- Date: 2026-10-19
- Related: [ADR 0018](0018-native-hosts-run-the-platform-javascript-engine.md)
  (native hosts and links that leave the app),
  [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
  (the manifest is the Tac configuration),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the per-target support table),
  [ADR 0104](0104-release-packages-are-opt-in-and-signed-from-the-environment.md)
  (signing comes from the environment)

## Context

Every Tac route has its own address on the web, so a link can open any page.
A native app could not be opened from a link at all: no host claimed a URL
scheme, no host read a launch URL, and the `route.open` bridge call was
implemented only on iOS. The platforms each have their own way to hand a link
to an app, and the verified web links (universal links on iOS, app links on
Android) also need files on the web site and the app's signing identity.

## Decision

1. **One member.** `tac.application.links` declares the links an app answers:
   `{ "schemes": ["myapp"], "hosts": ["example.com"] }`.
   - `schemes`: up to 16 lowercase URL schemes of at most 64 characters, each
     starting with a letter and using letters, digits, `+`, `-` or `.`.
     `http`, `https`, `mailto`, `file`, the browser's own schemes and the
     schemes Tachyon uses (`tachyon`, `tachyon-app`) are refused.
   - `hosts`: up to 16 lowercase host names with a dot, without scheme, port,
     path or wildcard.
   - Anything else stops the build with `TY1601`.
2. **One rule for every host.** A link opens the app when its scheme is listed,
   or when it is an `https` link whose host is listed (no user name, no port).
   - For a scheme link, the route is everything after `scheme:` without its
     leading slashes: `myapp://settings`, `myapp:///settings` and
     `myapp:settings` all open `/settings`.
   - For a host link, the route is the link's path.
   - The query and fragment are kept. A trailing slash is ignored. On macOS,
     a fragment needs the `scheme://` form: the system rewrites `#` to `%23`
     in a link such as `myapp:settings#top`, which then opens `start_url`.
   - A route the app does not have, or one that is not a canonical local
     route, opens `start_url` instead.
   - The rule is written once per host language: `native_deep_link.h` (C, for
     Windows and Linux), `TacDeepLink.swift` (macOS and iOS) and
     `native_deep_link_android.java` (Android). One table of cases checks all
     three (`native_deep_link_tests`).
3. **Registration and delivery per target.**
   - **macOS:** `CFBundleURLTypes` in `Info.plist`, and a `kAEGetURL` Apple
     event handler registered before launch finishes. A link that arrives
     before the window exists becomes the first page.
   - **iOS:** `CFBundleURLTypes`, the launch option URL,
     `application(_:open:options:)` and, for universal links,
     `application(_:continue:restorationHandler:)`. The host has no scenes,
     so the application delegate receives the links instead of the scene
     delegate that LK2's brief named.
   - **Android:** one `VIEW` intent filter per scheme, and one `https` filter
     with `android:autoVerify="true"` for the hosts. The activity is
     `singleTask`, so a later link reaches `onNewIntent`. `route.open` now
     navigates instead of only recording the call.
   - **Linux:** the `.desktop` file lists `MimeType=x-scheme-handler/<scheme>;`
     and `Exec=... %u`. The app is a `GApplication` with
     `G_APPLICATION_HANDLES_OPEN`, so a second launch hands its link to the
     running app over D-Bus and exits. The scheme is claimed once a
     `.desktop` file is installed: the release `.deb` and AppImage carry it
     (ADR 0104).
   - **Windows:** the unpackaged app registers each scheme under
     `HKCU\Software\Classes\<scheme>` every time it starts, so a moved app
     keeps working. A named mutex makes the first instance the only one; a
     later launch sends its link to the first instance's message-only window
     with `WM_COPYDATA`, lets it take the foreground, and exits. A
     `--release` MSIX package declares each scheme as a `windows.protocol`
     extension instead, and the packaged app writes nothing to the registry.
4. **Verified web links.**
   - The web build writes `/.well-known/apple-app-site-association` when
     `hosts` is listed and `TAC_APPLE_TEAM_ID` is set, with the app ID
     `<team>.<tac.application.id>` and every path.
   - The web build writes `/.well-known/assetlinks.json` when `hosts` is
     listed and both `tac.origin` and `TAC_ANDROID_CERT_SHA256` are set, with
     the Android package name and the listed fingerprints.
   - For each file it cannot write, the web build prints one note naming what
     is missing.
   - An iOS build with `hosts` and `TAC_APPLE_TEAM_ID` writes
     `project/TachyonHost.entitlements` with
     `com.apple.developer.associated-domains` (`applinks:<host>`) and signs the
     app with it. A `--device` or `--release` build adds the same domains to the
     entitlements it signs with its provisioning profile (ADR 0104 amendment).
   - The team ID and the fingerprints are environment variables, never
     manifest members (ADR 0104). A malformed value stops the build with
     `TY1601`.
5. **Support table.** `tac.application.links` is honored on the web and
   adapted, with a note, on every native target. The note names what each
   target does not do yet: links to hosts on macOS (PKG1), Windows (no app URI
   handlers) and
   Linux (no verified web links), universal links on iOS until a signed
   device build, and unverified hosts on Android.

## Consequences

- A developer declares links once, and each target either claims them or says
  in one note why not.
- Tachyon's own server serves `/.well-known/apple-app-site-association` as
  `application/octet-stream`, because the file has no extension. A site that
  serves the web build from another server or CDN should send it as
  `application/json`.
- The Windows app becomes single-instance when it declares a scheme.
- Evidence (2026-10-19, x86_64 Ubuntu 24.04 cloud container): the C rule ran
  with `cc`, the Android rule with OpenJDK, and the Windows registration and
  forwarding code with MinGW-w64 under Wine 9.0, and `makemsix` validated an
  MSIX manifest that declares two schemes. The generated Linux host
  opened a launch link at its route, and a second instance's links reached the
  running window, under Xvfb with a D-Bus session bus. On the Mac, the Swift
  rule passed, and a built app opened links at launch and while running on
  macOS, an iOS 26.5 simulator and the Android emulator. Owed: WebView2 on a
  Windows desktop, an installed MSIX package, a Linux desktop with the `.desktop` file installed, and a
  signed iOS device check of universal links once `TAC_APPLE_TEAM_ID` is set.
