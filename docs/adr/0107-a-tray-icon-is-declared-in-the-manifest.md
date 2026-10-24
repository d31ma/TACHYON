# ADR 0107: A Tray Icon Is Declared in the Manifest

- Status: Accepted; macOS desktop run passed 2026-10-19; Windows desktop run passed 2026-10-24
- Date: 2026-10-19
- Works with: [ADR 0097](0097-native-menu-actions-call-application-services.md)
  (menu items call application services),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the per-target support table)

## Context

Desktop apps often live in the system tray: the macOS menu bar extras at the
top right, the Windows notification area, and the Linux StatusNotifierItem
area. Tachyon hosts once had page calls named `tray.set` and `tray.remove`.
They were removed, together with `window.show`, `window.hide` and `app.quit`,
because a fixed list of page verbs grows forever and limits what an
application can do. The docs then said that an application service should
create its own tray through the platform SDK.

No application ever did that, and it is hard to do well. A tray needs three
different implementations (AppKit, Win32 and D-Bus), a packaged icon, and a
way to hide the window and bring it back, which the hosts no longer offered.
A developer who wants a tray icon should not have to write three host
integrations.

## Decision

1. **One declarative member.** `tac.application.tray` declares the icon, its
   menu and how the window behaves:

   ```json
   "tray": {
     "icon": "/shared/assets/tray.png",
     "tooltip": "System Monitor",
     "template": true,
     "items": [
       {"id": "dashboard", "label": "Dashboard", "route": "/"},
       {"id": "pause", "label": "Pause sampling", "action": {"service": "SamplerService", "method": "pause"}}
     ],
     "closeHidesWindow": true,
     "dock": false
   }
   ```

   - `icon` names a PNG file in the client bundle. The build stops with
     `TY1601` when the file is missing from the built bundle, is not a PNG,
     is larger than 1 MiB, or is not 16 to 1024 pixels on each edge.
   - `tooltip` is 1 to 127 UTF-8 bytes; it defaults to the app name.
   - `template` makes the macOS menu bar tint the icon for light and dark mode.
   - `items` uses the menu item shape of ADR 0097: an `id`, a `label` and
     exactly one of `route` or `action`. A tray item has no `shortcut`. At
     most 32 items, and the ids must differ from the File, View and Help item
     ids, because one generated dispatch serves menus and tray. Actions are
     checked like menu actions (`TY1370`).
   - `closeHidesWindow` makes closing the window hide it. The page and its
     services keep running until the user quits from the tray.
   - `dock: false` runs a macOS app from the menu bar only (`LSUIElement`).
2. **The host adds Open and Quit.** Every tray menu starts with
   "Open <app name>" and ends with "Quit <app name>" ("Exit" on Windows), so
   the window can always come back and the app can always stop. A route item
   opens the window and then its page. An action item calls the application
   service, and an action with an `event` is enabled only while the page
   listens for it, as in ADR 0097.
3. **Reach per target.**
   - **macOS:** an `NSStatusItem` with an `NSMenu`. `dock: false` sets
     `LSUIElement` and the `.accessory` activation policy. With
     `closeHidesWindow` the window is ordered out instead of closed.
   - **Windows:** `Shell_NotifyIconW` with `NOTIFYICON_VERSION_4`. A click
     shows the window and the context menu shows the tray menu. The build
     wraps the PNG as `resources/tray.ico`. The icon comes back when Explorer
     restarts (`TaskbarCreated`). `dock` is adapted with a note.
   - **Linux:** a StatusNotifierItem and a `com.canonical.dbusmenu` menu,
     exported on the application's session bus connection with GDBus. GTK 4
     has no tray widget and libayatana-appindicator needs GTK 3, so the host
     speaks the protocol itself. The icon is sent as `IconPixmap`. The entry is
     **adapted**: a desktop without a StatusNotifierWatcher (GNOME without the
     AppIndicator extension) shows nothing, so `closeHidesWindow` hides the
     window only while a watcher accepted the item, and the window comes back
     if the watcher goes away.
   - **iOS and Android:** refused with `TY1601`. Neither has a status area.
   - **Web:** not applicable, like the rest of `tac.application`.
4. **No page verbs come back.** The page still cannot call `tray.set`,
   `window.show`, `window.hide` or `app.quit`, and the host test that forbids
   them stays. The tray is configuration that the host builds, like menus.

## Consequences

- A developer gets a tray icon on three desktop platforms from one manifest
  member, with the same items and actions as the menu bar.
- Changing the icon or tooltip at run time is not part of this decision. A
  later decision can add an application-service provider for it, as
  `CurrentPageEvents` does for menus.
- The Linux tray depends on the desktop. The build says so with a note.
- The macOS host was run on a signed-in Mac desktop on 2026-10-19: the status
  item and its menu, no Dock icon with `dock: false`, a route item, an action
  item, closing to hide and reopening the same window, Quit, the colored icon
  with `template: false`, and the Dock icon and ordinary close with
  `dock: true` and `closeHidesWindow: false`. The Windows host was checked in
  the cloud only by source and unit tests and under Wine; its desktop run is
  owed. The Linux host was run against a test StatusNotifierWatcher on a
  private session bus.

## Amendment (2026-10-24): the Windows desktop run (SD1)

`scripts/windows/menu-tray-test.ps1` ran in the signed-in desktop session of a
Windows 11 ARM64 machine, started from SSH by `scripts/windows/desktop-run.ps1`.
The tray icon appeared; its menu held Open, the declared items and Exit; a
declared item ran the application service and its event reached the page;
closing the window hid it, `NIN_SELECT` showed it again, and Exit ended the
app with code 0 and removed the icon. UI Automation shows the tray menu
empty, so the gate reads the popup menu through `MN_GETHMENU`,
`GetMenuItemRect` and `GetMenuState` and clicks the item rectangle. The run
also found that the page did not show a tray or menu action's event until
its next call; the Rust companion now signals the listening page owners
(ADR 0097).
