# ADR MMMM: A Shell Lists Its Page Apps and Builds Them as One Site

- Status: Accepted (slice 3 of step 2 of micro-frontends)
- Date: 2026-11-14
- Relates to: [ADR NNNN](NNNN-a-tac-app-can-be-built-under-a-path-prefix.md)
  (a Tac app can be built under a path prefix),
  [ADR 0086](0086-a-component-can-be-exported-as-a-custom-element.md)
  (step 1 of micro-frontends),
  [ADR 0100](0100-a-route-with-several-web-runtimes-gets-a-measured-allowance.md)
  (the route budget)

## Context

A page app with `tac.mount` can be built and deployed on its own under its
prefix. Mesail (CADUCEUS ADR 0010) also wants the same page apps built
into one site, as one deploy, from the same source, with the same web
addresses. Which form an install uses must be a deploy choice, not a code
change. Both forms need one service worker and one web app manifest for
the origin.

## Decision

### The shell lists the page apps

The app at `/` is the shell. Its manifest lists the site's page apps by
prefix, with each app's project folder, relative to the shell's project
folder:

```json
{ "tac": { "apps": { "/chat": "../chat", "/mail": "../mail" } } }
```

The build stops with `TY1601` when:

- a key is not a valid `tac.mount` value;
- a folder is absolute, cannot be opened, or lies inside the shell's
  project;
- an app's own `tac.mount` differs from its key;
- one prefix lies inside another (`/chat` and `/chat/old`);
- the shell has a page under an app's prefix;
- a mounted app lists `tac.apps`;
- the list has more than 32 apps.

### One bundle

`ty bundle <shell>` builds the shell and every listed app into one output
folder. Each app is built as in the mounted form, so the files are the same
in both forms: the shell at `/`, each app under its prefix. The apps'
route manifests, API references and build states are not copied; the
shell's `route-manifest.json` lists every app page. Two apps that would
write the same file stop the build with `TY1601`.

The shell then writes the service worker over all files, so its asset
table holds every app's files, and one `manifest.webmanifest`.

Each route still meets its route budget (`TY1337`): the shell's pages use
the shell's `tac.budgets`, and each app's pages use that app's settings.

The apps are built without incremental route reuse inside a bundle.

### Separate deploys

`ty bundle <shell> --shell-only` builds only the shell: its pages, the
service worker, the web app manifest, the site map and the site
association. Each app is built with `ty bundle <app>` and deployed under
its prefix. In both forms the shell writes `/.tachyon/apps.json`, the list
of prefixes, for the slices that follow (the worker and the outlet).

### Development

`ty start <shell>` serves the one bundle, and it serves the dynamic pages
of every app (such as `/chat/42` from `chat/_id/index.html`). In this slice
the server watches the shell's folder only: after a change in an app, start
the server again, or start the app alone with `ty start <app>`.

### Native apps

A native build of a shell with `tac.apps` stops with `TY1601` until the
native form of a site ships.

## Consequences

- The same page apps build into one site or into separate deploys, with the
  same files and addresses.
- The site has one service worker and one web app manifest in the bundle
  form. In the separate form, the shell's worker knows only the shell's
  files until a later slice reads each app's asset table.
- Watching every app's folder in `ty start`, the shell that stays on screen
  while pages change, and native apps of a site come in later slices.

## Acceptance (slice 3)

- `tac.apps` with every `TY1601` case above and a row in
  `target_support.json`.
- `ty bundle <shell>` writes the shell and every app, one route manifest
  that lists every app page, `/.tachyon/apps.json`, and a worker whose asset
  table holds every app file.
- `ty bundle <shell> --shell-only` writes no app page and still writes
  `/.tachyon/apps.json`.
- `ty start <shell>` serves an app's dynamic page.
