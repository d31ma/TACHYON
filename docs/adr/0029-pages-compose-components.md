# ADR 0029: Pages Compose Components

- Status: Accepted; component-owned companions superseded by [ADR 0030](0030-components-use-services.md);
  site-wide scripts and styles moved to the manifest by [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
- Date: 2026-09-15
- Amends: [ADR 0018](0018-native-hosts-run-the-platform-javascript-engine.md)
  and [ADR 0019](0019-companions-compile-for-their-target.md) (per-route
  companion selection), [ADR 0020](0020-decorator-free-authoring-conventions.md)
  (page-level literal state)
- Preserves: client-only Tac, REST-only Yon, filesystem routing

## Context

A page could own these parts:

- markup;
- literal state;
- a companion in any of six languages;
- a stylesheet.

A component could own the same set. Two kinds of owner caused three problems:

- two lifecycles to document;
- two places where behaviour could live;
- a native pipeline that only ever compiled the page's companion.

The website itself had five page companions. The whole body of each one was
one import. The website also had one page-level development note that no
component could reuse.

## Decision

A page composes components and holds nothing else.

- A page directory under `client/pages/` contains only `tac.html`. Discovery
  fails with `TY1309` when any of these files is beside a page: `tac.css`,
  `tac.js`, `tac.ts`, `tac.swift`, `tac.kt`, `tac.cs` or `tac.rs`. The
  diagnostic names the component to move the file into.
- A page's `tac.html` contains component tags, nested as the composition
  needs, plus whitespace and comments. Nothing else is allowed. These fail
  with `TY1309` in a page: text, plain elements, expressions, control flow,
  slots and `<script>` state. Custom elements from libraries count as plain
  elements. A component wraps them.
- A component under `client/components/<name>/` is the unit of view
  (`tac.html`), behaviour (`tac.js`/`tac.ts`, or a target-native companion)
  and style (`tac.css`). The target matrix of ADR 0019 applies to each
  component:
  - JS/TS reaches every target.
  - One native language is selected per target.
  - A component fails with `TY1010` when its declared companions reach none
    of the requested target.
- Site-wide scripts and stylesheets are declared once in `tac.config.js`
  (`scripts`, `styles`). A companion no longer imports them for each route.
- Native companions are activated per component instance, not per route:
  - The browser addresses a companion by component name and instance owner.
  - The generated dispatch table uses the component as its key.
  - Each activation keeps its own state.

  The page's canonical route still limits every request that the host
  accepts.
- `ty init` scaffolds (creates the starting files for) a page that composes
  components, and two components. `ty migrate check` reports every page
  companion as `companion.page`, together with the move to make.

## Consequences

- There is one owner model. A component owns state, lifecycle, resources and
  styles. A page owns a URL and a composition. The runtime re-renders a
  component once its PostConstruct has completed. The runtime keeps focus
  across a retained component root. Page-level rendering had done this
  implicitly.
- Tests, gates and published examples used to author markup into pages. They
  now author it into a component that the page places. The compat
  (compatibility) ledger and the browser gates were migrated with the rule.
  They were not exempted from it.
- Native hosts gain multi-activation preludes. Until that work is finished
  and qualified on each host, a native companion beside a component compiles
  but is not linked yet.

## Acceptance

Acceptance requires:

- discovery and composition diagnostics with source spans;
- the scaffold and the migration report;
- core, CLI and browser gates that run against composed pages;
- the website, built and verified under the rule;
- per-component native activation, verified through the existing native
  gates on each host.
