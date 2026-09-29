# ADR 0022: Shared Tac and Yon Behavior Adapters

- Status: Accepted; executable polyglot integration
- Date: 2026-09-12
- Supersedes: ADR 0020's blanket prohibition on Tac annotations
- Preserves: ADR 0015 client-only rendering, ADR 0016 REST-only Yon, ADR 0021's HTTP verb method convention
- Amended by: [ADR 0044](0044-an-annotated-file-holds-one-class.md): "source-local" means the unit that discovery composes, and a behavior-only class, such as a `Validated` input, lives in its own file in a role folder
- Amended by: [ADR 0046](0046-tachyon-annotations-are-global.md): the annotations and types are global in every language, and JS/TS refuses an import from `tachyon:behaviors`
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them
- Amended on 2026-09-29 (see the end): a `Store` is a contract with no implementation, like a Spring Data repository interface; a bare Store is refused

## Decision

Tachyon implements behavior annotations. They work over cache, persistence and
HTTP adapters that the application selects. The design has two parts:

- one shared Rust frontend, which never executes code;
- execution adapters that are implemented separately and stay local to the
  source.

The supported paths are:

- JS/TS Tac components;
- Swift/Kotlin/C#/Rust native Tac component companions;
- all eight Yon languages (JS/TS/Python/PHP/Java/Kotlin/C#/Rust).

The native behavior adapters have no embedded JavaScript interpreter. They
also use no cross-language RPC (remote procedure call). The existing Yon
controller/DI (dependency injection) support stays a separate contract that
applies only to Yon.

Use named imports from `tachyon:behaviors`. They give accurate TypeScript
types that are local to the source. They also do not hide (shadow) the
browser's global `Cache` API. The compiler erases these virtual imports and
annotations. During discovery or bundling, the compiler does not execute a
configuration, constructor, adapter or application method.

Applications create a behavior context explicitly. The context has a partition
and an abort signal. The application assigns the context to service instances.
No global mutable container exists. Tac passes its mount signal and owns the
browser/API adapters. Yon owns invocation contexts. Yon selects external
storage for state that must last across requests. No browser adapter receives
server credentials or new native capabilities.

Caching requires:

- an explicit authorization hook, which runs even on cache hits;
- selected key arguments that are stable;
- JSON values and expiry times that have limits.

Cacheable may wrap provider methods. It may not wrap HTTP verb endpoints.
Pageable validates its input. Entity/Table provide mappings to explicit
repository operations. They generate no DDL (data definition language, the
statements that create or change database tables). HttpExchange is for
outbound requests only. Constructor generation uses explicit field lists.
TypeScript uses a validated factory instead, because decorators cannot extend
constructor signatures.

The runtime limits active operations, arguments, serialization, deadlines and
responses. Adapters that the application selects are still trusted code. They
must respect cancel signals, storage quotas, streaming response limits and
transport policy. These controls are not browser process isolation. They are
also not a database ORM (object-relational mapper).

## Extension: bounded policy families

The same source-local contract extends to these annotation families:

- CachePut/CacheEvict;
- object-form Repository/Query;
- ConfigurationProperties/Profile/ConditionalOnProperty;
- Validated/Valid/NotNull/NotBlank/Size/Min/Max.

In total, there are 22 shared names.

Cache mutations authorize and validate their target before application code
runs. They set or delete an entry only after these checks pass: the result is
successful, the result is within its limits, and the lifetime checks pass.
Cache mutations are not atomic database/cache transactions.

Repository queries lower empty methods into one of two forms:

- bounded structured commands that use only AND;
- derived names that have exactly one meaning.

The required authorizeEntity hook runs before the selected entity adapter.
There is no SQL/expression evaluator, package scanner or implicit database
connector. A bare Yon Repository keeps its separate DI meaning.

Property binding and DTO (data transfer object) validation are pure, explicit
factories. They execute no constructor. Before configure runs, configuration
selection takes a snapshot of the explicitly supplied profiles and
framework-prefixed string values. Validation:

- applies only to direct JSON fields;
- measures Size in UTF-8;
- uses a specified set of Unicode whitespace characters;
- returns field/rule errors with the values redacted.

Browser validation is not server authorization. Transactional stays deferred.
It waits until real adapters own transactions and their failure semantics
exist.

ADR 0024 supersedes the optional role-folder taxonomy. Role declarations have
strict names and placement. Explicit module lists assemble their declarations
and the schemas they reference into one source-local behavior unit. Ordinary
adapters stay owned by the application. See
[layout guidance](../BEHAVIOR_PROJECT_STRUCTURE.md).

## Consequences and Acceptance

These stay valid: plain authoring, explicit lifecycle and signals, and existing
Yon responses. The compiler still rejects retired Tac lifecycle decorators and
redundant HTTP verb decorators.

Native-language markers are compiler syntax. They are not annotations of a
reflection library. The compiler validates their placement on their own line
and their literal options before the host compiler runs.

Synchronous adapters check cancellation and deadlines cooperatively at
boundaries. They cannot interrupt (preempt) an application call that is
blocked. Native Tac owners dispose contexts explicitly. Browser mount
cancellation is not implicitly connected to native companion lifetimes. The
public behavior guide records the limits of the runtime, native typing and the
source-local registry.

Acceptance requires:

- parser failures before execution;
- semantic type tests in both TypeScript decorator modes;
- real supervised Yon calls;
- emitted Tac browser execution and cancellation on unmount;
- adapter boundary tests and repository gates.

Polyglot CI (continuous integration) also compiles and executes real JVM (Java
virtual machine), .NET, Swift and Rust fixtures. It also runs supervised
Python/PHP endpoints. It requires a toolchain preflight check. See
[the behavior contract](../BEHAVIOR_ANNOTATIONS.md).

## Amendment (2026-09-29): a Store is a contract

`Store` follows Spring Data repository interfaces: it names its Entity, it has
no implementation, and Tachyon generates each query.

- A store is declared in the bodiless form of its language, the same form as
  an `HTTPClient` (ADR 0048 amendment): an `interface`, a Go interface, an
  `abstract interface class`, a `trait`, a `protocol`, a `Protocol` class, an
  F# type with only `abstract` members, or a class with empty methods in
  JavaScript, TypeScript and Ruby. Each method is a derived query
  (`findBy…`, `countBy…`, `existsBy…`) or a `@Query`.
- The build fails with `TY1308` for a Store without an Entity (a Spring Data
  repository always names its entity), a class store where the language has a
  contract form, any method body, a field, a constructor, a method that is not
  a query, and a store that also carries another role: "A Store has no
  implementation: Tachyon generates each query. Move the logic into a
  service." A bare Store no longer means a Yon provider with hand-written
  methods; that code moves into a service, and SQL through a `DataSource`
  moves into a service that asks for the `DataSource`, like a Spring service
  with a `JdbcTemplate`.
- Before events, lifecycle and injection read the role files, the compiler
  turns each contract into a concrete store whose behavior context comes from
  a Setup: `{"setup":"CatalogSetup"}` in the Store options, by default the
  store name with `Setup` instead of `Store`. In Yon the store is a provider
  that `@Needs` injects. A Tac service in Dart or a WebAssembly language
  builds it with its constructor; a Tac JavaScript or TypeScript service
  passes a context that it created, with `Behaviors.construct`, because a
  browser creates a context asynchronously.
- A first slice kept one exception: a Store that declared an `@AllOrNothing`
  method kept its class form, because a transaction lived in one behavior
  context. The next amendment ends it.
- `ty migrate fix --write` rewrites a class store whose methods are only
  queries; a store with logic, state or no Entity is a manual step.

## Amendment (2026-09-29): a transaction follows the call

- An open transaction no longer lives in one behavior context. Each runtime
  keeps the open transactions of the current call in call-local storage, keyed
  by entity adapter name: AsyncLocalStorage on the JavaScript server (one
  instance shared by every module through
  `globalThis[Symbol.for('tachyon.behavior.transactions')]`; the browser keeps
  the context-local form), `ThreadLocal` in Java, Kotlin and Groovy,
  `AsyncLocal` in C# and F#, a `ContextVar` in Python (one variable shared by
  every module through `sys`), fiber storage in Ruby (a class variable in
  mruby), a zone value in Dart, a `@TaskLocal` in Swift, a thread local in
  Rust, one guarded entry for each goroutine in Go (TinyGo reports no
  goroutine number, so its single-threaded browser build shares one entry),
  and a static in PHP, which runs one request at a time.
- `@AllOrNothing` joins when its adapter already has an open transaction
  (Spring's propagation REQUIRED). Otherwise it sends `begin`, runs the body
  with that adapter added, then restores the outer state and sends `commit` or
  `rollback`. A Store or Query command carries `transaction` only when its
  entity's adapter has an open transaction; before, any open transaction of
  the same context was sent, also to another adapter.
- So a service method with `@AllOrNothing` covers the stores it injects: a
  failure in the store's second query rolls back the first. The Store
  exception above ends: an `@AllOrNothing` method on a Store is refused with
  `TY1308` like any other method that is not a query, and
  `ty migrate fix --write` leaves that logic for a person to move into a
  service. Each runtime has a test with two behavior contexts in which the
  second write fails and the first is rolled back.

## Amendment (2026-09-29): a Converter is a contract

- `@Converter` joins `@HTTPClient` and `@Store` in the declared contract
  table: the same bodiless form per language, like a MapStruct `@Mapper`
  interface. Each method is a `@Convert` signature, and the existing mapping
  generation writes its body. A method body, a field, a constructor, a method
  without `@Convert`, or a class form where the language has a contract form
  fails with `TY1308`: "A Converter has no implementation: Tachyon generates
  each mapping."
- The concrete converter is still built by each language's behavior lowering,
  as before; the contract lowering only adds the `@Needs behaviors` field of a
  JavaScript or TypeScript converter, whose caller passes its context with
  `Behaviors.construct`. `ty migrate fix --write` rewrites a class converter.


## Amendment (2026-09-29): a Tac service asks for its store

- A Tac service asks for an entity store with `@Needs`, like a Yon service,
  in JavaScript, TypeScript, Dart, Kotlin, Swift, Java, C#, Rust and Go. It
  no longer builds the store. The store still opens the behavior context of
  its Setup, so a Tac store is a provider with no dependencies.
- JavaScript and TypeScript: the store is a provider in the service's owner
  graph. It gets a static `tachyonContext(signal)` that creates the context
  from its Setup, and the owner creates that context before it builds the
  service, because a browser creates a context asynchronously. The owner's
  signal ends the context with the owner. A store no longer has a
  `@Needs behaviors` field, so `Behaviors.construct(ProductStore, …)` is
  gone. A root service that injects a store must not declare its own
  `behaviors` member, like any managed Tac root.
- Dart builds the store with the service, as it builds other collaborators.
- Kotlin, Swift, Java and C#: the field that needs a store gets a new store
  as its private initial value, and the `@Needs` marker is removed before
  the build. Rust: the Tac store implements `Default`, the field is a
  `std::sync::Arc<ProductStore>` (as in Yon), and the service fills it with
  `Default` (derive it, or write no `fn new`). Go: Tachyon
  adds a method that builds each needed store with `New…Store()`, and the
  service's table calls it before the service starts.
- Ruby and F# Tac services cannot ask for a store yet; that is planned.
