# ADR 0034: Tac Configuration Is the Web App Manifest, and Each Realm Keeps Its Own

- Status: Accepted; amended by [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
  (the `tac` member also holds Tachyon's own settings)
- Date: 2026-09-23
- Amends: [ADR 0026](0026-json-results-and-separated-configuration.md)
  (where public Tac configuration lives and is published)
- Preserves: private Yon configuration and `.env` secrets (ADR 0023, ADR 0026),
  client-only Tac, the `TAC_` names and `tac.` paths services bind

## Context

`application.json` carried two namespaces:

- `tac`, which is public and published to every client;
- `yon`, which is private to the server.

Each namespace has one key property. `tac` values ship to every visitor. `yon`
values never leave the server. With both in one file, that property depended
only on which key a value sat under. The build also published the `tac`
section a second time, at `/.tachyon/application.json`.

A Tac application already has a public document that describes it: the web
app manifest. Tachyon read a `manifest.json` at the project root to get a
native build's name, icons and window. It publishes that file at
`/manifest.webmanifest`, and a browser installs the app from there. The
manifest is public by definition. A manifest may also carry its own members,
which a browser ignores.

## Decision

- Tac's public configuration is the `tac` member of `client/manifest.json`.
  It has the same shape as the old `tac` section of `application.json`. So
  `TAC_` property names, `tac.` value paths and every binding keep working.
- Discovery validates the `tac` member with the same rules that
  `server/application.json` uses:
  - lowercase keys;
  - bounded size;
  - no names that look like secrets.

  The build publishes the manifest unchanged at `/manifest.webmanifest` and
  links it from every page. The build no longer writes
  `/.tachyon/application.json`. No profile overlays the manifest.
- Each realm's file sits inside that realm's folder:
  - the manifest at `client/manifest.json`;
  - Yon's configuration at `server/application.json`, with its profiles as
    `server/application-<profile>.json`.

  A file at the old location in the project root stops discovery and server
  start. The message names where the file belongs. `.env` stays at the
  project root, where secret managers and tools expect it.
- `server/application.json` is the `yon` namespace itself, with no root key.
  The loader reads its members as `yon.*` paths. The loader adds the
  namespace, so paths, `YON_` names and the `YON_APPLICATION_JSON` snapshot do
  not change. The loader refuses a `yon` or `tac` root key and says what to do
  instead.
- `server/application.json` and its profiles configure Yon only (Application
  Configuration v3). A `tac` section in one of them stops discovery and server
  start with a diagnostic that names the manifest. `ty migrate` reports it.
- A configuration document passed to `Behaviors.properties` or
  `Behaviors.values` must contain a `tac` or `yon` section. Other top-level
  members, such as a manifest's `name` or `icons`, are ignored. A browser
  service binds the manifest it loads without unwrapping it.

## Consequences

- Public and private configuration are in separate files. Putting a value
  under the wrong key cannot publish anything private, because a client build
  never reads the private file.
- A project's configuration and its install metadata are one document. A
  project without a manifest has no Tac configuration. A service that loads
  the configuration must expect that.
- A client that loaded `/.tachyon/application.json` now loads
  `/manifest.webmanifest`.
- The application-config contract moves to v3 (Yon only). The v2 schema stays
  as published history.
