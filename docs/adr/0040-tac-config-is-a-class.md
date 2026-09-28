# ADR 0040: tac.config Is a Class With Static Fields

- Status: Superseded by [ADR 0052](0052-the-manifest-is-the-tac-configuration.md);
  `tac.config` is removed and its values live in `client/manifest.json`
- Date: 2026-09-24
- Amends: [ADR 0029](0029-pages-compose-components.md) (how `scripts` and
  `styles` are declared) and [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md)
  (how the native `application` identity is declared)
- Preserves: every value the file declares, its limits and checks, and the
  order in which the manifest, `tac.config` and `tachyon.json` apply

## Context

The project rules now say that a file declares no variables at file level.
Every variable belongs to a class. The one exception is the code that creates
the framework's decorators and annotations.

`tac.config.js` (or `.mjs`, `.ts`) broke this rule by design. The framework
read these named exports from it:

- `application`: the native application's name, id, version and entry route.
- `scripts` and `styles`: the files linked into every document.
- `metadata`: the head tags of each route.
- `cache`: the read policies of the service worker.
- `postBundle`: a function that runs after a build.

Two older readers also took values from page files: the page state script
(`<script>let count = 0</script>` in a page's `tac.html`) and the named
exports of a page's client module. Since ADR 0029 no project can reach them,
because the build rejects a script or module next to a page.

## Decision

- `tac.config` default-exports a class. The values are static fields of that
  class: `static application`, `static scripts`, `static styles`,
  `static metadata` and `static cache`. The hook is a static method:
  `static postBundle(context)`.
- A static field may hold a function, as a named export could before. The
  framework calls it with the class as `this` and awaits the result.
- The build rejects the old forms with diagnostic TY1601:
  - A named export with one of these names. The message names the static
    field to write instead.
  - A default export that is not a class, for example an object literal.
- The unreachable readers of page state scripts and of a page module's named
  exports are removed.
- The scaffold, the website, the documentation and the examples use the class
  form.

## Consequences

- A project that uses named exports in `tac.config` stops building until they
  move into the class. The diagnostic says what to write.
- The values, their limits and their checks do not change. Only where they are
  declared changes.
- Code that the compiler generates into an application (for example the event
  plan of a lowered service) is not a contract that users write. It is part of
  how the framework creates its annotations, so this decision does not change
  it.
