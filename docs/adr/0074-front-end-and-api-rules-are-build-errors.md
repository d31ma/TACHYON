# ADR 0074: Front-End and API Rules Are Build Errors

- Status: Accepted
- Date: 2026-09-28
- Preserves: [ADR 0069](0069-design-rules-are-build-errors.md) (rules are
  build errors with a switch per realm)

## Context

Three mistakes reach users without a failing test:

1. A view that a screen reader cannot use: a picture without alt text, a
   button without a name, a field without a label.
2. A page that grows slowly until it loads megabytes of script before it
   shows anything.
3. A release that removes a route or a field that callers of the last
   release still use.

The shared engineering instructions ask for all three: WCAG basics in every
view, a size budget per route, and no API break without a new major version.
Tachyon checks them when it builds.

## Decision

| Rule | Code | What the build refuses | Switch |
|---|---|---|---|
| Accessible views | `TY1336` | A Tac view (`client/pages` and `client/components`) that breaks a WCAG basic | `tac.accessibility.check` in `client/manifest.json` |
| Route size budget | `TY1337` | A route that loads more script or WebAssembly than its budget | `tac.budgets.check` in `client/manifest.json` |
| API baseline | `TY1338` | An API that breaks `server/api-baseline.json` without a new major version | the baseline file itself |

### 1. Accessible views

`ty build`, `ty preview`, `ty serve`, `ty bundle` and `ty test` check every
`tac.html` view; `ty migrate check` lists every finding. The view must:

- give every `<img>` an `alt`; an empty `alt=""` is allowed only together
  with `role="presentation"`, `role="none"` or `aria-hidden="true"`;
- give every `<button>` and every `<a href>` a name: text inside it (text in
  braces counts, and so do components, slots and pictures with an `alt`),
  `aria-label` or `aria-labelledby`;
- label every `<input>` (except `hidden`, `submit`, `reset`, `button` and
  `image`), `<select>` and `<textarea>`: inside a `<label>`, named by a
  `<label for>` in the same view, or with `aria-label` or `aria-labelledby`;
- go down one heading level at a time (`<h2>` then `<h3>`) in each view; the
  first heading of a view may have any level, because components nest;
- use no `tabindex` above 0;
- use only WAI-ARIA 1.2 attribute names, `true`/`false` (and `mixed` for
  `aria-checked` and `aria-pressed`) where the attribute takes them, no
  `aria-hidden="true"` on an element that the keyboard can reach, and no
  `aria-label`/`aria-labelledby` on `div`, `span`, `p` and the other
  elements without a role that cannot have a name;
- give every `<iframe>` a `title`, `aria-label` or `aria-labelledby`.

Values in `:attr="..."` are checked by the browser, not by the build. The
document always has `<html lang>`: `en` by default, and
`tac.metadata.defaults.lang` or a route's `lang` in `client/manifest.json`.

### 2. Route size budgets

A release build (`ty build`, `ty bundle`) follows every file that each route
page loads: script tags, the service modules of the view plan, `import`s,
`new URL(..., import.meta.url)` and other paths written as text, and the
file lists in JSON files that a script loads (such as .NET's
`blazor.boot.json`). The service worker is not counted. A path that a script
builds at run time is not followed. It adds the sizes of the `.js`/`.mjs`
files and of the `.wasm` files and compares them with `tac.budgets.script`
(default 200) and `tac.budgets.webassembly` (default 3584), both in
kilobytes. The error names the five biggest files.

A size is the number of bytes that Tachyon sends: the Brotli copy when the
build wrote one ([ADR 0075](0075-release-builds-precompress-and-the-server-negotiates.md)), else the file.
`ty preview` does not check budgets: while it watches the sources it sends
files without compression, so its sizes would not be the sizes that users
download. Budgets use the lowercase key form of every other `tac` setting.

### 3. API baseline

`ty api freeze` writes the current `api.json` to `server/api-baseline.json`,
with the application version (`tac.application.version`) at that time. Commit
the file after a release. Each later build compares its `api.json` with the
baseline and refuses:

- a removed route, method or model, or a removed field of a model or of a
  request (query, path, header and cookie values);
- a new required request value, a request value that became required, and a
  model field that became required;
- a narrowed request type (the new type does not accept every old value; for
  example `number` to `integer` or `string` to `integer`);
- a response (`ok`) type that returns values that the old type did not
  allow, and a changed model field type (models travel both ways);
- a changed success status.

Additions are allowed: new routes, methods, models, optional values and
fields, wider request types. When the major number of
`tac.application.version` is higher than the baseline's, every break is
allowed; run `ty api freeze` again after that release. A project without a
baseline file is not checked.

## Consequences

- The accessibility check and the size budgets are on by default. New
  projects, the website, the corpus and the fixtures pass them; the website
  raises `tac.budgets.script` to 320 because it loads services in five
  languages on every page.
- A project that is not ready turns a rule off with its switch; `ty migrate
  check` still lists the accessibility findings.
- The API check starts with the first `ty api freeze`.

## Amendment (2026-10-04): Tachyon's own scripts ship compact

- The build writes `tac-client.js` and `behavior-runtime.js` without
  comment lines, indentation or blank lines. The code in each line stays as
  written, so a browser error still points at readable code. The runtime
  sources have no text that spans lines, such as a template literal over
  several lines; a test checks that the shipped runtimes parse to the same
  program as the authored ones. The Tac client runtime is about 1.2 KB
  smaller with Brotli on every Tac route.
- With this and the hexadecimal Rust module (ADR 0035), the event-mesh
  browser fixture fits the default script budget again, and its check is on.
  Fixtures that load the web runtimes of many languages on one route, such as
  window-controls and cached-client, keep `tac.budgets.check: false`; the
  budget is not raised.
