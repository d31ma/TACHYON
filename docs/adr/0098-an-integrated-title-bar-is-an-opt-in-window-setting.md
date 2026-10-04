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
- Windows and Linux still show the standard title bar.
