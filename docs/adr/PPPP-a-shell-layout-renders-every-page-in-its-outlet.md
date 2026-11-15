# ADR PPPP: A Shell Layout Renders Every Page in Its Outlet

- Status: Accepted (slice 4a of step 2 of micro-frontends)
- Date: 2026-11-15
- Relates to: [ADR 0138](0138-a-tac-app-can-be-built-under-a-path-prefix.md)
  (a Tac app can be built under a path prefix),
  [ADR 0139](0139-a-shell-lists-its-page-apps-and-builds-them-as-one-site.md)
  (a shell lists its page apps and builds them as one site),
  [ADR 0086](0086-a-component-can-be-exported-as-a-custom-element.md)
  (a component can be exported as a custom element),
  [ADR 0009](0009-platform-navigation-over-a-client-renderer.md)
  (platform navigation over a client renderer)

## Context

A site of page apps (ADR 0139) has one shell, the app at `/`. Every page of
the site should show the shell's frame: the header, the navigation, the sync
bar and the lock screen. Until now each page app had to repeat that frame in
every page, and the frame of one app could drift from the frame of another.

Tac is multi-page: each route is its own document with its own view plan
(`#tachyon-view`). The frame must therefore come with every document, also
with the documents of the page apps.

## Decision

The shell names a layout component in its manifest:

```json
{ "tac": { "layout": "app-shell" } }
```

- The layout component's template holds exactly one `<outlet />`. The
  outlet takes no attributes. A page that holds `<outlet />` is refused.
- The build encodes the layout like an exported element (ADR 0086): one
  component node with the catalog of the services it reaches. It writes the
  result into every page document of the shell as
  `<script id="tachyon-shell" type="application/json">`. In the one-bundle
  form (`ty bundle` of a shell with `tac.apps`) it also writes it, with the
  shell's component stylesheet, into every page document of every page app.
- The runtime renders the layout into the body and renders the document's
  own plan inside the outlet. The page keeps its own state and environment,
  and its components get keys under `outlet:<route>`, so a later swap of
  the outlet (soft navigation, next slice) disposes only the page's
  components and keeps the shell's.
- The shell's services join the page's catalog; the page's entry wins when
  both name the same service. A page app's runtime also accepts service
  modules from the shell's `/.tachyon/services/` folder.
- Only the shell, the app at `/`, may declare `tac.layout`. Every target
  honors it: native apps render the same documents.

Errors are `TY1601`.

## Consequences

- A site shows one frame on every page, built from one component.
- Each document still loads on its own: a direct visit or a reload of any
  route renders the frame and the page, as before.
- Limits of this slice: links still load the whole document (soft
  navigation is the next slice); page apps deployed apart from the shell
  do not get the frame yet (`--shell-only` builds); and a service name used
  by both the shell and a page means the page's service.

## Alternatives considered

- **Each app includes the frame component through `tac.include`:** every
  page would still have to place the frame itself, and a page could forget
  it. The layout puts the frame in one place.
- **A server-side wrapper around each document:** Tac renders in the
  browser
  ([ADR 0015](0015-tac-is-exclusively-client-rendered.md)), and separately
  deployed apps have no common server.
