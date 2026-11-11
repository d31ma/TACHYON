# ADR 0114: Event Bindings Take Key Filters, Modifiers and Document Listeners

- Status: Accepted
- Date: 2026-11-10
- Amends: [ADR 0008](0008-event-binding-contract.md) (the event binding
  contract)

## Context

ADR 0008 accepted `on:<event>="handler()"` with one plain event name. That
was enough for buttons and inputs. It was not enough for the things every
real app builds: a menu, a dialog and a message composer.

- A menu closes when the user presses Escape or clicks somewhere else. The
  Escape key is pressed on the document, not on the menu. A click "somewhere
  else" has no element to bind to.
- A composer sends on Enter and adds a new line on Shift+Enter. A handler
  that reads `$event.key` cannot stop the browser's own action, because
  Tac handlers run asynchronously, after the browser has already acted.
- A form must not reload the page when it is submitted. For the same reason,
  an asynchronous handler cannot call `preventDefault()` in time.
- Web components fire custom events with hyphens, such as `w-change`. The
  event name rule (`[a-z0-9:]`) refused them.

Go Tac services (Go compiled to WebAssembly, `// @Service`) cannot work
around any of this, because the Go code never sees the DOM. The Mesail
rewrite on Go Tac needs all of it.

## Decision

An event attribute has this shape:

```text
on:[document:|window:]<event>(.<modifier>)*
```

- `<event>` is 1 to 64 characters of `a-z`, `0-9`, `:` and `-`. It must not
  start or end with `-`. So `on:w-change` and `on:update:selected` both work.
- `document:` and `window:` are reserved prefixes. The listener goes on
  `document` or `window` instead of the element. Before this ADR, a custom
  event named `document:...` or `window:...` was valid. No project in the
  repository used one.
- The modifiers come from one closed list. An unknown modifier, a repeated
  modifier, or a second key fails the build with `TY1306`, and the message
  lists every allowed name:

| Modifier | Meaning |
| --- | --- |
| `escape`, `enter`, `space`, `tab`, `up`, `down`, `left`, `right`, `home`, `end` | The handler runs only when `KeyboardEvent.key` is `Escape`, `Enter`, `" "`, `Tab`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Home` or `End`. Only on `keydown` and `keyup`. At most one key per binding. |
| `shift`, `ctrl`, `alt`, `meta` | The modifier keys that must be held. |
| `prevent` | Calls `preventDefault()` before the handler starts. |
| `stop` | Calls `stopPropagation()` before the handler starts. |
| `outside` | The handler runs only for an event whose path does not contain the element. Not allowed with `document:` or `window:`. |

Rules that the runtime follows:

1. **Exact modifier keys.** When a binding names a key or a modifier key, the
   set of held modifier keys must equal the named set. `on:keydown.enter`
   does not run on Shift+Enter, and `on:keydown.shift.enter` does not run on
   Ctrl+Shift+Enter. This makes the composer case work without extra code.
2. **Synchronous checks.** The filters, `preventDefault()` and
   `stopPropagation()` run synchronously in the listener, and only when every
   filter matched. Then the asynchronous handler starts, as before.
3. **No input-method events.** A key filter ignores an event whose
   `isComposing` is true, so confirming a Japanese or Chinese word with Enter
   does not send a message.
4. **`.outside` reads the composed path.** It uses
   `event.composedPath().includes(element)`, so it works through the shadow
   roots of custom elements. A disconnected element never fires.
5. **Document and window listeners follow the render.** Each render collects
   the `document:`, `window:` and `.outside` bindings it rendered, keyed by
   the element's render path and the attribute name. After the render, a key
   that is still there keeps its listener (with the new handler), a new key
   adds one, and a key that is gone removes its listener. So a listener lives
   exactly as long as its element is rendered. A menu's Escape listener goes
   away when the menu closes.
6. **The opening event is ignored.** A newly added document or window
   listener ignores events whose `timeStamp` is older than the moment it was
   added. The click that opened a menu cannot also close it.

The compiled plan carries the options on the event object, only when they are
written: `listen`, `key`, `modifierKeys` (the `KeyboardEvent` property names,
in the fixed order `shiftKey`, `ctrlKey`, `altKey`, `metaKey`), `prevent`,
`stop` and `outside`. A binding with no modifiers compiles to the same plan
as before. `eventType` is the plain event name. The marker keeps the written
text (`data-tac-on-document__keydown.escape`), so two bindings on one element
never collide.

### Targets

Every Tac service language gets this through the one JavaScript runtime, so
Go, JavaScript, TypeScript and every other Tac language behave the same.
Native apps run the same runtime in the platform web view (ADR 0018, 0108).
`addEventListener`, `KeyboardEvent.key`, `isComposing`, `composedPath()` and
`Event.timeStamp` exist on every floor: WKWebView on macOS and iOS,
WebView2 118 or newer on Windows, WebKitGTK 2.40 or newer on Linux, and
Android System WebView 118 or newer. So every target supports the whole
contract. Nothing is declared in the manifest, so ADR 0102's reach table
gets no new row.

### Every Tac language

The contract is the same in every Tac service language, because the template
compiler and the one browser runtime own it. A service only writes the method
that a binding names, in its own style. The view always calls it in lower
camel case (`menu.close()`).

| Language | The method `menu.close()` calls | The field `menu.expanded` reads |
| --- | --- | --- |
| JavaScript, TypeScript | `close()` | `expanded` |
| Python | `def close(self) -> None` | `expanded: bool` |
| Java | `void close()` | `boolean expanded` |
| Kotlin | `fun close()` | `var expanded: Boolean` |
| C# | `public void Close()` | `public bool Expanded` |
| F# | `member this.Close() : unit` | `member val Expanded : bool` |
| Rust | `pub fn close(&mut self)` | `expanded: bool` |
| Dart | `void close()` | `bool expanded` |
| Go | `func (service *MenuService) Close()` | `Expanded bool` |
| Swift | `func close()` | `var expanded: Bool` |
| Ruby | `def close` | `attr_reader :expanded` |
| Lua | `function MenuService:close()` | `expanded = false` |
| C++ | `void close()` | `bool expanded` |

No language needs a new type, marker or test fake for this ADR: the filters,
`.prevent`, `.stop`, `.outside` and the document and window listeners run
before the runtime calls the service. No language is unsupported. The website
section "Keys, modifiers and outside clicks" shows the same menu in all
fourteen languages, and `tests/it/tac_event_modifiers.rs` builds it once per
language and checks the compiled plan.

### Limits

- Modifiers apply to `on:` bindings on HTML elements. An `on:` binding on a
  Tac component tag is unchanged (the component's own event contract).
- Keys outside the list (letters, function keys) are not filters. A handler
  can still read `$event.key`.
- `.outside` on an element that also opens it: put the toggle button inside
  the element that has `.outside`, or a click on the button closes and opens
  the menu in the same event.

## Consequences

- A menu, a dialog and a composer need no JavaScript in any language.
- The modifier list is closed. A new modifier needs an ADR amendment, so a
  typo is a build error instead of a binding that never fires.
- Bindings with modifiers cost one extra function call per event. Bindings
  without modifiers keep their old plan.

## Alternatives Considered

We rejected letting the handler call `preventDefault()`. Tac handlers are
asynchronous and may run in WebAssembly, so the call comes too late.

We rejected matching modifier keys loosely (`on:keydown.enter` also running
on Shift+Enter). It is what some frameworks do, and it makes the most common
composer bug the default.

We rejected a separate `on:outside-click` event name. A modifier works with
every event (`pointerdown`, `focusin`) and keeps the name rule simple.

## Acceptance Gate

- [x] The parser accepts the grammar and refuses unknown, repeated and
  conflicting modifiers with `TY1306` (`template::frontend` tests).
- [x] The plan carries only the written options (`compiler` test
  `event_modifiers_reach_the_client_plan_only_when_written`).
- [x] Exact modifier keys, `.prevent`, `.stop`, `.outside` and the
  document/window listener lifecycle are tested in the runtime
  (`template::event_options_tests`).
- [x] The menu example builds in every Tac language with the same event
  plan (`tests/it/tac_event_modifiers.rs`).
- [x] Chromium runs a menu (Escape and outside click), a form with
  `submit.prevent`, a composer (Enter and Shift+Enter) and a hyphenated
  custom event (`scripts/events-browser-test.mjs`).
