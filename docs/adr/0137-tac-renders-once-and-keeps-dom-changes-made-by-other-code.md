# ADR 0137: Tac Renders Once and Keeps DOM Changes Made by Other Code

- Status: Accepted (the number is the next free one at merge)
- Date: 2026-11-14
- Amended: 2026-11-15 (TAC-B1F: nodes an element paints into itself while
  Tac builds it are not Tac's children)
- Amended: 2026-11-15 (TAC-RND2: a changed observed attribute no longer
  replaces a custom element)
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
   - Nodes that an element adds to itself while Tac builds it are the
     element's own, not Tac's. A custom element may paint when Tac gives it
     a bound property (ADR 0136), before Tac adds the view's children. Tac
     records only the children it rendered, so an update that keeps the
     element never moves the element's own paint into it a second time
     (TAC-B1F, amended 2026-11-15).
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
   - **A changed observed attribute keeps the element** (TAC-RND2, amended
     2026-11-15). Before, when a bound attribute that a custom element lists
     in `observedAttributes` changed, Tac replaced the whole element. The
     rule came from an early runtime (2026-08-30) and protected Tac's
     children from an `attributeChangedCallback` that rebuilds the
     element's inner DOM. Its cost was found by DuVay's DUVAY-7 test on
     448e35032: `<w-input :hint>` and `<w-select :hint>` were replaced on
     every update, the focus went to `<body>`, and the typed text and the
     chosen option were lost. The full rule for a custom element that Tac
     rendered before is now:
     - **Only its attributes change** (observed or not): Tac keeps the
       element and sets only the changed attributes in place:
       `setAttribute`, or `removeAttribute` when the value is `null`,
       `false` or `undefined`. B1's bound-property rules (ADR 0136) and
       B1F's rule above do not change.
     - **Its tag changes:** Tac replaces it.
     - **Other code moved Tac's children into another node:** Tac replaces
       it (the limit below).
     - **Its own `attributeChangedCallback` detached Tac's children** while
       Tac wrote an observed attribute (the children have no parent now):
       Tac keeps the element and renders those children again inside it,
       after the element's own nodes, as on the first render. Why: Tac must
       write the attribute to keep the element, so Tac itself caused the
       removal. Leaving the children out loses Tac's content, and replacing
       the element brings the focus loss back. The element's own nodes are
       not touched, and Tac's children appear once.
     - Tac writes the changed attributes of such an element while it plans
       the update, before it checks the element's children, so the check
       sees what the element holds after its callback. Tac does not write
       the same value again later in the update, because setting an
       attribute to its current value runs the callback again. So the
       callback runs once per changed attribute and update.
     - **Focus:** Tac gives the focus back to a control it rendered again
       only when the focus was inside the detached Tac children and is now
       on `<body>` (or nowhere). It never moves the focus when the user or
       other code already put it somewhere else. Focus inside nodes that
       the element paints itself is the element's job.
     - Two older browser assertions changed on purpose: the `observed-wrap`
       cases in `scripts/runtime-browser-test.mjs` and
       `scripts/authored-node-browser-gate.mjs` expected the element to be
       replaced and its callback not to run. They now expect the same
       element, one callback, and the focus and caret on the input that
       Tac rendered again.
     - `scripts/dom-keep-browser-gate.mjs` checks this in Chromium, Firefox
       and WebKit. A plain custom element with a bound observed attribute
       holds an `<input>` and a `<select>`: after typing, after choosing an
       option and after three updates each (one removes the attribute),
       the element, the controls, the focus, the typed text, the caret and
       the choice are the same, and the element saw each value once. A
       second element rebuilds its inner DOM in its callback: it is kept,
       shows the new value, and Tac adds nothing to it. A third element's
       callback detaches Tac's `<input>`: it is kept, the callback runs once
       per update, the input is rendered again exactly once after the
       element's own node, and the focus comes back to it; when the user
       moved the focus elsewhere first, the focus stays there.
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
  parent element, as before. One exception (TAC-RND2): a node that an
  element's `attributeChangedCallback` removed while Tac wrote an observed
  attribute is rendered again in the kept element. A vendor element that wraps Tac's children in
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
