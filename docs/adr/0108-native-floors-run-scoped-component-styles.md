# ADR 0108: Native Floors Run Scoped Component Styles

- Status: Accepted
- Date: 2026-10-24
- Works with: [ADR 0033](0033-dart-browser-services.md)
  (system floors raised by WebAssembly services),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the per-target support table)

## Context

Tac compiles each component's `tac.css` into one CSS `@scope` block, so a
component's rules reach only its own elements. WebKit runs `@scope` from
Safari and iOS 17.4, and Chromium from version 118.

The native floors were lower. The iOS app ran from iOS 17.0, the macOS app
from macOS 13.0 (whose first WebKit is Safari 16), the Windows app from the
WebView2 Runtime 86.0, and the Android app from API level 26 with any
Android System WebView. RM2 ran the WebAssembly page services on the iOS
17.0.1 simulator: clicks, methods, routes and linked Rust worked, but no
component style applied. An app on an older engine looks broken without any
error.

## Decision

1. **iOS starts at 17.4.** The default `MinimumOSVersion` is 17.4, and a
   `tac.application.minimumSystem.ios` below it stops the build (`TY1601`).
   Dart, Kotlin and Java web services still raise it to 18.2.
2. **Windows starts at the WebView2 Runtime 118.0.** The app checks the
   installed runtime at startup, as before, and explains an older one.
   Dart, Kotlin and Java web services still raise it to 119.0.
3. **macOS and Android keep their floors and print a note.** Safari, and
   with it the macOS web view, updates apart from macOS. Google Play updates
   Android System WebView apart from Android. So raising the system floor
   would turn away up-to-date systems. A build of an app with at least one
   component stylesheet prints that the app needs Safari 17.4, or Android
   System WebView 118, for its component styles.
4. **Linux is unchanged for now.** No run has shown which WebKitGTK release
   first runs `@scope`, so the 2.40 floor stays until one does.

## Consequences

- Component styles look the same on every supported iOS and Windows system.
- The iOS 17.0 to 17.3 and WebView2 86 to 117 systems are no longer
  supported. Both update to a newer version where the device allows it.
- A style fallback that works without `@scope` is not needed and not built.
- A browser without `@scope` drops the whole block, so web visitors on such
  a browser see components without their styles; the web target has no
  floor of its own.

## Amendment (2026-10-25): an Android web view without the message listener is refused at startup

The Android host bridge runs on the AndroidX WebKit web message listener
(`WEB_MESSAGE_LISTENER`), which answers only the app's own main frame. Before
this amendment, a System WebView without it loaded the pages with no bridge,
and every native call failed on the page with no explanation.

- The activity checks for the listener before it creates the web view. A
  missing listener, or a device with no System WebView, gets a native refusal
  page that names the installed web view package and version and asks for an
  update from Google Play. The app loads no page. This matches the Windows
  startup check for the WebView2 Runtime.
- There is still no `addJavascriptInterface` fallback. That interface answers
  every frame of the page, including embedded ones, so it would widen who can
  reach native services.
- Like the `@scope` and WebAssembly garbage collection needs, the check does
  not raise the API level, because Google Play updates the web view apart
  from Android.
- The generated activity is covered by a unit test. The run on an emulator
  image whose web view lacks the listener is still owed (GP2).
