# ADR 0020: Decorator-Free Tac and Yon Authoring Conventions

- Status: Accepted with annotation prohibitions superseded by [ADR 0021](0021-yon-annotation-frontend.md) and [ADR 0022](0022-shared-behavior-adapters.md); the `mount()` hook and `this.tac` signals are replaced by [ADR 0028](0028-owned-application-events-and-lifecycle.md); the `tac.js`/`tac.ts` presentation-model class is superseded by `client/services/<Name>Service` under [ADR 0030](0030-components-use-services.md); the marker sets and event model are amended by [ADR 0031](0031-realms-mirror-each-other.md); plain authoring remains valid
- Date: 2026-09-10
- Supersedes: the authoring model in [ADR 0017](0017-mandatory-yon-layers-relay-and-streaming.md)
- Amended by: [ADR 0048](0048-yon-routes-come-from-annotations.md): Yon routes come from annotations in `server/controllers/` (`RequestRoute`, `GetRoute`, ...), not from `server/routes/` folders; clients use `HTTPClient` and `GetRequest`, ...; `OPTIONS.schema.json` is replaced by schemas built from models

## Context

Tachyon's decorators made framework metadata the main way to organize both
halves of an application. The decorators had these costs:

- each language needed its own spelling;
- they needed compiler or runtime support;
- they needed generated annotation declarations;
- they put framework knowledge into application modules that were otherwise
  ordinary.

A separate streaming marker could also disagree with the generator that it
described.

The cost was largest in two places. Yon only dispatches REST endpoints. In
Tac, fields and methods already form a natural presentation model.

The authoring contract must meet four goals:

- It is easy to learn in every language.
- It hides the protocol details.
- It runs without application dependencies.
- The compiler can still find it statically, before a request chooses
  ordinary HTTP or SSE (server-sent events).

## Decision

### Tac

Tac uses a presentation-model convention with no dependencies:

- `tac.js` or `tac.ts` default-exports a plain class;
- fields are view state, and methods are view behavior;
- the optional `mount()` method runs initialization after persistence and
  runtime attachment;
- `$field` and `$$field` keep their meanings: per tab and across sessions; and
- signals are explicit calls, scoped to their owner, to
  `this.tac.publish(...)` and `this.tac.subscribe(...)`.

### Yon

Yon uses one naming convention in all eight languages it owns. The convention
replaces configuration. A `yon.*` route declares a plain type named `Handler`.
The static or associated methods of that type that have HTTP method names are
the endpoints.

Tachyon supplies process startup, the request and response protocol types, and
bounded execution. Application services, repositories, clients, delegates,
functions, and modules have no framework role. They use the host language
normally.

### Streaming and relay

A generator HTTP method is streaming by definition. Its native `yield` syntax
is the declaration. So no second marker can drift away from the
implementation.

Yon invokes programs outside its own languages explicitly with
`relay(command, request)`, or with the Yon equivalent in the casing of the
language. Relay keeps the process behavior accepted in ADR 0017: no shell,
bounded, sanitized, and aware of deadlines.

### Dependencies

This decision adds no package and no new dependency. JavaScript defaults to
Node. Tachyon still probes every runtime override for its capabilities.

## Consequences

- `@onMount`, `@publish`, `@subscribe`, the Yon layer decorators, `@Relay`, and
  `@Stream` are retired. Active uses fail with migration guidance. The
  compiler does not silently ignore them.
- Layer suffixes are removed. The framework no longer enforces the direction of
  application dependencies. Teams may adopt a domain architecture without tying
  it to Yon.
- Runtime adapters no longer install decorator shims. Compiled handlers no
  longer build support for annotations or procedural macros.
- Lifecycle and signal effects are visible where they happen. Authors can still
  organize more complex Tac state behind ordinary methods. Simple pages do not
  need a reducer.
- Existing release tags do not change. This change breaks compatibility as part
  of the rewrite from scratch. Its compatibility ledger records the intended
  difference.
