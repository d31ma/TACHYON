# ADR 0052: The Manifest Is the Tac Configuration

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0029](0029-pages-compose-components.md) (where site-wide
  `scripts` and `styles` are declared) and
  [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md) (the `tac`
  member now also holds Tachyon's own settings)
- Supersedes: [ADR 0040](0040-tac-config-is-a-class.md) (`tac.config` is
  removed) and the `tachyon.json` file of the Phase 4 native slice
- Preserves: the web app manifest as a public document, its location at
  `client/manifest.json`, `TAC_` names and `tac.` paths, and every limit and
  check the old fields had

## Context

A Tac project kept its configuration in three places:

- `client/manifest.json`, the web app manifest, for the name, entry route,
  icons, window and public Tac values;
- `tac.config.js` (or `.mjs`, `.ts`), a class whose static fields held the
  native application identity, site-wide scripts and styles, service-worker
  cache rules and page metadata, plus a `postBundle` hook;
- `tachyon.json`, an older file with the native application identity.

The same value, such as the application name, could be written in two files,
and an order decided which one won. `tac.config.js` was code: every build
started Node.js or Bun to read it, and the website used `postBundle` only to
write `sitemap.xml` and `robots.txt`.

## Decision

`client/manifest.json` is the only Tac configuration file, apart from
`jsconfig.json` and `tsconfig.json`.

- The standard manifest members keep their meaning and feed native builds:
  `name` (or `short_name`) names the app, `start_url` is the entry route, and
  `icons` gives the app icons.
- The `tac` member holds Tachyon's own settings next to the public values that
  services bind:
  - `application`: `id` (the bundle id), `version` and `window` (`width`,
    `height`, `minWidth`, `minHeight`, `controls`). Unknown keys are refused.
    Without them, the id comes from the folder name, the version is 1.0.0 and
    the window is 1024 by 768.
  - `scripts` and `styles`: up to 8 local paths each, linked into every page.
  - `styleSources`: optional declarations keyed by exact local `/shared/` stylesheet
    URL. `{ "kind": "foundation" }` receives the narrow colors/fonts/reset audit;
    an undeclared legacy `styles` string receives the same audit. A pinned vendor
    declares `{ "kind": "vendor", "package", "version", "license", "sha256" }`
    and the build verifies captured source bytes. CSS imports retain file/byte/
    depth/path/encoding/no-symlink bounds and cannot hide undeclared application
    layout. The build error names the limit or rule that a stylesheet breaks.
  - `scriptSources`: optional exact local shared vendor-script declarations with
    `package`, `version`, `license` and `sha256`. Only byte-verified declared vendors
    receive the authored-script exception; folder and filename do not grant it.
  - `cache`: up to 64 service-worker read rules, `{ "path", "policy" }`.
  - `metadata`: `defaults` and `routes` (keyed by route, such as
    `/docs/_topic`) with `title`, `description`, `canonical`, `image`,
    `siteName`, `lang` and `noindex`. A page gets its route entry over the
    defaults. Page metadata is data in the manifest, not markup in the page,
    because a page holds only components (ADR 0029).
  - `origin`: the site's public address. A `canonical` or `image` that
    starts with `/` is joined to it.
  - `sitemap`: for each page route with one path variable, a data file under
    `/shared/`, a JSON pointer to a list in it, and optionally the member of
    each listed object that fills the variable.
- These Tachyon members have fixed shapes and are read by the build. The
  service-configuration rules of ADR 0034 (lowercase keys, key and size
  bounds) apply to the other members of `tac`.
- When `tac.origin` is set, the build writes `sitemap.xml` and `robots.txt`.
  The sitemap lists every page route without a path variable, except pages
  whose metadata sets `noindex`, and every value `tac.sitemap` lists for a
  dynamic page. `robots.txt` allows everything and names the sitemap.
- The `postBundle` hook is removed. The build no longer starts a JavaScript
  runtime to read configuration.
- A `tac.config.js`, `tac.config.mjs`, `tac.config.ts` or `tachyon.json` at
  the project root stops discovery, builds and server start. The message says
  where each value moved. The manifest member `tachyon` (the earlier home of
  `id`, `version` and `window`) is refused the same way. `ty migrate fix
  --write` moves plain values into the manifest and lists the rest as manual
  steps.

## Consequences

- One file answers "what is this application called, where does it start,
  and how is it published". There is no precedence order to learn.
- Everything in the manifest ships to every browser, as ADR 0034 says. The
  application id, version, window, metadata and sitemap settings are public
  by design. A secret must never be written there.
- Computed configuration, such as an origin read from an environment
  variable or a sitemap built by a script, is no longer possible inside the
  build. A project that needs it generates the manifest or the files before
  it runs `ty bundle`.
- Builds are faster and need no JavaScript runtime for configuration.

## Amendment (2026-10-10): Blocking scripts run before the first paint

Every `tac.scripts` entry is a deferred `type="module"` script after the Tac
runtime, `/.tachyon/tac-client.js`. A module script runs only after the
document is parsed, and a browser may paint before that. A page that must
change the document before the first paint, such as the website applying a
visitor's saved light theme, could show one frame of the default theme
first. The site's Content Security Policy (`script-src 'self'`) refuses
inline scripts, so the work cannot move into the page head as text.

### Decision

- `tac.blockingScripts` lists up to 8 local paths, with the same path rules
  and limit as `tac.scripts`.
- The build links each one into every page, in the declared order, as
  `<script src="..." data-tachyon-runtime></script>`: a classic script
  without `type`, `defer` or `async`, placed directly before the Tac
  runtime's module tag. The parser stops until a classic script has loaded
  and run, so it runs before the body is parsed and before the first paint.
  `script-src 'self'` allows it without a hash or nonce.
- `blocking="render"` on a module tag was not chosen. Of the pinned
  Playwright engines (Chromium 151, Firefox 153 and WebKit 26.5), Firefox
  does not expose `HTMLScriptElement.prototype.blocking`, so it ignores the
  attribute and the module stays deferred. A classic script blocks
  rendering in all three.
- `tac.scripts` does not change: each entry is still a deferred module
  after the runtime. A vendor in `tac.scriptSources` must still appear in
  `tac.scripts` (`TY1365`). A blocking script is an authored shared script
  and receives the authored-script check (`TY1364`).

### Consequences

- The first paint waits for every blocking script to load and run. Keep the
  list short and each file small.
- A blocking script is a classic script: it cannot use `import` or `export`,
  and its top-level class becomes a global name of the page.

## Amendment (2026-10-11): The theme color reaches every page head

A browser reads the manifest's standard `theme_color` only for an installed
application. A page in a browser tab, and a native build's page, colored the
browser's interface only with `<meta name="theme-color">` in the page head,
and an application could not add that tag: the build writes the head.

### Decision

- When `theme_color` in `client/manifest.json` is a text value that is not
  blank, the build adds `<meta name="theme-color" content="...">` to every
  page head, after the manifest and icon links. The value is trimmed and
  HTML-escaped, so it stays one attribute value.
- A missing, blank, number or other non-text `theme_color` writes no tag and
  still does not stop the build. The manifest is still published as written.
- The manifest models one theme color. A light and dark pair
  (`media="(prefers-color-scheme: dark)"`) is not added until the manifest
  has a member for it.

### Consequences

- The color is declared once, in the manifest, for installation, browser
  tabs and native pages.
