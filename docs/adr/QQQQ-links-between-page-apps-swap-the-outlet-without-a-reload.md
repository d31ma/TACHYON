# ADR QQQQ: Links Between Page Apps Swap the Outlet Without a Reload

- Status: Accepted (slice 4b of step 2 of micro-frontends)
- Date: 2026-11-15
- Relates to: [ADR PPPP](PPPP-a-shell-layout-renders-every-page-in-its-outlet.md)
  (a shell layout renders every page in its outlet),
  [ADR 0139](0139-a-shell-lists-its-page-apps-and-builds-them-as-one-site.md)
  (a shell lists its page apps and builds them as one site),
  [ADR 0009](0009-platform-navigation-over-a-client-renderer.md)
  (platform navigation over a client renderer)

## Context

With a shell layout (ADR PPPP), every document of a site renders the same
frame. A link from one page to another still loads a whole new document:
the shell's state (an open menu, a running call, an unlocked Store) is lost,
and the shell's services start again on every page.

## Decision

When a document has a shell plan (`#tachyon-shell`), the runtime handles
links itself:

- **Which links:** a click with the main button and no modifier key, on an
  `<a href>` (also inside a shadow root) to the same origin, without
  `download` and without a `target` other than `_self`. A link that only
  changes the fragment of the current page stays with the browser.
- **What happens:**
  1. The runtime fetches the target document, the same document a direct
     visit gets, so separately built apps need no extra files. Redirects
     inside the origin are followed.
  2. It keeps the soft navigation only when the target has a view plan, the
     same shell plan, and no head script that the current document has not
     loaded. Otherwise it loads the target as a normal page.
  3. It adds the target's stylesheets that the document does not have yet
     and waits for them.
  4. It disposes the singleton services of the old page; the shell's
     services stay. It sets the new plan, page state, service catalog,
     title, the Store owner app (ADR 0059) and the service folder of the
     target app.
  5. It renders, inside `document.startViewTransition` where the browser
     has it. The old page's components leave with the outlet.
  6. It scrolls to the top (or to the fragment) and moves focus to the new
     page's first heading.
- **History:** a link adds a history entry; Back and Forward (`popstate`)
  do the same steps without adding one, and restore the scroll position
  saved for that entry.
- **Fallback:** any failure loads the target as a normal page (Back and
  Forward reload the current address).
- A document without a shell plan keeps the browser's own navigation.

`bun run test:page-apps-browser` checks this in Chromium, Firefox and
WebKit: the shell's state survives a link to another app and Back and
Forward, a page's service ends with its page, focus moves to the heading,
and a direct visit to an app renders the shell around it.

## Consequences

- The shell's state and services live as long as the tab.
- One runtime renders the pages of every app. The site uses one TACHYON
  version (decision 5 of the page-app plan); a different shell plan makes
  the navigation a normal load.
- Limits of this slice: a target page whose head loads a browser-API helper
  script that the current document did not load (for example another app's
  `tac-crypto.js`) is loaded as a normal page; and a service name that both
  the shell and a page use still means the page's service (per-app catalogs
  come with separately deployed apps).

## Alternatives considered

- **The Navigation API (`navigation.navigate`):** not in every engine the
  project supports.
- **Keep full loads and restore the shell's state from storage:** the state
  of a running call or an unlocked key cannot be stored and restored.
