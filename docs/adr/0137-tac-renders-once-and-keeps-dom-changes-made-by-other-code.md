# ADR 0137: Tac Renders Once and Keeps DOM Changes Made by Other Code

- Status: Accepted (the number is the next free one at merge)
- Date: 2026-11-14
- Amends: [ADR 0052](0052-the-manifest-is-the-tac-configuration.md) (what a script in
  `tac.scripts` can count on)

## Context

DuVay moved its documentation site to Tac and pins its own script with
`tac.scripts` and `tac.scriptSources`. The script adds nodes and attributes
to elements that Tac rendered. A minimal page without DuVay (DUVAY-3,
TACHYON 26.46.02) showed three problems, the same in Chromium, Firefox and
WebKit, with `ty preview --static` and with the development server:

1. **Every page load rendered twice.** The end of `tac-client.js` called
   `await render()` two times. The second call came in with a checkpoint
   commit (9077eeea7, 2026-09-28), when mount hooks moved from start-up into
   the render. A mount, and any change during a render, already asks for
   another render (`Rendering.renderAgain`), so the second call did no
   needed work.
2. **A render threw away changes made by other code.** `AuthoredRootUpdate`
   kept an element only when its child nodes were exactly the children that
   Tac recorded. One added child made Tac replace the whole element, so the
   added child and every attribute that other code had set were lost. With
   two renders at start-up, a change made after the first render was lost at
   once; a later change was lost at the next render.
3. **A page script had no supported way to know that Tac had rendered.**
   The runtime is a module script that waits for storage with a top-level
   `await`. A script in `tac.scripts` is the next deferred module, so it
   usually runs before the first render, while the page is still empty.

## Decision

1. **A page load renders once.** The runtime calls `render()` one time at
   start-up. A component that mounts, and a change during a render, still
   cause the renders they caused before.
   - **The one needed second render:** a component that uses a service
     mounts with the `load` policy. After the first render Tac starts its
     services (their start-up methods may restore saved fields or change
     state), then renders again so the view shows that state. When the
     services start before the first render ends, the two passes merge into
     one render. So a page with such a component renders once or twice at
     load, and a page without one renders once. Waiting for the services
     before the first render ends was rejected: a slow or stuck service
     start would hold back `data-tachyon-rendered` and `tachyon:rendered`.
   - A request for a render that arrives while a render finishes is no
     longer lost. Before, `render()` only marked "render again" while a
     render was running, and a request after the last check of that mark,
     but before the render cleared its state, did nothing. The second
     start-up render hid this at start-up. Now the finished render starts
     one more render when the mark is set. Tac elements that connect while
     a render finishes, such as several elements in a plain host page, need
     this (the `web-components` browser gate in Firefox).
2. **Tac changes only the nodes it owns.**
   - Tac owns the children it rendered into an element (the element's
     recorded children). For a root, the page `<body>` or the shadow root of
     a Tac element, it owns the children of its last render. Before the first
     render it owns everything in the `<body>`. In a vendor element whose
     own `<slot>` elements hold Tac's children, Tac owns the children it put
     in each slot, so a node that other code adds to such a slot stays too.
   - Tac keeps an element when the children it owns are still direct
     children of that element, in the same order. Other nodes before,
     between or after them may be there too.
   - Tac removes only owned nodes that it no longer renders. It inserts new
     nodes in its own order. A node that other code added stays where it is,
     after the Tac node it followed.
   - Tac sets an attribute only when it writes that attribute and its value
     changed. Other attributes stay. If other code changes an attribute that
     Tac writes, the change stays until Tac's own value changes, then Tac's
     value wins.
   - The same rule holds when the focus is inside the update (the focused
     update path) and when a component element is put back after a render.
3. **Tac fires `tachyon:rendered` on `document` after each render.**
   - The event is a `CustomEvent`. Its `detail` is `{ count, first }`:
     `count` is the number of renders on this page so far, starting at 1,
     and `first` is `true` only for the first render.
   - It fires after Tac set `data-tachyon-rendered="client"` on `<html>` and
     after Tac cleared its own render state. So a listener may change the
     page or ask for another render, and that render runs.
   - It does not fire when a render fails; `data-tachyon-render-error` marks
     that case, as before.
   - A script that may run after the first render checks
     `document.documentElement.dataset.tachyonRendered === 'client'` first,
     then listens for the event.

## Limits

- When other code removes or moves a node that Tac owns, Tac replaces the
  parent element, as before. A vendor element that wraps Tac's children in
  its own node is still replaced (the `authored-node` browser gate checks
  this).
- In the focused update path, the children of the focused element itself
  and of a vendor slot that does not hold the focus are still replaced.
- A pinned script that changes the `<body>` before the first render loses
  that change: before the first render Tac owns the whole `<body>`.
- Changing a Tac component's own markup from outside is still not the way to
  build a view. `TY1364` keeps refusing it in services and shared scripts;
  only a declared vendor file in `tac.scriptSources` may do it.

## Consequences

- A page without a mounted component does half the render work at start-up,
  and a page script sees one render per load. With a mounted component it
  sees at most two, and both keep its changes.
- `scripts/dom-keep-browser-gate.mjs` (`bun run test:dom-keep-browser`) runs
  DuVay's case in Chromium, Firefox and WebKit. On a page without a service:
  one render per load, and the pinned changes made at load, when the
  component appears, right after the first render and 500 ms later all
  stay, with an attribute-only change, also after `tachyon:rerender`. On a
  page whose component uses a service: one event per render, at most two
  renders at load, the component element is inserted once, and Tac still
  updates, adds and removes its own nodes beside the kept ones.
  `tachyon:rendered` fires with the right `detail` on both pages.
