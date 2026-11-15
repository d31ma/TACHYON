# ADR NNNN: A Tac App Can Include Shared Client Folders

- Status: Accepted (slice 3b of step 2 of micro-frontends)
- Date: 2026-11-15
- Relates to: [ADR 0138](0138-a-tac-app-can-be-built-under-a-path-prefix.md)
  (a Tac app can be built under a path prefix),
  [ADR 0139](0139-a-shell-lists-its-page-apps-and-builds-them-as-one-site.md)
  (a shell lists its page apps and builds them as one site),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (every manifest member states its reach)

## Context

Mesail splits its screens into several Tac page apps (ADR 0138, ADR 0139).
Some code belongs to every app: the session code, the device Store entities
that two apps share (for example messages, used by chat and mail), the sync
bar and the DuVay layer. Copying that code into every app would make the
copies drift apart. A shared entity must have one declaration, so that every
app gives its table the same partition field (ADR 0059, amendment of
2026-11-12).

## Decision

An app's manifest can list folders of shared client code:

```json
{ "tac": { "include": ["../kit"] } }
```

- Each name is a folder relative to the app's project folder. A folder that
  is inside the project, or that holds the project, is refused. At most 8
  folders.
- An included folder has the app's client layout: `client/components`,
  `client/shared` and every client role folder (`client/services`,
  `client/entities`, `client/models`, `client/stores`, `client/helpers` and
  the others). It has no `client/pages` and no `server` folder: pages and
  Yon code belong to an app, and a folder with them is refused.
- The build reads the included files when it reads the app's own files and
  stages them under the same paths, as if they were in the app. So every
  later step (components, services, models, shared assets, the code rules,
  the native builds) sees one project. Nothing is written into the included
  folder.
- A file that the app and an included folder both have, or that two
  included folders both have, stops the build. Each shared component,
  service, model and asset has one source.
- Each app compiles its own copy of the shared code (decision 4 of the
  page-app plan). With one URL per shared stylesheet or script, a copy costs
  bytes, not behavior.
- Every target honors the member: the included files are part of the app
  on the web and in the native shells.

Errors are `TY1601`.

## Consequences

- Mesail can keep shared entities, services and components in one folder
  and include it in every page app.
- An error in an included file names the path inside the included folder
  (`client/components/...`), the same form as the app's own files.
- Limits in this slice: `ty start` does not watch included folders yet (a
  change there needs a restart), and the composed TypeScript types that
  editors read (`.tachyon/types`) list only the app's own models.

## Alternatives considered

- **A package that each app installs:** needs a registry or a package
  manager step for every language, and still copies the files.
- **One shared build loaded at run time:** the apps would depend on one
  deploy of the shared code; a change there could break every app at once.
  Decision 4 of the plan keeps one build per app in v1.
