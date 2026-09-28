# ADR 0030: Components Use Services

- Status: Accepted; native service activation qualification in progress
- Date: 2026-09-15
- Amends: [ADR 0029](0029-pages-compose-components.md) (component-owned
  companions), [ADR 0019](0019-companions-compile-for-their-target.md)
  (where the target matrix applies), [ADR 0020](0020-decorator-free-authoring-conventions.md)
- Preserves: client-only Tac, REST-only Yon, filesystem routing
- Amended by: [ADR 0057](0057-needs-all-requires-its-fields.md) (NeedsAll
  requires every field without an initial value)
- Amended by: [ADR 0031](0031-realms-mirror-each-other.md), which mirrors these
  shared role folders and markers in the server realm;
  [ADR 0033](0033-dart-browser-services.md), which adds Dart as a browser
  service language

## Context

ADR 0029 made a component the unit of view, behaviour and style. The behaviour
lived in a `tac.js`, `tac.ts` or target-native `tac.*` file beside the view.
That tied one script to one view:

- When two views needed the same behaviour, each view had to copy it.
- When one view needed two behaviours, they had to be merged into one class.

Yon already solves this on the server. It has a `services/` folder of
`<Name>Service` classes, and any handler can inject them. Tac now does the
same.

## Decision

A component is a view and its style. Behaviour is a service.

- A component directory under `client/components/<name>/` holds `tac.html`
  and, optionally, `tac.css`. A `tac.js`, `tac.ts`, `tac.swift`, `tac.kt`,
  `tac.cs` or `tac.rs` file beside a component view fails discovery with
  `TY1310`. The error names the service file to move the code into.
- Services live in `client/services/`, with one class per file:
  - The file is named `<Name>Service.<ext>`.
  - The class is named `<Name>Service`.
  - The class is annotated `@Service()` and is the default export.
  - `js` and `ts` are the browser languages. `swift`, `kt`, `cs` and `rs` are
    the native ones. A service has one file for each language it reaches.
  - The target matrix of ADR 0019 applies to each service.
- A service's collaborators live in the sibling role folders `client/clients/`,
  `client/repositories/`, `client/configurations/`, `client/entities/` and
  `client/delegates/`. `@RequiredArgsConstructor` injects them, exactly as it
  does for a Yon service. When a service declares an uninitialised field named
  after another service, it receives that service.
- A view addresses a service by its instance name. The instance name is the
  class name without the `Service` suffix, with a lowercase first letter.
  `BasketService` is `basket`, so a view:
  - reads `{basket.count}`;
  - binds `on:click="basket.add()"`;
  - assigns `on:input="basket.query = $event.target.value"`.

  Loop bindings and the properties that a parent passes stay unqualified.
- A service is a page singleton. The runtime creates it on first use and
  initialises it once. Every component that names it shares it. The runtime
  disposes of it when the user leaves the page.
  `@Scope({"value":"prototype"})` makes one service per component instance
  instead. The runtime constructs it with that instance's properties and
  disposes of it with that instance.
- Lifecycle and event markers (`PostConstruct`, `PreDestroy`,
  `EventPublisher`, `EventListener`, `Resource`) and `$field` persistence
  belong to services. A component's plan names the services that its view
  uses, so the runtime loads only those services.
- The host activates native services once per browser-side instance, the same
  way ADR 0029 activated components. Requests carry `service` instead of
  `component`. An unknown service answers `TY_NATIVE_SERVICE`.

## Consequences

- The compiler emits browser modules below
  `dist/.tachyon/services/<Name>Service.js`. Components emit no module.
- `ty migrate` reports every companion beside a component as `Changed`, with
  its service destination.
- Existing per-instance component state moves to a prototype-scoped service,
  or to keyed state on a singleton.
