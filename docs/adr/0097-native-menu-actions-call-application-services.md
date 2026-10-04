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
- Linux, Windows, iOS and Android still refuse declared menu items at build
  time. Their hosts are unchanged.
- The macOS host test
  `native_menu_host_tests::macos_menu_actions_reach_the_page_service_and_close_window_reopens_cleanly`
  compiles the generated AppDelegate with a real Rust library and checks: the
  action reaches the page service without a reload and keeps its state, the
  item is disabled with no recipient, Close Window disposes the page, reopen
  gives fresh owners, and Quit cleans up once.
- Real menu-bar clicks and keyboard shortcuts by a person, and JavaScript page
  services as direct recipients, are not covered by this decision.
