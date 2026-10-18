# ADR 0097: Native Menu Actions Call Application Services

- Status: Accepted
- Date: 2026-10-04
- Works with: [ADR 0089](0089-native-capabilities-are-activation-owned-service-providers.md)
  (activation-owned providers; the owned macOS window), ADR 0028 (owned
  application events), ADR 0030 (native services)

## Context

A macOS app that Tachyon builds has the standard menu bar, and
`tac.application.menus` adds File, View and Help items. Until now an item could
only name a `route`. Selecting it called `WKWebView.load`, so the page reloaded
and lost its state: inputs, selection, scroll position and the state of its
services. Apps such as a system monitor need Command-1 to Command-5 to change
the section of the page that is open, without a reload.

The host also had no Close Window item and no policy for the last window. A
window closed with the red button left the app running with no way to open the
window again, and the page services of the closed window were not disposed.

## Decision

### Menu actions

A menu item has exactly one of `route` or `action`. An action is
`{"service": "SectionMenuService", "method": "show_running", "event": "SectionChosen"}`:

- `service` is the type name of a client Rust Service marked
  `#[Lifetime({"value":"application"})]` (an application service). It lives as
  long as the app, so an action also works when no page is open.
- `method` is a method of that service that takes only `&self`. Its return
  value is ignored.
- `event` is optional. It names the Rust model in `client/models` that the
  method forwards to the page.

The build checks each action against the selected native companions and stops
with `TY1370` when the service is not a client Rust application service, the
method does not exist or takes more than `&self`, or the event is not a Rust
model. Shape errors (names, exactly one destination) keep `TY1601`.

The generated Rust library exports `tac_native_menu_action(id)` and
`tac_native_menu_ready(event)`. The action dispatch is generated from the
manifest inside the service module, so the host can only call declared
methods. It runs on the host thread, refuses while another native call is
running, and refuses after the application has retired.

### Forwarding to the current page

An application service forwards with the injected built-in provider
`CurrentPageEvents`. `publish_event(model)` delivers the model to every live
Rust page Service activation with an `OnEvent` listener for it, using the
ordinary listener path (model shape, condition, order, `InBackground`, return
events). A page singleton that several activations share receives one
delivery; each prototype instance receives one delivery. A listener cannot
publish to the page again while it receives the event (the nested call answers
`false`). `publish_event` answers `false` when nothing received the event.
Recipients are Rust page Services, because only they can inject an application
service; a JavaScript service can listen to an event that a Rust listener
returns. No page owner, publisher lease or disposed handler is retained.

### Enablement

One rule: an action item with an `event` is enabled only while the owned window
shows a page that has a live Rust page Service listener for that event. With no
such recipient (no window, another page such as Help, a page that is still
loading, or retired and replaced owners), the item is disabled; a stale
shortcut is refused with the system beep and the service is not called. An
action without an `event` belongs to the app and is enabled while the app runs,
also with no window. Route items keep their rule: the owned window and view
must still be attached.

### Close Window and the last window

File > Close Window (Command-W) sends `performClose:` to the owned window;
Command-W is now a reserved shortcut. When the owned window closes, the host
disposes that page's native companions and file-picker handles, drops the
bridge's view and window, and rebuilds the menu bar without a window: window
items and page actions are disabled, application actions stay enabled. The
bridge only answers messages from the current owned view.

Following macOS convention for apps such as Mail and Notes, closing the last
window does not quit the app (`applicationShouldTerminateAfterLastWindowClosed`
is false). Application services keep running. Reopening the app from the Dock
(`applicationShouldHandleReopen`) opens a new window at the entry route with a
new web view, new page services and new menu owners. Quit runs the termination
cleanup once (the host guards it, and the Rust application shutdown is
idempotent), whether a window is open or not.

## Consequences

- Route items keep working unchanged; no migration is needed.
- iOS and Android still refuse declared menu items at build time. Linux and
  Windows were added by the amendment below.
- The macOS host test
  `native_menu_host_tests::macos_menu_actions_reach_the_page_service_and_close_window_reopens_cleanly`
  compiles the generated AppDelegate with a real Rust library and checks: the
  action reaches the page service without a reload and keeps its state, the
  item is disabled with no recipient, Close Window disposes the page, reopen
  gives fresh owners, and Quit cleans up once.
- Real menu-bar clicks and keyboard shortcuts by a person, and JavaScript page
  services as direct recipients, are not covered by this decision.

## Amendment (2026-10-04): Linux and Windows hosts

The Linux host (GTK 4 and WebKitGTK 6.0) and the Windows host (Win32 and
WebView2) now accept `tac.application.menus` and a client Rust Service marked
`#[Lifetime({"value":"application"})]`. The build no longer refuses them for
`--target linux` and `--target windows`. iOS and Android still refuse both
(`TY1601` for menus, `TY1406` for the application lifetime): they have no menu
bar and no application lifetime in their hosts.

### Application lifetime

The contract is the same as on macOS. It is in the process only: no operating
system service, daemon, login item or autostart entry is created.

- Linux starts the application services once in the `GApplication` `startup`
  signal, before any window, and stops them once in `shutdown`, after the page
  companions are disposed. `SIGTERM` and `SIGINT` quit the application, so
  `shutdown` runs for them too.
- Windows starts the application services once before the window is created and
  stops them once after the message loop ends, after the page companions are
  disposed. A session end (`WM_ENDSESSION`) stops them too.
- If the start fails, the host records `application.start.failed` and exits
  without a window.
- Navigation (a new document) disposes page services only. The application
  services keep running.

### Menus

The host shows a menu bar only when the manifest declares at least one menu
item, so applications without menus keep their window unchanged.

- File has the declared File items, then Close Window (Ctrl+W) and Quit
  (Ctrl+Q; Exit on Windows). View and Help appear only when they have items.
- A shortcut maps `command` to Ctrl, `option` to Alt and `shift` to Shift. A
  shortcut that uses `control` gets no accelerator on Linux and Windows (Ctrl is
  already `command`); the item still appears and works from the menu.
- Linux uses a `GMenu` menu bar (`gtk_application_set_menubar`), one
  `GSimpleAction` per item and `gtk_application_set_accels_for_action`. GTK has
  no menu-validation callback, so the host updates each action's enabled state
  after every page call, every document load and every menu action, and checks
  it again when the action runs.
- Windows uses an `HMENU` menu bar and an accelerator table. The host updates
  the enabled state in `WM_INITMENUPOPUP` and checks it again in `WM_COMMAND`.
  Keys pressed while the web view has focus arrive through WebView2's
  `AcceleratorKeyPressed` event and are sent as the same menu commands.
- Enablement follows the macOS rule: an action with an `event` needs a live Rust
  page listener in the current local page; an action without an `event` is
  enabled while the application runs; a route item needs the window and its web
  view. A refused item beeps and does not call the service.

### The last window

Linux and Windows follow their own convention: closing the last window quits
the application, so Close Window and Quit both end the process and stop the
application services once. There is no reopen step as on macOS.

### Evidence

- Linux: `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime`
  builds the generated host with GTK 4 and WebKitGTK 6.0 and a real Rust
  library, then runs it (under Xvfb on WSL Ubuntu 24.04 for this amendment):
  the service starts once at launch, a menu action reaches
  the page service without a reload, the item is disabled with no recipient,
  Help and Home keep the service, and Close Window stops it once.
- Windows: generated-source tests, and the Win32 menu code compiled and linked
  with MinGW-w64 GCC. The MSVC build, the WebView2 accelerator handler and a run
  on Windows are not yet verified (see `docs/SUPPORT_TIERS.md`).

## Amendment (2026-10-04): Windows architecture, page calls and paths

### Windows architecture

`ty bundle --target windows` builds for the architecture of the build
machine by default (ARM64 on Windows on ARM, x64 otherwise). `--arch x64`
or `--arch arm64` (aliases `amd64`/`x86_64`, `aarch64`; `native` is the
default) chooses one; `--arch` without a Windows target is refused. The
choice sets all four architecture values together:

| Value | x64 | ARM64 |
| --- | --- | --- |
| Rust companion target | `x86_64-pc-windows-msvc` | `aarch64-pc-windows-msvc` |
| WebView2 loader folder | `build/native/x64` | `build/native/arm64` |
| C# runtime identifier | `win-x64` | `win-arm64` |
| Linker machine | `/MACHINE:X64` | `/MACHINE:ARM64` |

`cl` comes from the developer prompt. When the prompt's
`VSCMD_ARG_TGT_ARCH` names another architecture, the build stops and names
the `vcvarsall.bat` argument to use. The artifact manifest records
`x86_64` or `aarch64`.

### Page calls

A page reaches its native companion through `companion.invoke`. The
contract is the same on the macOS, Linux and Windows hosts:

- The page sends one JSON request and waits for one JSON reply (a promise).
  The host runs the companion call synchronously on its user interface
  thread: the main actor on macOS (`WKScriptMessageHandlerWithReply`), the
  GTK main loop on Linux (`script-message-with-reply-received`), and the
  window's thread on Windows (`WebMessageReceived`; the page side sends one
  call at a time, with a 10 second timeout). A slow method blocks the
  window, so long work belongs in a background task.
- The operations are `init`, `get`, `set`, `call` and `dispose`, for the
  route of the current document only.
- A request is at most 64 KiB. On Linux and Windows a reply is at most
  64 KiB too; a larger reply is answered with an error that says to send
  smaller pages (before, Linux and Windows answered "Invalid native
  response."). A request that becomes larger than 64 KiB when Windows
  converts it to UTF-8 is now answered with an error instead of timing out.
  The macOS host does not limit the reply, but the page does (see the
  2026-10-18 amendment).
- Arguments are positional. A model argument is a JSON object whose keys
  are the Rust field names, the same on every host, because every desktop
  builds the same Rust companion code.
- Larger data goes in pages. An app keeps the data in its service and
  returns it page by page: with at most 24 KiB of text per page, a reply
  stays under 64 KiB even when JSON escaping doubles every byte.
  `linux_page_call_tests` checks this through the real WebKitGTK bridge.

### Paths

Tachyon imposes no sandbox root on Linux or Windows and plans no MSIX or
Flatpak packaging. The app process has the user's normal rights, and an app
chooses its own per-user paths, for example `%LOCALAPPDATA%\<app>` on
Windows and `$XDG_RUNTIME_DIR` or `$XDG_STATE_HOME` on Linux. The hosts
write their event log to `%LOCALAPPDATA%\Tachyon` and
`$XDG_STATE_HOME/tachyon`. The Linux web view uses an ephemeral network
session. The Windows host passes no user data folder to WebView2, so
WebView2 keeps its data in its default folder beside the executable
(`<App>.exe.WebView2`); an app installed in a folder the user cannot write
needs a per-user data folder, which is not part of this amendment.

## Amendment (2026-10-18): Standard menus and Ctrl shortcuts on Linux and Windows

The Linux and Windows menu bars now match the macOS one where the platforms
allow it.

### Always a menu bar

The host shows a menu bar even when the manifest declares no menu items.
This replaces the rule of the 2026-10-04 amendment that showed it only when
at least one item was declared. The bar has, in order:

- File: the declared File items, then Close Window (Ctrl+W) and Quit
  (Ctrl+Q; Exit on Windows), as before.
- Edit: Undo, Redo, Cut, Copy, Paste and Select All. Each item sends the
  editing command to the web view, so it acts on the focused field of the
  page. Linux calls `webkit_web_view_execute_editing_command`. Windows moves
  the focus to the web view and sends the command through the DevTools
  protocol (`Input.dispatchKeyEvent` with `commands`), because WebView2 has
  no editing-command call. Redo is Ctrl+Shift+Z on Linux and Ctrl+Y on
  Windows, each platform's convention.
- View: the declared View items, when there are any.
- Window: Minimize, and Maximize, which restores a maximized window (the
  macOS item is Zoom). Neither has a shortcut, because neither platform has
  a Ctrl shortcut for them.
- Help: the declared Help items, when there are any.

On Linux the Edit shortcuts are application accelerators, as in other GTK
apps. On Windows the Edit items show their keys, but the keys are left to
WebView2, which already handles them in the page. An Edit item runs while a
web view exists; otherwise it beeps.

### `control` is Ctrl

A shortcut that uses `control` now gets an accelerator on Linux and Windows:
`control` is Ctrl there, the same key as `command`. On macOS `control` stays
Control. Because two shortcuts that differ only by `control` would get the
same key, `--target linux` and `--target windows` refuse them with `TY1601`.
They also refuse a shortcut that becomes a standard one there (for example
Command-Control-Q becomes Ctrl+Q, which is Quit), and on Windows a shortcut
that becomes Ctrl+Y (Redo). macOS builds keep the rules they had.

### Evidence

- Linux: `linux_host_tests::linux_generated_host_runs_menu_actions_and_one_application_lifetime`
  now also checks the five menus, the Copy and Redo accelerators, that the
  Edit and Window items are enabled, and that Edit > Select All selects the
  page text through the real WebKitGTK web view.
- Windows: generated-source tests and the MinGW-w64 compile of the menu code.
  The Edit commands inside WebView2 and Minimize/Maximize on a desktop still
  need the interactive Windows run (with the other WebView2 checks in
  `docs/SUPPORT_TIERS.md`).

## Amendment (2026-10-18): The 64 KiB reply limit applies on macOS too

The 2026-10-04 amendment said that macOS does not limit the reply. That was
wrong. The macOS host itself sends any reply, but the page runtime checks
every reply on every host. `nativeCall` in `tac-client.js` rejects a text
reply that is longer than 64 KiB with "Tac native response exceeds 64 KiB."
The macOS host always replies with text, so the limit applies there too.

- A reply is at most 64 KiB on macOS, Linux and Windows.
- A request is at most 64 KiB, except the start-up request (`init`). The
  start-up request may be up to 1 MiB.
- The advice stays the same: return large data page by page.
