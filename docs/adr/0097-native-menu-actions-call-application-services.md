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
After the delivery, the companion sends each page owner that listens for the
event a `{"route", "service", "owner", "changed": true}` signal through the
host's publish path, and the page reads that owner's fields again and
renders. Without it the page kept its old field values, because the change
did not start from a page call.

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
  On 2026-10-24 (SD1) `scripts/windows/window-shell-test.ps1` and
  `scripts/windows/menu-tray-test.ps1` ran the Edit commands inside WebView2
  and Window Minimize and Maximize on the signed-in desktop of a Windows 11
  ARM64 machine.

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

## Amendment (2026-10-18): The Linux menu bar is shown and named

GTK 4 changed the default of `GtkApplicationWindow:show-menubar` to false.
The Linux host installed its menu bar on the application, but the window
never drew it. The menu shortcuts worked, but nobody could see the menus,
open them with F10, or find them with a screen reader. The Linux host now
sets `show-menubar` to true on its window.

The items inside the menus also had no accessible name. GTK 4.14 and 4.18
name each popover menu item after a label that they leave out of the
accessibility tree, so AT-SPI (the Linux accessibility bus) reported every
item, for example Close Window and Quit, with an empty name. When a menu
opens, the Linux host now gives each item its visible text, without the
mnemonic underscore, as its accessible label.

### Evidence

- `linux_host_check_fixture.c` checks that the generated window shows its
  menu bar.
- `scripts/linux/a11y-probe.py`, run by `scripts/linux/native-test.sh`,
  finds the File, Edit and Window menu items over AT-SPI, opens the File
  menu with F10, and finds Close Window and Quit by role and name. It passed
  on 2026-10-18 under Xvfb with GTK 4.14.5 and WebKitGTK 2.52.6 (Ubuntu
  24.04), and in the pinned `scripts/linux/Dockerfile` image with GTK 4.18.6
  and WebKitGTK 2.54.0 (Debian trixie).

## Amendment (2026-10-18): `--arch` chooses the macOS architecture too

`--arch` is no longer a Windows-only switch. Each target either honors it,
says in one `Note:` line that it does not, or the build refuses it:

| Target | `--arch` |
| --- | --- |
| Windows | `x64` or `arm64`, as above. `universal` gets a note and builds for the build machine. |
| macOS | `x64`, `arm64` or `universal`. `universal` builds an `arm64` and an `x86_64` app (Swift `-target <arch>-apple-macosx<floor>`, where the floor is `tac.application.minimumSystem.macos` or the one ADR 0033 chooses, Rust `aarch64-apple-darwin` and `x86_64-apple-darwin`) and joins the app and each Rust helper executable with `lipo`. The artifact manifest names `universal`. |
| Linux | Built for the build machine. A matching `--arch` (or `native`) is honored; any other gets a note. |
| iOS | The simulator app is built for the build machine, as Linux. `--device` and `--release` builds are always `arm64` (ADR 0104 amendment). |
| Android | No `--arch`: the APK holds no native code yet, so it runs on every ABI. ABIs follow linked Rust on Android (MR2). |
| Web | No `--arch`: the web build holds no native code. |

When none of the targets being built honors the choice, the build stops
with `TY1001` and names what each target takes. A build for the build
machine's own architecture runs the same commands as before; only another
architecture, or `universal`, passes `-target` and `--target`. A Rust
companion for another architecture needs its standard library (`rustup
target add x86_64-apple-darwin` on Apple silicon); without it the build stops
with `TY1605` and that command. The notes come from the
`build architecture (--arch)` entry of the per-target support table
(ADR 0102).

### Evidence

- Cloud: `native_architecture_cli` stages the macOS host for `universal`,
  `x64` and `arm64` and reads the architecture back from the artifact
  manifest, checks the `TY1001` refusals, and checks the notes for Windows
  and Android in a three-target build.
- Mac: `native_rust_crate_tests::the_universal_macos_bundle_holds_both_architectures_and_its_helper_runs`
  (ignored; run on the Mac lane) builds, joins and signs the app and its Rust
  helper, checks `lipo -archs` for both architectures, runs the helper and
  verifies the signature. It has not run yet.

## Amendment (2026-10-19): Windows paths inside an MSIX package

ADR 0104 adds an MSIX package for Windows release builds, so the "Paths"
section's "plans no MSIX" no longer holds for Windows. Inside the package the
install folder is read-only. The host then gives WebView2 the package's
`LocalState\WebView2` folder as its user data folder. The event log stays
in `%LOCALAPPDATA%\Tachyon`, which Windows redirects into the package's own
data. An unpackaged Windows app is unchanged.

## Amendment (2026-10-18): Linux web data

The Linux web view no longer uses an ephemeral network session. It keeps its
web data in `$XDG_DATA_HOME/<application id>/webkit` and its HTTP cache in
`$XDG_CACHE_HOME/<application id>/webkit` (ADR 0059 amendment of the same
date). The event log stays in `$XDG_STATE_HOME/tachyon`.

## Amendment (2026-10-19): Route items on iOS and Android (MN1a)

iOS and Android no longer refuse `tac.application.menus`. A phone has no
menu bar, so each platform shows the route items in its own place, and the
build prints the support table's note (`adapted`).

### iOS

The app delegate's `buildMenu(with:)` inserts the items into the File, View
and Help menus of the main menu, as one inline group per menu, which iPadOS
shows in its menu bar. An item with a shortcut is a `UIKeyCommand`; one
without is a `UICommand`. iPhone shows no menu bar, so there the root view
controller also returns the items that have a shortcut from `keyCommands`.
They work from a hardware keyboard; an item without a shortcut cannot be
reached on iPhone. `command` is Command, `control` is Control, `option` is Option,
and the macOS shortcut rules apply. Both paths call the root view
controller, which loads the route only when it is a page of the app.

### Android

The activity puts an app bar (`android.widget.Toolbar`, set as the action
bar) above the web view when the manifest declares items, and lists them in
its overflow menu, File, View and Help as three groups with dividers. An app
without items shows no app bar, as before. Shortcuts follow the Linux and
Windows rules: `command` and `control` are both Ctrl, so shortcuts that then
collide, or that become a standard one, stop the build with `TY1601`. The
activity's `dispatchKeyEvent` runs a matching shortcut before the web view
sees the key, and `onProvideKeyboardShortcuts` lists the shortcuts in the
system keyboard shortcuts list, grouped as File, View and Help.

### Action items

An item with an `action` still stops an iOS or Android build, now with
`TY1601` from the menu check, whose message says that menu actions come with
the application lifetime. Mobile apps get that lifetime in LT1, and MN1b
then lets action items call the application service as on the desktop.

### Evidence

- Cloud and Mac: `mobile_menu_host_tests` checks the generated Swift and
  Java, and `native_menu_config_tests` checks the mobile route items, the
  `TY1601` refusal of action items and the Android Ctrl collisions.
- iOS: `scripts/ios/menus-test.mjs` builds an app with a File item that has
  Command-K and a Help item without a shortcut, then presses Command-K with
  a simulated hardware keyboard (`scripts/ios/simulator-keyboard.m`, which
  sends HID key events through Xcode's SimulatorKit). A Swift service on each
  page records `location.pathname`. On an iPhone 17 Pro and an iPad Pro
  13-inch (M5) simulator with iOS 26.5, Command-K opened `/status` (on iPad
  only the main menu holds the command, because `keyCommands` is empty
  there).
- Android: `scripts/android/menus-test.mjs` (now part of
  `scripts/android/native-test.sh`) builds the same menus and runs them on the
  emulator (API 35, arm64). The system keyboard shortcuts list shows the File
  group with Status and Ctrl+K, Ctrl+K opens `/status`, and More options >
  Guide in the app bar opens `/guide`. The page address is read through the
  WebView's DevTools socket.

## Amendment (2026-10-19): Android ABIs after MR2

This amendment supersedes the Android row of the `--arch` table in the
2026-10-18 amendment above ("the APK holds no native code yet ... ABIs follow
linked Rust on Android (MR2)"), which stays as the history of this decision.
MR2 linked Rust on Android: an app with Rust services carries its Rust
library for `arm64-v8a` and `x86_64`, so it installs on 64-bit Arm phones and
x86_64 emulators only. Without Rust services the APK still holds no native
code and runs on every ABI. `--arch` still does not choose Android ABIs.

## Amendment (2026-10-24): Application lifetime and menu actions on iOS and Android (LT1, MN1b)

iOS and Android builds no longer refuse a client Rust Service marked
`#[Lifetime({"value":"application"})]` (`TY1406`), or a menu item with an
`action` (`TY1601`). Both work as on the desktop: an application service is a
client Rust Service, and it lives only inside the app. The browser still
refuses it with `TY1406`; the message names a page or singleton Service as
the alternative, because a browser page has no application lifetime.

### When the services start and stop

- **iOS.** The app delegate starts the application services in
  `application(_:didFinishLaunchingWithOptions:)`, before the window, and stops
  them in `applicationWillTerminate`, after the page services. If the start
  fails, the host records `application.start.failed` and exits. iOS ends a
  suspended app without telling it, so a stop runs only when the app is
  terminated while it still runs (for example in the background task that the
  host begins when the app leaves the screen). An `OnStop` method must not be
  the only place where an app saves its data.
- **Android.** The activity starts the application services in `onCreate`,
  once the Rust library is loaded, and stops them in `onDestroy` when the
  activity finishes (Back, or `finish()`). A recreated activity (a rotation or
  another configuration change) keeps them: `onCreate` finds them running.
  Android keeps the process after the activity finishes, so a later launch in
  the same process starts new application services. This is the one change to
  the shared runtime: after a clean stop, the host may start the application
  again, and it gets new instances. A start that failed stays refused. If the
  start fails, the activity records `application.start.failed` and finishes.
- Every Rust call of an Android app now runs on one thread for the whole
  process (`tachyon-rust` in `TachyonRust.java`): page calls, the application
  start and stop, and menu actions. The application services belong to the
  thread that started them, so a new activity, with its own bridge worker,
  must reach them on that same thread. A page call waits for its answer as
  before. The old activity's last dispose no longer reaches Rust, so it
  cannot dispose the pages of the new activity; the new activity's first page
  load disposes what is left. `TachyonRust.attach` now hands later signals to
  the newest activity; before, a recreated activity's page never heard them.

### Menu actions

- **iOS.** An action item is a `UICommand` (or a `UIKeyCommand` when it has a
  shortcut) in the iPad menu bar, and the iPhone `keyCommands` list items with
  a shortcut, as for route items. The root view controller runs the action
  through `tac_native_menu_action`. It enables the item with the desktop rule:
  `validate(_:)` marks an item disabled, and `canPerformAction` refuses its
  shortcut, while the item has an `event` that no live Rust page listener
  hears. An item without an `event` is always enabled.
- **Android.** Action items join the app bar overflow menu with the route
  items. `onPrepareOptionsMenu` enables each item with the same rule, and a
  shortcut that is not enabled does nothing and records `menu.refused`. The
  readiness check waits at most 250 ms for the Rust thread and answers
  disabled after that, so a slow page call cannot hold the window. The action
  itself runs on the Rust thread without blocking the window.
- The Rust library exports two more JNI entry points for this,
  `TachyonRust.runMenuAction` and `TachyonRust.menuEventReady`, only when the
  manifest declares an action, beside `start` and `stop`, which every Android
  Rust library exports.

### Evidence

- Cloud: `mobile_menu_host_tests` checks the generated Swift and Java,
  `native_menu_config_tests` checks that every native target accepts action
  items, `native_application_tests` checks that a cleanly stopped application
  starts again with new instances, and `android_rust_library_tests` checks the
  JNI names. A real `ty build --target android` of the gate app in
  `scripts/native/application-lifetime-fixture.mjs` compiled the Java host and
  the Rust library for `arm64-v8a` and `x86_64`, and both libraries export
  `start`, `stop`, `runMenuAction` and `menuEventReady`.
- Simulator and emulator: `scripts/ios/application-lifetime-test.mjs` (iPhone
  and iPad simulators) and `scripts/android/application-lifetime-test.mjs`
  (emulator, now part of `scripts/android/native-test.sh`). The runs are
  recorded below when they pass.
