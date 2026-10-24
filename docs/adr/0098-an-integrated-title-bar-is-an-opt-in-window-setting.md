# ADR 0098: An Integrated Title Bar Is an Opt-In Window Setting

- Status: Accepted
- Date: 2026-10-04
- Works with: [ADR 0089](0089-native-capabilities-are-activation-owned-service-providers.md)
  (the owned macOS window), [ADR 0087](0087-services-reach-the-window-and-the-network-through-injection.md)
  (`WindowControls`), [ADR 0097](0097-native-menu-actions-call-application-services.md)
  (the macOS host and its window lifecycle)

## Context

A macOS app that Tachyon builds shows the page in a standard window: a grey
title bar with the window buttons and the app name, and the page below it.
Apps such as Xcode, Notes and Music instead show one continuous surface: their
own content reaches the top edge of the window, and the window buttons sit on
that content. Tachyon apps such as the Swarx observatory want the same look.

Hiding the title text alone is not enough. The grey bar stays, and a page that
calls `WindowControls.setTitle('')` to hide the text also removes the window
name that VoiceOver, Mission Control and the Window menu read.

When the page reaches the top edge, the page also covers the area that a person
uses to move the window. A probe on macOS 26 showed that a `WKWebView` under a
transparent title bar receives every click in the title bar strip, so nothing
moves the window unless the host adds a way to do it.

## Decision

### Manifest

`tac.application.window` accepts two optional keys:

```json
"window": {
  "titleBar": "integrated",
  "titleBarButtonInset": { "leading": 20, "top": 35 }
}
```

- `titleBar` is `"standard"` (the default) or `"integrated"`. Any other value
  stops the build with TY1601.
- `titleBarButtonInset` moves the window buttons. `leading` is the distance in
  points from the leading window edge to the close button; `top` is the
  distance from the top edge to the buttons. Each is 0 to 200. It needs
  `"titleBar": "integrated"`; otherwise the build stops with TY1601.

The setting is a window setting, not a page grant: it changes how the host
draws its own window and gives the page no new capability.

### macOS

With `"integrated"`, the host creates the window with
`.fullSizeContentView`, a transparent title bar
(`titlebarAppearsTransparent`), a hidden title (`titleVisibility = .hidden`)
and no title bar separator. The web view fills the whole window, so the page
background is also the background of the title bar area in light and dark
mode; there is no strip of another color. The page draws its own colors with
`prefers-color-scheme` or `light-dark()`. No `NSVisualEffectView` material is
added.

The window keeps the real close, minimize and zoom (full screen) buttons with
their own behavior and accessibility. The window stays titled, closable,
minimizable and resizable, and the keyboard window shortcuts do not change.

With `titleBarButtonInset`, the host moves the three buttons to the inset and
makes the title bar container tall enough to hold them (`top`, the button
height, and `top` again). It places them again after every resize, appearance
change, key change and full screen exit, because AppKit lays the buttons out
again at those moments.

### Insets for the page

The host sets three CSS custom properties on the page root with a constructed
style sheet (so it works before the document exists and under a strict content
security policy):

| Property | Meaning |
| --- | --- |
| `--tac-titlebar-height` | Height of the title bar area: the larger of the system title bar and the buttons with their top inset above and below. |
| `--tac-titlebar-leading-inset` | Space from the leading edge to the end of the buttons, plus the buttons' own leading margin. |
| `--tac-titlebar-trailing-inset` | Space the window needs at the trailing edge. Always `0px` on macOS; on Windows it would hold the caption buttons. |

In full screen all three are `0px`, because macOS hides the title bar and
shows it again over the content when the pointer reaches the top of the
screen, as in other native apps. The host updates the values on resize, on
entering and leaving full screen, on a backing scale or screen change and on
an appearance change. It also stores the current values in the document-start
script, so a page that opens in full screen starts with the right values.

A browser or a standard window never sets the properties, so pages write them
with a fallback, and keep their own margin for full screen:
`padding-inline-start: max(16px, var(--tac-titlebar-leading-inset, 0px))`.

### Drag regions

A page marks the areas that move the window with the `data-tac-drag-region`
attribute. Inside a drag region, interactive controls are not drag areas: links,
buttons, form fields, `summary`, `label`, `iframe`, media with controls,
editable content, focusable elements, elements with an interactive ARIA role,
and any element marked `data-tac-no-drag`.

The page script that the host injects measures these elements after layout
changes (load, resize, scroll, DOM changes; at most once per animation frame)
and sends the rectangles to the host with the `window.dragRegions` host call.
The host accepts at most 256 drag and 256 excluded rectangles of four bounded
numbers; a page with more sends none, so no control is ever covered.

The host hit-tests natively. Its web view class overrides `mouseDown`: when the
point is inside a drag rectangle and outside every excluded rectangle, the host
calls `NSWindow.performDrag(with:)`, so the window server moves the window with
snapping, tiling and Spaces like a standard title bar. A double-click follows
the person's title bar setting in System Settings (the global
`AppleActionOnDoubleClick` default): Minimize minimizes the window, Do Nothing
does nothing, and Zoom or Fill zoom the window. AppKit has no public call for
Fill; zooming a resizable window fills the visible screen area, which is what
Fill shows, and a second double-click restores the frame. Handing the event to
AppKit's private title bar view was tried and rejected: it works only when the
click is not the current event. `acceptsFirstMouse` is true in a drag region,
so an inactive window can be dragged at once. Every other click goes to the
page unchanged.

### Accessibility

The window title stays set (the app name, or the title that a page sets with
`WindowControls.setTitle`). It is only hidden visually, so VoiceOver, Mission
Control and the Window menu still name the window. With an integrated title
bar, an empty title from the page keeps the app name, because an empty window
name helps no one. The window buttons are the system buttons, with their own
labels and actions.

### Windows and Linux

These hosts accept the key and keep the standard title bar. `ty build
--target windows|linux|ios|android` prints a note that the integrated title bar
is built only for macOS. The equivalents, for a later decision:

- Windows (WebView2): `DwmExtendFrameIntoClientArea` with a custom `WM_NCCALCSIZE`
  and `WM_NCHITTEST` (or WinUI `ExtendsContentIntoTitleBar` with
  `SetTitleBar`), the system caption buttons kept by the window frame, and the
  drag rectangles answered as `HTCAPTION` in `WM_NCHITTEST`. The caption
  buttons sit at the trailing edge, so `--tac-titlebar-trailing-inset` would
  hold their width.
- Linux (GTK): client-side decorations with a `GtkHeaderBar` set through
  `gtk_window_set_titlebar`, the web view inside the header area, and
  `gtk_window_begin_move_drag` for the drag rectangles. On Wayland the
  compositor chooses server- or client-side decorations through
  `xdg-decoration`; where it insists on server-side decorations the standard
  title bar stays.

### Adopting it (Swarx)

1. Add `"titleBar": "integrated"` to `tac.application.window`. To center the
   buttons in an 84-point bar, add `"titleBarButtonInset": { "leading": 20, "top": 35 }`.
2. Delete the `WindowControls.setTitle('')` call in
   `ObservatoryService.initialize()` (and the `title` control grant if nothing
   else uses it). The title text is hidden by the host and the window keeps its
   accessible name.
3. Mark the top bar with `data-tac-drag-region` and give it
   `padding-inline-start: max(16px, var(--tac-titlebar-leading-inset, 0px))` and
   `min-height: max(84px, var(--tac-titlebar-height, 0px))`. The bar has no
   interactive controls, so all of it moves the window; everything below it
   keeps its clicks.

## Consequences

- Existing apps keep the standard title bar; no migration is needed.
- A page that uses an integrated title bar must leave room for the buttons
  with the inset properties and declare where the window can be dragged.
  Without a drag region, a person cannot move the window with the pointer.
- The macOS host test
  `native_title_bar_host_tests::macos_integrated_title_bar_keeps_buttons_insets_drag_regions_and_full_screen`
  builds the generated AppDelegate with the system SDK, with and without a
  button inset, and checks the style mask, the buttons (present, enabled, not
  covered, minimize and close work, the zoom button enters full screen), the
  accessible title, the insets in light mode, dark mode, after a resize, in
  full screen (also after a navigation) and after leaving it, that the title
  bar strip has the page's color in screenshots, that a drag region starts a
  window drag while a button inside it and the content below receive their
  clicks, that a double-click follows the zoom setting, and that the page
  follows a resize.
- Synthetic events cannot drive a window server drag (and posting real
  pointer events needs an Accessibility grant), so the test checks that the
  host starts `performDrag` exactly for clicks in a drag region. The window
  movement itself is AppKit's documented `performDrag(with:)` behavior.
- Windows and Linux still show the standard title bar. (Linux and Windows:
  see the 2026-10-19 amendments.)

## Amendment (2026-10-19): Linux

The Linux host (GTK 4 and WebKitGTK 6.0) now builds `"titleBar": "integrated"`
with the same page contract as macOS.

- **Window.** The window uses GTK client-side decorations with a hidden,
  empty title bar widget (`gtk_window_set_titlebar`), so neither GTK nor the
  window manager draws a title bar, and the web view fills the window from
  its top edge. The window title stays set, so the window keeps its name for
  screen readers, the task switcher and the window manager. As on macOS, an
  empty title from `WindowControls.setTitle` keeps the app name.
- **Window buttons.** GTK's own window buttons (`GtkWindowControls`, the
  widget a `GtkHeaderBar` draws them with) sit in a `GtkOverlay` over the
  page, one group at the leading edge and one at the trailing edge. They
  follow the desktop's button layout (`gtk-decoration-layout`), so the
  desktop decides on which side the close button is. A full `GtkHeaderBar`
  over the page was rejected: it would take every click in the strip, while
  the contract gives those clicks to the page.
- **Menus.** The menu bar would cover the top edge, so it is hidden and its
  menus move into a main menu button (`open-menu-symbolic`, labeled "Main
  Menu") before the trailing window buttons, as GNOME apps do. Keyboard
  shortcuts do not change.
- **Insets.** `--tac-titlebar-height` is the button groups' height with their
  top inset above and below. `--tac-titlebar-leading-inset` and
  `--tac-titlebar-trailing-inset` are each group's width plus its distance
  from its window edge, twice; a side without buttons is `0px`. On Linux the
  trailing inset is usually the larger one. In full screen the buttons are
  hidden and all three are `0px`. The host measures the groups one frame
  after the window is shown, after a full screen or maximize change, and
  after the button layout changes, and stores the values in the
  document-start script, so a new page starts with them.
- **`titleBarButtonInset`.** `top` is the distance from the top edge to both
  groups, and `leading` the distance from each group's own window edge,
  because the close button is not always at the leading edge. Without it,
  both are 6 pixels.
- **Drag regions.** The page script is the macOS one and sends the same
  `window.dragRegions` call with the same limits. A click gesture and a drag
  gesture in the capture phase of the web view hit-test a primary-button
  press in page coordinates (divided by the zoom level). A press inside a
  drag region and outside every excluded rectangle is claimed, so the page
  never sees it; every other press goes to the page unchanged. Once the
  pointer moves past the drag threshold, the host calls
  `gdk_toplevel_begin_move`, so the window manager moves the window as from a
  title bar. A double-click follows `gtk-titlebar-double-click`: toggle
  maximize (the default), minimize, lower, the window menu, or nothing.
- **`theme_color`.** With an integrated title bar there is no header bar to
  color; the page colors the top edge.
- **Wayland.** GTK 4 always draws client-side decorations on Wayland, so the
  same code applies there, but it has only run on X11.

Test: `linux_title_bar_tests::linux_generated_host_integrates_the_title_bar_with_insets_and_drag_regions`
compiles the generated host and runs it under Xvfb with xfwm4. It sends real
X11 pointer events through XTest and checks the hidden title bar and menu
bar, the main menu button and its menus, that the page starts at the top
edge, that the page gets the host's insets (also after a button layout
change and after a navigation), that a press on a button inside a drag
region and a press below it reach the page while a press in the drag region
does not, that dragging the region moves the window by the pointer's
distance, that a double-click maximizes and a second one restores, that full
screen hides the buttons and zeroes the insets, and that an empty page title
keeps the app name.

## Amendment (2026-10-19): Windows

The Windows host (Win32 and WebView2) now builds `"titleBar": "integrated"`
with the same page contract as macOS and Linux.

- **Window.** The window keeps `WS_OVERLAPPEDWINDOW`, so it stays a normal
  window for Snap, the taskbar and the keyboard. The host answers
  `WM_NCCALCSIZE` with the default frame on the sides and the bottom and no
  caption at the top, so the client area starts at the top edge of the
  window, and extends the DWM frame into the client area by the caption
  height (`DwmExtendFrameIntoClientArea`). A maximized window keeps the
  frame thickness at the top, so the page stays on the screen. The window
  opens with the manifest size as its client area, without a caption. The
  window title stays set, so the taskbar, Alt+Tab and screen readers keep
  the window's name; an empty title from `WindowControls.setTitle` keeps the
  app name, as on macOS and Linux.
- **Caption buttons.** The minimize, maximize and close buttons are the
  system's, drawn by DWM in the extended frame and hit-tested with
  `DwmDefWindowProc`, which answers `HTMAXBUTTON` for the maximize button,
  the answer Windows 11 needs to offer Snap Layouts. The web view lives in its own child window whose region leaves out
  the caption button rectangle (`DWMWA_CAPTION_BUTTON_BOUNDS`), so the
  buttons are never covered and their clicks reach the frame. Windows
  cannot move them, so `titleBarButtonInset` is ignored, and the build says
  so in a note.
- **Menus.** The menu bar would sit under a caption that no longer exists,
  so its menus move into the window menu (Alt+Space, or a right-click on a
  drag region), after a separator. Their keyboard shortcuts and enabled
  states do not change.
- **Insets.** `--tac-titlebar-height` is the bottom of the caption buttons
  and `--tac-titlebar-trailing-inset` their width from the trailing edge,
  in CSS pixels at the window's DPI. `--tac-titlebar-leading-inset` is
  `0px`, because Windows has no buttons at the leading edge. In full screen
  the window has no frame and all three are `0px`. A new page asks the host
  for the values with the `window.titleBarInsets` host call as it starts,
  and the host sends new values after a resize, a DPI or theme change,
  activation, and entering or leaving full screen.
- **Drag regions.** The page script is the macOS one. On Windows it does
  not send `window.dragRegions`: it turns `data-tac-drag-region` into the
  CSS `app-region: drag` and every control inside it (the same list as on
  macOS) into `app-region: no-drag`, and the host turns on WebView2's
  non-client region support (`ICoreWebView2Settings9`,
  `IsNonClientRegionSupportEnabled`). WebView2 then moves the window from a
  drag region, maximizes or restores it on a double-click, and opens the
  window menu on a right-click. A WebView2 Runtime older than 123 lacks the
  setting; the host then records `window.dragRegions.unavailable` and the
  regions do nothing.
- **Resizing.** The page covers the top edge, so the window resizes from its
  sides, its bottom and their corners. The host answers `HTTOP` for the
  resize border at the top only where no web view covers it.
- **`theme_color`.** It still sets the DWM caption color. With an integrated
  title bar the page colors the top edge; the caption color can show only
  where DWM draws the frame, which may include the area behind the caption
  buttons.

Tests: `windows_title_bar_tests` checks the generated host for both title
bars; `the_windows_host_with_each_title_bar_compiles_against_the_win32_and_webview2_headers`
compiles the whole generated host with MinGW-w64 against the Win32 and
WebView2 SDK headers; and `the_windows_title_bar_drives_a_win32_window_under_wine`
runs the frame, hit test, menu move, insets, button region, maximize and
full screen code on a real Win32 window under Wine (which has no DWM, so it
uses the fallback caption button size). The look, Snap Layouts and a drag
in WebView2 need the signed-in Surface desktop (SD1). On 2026-10-24
`scripts/windows/window-shell-test.ps1` ran them there: the client area
started at the window's top edge, `WM_NCHITTEST` answered the three caption
buttons and no web view covered them, the page read a 30px title bar height
and 0px and 145px insets at 96 DPI, a real click in the drag region reached
the page, a `SendInput` drag moved the window by (120, 60), a double-click
maximized and restored it, and a right-click opened the window menu with the
File, Edit and Window menus. Hovering the maximize button showed the Windows
11 Snap Layouts flyout. UI Automation shows no TitleBar element for the
integrated window.

## Amendment (2026-10-19): iOS and Android

`titleBar: "integrated"` now has a meaning on phones and tablets: the page
draws edge to edge, under the status bar. A mobile app has no title bar,
window buttons or window to drag, so only the three CSS properties carry
over.

- **iOS.** The web view covers the whole screen and its scroll view stops
  adjusting its insets (`contentInsetAdjustmentBehavior = .never`). With the
  standard title bar the web view starts at the top safe-area edge, and the
  area behind the status bar shows `theme_color`, or else
  `background_color`, or else the system background.
- **Android.** From API 30 the activity draws behind the system bars
  (`setDecorFitsSystemWindows(false)`, which API 35 enforces anyway); before
  API 30 it does so only under an integrated title bar
  (`SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN`). The root view pads the page by the
  system-bar and display-cutout insets, except at the top when the title bar
  is integrated. The navigation bar keeps its padding, so the page needs no
  bottom inset.
- **Properties.** `--tac-titlebar-height` is the top inset in CSS pixels,
  and `--tac-titlebar-leading-inset` and `--tac-titlebar-trailing-inset` are
  the side insets, such as the camera cutout in landscape, in the page's
  reading direction. A small page script (`tac-mobile-title-bar.js`) asks
  the host for them with `window.titleBarInsets` as the page starts, and the
  host sends new values when the safe area or the insets change. A standard
  title bar sets none of the three.
- **Drag regions.** `data-tac-drag-region` does nothing on mobile.

Tests: `ios_window_settings` and `android_window_settings` check the
generated sources. `scripts/ios/window-settings-test.mjs` (run by
`scripts/ios/native-test.mjs`) and `scripts/android/window-settings-test.mjs`
(run by `scripts/android/native-test.sh`) build apps with each title bar,
read the properties from the running page and compare a screenshot of the
status bar with the page's own bar.
