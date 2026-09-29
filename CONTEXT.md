# Tachyon Product Context

## Purpose

Tachyon is a full-stack framework. It supports many programming languages
(it is polyglot). Folders define Tac pages; annotations define Yon routes. Application
developers write standard HTML for views. Tachyon does the rest:

- it finds the source files (discovery);
- it compiles them;
- it runs the server and manages its lifecycle;
- it starts and watches handlers (handler supervision);
- it produces the web output;
- it generates native applications.

The Rust rewrite is a new implementation written from scratch. The JavaScript
framework that lived in this repository was removed after the switch to Rust
(the cutover). Compatibility is measured against two things: the release
binary v26.30.04, which never changes, and the neutral fixtures in `corpus/`.
The rewrite never imports private code from the legacy implementation.

## Domain Language

- **Tac** is the client-side layer for views and user interaction. A page's
  `tac.html` combines components. A component's `tac.html` can have a
  `tac.*` presentation model and styles in the same folder.
- **Yon** is the server-side layer for REST endpoints and handlers.
  `RequestRoute` and method route annotations on classes in
  `server/controllers/` declare paths and HTTP verbs (ADR 0048).
- **HTTPClient** is Tachyon's declarative HTTP role, like a Spring
  `@HttpExchange` interface: a contract with no implementation that Tachyon
  generates. `GetRequest` and the other request annotations give the request
  metadata. Tachyon finds browser JavaScript/TypeScript clients next
  to their entry point. It injects them with a cancellation that is bound to
  their owner. Native HTTP adapters still need explicit configuration.
- **Managed resource** is a datasource or Redis connection that the server
  owns. Private Yon configuration selects it. Tachyon injects it into
  stores that live only for one invocation. The datasource driver can be
  SQL or `fylo`, a local document store (ADR 0032; ADR means architecture
  decision record). Tac never receives database connections or credentials.
- **Application event** is local to one of these:
  - one Tac document or native activation;
  - one Yon invocation graph.

  A class injects an `ApplicationEventPublisher` and calls
  `publishEvent(model)`. The event is the model: its type name is the event
  name (ADR 0055). An OnEvent listener:
  - receives copies (clones) of the payloads;
  - listens for the model type of its one parameter, and a payload that does
    not fit that model is not delivered;
  - can filter them with a compiled `condition`;
  - runs in the sequence that Order sets;
  - can run later when it uses InBackground.

  OnStart and OnStop handle initialization and cleanup. An
  `@Needs` field named after a resource that a realm owns receives that
  resource (ADR 0038). These fields replace the removed public this.tac
  object. ADR 0028, ADR 0031 and docs/APPLICATION_EVENTS.md describe
  the limits and the rules for each language.
- **View source** is `tac.html`. It contains HTML. It can use structural
  control tags, which the client interprets. A page combines components. A
  component is a view and an optional `tac.css`.
- **Service** is Tac's unit of behaviour. It is one `@Service()` class in
  `client/services/<Name>Service.<ext>`. A view refers to it by its instance
  name (`BasketService` is `basket`). A service has one instance per page (a
  page singleton), unless it declares `@Lifetime({"value":"prototype"})`.
  Services in `js`/`ts` run in the browser. Services in
  `swift`/`kt`/`cs`/`rs` run on native targets. See ADR 0030.
- **Route graph** is the result of file-system discovery. Tachyon creates it
  before a build or server generation starts. It is deterministic (the same
  input always gives the same result) and immutable (it never changes).
- **Authored role file** contains one annotated class and its imports, with
  nested types and same-type Rust impl blocks allowed (ADR 0044). Yon handler,
  worker and root middleware files follow the same rule. Yon route marker
  files are retired; each controller is an entry point. Root middleware is a `Middleware` class with `before(request)` and
  `after(request)`. A test selects a TestSetup from a separate
  same-language sibling file; it is never bundled for clients.
- **Control tag** is `if`, `else`, `for` or `loop`. It is compiler syntax, and
  the Tac browser renderer interprets it. A browser or native renderer never
  receives it as an unknown HTML element.
- **Tac component** is a framework component. Tachyon finds it through the
  project's component graph.
- **Web component** is a custom element that follows the web standards. It can
  render on the web or use a registered native adapter.
- **View IR** is the canonical intermediate representation (IR) of a view. It
  does not depend on a platform. Tachyon produces it after these steps:
  parsing, validation, finding components, and turning control tags into
  simpler structures (lowering).
- **Native adapter** maps a View IR element or component to a basic building
  block of a native platform.
- **WebSurface** is a separate part of the view that a native application
  renders locally with web technology. Tachyon uses it when no native adapter
  exists for that part. It is not a render mode for the whole application.
- **Capability** is an explicit permission to use a sensitive resource of the
  host or platform. By default, access is denied.
- **Buildable**, **simulator-tested**, **native-tested**, **preview** and
  **supported** are separate evidence levels. `docs/SUPPORT_TIERS.md` defines
  them.

## Relationships

1. Project discovery produces a route graph.
2. The HTML frontend parses `tac.html` into syntax that knows its source
   location.
3. Validation resolves control tags, bindings and components.
4. Lowering produces deterministic View IR.
5. Web code generation reads View IR and produces web artifacts.
6. Native planning reads View IR. For each subtree, it chooses a native
   adapter or a WebSurface boundary.
7. The server calls Yon handlers through a versioned protocol. The environment
   selects a supervised isolation backend for them. The server validates their
   HTTP responses and passes them on without changes.
8. Packaging records the inputs, toolchains, contract versions, outputs and
   digests in an artifact manifest.

## Non-Negotiable Invariants

- HTML is the only view language that application developers use.
- Only the client renders Tac. The server never renders a Tac expression,
  branch, loop or component subtree.
- Yon supports REST only. It never renders a template, and it never runs
  during a build.
- When Yon returns HTML, the response must set `Content-Type: text/html`
  explicitly. Tachyon sends that HTML as it is. It does not interpolate it or
  inject framework markup into it.
- Control tags never reach a browser or native renderer as unknown elements.
- Source discovery, route order, diagnostics, IR and artifacts are
  deterministic: the same inputs give the same results.
- A route can contain several `yon.*` handlers at the same level. They run in
  a deterministic source order.
- When native rendering does not support some content, only the smallest safe
  subtree falls back.
- When one subtree falls back, its supported sibling subtrees stay native.
- Remote web content never gets a bridge to native capabilities.
- Capabilities are denied by default. Each capability is limited to one
  resource and one target.
- Handler input, output, runtime, environment, concurrency and lifetime all
  have limits.
- Tachyon never starts a handler process through a shell.
- Public schemas and diagnostic codes are contracts. Each has its own version.
- Cross-compilation proves only that the code builds.
- No platform or feature is called supported until its evidence is published.

## Explicit Phase 0 Decisions

- The new core is written in Rust. It is one program divided into modules (a
  modular monolith).
- Public machine contracts use JSON Schema Draft 2020-12.
- Handler envelopes use a JSON process protocol. Each message starts with its
  length (length-prefixed).
- Native fallback happens per subtree. It prefers a local bundle first.
- The current CLI and fixture behavior are the baseline for compatibility.

## Current Implementation Boundary

Engineering phases 0–7 are implemented. The current Rust data path does these
steps:

1. It finds static and dynamic `tac.html` routes and `yon.*` REST handlers.
2. It parses, with size limits: HTML, expressions, control tags, components,
   slots, events, styles, the schedules for mounting browser components, and
   the services that a view names.
3. It validates Yon endpoints. These are plain `Handler` endpoints, or
   annotated controllers with scoped provider plans, composed from role files
   in the same language (ADR 0044). It lowers constructor injection without running
   application code. It supervises JavaScript, TypeScript, Python, Java, C#,
   Kotlin, PHP, Rust, Dart and Go endpoints. Go endpoints use the Rust
   annotation spelling (ADR 0045). It starts them only when an HTTP request, a
   middleware phase, a worker tick or an explicit invocation reaches them.
4. It produces deterministic Tac client render plans, View IR, source maps,
   client modules, shared assets, a service worker and Route Manifest v1.
5. A development server that is safe on loopback sends requests to exact and
   dynamic HTTP routes, before/after middleware, workers and durable SSE topic
   logs.
6. It prepares the same client-rendered bundle for web-view hosts on macOS,
   iOS, Android, Linux and Windows. It selects one native language per service.
7. It compiles the generated host and the selected native services with the
   platform toolchains. It publishes the capability and artifact manifests
   atomically.
8. Executable gates check compatibility, recovery, performance, supply-chain
   and release lifecycle behavior.

The development server also publishes Hot Update Protocol v1. Tac's
production client renderer owns the initial structure and the reactive
structure. Development updates must target that render-plan boundary, or
reload safely. They must not bring back SSR (server-side rendering) islands
as owners of the structure. See ADR 0015.

ADR 0022 adds optional behavior annotations to every supported Tac/Yon
language. It provides:

- cache, persistence and HTTP adapters that the application selects;
- explicit configuration and constructors;
- pagination with limits.

ADR 0046 makes the annotations, lifecycle names, `Behaviors` and the framework
types global in every language. No source file imports them, and JS/TS refuses
an import from `tachyon:behaviors` with `TY1308`. The markers are erased at
compile time; the types are ambient declarations only.

Contexts have explicit partitions and cancellation lifetimes. There is no
global container. Tac never receives server secrets. Tac JS/TS services,
native Swift/Kotlin/C#/Rust services and Go web services have their own
execution adapters. Yon adds Python/PHP/Java to those shared languages.
Deadline checks in native and synchronous code are cooperative: the code must
check them itself. The owner
disposes of them explicitly. The existing Yon controller and dependency
injection (DI) support for eight languages does not change.

The shared extensions add:

- cache changes that run after success;
- explicit typed configuration;
- activation by profile or property;
- DTO (data transfer object) validation, with each DTO in its own file that
  discovery composes into the unit that uses it (ADR 0044);
- Store/Query methods bound to an entity, with queries derived at
  compile time.

Query execution requires explicit entity authorization. It passes limited,
structured data to adapters that the application selects. Tachyon adds no SQL
interpolation, no reads of the ambient environment, no ambient annotation
scanning and no transaction manager. Types that have a role must use the
required suffixes, matching file names and the shared role folders of their
realm. Tachyon finds the same-language declarations in those folders from the
captured entry scope, before it lowers each source file. Tachyon rejects
legacy module manifests. See `docs/BEHAVIOR_PROJECT_STRUCTURE.md` and ADR
0027. ADR 0027 keeps the naming rules of ADR 0024, but replaces the module
selection that developers wrote by hand.

ADR 0025 adds constructor inference from required fields for managed Yon
roles. NeedsAll:

- selects the instance fields that have no initial value;
- infers the concrete provider tokens;
- generates the construction code, so developers write no `Needs` lists or
  constructors.

Default values and static state stay as they are. When the choice is
ambiguous, the build fails; it never guesses. Invocation-local scopes, Tac
ownership and the private configuration boundaries do not change. ADR 0027
replaces the original explicit module selection.

ADR 0026 makes ordinary JSON-compatible Yon return values work without the
optional HTTP response helper. Configuration v2 works like this:

- Settings that are not secret go under tac/yon.
- Private literal Yon secrets go in .env.
- In names with a prefix, underscores mean nested paths.
- Builds publish only the captured base tac projection at
  /.tachyon/application.json. Builds never open .env.
- The resolved Yon snapshot is private. It changes only on restart.

ADR 0027 adds managed datasource/Redis injection in Yon, and the construction
of browser Clients. It does this without a global dependency container.

ADR 0029 makes a page only a composition of components. ADR 0030 moves
behaviour out of the component directory into `client/services/`. A component
view names the services it uses, and the runtime loads only those services.
`Service` and `Lifetime` are Tac markers as well as Yon markers. The host
activates native services once for each browser-side service instance, and
refers to them by instance name.

ADR 0031 makes the two realms mirror each other:

- Each realm has shared role folders, `client/<role>` and `server/<role>`.
  Data types without a role live in `client/models/` and `server/models/`,
  in every service language except JavaScript (ADR 0041, ADR 0043).
  Every route uses them.
- A Yon route directory holds only its `yon.<ext>` entry and its
  `controllers/` folder. A role file next to a route fails with TY1310.
- Only `Controller` and `Stream` are still Yon-only. Tac gained `Component`
  and bare `Store`. Both realms inject with `@Needs` (ADR 0038).
- Events have the same shape as Spring's. `EventPublisher` is removed. A class
  injects an `ApplicationEventPublisher` and calls `publishEvent(model)`.
  Listeners hear the model type of their parameter and have `condition`, `id`,
  `Order` and `InBackground` (ADR 0055).

ADR 0032 adds `fylo` as a datasource driver. It is a local document store
behind the same repository commands. It uses one subprocess that the server
owns, and it needs no credentials.

ADR 0047 gives every annotation a plain English name, such as `Needs`,
`Store`, `Setup` and `OnStart`. The names no longer copy Spring, Lombok or
JUnit. The role folders follow: `stores/`, `setups/` and `converters/`. An old
name fails with TY1308, and an old folder fails with TY1310. Each message
names the new name. `ty migrate fix --write` renames markers, moves files and
renames role types.

ADR 0052 makes `client/manifest.json` the whole Tac configuration. Its
standard members give a native build its name, entry route and icons. Its
`tac` member holds the application id, version and window, the shared scripts
and styles, the service-worker cache rules, page metadata, the site origin
and the sitemap values. `tac.config.js` and `tachyon.json` are removed and
stop the build; the build writes `sitemap.xml` and `robots.txt` itself. The
manifest is public, so none of these values may be secret.

The implementation is not yet a supported release. Buildable,
simulator-tested, native-tested, preview and supported remain separate
evidence levels in `docs/SUPPORT_TIERS.md`.

## Deliberately Deferred Decisions

These are not hidden gaps in a finished compatibility claim. Each one is a
named decision about the product or about whether something is viable:

- a first-party Firecracker control program and host profile that are ready
  for production. The transport boundary that the environment selects is
  implemented, but it does not prove isolation at the operating-system level
  by itself;
- a stable plugin ABI (application binary interface) for third-party
  compilers or native adapters;
- production signing identities, and the publication policy for each store;
- whether the legacy OpenAPI and telemetry products, which are out of scope,
  come back as separate products with their own owners.

When the compatibility or security effects of a deferred decision become
concrete, it needs an ADR or RFC (request for comments). Nobody may add it
to production code quietly without one.

## Current Non-Goals

- copying the legacy client renderer, which had no limits, or its mutable
  global component registry;
- implicit imports from the application's dependency graph inside handler
  adapters;
- pooling or reusing handler processes;
- giving remote or local WebSurfaces a bridge to native capabilities;
- treating a successful cross-compilation as evidence that the native runtime
  works;
- publishing a production release before the cutover gate passes.
