# ADR 0109: System Back Goes Through the Page History

- Status: Accepted; Linux run passed 2026-10-24; macOS, iOS, Android and Windows runs owed
- Date: 2026-10-24
- Works with: [ADR 0009](0009-platform-navigation-over-a-client-renderer.md)
  (every route is its own document),
  [ADR 0097](0097-native-menu-actions-call-application-services.md)
  (the standard menus),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the per-target support table)

## Context

Every platform has a way to go back: the Android Back button and back
gesture, the iOS edge swipe, Command-[ and the trackpad swipe on macOS, and
Alt+Left and the mouse back button on Windows and Linux. In a browser tab,
the browser handles all of them. In a Tachyon app, no host did anything
with them. On Android, Back closed the app from any route, so the user lost
their place. On iOS and macOS, the web view does not turn its swipes on by
itself. On Linux, a run showed that WebKitGTK does nothing on Alt+Left or
on mouse buttons 8 and 9: the key reaches the page as `ArrowLeft` and the
button as a plain mouse event.

Tachyon routes are separate documents (ADR 0009), so the web view's own
history already holds the routes the user visited. Going back there loads
the previous route as a normal navigation, with the usual page load, page
services and route events.

## Decision

Each host sends System Back to the web view's history. No manifest member
turns it on or off, because no host needed one.

- **Android.** The app opts in to `android:enableOnBackInvokedCallback`.
  From Android 13, the activity registers an `OnBackInvokedCallback` only
  while the web view can go back, and updates it from
  `doUpdateVisitedHistory`. Back and the back gesture then go back one
  route. On the first route no callback is registered, so the system does
  what it does for any app, with the predictive back-to-home animation.
  Before Android 13, `onBackPressed` goes back in the web view when it can
  and otherwise calls the system's own handling.
- **iOS.** `allowsBackForwardNavigationGestures` turns on the web view's
  edge swipes: from the left edge to go back, from the right edge to go
  forward.
- **macOS.** The View menu starts with Back (Command-[) and Forward
  (Command-]), above Enter Full Screen. They go through the web view's
  history and are disabled when there is nowhere to go or the window is
  closed. The trackpad swipes are on too.
- **Linux.** The window catches Alt+Left, Alt+Right, the Back and Forward
  keys, and mouse buttons 8 and 9 before the web view, and goes back or
  forward in the web view's history when the page is one of the app's own.
- **Windows.** WebView2 already goes back and forward on Alt+Left,
  Alt+Right and the mouse back and forward buttons (its browser accelerator
  keys are on, and no declared menu shortcut can take Alt+Left), so the
  Windows host adds nothing. A run on a signed-in desktop proves it.

The hosts record `navigation.back` (and `navigation.forward` where the host
handles it) in the telemetry log. The support table has a "system back
navigation" row.

## Consequences

- An Android user who opened a second route returns to the first with Back,
  instead of leaving the app. Leaving still takes one more Back.
- A page that wanted Alt+Left for itself on Linux no longer receives it,
  as in a browser.
- Back is a page load, not a client-side transition, so state that must
  last after Back belongs in storage or a session field, as ADR 0009 says
  for any navigation.

## Evidence

- Cloud: `native_system_back_tests` checks the generated Android Java,
  Swift and C; `android::tests` checks the manifest attribute; the
  generated Android activity compiles with `javac` against `android.jar`
  (API 35) and AndroidX WebKit and Core.
- Linux, 2026-10-24: `linux_generated_host_goes_back_and_forward_through_page_history`
  (ignored, under Xvfb with xfwm4 and xdotool) passed in the x86_64 cloud
  container with GTK 4.14.5 and WebKitGTK 2.52.6. From `/help/`, Alt+Left,
  mouse button 8 and the Back key each returned to `/`; Alt+Right and mouse
  button 9 returned to `/help/`; Alt+Left on the first route stayed there.
  The host's route followed each step.
- Owed: `macos_view_menu_goes_back_and_forward_through_the_web_view_history`
  on a Mac, `scripts/ios/system-back-test.mjs` (XCUITest edge swipes) on the
  iOS simulator, `scripts/android/system-back-test.mjs` on the emulator, and
  `scripts/windows/system-back-test.mjs` on a signed-in Windows desktop.
