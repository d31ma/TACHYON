# ADR 0106: Deep Links Come From the Manifest

- Status: Accepted; a signed iOS device check of universal links is pending
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
  macOS, an iOS 26.5 simulator and the Android emulator. On a Windows 11
  desktop, the WebView2 app and an installed MSIX package opened links at their
  route. Owed: a Linux desktop with the `.desktop` file installed, and a
  signed iOS device check of universal links once `TAC_APPLE_TEAM_ID` is set.

## Amendment (2026-10-19): host links on macOS and Windows (LK3)

Links to the listed `hosts` were a note only on macOS and Windows.

1. **Windows.** A `--release` MSIX package declares the hosts as a
   `windows.appUriHandler` extension (`uap3:AppUriHandler`, one `uap3:Host`
   per host) in the same `<Extensions>` as the scheme protocols. With `hosts`
   listed and `TAC_WINDOWS_CERTIFICATE_PUBLISHER` set, the web build writes
   `/.well-known/windows-app-web-link`: one entry with the package family name
   and the path `*`. The family name is the package name from
   `tac.application.id`, `_`, and the 13-character publisher ID that Windows
   derives from the publisher: the first 8 bytes of the SHA-256 of its UTF-16LE
   text, as base32 with the alphabet `0-9a-z` without `i`, `l`, `o` and `u`.
   A publisher without `CN=` stops the web build with `TY1601`; without the
   variable the web build prints a note and writes nothing for Windows.
   Windows launches a packaged desktop app for such a link only when the shell
   opens it (another app, Run, `start`); a link clicked in Edge or another
   browser stays in the browser. The unpackaged app has no package identity,
   so it cannot claim hosts. An app that lists hosts and no scheme is now single
   instance too, so a later host link reaches the open window.
2. **macOS.** `TAC_MACOS_PROVISIONING_PROFILE` names a Developer ID
   `.provisionprofile`. When `hosts` are listed, a `--release` build decodes it
   with `security cms -D`, checks that it belongs to `TAC_APPLE_TEAM_ID`, covers
   `<team>.<tac.application.id>` and has the Associated Domains capability
   (`TY1605` otherwise), copies it to `Contents/embedded.provisionprofile`, and
   signs the app with `com.apple.application-identifier`,
   `com.apple.developer.team-identifier` and
   `com.apple.developer.associated-domains` (`applinks:<host>`). An ad hoc
   release (`TAC_MACOS_SIGNING_IDENTITY=-`) with the variable stops with
   `TY1605`, because an ad hoc signature cannot carry a restricted entitlement.
   Without the profile, and in builds without `--release`, the app keeps
   leaving host links in the browser. The `AppDelegate` now answers
   `application(_:continue:restorationHandler:)` for
   `NSUserActivityTypeBrowsingWeb` with the shared link rule.
3. The support table keeps `tac.application.links` adapted on macOS and
   Windows, with notes that name the build each needs.
4. Verifying a link from a hosted domain needs a public host serving the files
   and, for macOS, an Apple Developer profile with the capability. Both are
   outside what the build machines can do, so those runs stay owed.
5. Evidence (2026-10-19). In an x86_64 Ubuntu 24.04 cloud container,
   `makemsix` from the MSIX SDK packed and signed a package that declares two
   schemes and a host, and refused a misspelled `uap3` element against
   Microsoft's schemas; the Windows forwarding code still passed under Wine
   9.0. On the Mac (macOS 26.6.2, Xcode 27.1), the generated `AppDelegate`
   opened browsing-web activities for a listed host at their route and refused
   the others, and an unsigned app opened a scheme link at its route.

## Amendment (2026-10-19): the Linux `.desktop` run (RL1)

This amendment narrows "Owed: a Linux desktop with the `.desktop` file
installed" in the Consequences above, which stays as the history of this
decision. RL1's `scripts/linux/desktop-links-test.sh` installed the `.deb`'s
`.desktop` file under `XDG_DATA_HOME`, and `gio open` and `xdg-open` of
`linkgate://` links reached the one running app, under headless mutter
Wayland and Xvfb in the cloud and on WSL (`docs/SUPPORT_TIERS.md`, Linux
runs owed, RL1). A physical Linux desktop session is still owed. The signed
iOS device check of universal links is still owed too.

## Amendment (2026-10-24): external links on macOS (RM1)

On a Mac (macOS 26.6.2, develop `4c052177`), a page's `http`, `https` and
`mailto` links and a page's `location.assign` to an `http` address went
through `NSWorkspace` to the default browser and mail handler (Microsoft
Edge), and the app kept its page. A local server logged the `http` requests
with Edge's user agent, Edge showed the `https` address, and the unified log
showed the `mailto` hand-off. A link with a user name
(`http://rm1user@127.0.0.1/...`) made no launch request
(`docs/SUPPORT_TIERS.md`, macOS runs owed, RM1). The support table marks
external links `honored` on macOS. The signed iOS device check of universal
links is still owed.

## Amendment (2026-10-24): verified links from a hosted domain (LINKS)

The verified-link runs owed above needed a public host. The organization
site `https://d31ma.github.io` (repository `d31ma/d31ma.github.io`, GitHub
Pages) now serves only the three files at its root. A GitHub Pages site for
the Tachyon repository would sit under `/Tachyon/`, and Apple, Android and
Windows read the files only from the root of a host.

1. `scripts/native/verified-links-files.mjs` builds the link fixtures with
   `ty build` and writes the files the host serves, from the environment of
   ADR 0104: `apple-app-site-association` for
   `<TAC_APPLE_TEAM_ID>.dev.tachyon.link-gate`, `assetlinks.json` for
   `dev.tachyon.link_gate` with the build machine's debug certificates, and
   `windows-app-web-link` for the Windows browser gate's package family. With
   `--compare`, it checks what the host serves.
2. **iOS simulator.** An iOS simulator build with `hosts` and
   `TAC_APPLE_TEAM_ID` signed the app ad hoc with `application-identifier`
   and `com.apple.developer.associated-domains`. The simulator refused to
   launch it ("The file is adhoc signed but contains restricted
   entitlements"). As Xcode does, a simulator build now links the
   entitlements into the executable (`__TEXT,__entitlements` as XML and
   `__TEXT,__ents_der` from `derq`) and signs ad hoc without them. A
   `--device` or `--release` build still signs with them through its
   profile.
3. **macOS.** No Developer ID profile with Associated Domains exists for the
   team, so a signed macOS host link run stays owed.
4. Evidence (2026-10-24). On the Mac (macOS 26.6.2, Xcode 27.1, iOS 26.5
   simulator, iPhone 17 Pro), `scripts/ios/universal-links-test.mjs` found
   the app ID in the served file and in Apple's CDN, found the entitlements
   in the executable and none in the signature, and
   `https://d31ma.github.io/orders?x=1` opened the app at `/orders?x=1` on a
   cold start; `https://d31ma.github.io/orders?x=2` reached the running app.
   On 2026-10-25 (develop `1f06e1f8`), `scripts/android/verified-links-test.mjs`
   passed on the Mac's API 35 `google_apis` arm64 emulator: the served
   `assetlinks.json` listed the certificate Gradle signed the APK with,
   Android verified `d31ma.github.io`, and its `https` links opened the app
   at their route without a chooser, on a cold start and while it ran. The
   Mac's Gradle signs debug APKs with the keystore under
   `ANDROID_USER_HOME`, not `~/.android`, so the served file lists both
   certificates. On the Surface (Windows 11 Home 25H2, build 26200.9711,
   ARM64), the Windows browser gate with `-InstallRoute Machine
   -LinkHost d31ma.github.io -RequireHostLink`, run in an elevated
   PowerShell on the signed-in desktop, installed the signed `.msix` with its
   throwaway certificate trusted machine-wide. Windows named the family that
   `windows-app-web-link` lists, and the shell opened
   `https://d31ma.github.io/alpha?x=3` in the running package at its route.
   A package registered from a folder in Developer Mode is listed as the
   host's handler as well, but Windows does not verify its hosts, and the
   same link opened in Edge. A signed iOS device check stays owed.
