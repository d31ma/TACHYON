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

## Amendment (2026-10-11): a route with several web runtimes

- [ADR 0100](0100-a-route-with-several-web-runtimes-gets-a-measured-allowance.md)
  changes the default script budget of a route that loads more than one web
  runtime: 200 KB plus a measured allowance for each runtime after the
  largest one. A route with no runtime or one runtime keeps 200 KB, and
  `tac.budgets.script` still wins when it is set.
- The statement above that window-controls and cached-client keep
  `tac.budgets.check: false` is no longer true. Their manifests do not set
  it, so the check is on, and the computed budget covers them.

## Amendment (2026-10-11): which services and models a route counts

- The view plan of a route lists the services that its views use and the
  services that those reach, and the size check counts the module of each
  listed service, because the page loads it when a view needs it. That stays
  as it is.
- A JavaScript or TypeScript service reaches another service when its file
  names the field that receives it, such as `middle;` or
  `this.middle = undefined`. The build cannot see the names that a write with
  a computed key adds (`this[key] = ...`, `Object.assign(this, ...)`,
  `Object.defineProperty(this, ...)`, `Reflect.set(this, ...)`), or the names
  that code in another file adds when the service hands `this` to it
  (`initialize(this)`, `helper.call(this)`, `const owner = this`). In those
  cases, and when it cannot read the file, the build lists every service of
  the project.
- `this` handed to `.bind`, `.call` or `.apply` of the service's own member
  (`this.save.bind(this)`, `this.pick(id).call(this)`) is no longer such a
  case: the method is in the same file, so the build already sees every name
  that it uses. Before, one such line made every route that reached that
  service list every service of the project.
- Each JavaScript or TypeScript service module carries only the models that
  it names (ADR 0043 amendment).
- In the CADUCEUS probe (70 services, 16 of them JavaScript, about 100
  JavaScript models), a simulation over its release output gives: every
  route except one listed all 70 services and loaded 200.4 KB of script as
  sent, at the budget. Two services bound `this` to their own methods. With
  the narrower rule a route lists 14 to 48 services and loads 133 to 172 KB.
  With the smaller service modules as well, it loads 115 to 136 KB.
