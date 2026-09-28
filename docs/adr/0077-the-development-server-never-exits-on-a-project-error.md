# ADR 0077: The Development Server Never Exits on a Project Error

- Status: Accepted
- Date: 2026-09-28
- Works with: ADR 0070 (coverage means a test called the method: the
  preview gate now shows its result instead of refusing to start)

## Context

`ty preview` stopped whenever the project had an error at start: a broken
`server/application.json` (`TY2010`), a build rule (`TY1328`, `TY1333`), a
view that did not compile, or the coverage gate (`TY2023`). Every rebuild of
`ty` restarts a running preview (`TY1320`), and the new `ty` loads the
project again, so a new rule that the project did not pass yet took the site
down: this happened six times in one day. Development servers such as Vite
and Next.js keep serving and show the error in the page instead.

## Decision

1. `ty preview` and `ty serve` (while they watch sources) never exit because
   of a problem in the project. Only a process problem stops them: the
   address is not allowed (`TY1301`) or the port cannot be bound (`TY1302`).
   `ty build`, `ty bundle`, `ty start`, `ty serve --no-watch`,
   `ty test --coverage` and CI keep failing on every error.
2. A failed rebuild keeps the last good build: the build writes into a stage
   folder and publishes it only when it succeeds, so the served files stay
   whole. The problems go to open pages as a hot `diagnostics` message and to
   the terminal.
3. The server keeps the current problems and answers them at
   `/.tachyon/diagnostics` (JSON: code, message, `file:line` locations and
   help). The live-reload client reads it when a page opens and after each
   `diagnostics` message, so a page opened after the failure shows it too.
   The file and line come from the byte span of the diagnostic and the file
   on disk; the hot-update contract does not change.
4. The overlay is an `alertdialog` with a heading, one item per problem and a
   Dismiss button (Escape works too). The next good build removes it.
5. When the project fails at start, the preview binds the same address and
   serves an error page (status 503) that lists the problems, with the
   live-reload client. It watches the sources and tries again after each
   change, while the error page stays up. When the project loads, the error
   page stops, the real server binds the address, and open error pages
   reload into the site.
6. A coverage refusal (`TY2023`) or a failed test run (`TY2020`) at start no
   longer blocks the preview: the site starts and the overlay lists the
   untested methods.
7. Before a self-restart, the old server runs the new `ty` with the same
   arguments and the hidden `--check-project` flag, which loads the project
   (discovery, rules and configuration) and exits. When the check fails, the
   old server keeps running and the overlay shows the new `ty`'s reason. After
   the next source change the old server checks again and restarts once the
   new `ty` accepts the project. A new `ty` that loads the project but fails
   the build serves the last good build with the overlay (point 2).

## Consequences

- The live preview stays up while `ty` and the project change in any order.
- The start of a broken project costs one extra listener and file watcher.
- The self-restart check loads the project once more (a few seconds on the
  website) before the restart.
