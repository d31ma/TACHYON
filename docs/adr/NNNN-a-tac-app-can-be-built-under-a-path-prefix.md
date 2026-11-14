# ADR NNNN: A Tac App Can Be Built Under a Path Prefix

- Status: Accepted (slice 1 of step 2 of micro-frontends)
- Date: 2026-11-12
- Relates to: [ADR 0086](0086-a-component-can-be-exported-as-a-custom-element.md)
  (step 1 of micro-frontends; this ADR starts step 2),
  [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md) (the
  manifest is the Tac configuration),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (every manifest member states its reach)

## Context

Mesail (CADUCEUS ADR 0010) splits its screens into several Tac page apps:
home at `/`, and chat, mail, channels, search, settings and admin below it.
Each app is built and deployed on its own, or all of them are built into
one bundle. The web addresses must stay the same in both forms.

A Tac build writes every file at the root of the site: the page documents
(`<route>/index.html`), `/.tachyon/tac-client.js`, `/.tachyon/services/`,
`/shared/`, the service worker `/tachyon-sw.js` and the web app manifest.
Two apps deployed under one origin would overwrite each other's files.

## Decision

A Tac app can declare a path prefix in its manifest:

```json
{ "tac": { "mount": "/chat" } }
```

### The prefix

`tac.mount` has one to four segments of lowercase letters, digits and single
hyphens, such as `/chat` or `/admin/people`. It has no trailing slash, no
`_parameter` segment, and it cannot start with `/shared`, because the site's
shared files live there. A wrong value stops the build with `TY1601`.

### Pages keep their full addresses

The page folders of the app are routes under the prefix:
`client/pages/tac.html` is the page `/chat`, and
`client/pages/_id/tac.html` is `/chat/_id`. Every place that names a route
uses the full address: the route manifest, `tac.metadata`, the server, the
route budget and `page.path` in the browser. A team that moves the folder
`client/pages/chat/` of a bigger app into its own app with
`"mount": "/chat"` keeps every address.

### Files move under the prefix

The app's runtime, services, component styles, view plans, exported
elements and `client/shared/**` files are written under the prefix:
`/chat/.tachyon/tac-client.js`, `/chat/.tachyon/services/...`,
`/chat/shared/...`. The page documents load them from there. Language
engines already load relative to their module, so they follow the prefix
without change. The runtime accepts a service module only from the
`services/` folder beside itself.

Files that describe the deploy stay at the root of the build output:
`route-manifest.json`, `api.json`, `api-docs/` and
`.tachyon/build-state.json`.

### The site owns some files

A mounted app does not write the files that belong to the whole origin:
the service worker and its registration, `manifest.webmanifest`,
`robots.txt`, `sitemap.xml` and `.well-known/`. Its pages still link
`/manifest.webmanifest`, the site's manifest. A mounted app that grants
`push`, declares `tac.application.links` or declares `tac.sitemap` stops
with `TY1601`: those settings belong to the site's shell, the app at `/`.

Author paths do not change. A `tac.styles` or `tac.scripts` entry such as
`/shared/styles/app.css` still means the site's shared file; the app's own
files are under its prefix, such as `/chat/shared/styles/chat.css`.

### Native apps

A native app holds the whole site, so a mounted app is not built alone for a
native target. `ty bundle --target <native>` on a mounted app stops with
`TY1601` and says to build the shell's project.

## Consequences

- Separately built Tac apps can share one origin without overwriting each
  other's files. The deploy routes `/chat/*` to the chat app's output.
- An app with no `tac.mount` builds exactly as before.
- A mounted app opened alone has no service worker and no manifest of its
  own. The later slices of step 2 add the shell that owns them, the one
  bundle form, soft navigation, one session and the native form.

## Acceptance (slice 1)

- `tac.mount` with `TY1601` for a bad value and a row in
  `target_support.json`.
- A mounted build writes its pages, runtime, services and shared files under
  the prefix, writes no worker, registration, manifest, site map, robots
  file or site association, and its pages load the runtime and services
  from the prefix.
- A mounted app that grants push, declares deep links or a site map, or is
  built for a native target, stops with `TY1601`.
