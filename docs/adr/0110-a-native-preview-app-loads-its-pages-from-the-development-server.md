# ADR 0110: A Native Preview App Loads Its Pages From the Development Server

- Status: Accepted; Linux (Xvfb and WSL), macOS, iOS simulator and Android
  emulator runs passed 2026-10-24; the Windows run is owed
- Date: 2026-10-24
- Works with: [ADR 0013](0013-semantic-hot-updates.md) (semantic hot
  updates), [ADR 0103](0103-native-apps-reach-yon-through-a-declared-origin.md)
  (native apps reach Yon through a declared origin),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the per-target support table)

## Context

`ty preview --target <native>` served the web stage of a native target from
a static server. The developer saw the pages in a browser, not in the app.
To see a change in the real app, they ran `ty bundle` again and opened the
app by hand. Hot updates, which `ty preview` gives a web page, never reached
the native web view.

A native page loads from its own origin: `tachyon://app` on Linux,
`tachyon-app://bundle` on macOS and iOS, `https://appassets.tachyon.local`
on Android and `https://tachyon.local` on Windows. The bridge, the native
capability routes and the host's origin checks all depend on that origin.
Loading the app from `http://127.0.0.1:3000` would change every one of them
for development only, so the preview would test a different app.

## Decision

1. **`ty preview --target <native>` builds and opens the app.** The preview
   builds the target into `dist/preview/<target>` and starts it. The desktop
   apps start as a child process. The iOS app is installed into the booted
   simulator (`TAC_IOS_SIMULATOR` picks another one). The Android app is
   installed on the running emulator or device with `adb install -r`, after
   `adb reverse tcp:<port> tcp:<port>`. Stopping `ty preview` stops the app.
   `--static` keeps the old static preview of an existing build.

2. **The app keeps its own origin and asks the development server for its
   files.** In a preview build, the host's scheme handler sends each page
   request to the development server, after it maps the route to its
   document. The handler answers with the server's status, content type
   and bytes, plus the host's own Content-Security-Policy and
   `Cache-Control: no-store`. Linux uses libsoup, macOS and iOS
   `URLSession`, Android `HttpURLConnection` and Windows WinHTTP. Redirects
   are refused, and an answer is limited to 16 MiB, like a bundled file.
   Other builds get a stub that answers nothing, so their host source holds
   no development code.

3. **The development server builds the native page stage.** It builds the
   pages for that target's native web view into
   `dist/preview/pages/<target>`. It adds the app's origin to the server's
   CORS origins, and it puts
   `<meta name="tachyon-development-origin" content="...">` before the hot
   update script. The script opens its event stream and reads diagnostics at
   that origin, because the page's own origin is the app's.

4. **`tac.yon.origin` points at the development server in preview builds
   only.** The preview writes the development origin into the build's copy
   of `client/manifest.json`. A declared origin is replaced, and the build
   prints a note naming both origins. The authored manifest is never
   changed. Desktop apps and the iOS simulator use `http://127.0.0.1:<port>`.
   Android uses `http://localhost:<port>`, which `adb reverse` forwards, and
   its network security policy allows clear text to that host only.

5. **A release never reaches the development server.** A release build with
   a development origin stops with TY1601. Tests check that every host
   fetches from the development server only in a preview build, and that a
   plain build holds no development origin.

6. **Edits arrive as hot updates. The app is rebuilt only for a native
   change.** A change to `client/manifest.json` or to a linked companion
   source for the target (Rust, Swift, Kotlin or C#) rebuilds the app and
   opens it again. Every other edit is a hot update from the server, as in a
   browser (ADR 0013).

7. **The preview needs a fixed loopback port.** The app is built to reach
   one address, so port 0 and hosts other than loopback stop with TY1301.

Native UI live update, which would replace native views without a rebuild,
stays a separate decision.

## Consequences

- A developer sees each edit in the real app, with the real bridge, menus,
  window and native capabilities.
- The preview app is a debug build that depends on the development server.
  It shows the server's error page while the project does not build, and
  that page names the development server, so the app recovers when the
  build passes.
- A Yon controller edit does not change Yon's answers until `ty preview`
  restarts. This is also true for a web preview, and it is not changed here.
- On Windows the resource handler waits for the development server on the
  WebView2 thread, for at most 30 seconds per request. That is acceptable
  for a local server; a release host never does it.

## Verification

- 2026-10-24, Linux under Xvfb in an x86_64 cloud container (GTK 4.14.5,
  WebKitGTK 2.52.6, libsoup 3.4.4): `scripts/linux/dev-loop-native-test.mjs`
  opened the app with `ty preview --target linux` and drove it through
  AT-SPI. A Yon call reached the development server. A component CSS edit
  arrived while the page kept its state, so it was not a reload. A component
  HTML edit appeared. A manifest edit rebuilt and reopened the app. Stopping
  `ty preview` closed the app. The Linux gate runs this script.
- 2026-10-24, code level in the cloud: the preview Windows host compiled
  with MinGW against the Win32, WinHTTP and WebView2 headers, and the
  preview Android host compiled with `javac` against `android.jar` and
  AndroidX.
- 2026-10-24, on the Mac (Apple M4, macOS 26.6.2, Xcode 27.1): the same
  steps passed for the macOS app (`scripts/native/dev-loop-macos-test.mjs`,
  read through the Accessibility API), the iOS 26.5 simulator
  (`scripts/ios/dev-loop-test.mjs`, read by OCR of simulator screenshots)
  and an API 35 Android emulator (`scripts/android/dev-loop-test.mjs`, read
  by OCR of screen captures, because UI Automator shows none of the web
  view's text). The Linux gate also passed on WSL Ubuntu 24.04.5 (aarch64).
  The Swift host compiled without a change.
- Owed: the Windows app on a signed-in desktop (SD1).
