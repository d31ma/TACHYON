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
   first runs `@scope`, so the 2.40 floor stays until one does. (Amended
   2026-10-25: 2.44 is the first, and the build notes it; see below.)

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
- The generated activity is covered by a unit test, and
  `scripts/android/web-view-refusal-test.mjs` ran it on 2026-10-25. An
  Android 8.0 (API 26) emulator with WebView 58.0.3029.125 showed the refusal
  page, recorded `bridge.refused` and mounted no page. An API 35 emulator with
  WebView 124 opened the page as before (GP2).

## Amendment (2026-10-25): the first WebKitGTK that runs `@scope` (GP1)

`scripts/linux/webkit-scope-probe.py` loaded a page with the `@scope` block
that Tac writes for a component, with a nested component and a sibling
paragraph, in each WebKitGTK release below (x86_64 Ubuntu 24.04 container,
Xvfb):

| WebKitGTK | Result |
| --- | --- |
| 2.40.0, 2.42.1, 2.42.4, 2.42.5 | the block is dropped (no rule in the stylesheet) |
| 2.44.0, 2.52.6 | the rule applies inside the component only |

2.42.5 is the last 2.42 release, so 2.44 is the first release that runs
`@scope`. Decision 4 changes: **Linux keeps the WebKitGTK 2.40 floor and
prints a note.** WebKitGTK comes with the Linux distribution, apart from the
app, as Safari does on macOS. A Linux build of an app with at least one
component stylesheet prints that component styles need WebKitGTK 2.44 or
newer. The packaged Linux app showed the same split: on 2.44.0 and 2.52.6 a
component `tac.css` rule colored its own paragraph and neither a nested
component's nor a sibling component's; on 2.40.0, 2.42.1 and 2.42.5 the page
ran with the component unstyled.

The same runs found that the Linux app did not start on WebKitGTK 2.40: it
linked `webkit_clipboard_permission_request_get_type`, which arrived in
2.42. The host now looks the clipboard permission type up by name at run
time, so the app starts on 2.40 and still grants the clipboard on 2.42 and
newer.

## Amendment (2026-10-25): the Android style run (MA3)

On the Android 15 emulator with System WebView 124.0.6367.219,
`scripts/android/page-checks-test.mjs` checked that a component `tac.css`
rule applies inside its component and not to the same element in another
component, and that `ty build --target android` prints the WebView 118 style
note for an app with component styles.
