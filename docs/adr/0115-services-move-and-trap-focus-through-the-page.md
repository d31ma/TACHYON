# ADR 0115: Services Move and Trap Focus Through the Page

- Status: Accepted
- Date: 2026-11-10
- Preserves: [ADR 0045](0045-go-is-a-service-language.md) (Go runs in the
  page as WebAssembly), [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (curated browser APIs, the caller rule and `BrowserCallFailed`) and
  [ADR 0018](0018-native-hosts-run-the-platform-javascript-engine.md) (native
  apps run the page in the platform web view)

## Context

A dialog, a menu or a drawer must move keyboard focus. When it opens, focus
goes inside it. While it is open, Tab and Shift+Tab stay inside it. When it
closes, focus goes back to the control that opened it. A JavaScript service
can call `element.focus()`, but a Go Tac service cannot: ADR 0065 refuses
`syscall/js` (`TY1317`), and the curated browser APIs had no focus call. The
Mesail rewrite (Go on Tac) needs accessible dialogs.

A service also cannot focus an element at once. The view that shows the
dialog renders after the method returns, so the element does not exist yet
when the method runs.

## Decision

1. The page runtime gets a `Focus` browser API in its own file,
   `template/tac-focus.js` (class `TacFocus`). It has three calls:
   - `move(target)`: focus the element named `target`.
   - `trap(container)`: focus the first element that Tab can reach inside the
     element named `container`, and keep Tab and Shift+Tab inside it.
   - `release()`: end this service's latest trap and focus the element that
     had focus when `trap` was called (the opener).
2. An element gets a name with the `data-focus` attribute
   (`<div data-focus="dialog" role="dialog">`). A name is a letter followed by
   up to 63 letters, digits, `-` or `_`. The page compares names with
   `getAttribute`, so a service never sends a CSS selector.
3. A service reaches only its own views. The page looks for the name in the
   shown components whose service graph holds the calling service, and keeps
   only elements that these components rendered (slotted content belongs to
   the component that wrote it). The caller is the one ADR 0065 records with
   `TacWebApis.calling`.
4. Every call waits for the next completed render. The call asks for a
   render, and `render()` in `tac-client.js` runs `TacFocus.settle()` after
   its render loop. Calls run in the order they were made, so
   `Trap("dialog")` then `Move("name")` in one method works, and a quick
   `Release()` after `Trap` works too.
5. Errors:
   - The call itself fails (Go `error`, a thrown JavaScript error) when the
     name is not valid, when no service is calling, when no shown component
     uses the calling service, or when 16 calls already wait for a render.
   - After the render, a failure is a `BrowserCallFailed { api: "focus",
     call, message }` event sent to the calling service only: no element has
     the name, more than one element has it, the element cannot take focus,
     16 traps are already open, or the service has no trap to release.
6. The trap:
   - Traps form a stack of at most 16. Only the top trap is active.
   - Tab stops are the usual focusable elements with `tabIndex >= 0`, not
     disabled, not inside `[inert]`, and with a layout box. If the container
     has none, the container itself takes focus (it then needs
     `tabindex="-1"`).
   - A capture `keydown` listener on the document wraps Tab at the last stop
     and Shift+Tab at the first stop. A capture `focusin` listener brings focus
     back when it lands outside the container (a click outside, or a
     programmatic focus).
   - If a render replaces the container, the trap finds it again by its name.
     If the container is gone (the component was removed), the trap ends.
   - If a render replaced the opener, `release` focuses the element with the
     opener's `data-focus` name in the same views. Otherwise focus stays where
     it is.
7. Languages:
   - Go: `type Focus struct{}` in the Tac prelude (`native/web_apis/focus.go`),
     with `Move(target string) error`, `Trap(container string) error` and
     `Release() error`. A service writes `// @Needs` above a `focus Focus`
     field. In a test unit, `BrowserFakes{}.Answer("focus", "move", nil)`
     answers a call (the same fakes as the other curated APIs).
   - JavaScript and TypeScript: `@Needs focus` (type `Focus` in
     `tachyon-behaviors.d.ts`), with `move`, `trap` and `release`.
   - Rust, C#, F#, Swift, Kotlin, Java, Dart, Ruby, Lua, C++ and Python get no
     client in this change. Their shared list of curated types
     (`web_api_clients.rs`) is unchanged, so they answer as before. A later
     change can add a client in the same way.
8. No permission and no user action are needed, so `Focus` has no grant in
   `tac.browserApis` and no manifest member. `target_support.json` needs no
   entry (ADR 0102 lists manifest members).
9. Targets: Go is never a native companion, so a Go Tac service always runs
   as WebAssembly in the page. On web, macOS, Windows, Linux, iOS and Android
   the page runs in a browser or the platform web view (ADR 0018), and the
   API uses only `focus()`, `getClientRects()`, `closest()`, `keydown` and
   `focusin`, which every native web view floor has (ADR 0108). Focus works
   the same on every target.
10. Size: `tac-web.js` only adds one guarded entry. Its budget test moves from
    11,600 to 11,700 bytes. `tac-focus.js` is part of the page runtime that
    `ShippedScript` compacts.

## Consequences

- Go Tac apps can build accessible dialogs, menus and drawers without
  JavaScript.
- A name in `data-focus` is part of the view. Renaming it breaks the service
  call with a `BrowserCallFailed` event, not a build error.
- Focus calls run after the render, so a service cannot read the result at
  once. A failure arrives as an event.
- Other service languages still need a JavaScript or TypeScript service to
  move focus.
