# ADR RRRR: A Page App Deployed Apart Reads the Shell Plan From One File

- Status: Accepted (slice 4c of step 2 of micro-frontends)
- Date: 2026-11-15
- Relates to: [ADR PPPP](PPPP-a-shell-layout-renders-every-page-in-its-outlet.md)
  (a shell layout renders every page in its outlet),
  [ADR QQQQ](QQQQ-links-between-page-apps-swap-the-outlet-without-a-reload.md)
  (links between page apps swap the outlet without a reload),
  [ADR 0139](0139-a-shell-lists-its-page-apps-and-builds-them-as-one-site.md)
  (a shell lists its page apps and builds them as one site)

## Context

A page app can be built as part of the shell's bundle (ADR 0139). Then the
build writes the shell plan into every document of the app. A team that
deploys its page app apart from the shell builds it alone, with its own
`tac.mount`. That build does not know the shell, so its documents have no
shell plan, and its pages render without the shell's frame.

## Decision

- **One file:** when the shell has `tac.layout`, its build (also
  `ty bundle --shell-only`) writes `/.tachyon/shell.json`. The file holds
  exactly the text of the inline `#tachyon-shell` plan.
- **Styles in the plan:** the plan lists the shell's stylesheets in
  `styles` (today `/.tachyon/components.css` when the shell has component
  styles, else an empty list). Before the first render the runtime adds
  every listed stylesheet that the document does not have yet and waits for
  it, so a page app deployed apart shows the shell with its styles.
- **Loading:** at start-up, a document that has a view plan, no inline shell
  plan, and a runtime outside the root app (`/`) fetches
  `/.tachyon/shell.json`. When the response is OK and holds a shell plan, the
  runtime uses it as if it were inline. When the file is missing or is not a
  shell plan, the page renders alone, as before. A page app built alone
  needs no change and no setting.
- **Same shell:** soft navigation (ADR QQQQ) compares shell texts. A target
  document without an inline plan whose runtime is outside the root app uses
  the shell file's text; the runtime fetches it again at that moment, so a
  shell that was deployed again since the page loaded makes the navigation a
  normal load.

`bun run test:page-apps-browser` now runs twice: once with the one bundle,
and once with the shell built by `--shell-only` and the chat app built alone
and copied under `/chat/`. Each pass checks Chromium, Firefox and WebKit.

## Consequences

- A page app can be deployed on its own schedule under the shell's origin.
  It renders in the shell and keeps the shell's state across links, as in the
  one bundle.
- A page app outside the root app fetches one small file at start-up, also
  on a site without a shell, where the answer is a 404.
- The shell and the app must use the same TACHYON version (decision 5 of
  the page-app plan); the shell plan format is not a stable contract between
  versions.
- One service worker for apps deployed apart is a later slice.

## Alternatives considered

- **Build the app against a copy of the shell:** couples the app's build to
  the shell's sources and makes every shell change a rebuild of every app.
- **Load the shell's own document and copy its plan:** a larger download, and
  the shell's routes would decide what a page app shows.
