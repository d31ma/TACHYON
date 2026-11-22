# ADR 0139: A Shell Lists Its Page Apps and Builds Them as One Site

- Status: Accepted (slice 3 of step 2 of micro-frontends)
- Date: 2026-11-14
- Relates to: [ADR 0138](0138-a-tac-app-can-be-built-under-a-path-prefix.md)
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

## Amendment (slice 5): one worker for separate deploys

**Status:** Accepted.

In the separate form the shell's worker now also keeps each page app's
files, so a page app works offline and a new deploy of one app touches
only that app.

- **Asset table.** A page app built alone (`tac.mount`, outside `/`) writes
  `<prefix>/.tachyon/app.json`: the TACHYON version it was built with, a
  `version` hash of its files, and the asset table of its files under their
  full addresses (such as `/chat/.tachyon/tac-client.js`). The table follows
  the same size limits as the shell's table. A bundle leaves the app's
  table out, because the shell's worker already lists every app file.
- **The shell's worker** reads `/.tachyon/apps.json` and each listed app's
  table at install, and again (network first) at each full page load inside
  that app. It keeps the tables in the cache `tachyon-app-tables`, so it
  can read them offline. A request under an app prefix uses that app's
  table and the cache `tachyon-static-<prefix>@<version>`.
- **Caches per app and version.** When an app's table names a new
  version, the worker deletes only the older caches of that app. A new
  shell worker deletes only the older shell caches
  (`tachyon-static-<hash>`). An app that leaves `apps.json` loses its
  caches. A new deploy of one app does not change the shell's worker, so it
  reloads no open page.
- **Servers.** `ty start` and `ty preview` treat a file listed in an app's
  table as public, like a file in the shell's table.
- **Version skew.** `/.tachyon/shell.json` names the TACHYON version of the
  shell. A page app's document renders inside the shell only when its
  `app.json` names the same version; otherwise it renders alone, and a link
  to it from the shell loads the whole page. A page of a bundle holds the
  shell plan itself and was built with the shell, so it skips this check and
  asks for no app table. After a TACHYON update, deploy every app again.

### Acceptance (slice 5)

- `ty bundle <app>` with `tac.mount` writes `<prefix>/.tachyon/app.json`
  with the version, the hash and the full addresses; a bundle and the shell
  write no app table.
- `bun run test:service-worker-browser` checks a shell built with
  `--shell-only` and a chat app deployed apart: the chat files are kept in
  the chat cache, the chat page works offline, and a new chat deploy gets a
  new chat cache, deletes the old one, keeps the shell cache and worker, and
  reloads no page.
- `bun run test:page-apps-browser` checks that a page app whose `app.json`
  names another TACHYON version loads as a whole page and renders alone.
