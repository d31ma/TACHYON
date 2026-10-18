# ADR 0022: Shared Tac and Yon Behavior Adapters

- Status: Accepted; executable polyglot integration
- Date: 2026-09-12
- Supersedes: ADR 0020's blanket prohibition on Tac annotations
- Preserves: ADR 0015 client-only rendering, ADR 0016 REST-only Yon, ADR 0021's HTTP verb method convention
- Amended by: [ADR 0044](0044-an-annotated-file-holds-one-class.md): "source-local" means the unit that discovery composes, and a behavior-only class, such as a `Validated` input, lives in its own file in a role folder
- Amended by: [ADR 0046](0046-tachyon-annotations-are-global.md): the annotations and types are global in every language, and JS/TS refuses an import from `tachyon:behaviors`
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them
- Amended on 2026-09-29 (see the end): a `Store` is a contract with no implementation, like a Spring Data repository interface; a bare Store is refused
- Amended on 2026-09-30 (see the end): a Setup is configured once and every context from it shares its adapters, like a Spring `@Configuration`

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
  in JavaScript, TypeScript, Dart, Kotlin, Swift, Java, C#, Rust, Go and
  Ruby. It no longer builds the store. The store still opens the behavior context of
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
- Ruby: the field is an `attr_reader` documented with
  `# @return [ProductStore]`; Tachyon replaces it with a private reader that
  builds the store once, and the Tac Ruby store opens its context in
  `initialize`.
- F#: F# refuses field `Needs`, so an F# service takes the store as a
  primary constructor parameter (`type NotesService(notes: NoteStore) =`).
  Tachyon removes that parameter and adds `let notes = NoteStore()` as the
  first line of the type, so the service builds the store, and the F# store
  opens the context of its Setup like the C# store.

## Amendment (2026-09-29): the Repository base and derived deletes

- A Store may extend `Repository<Entity, Id>`, like Spring Data's
  `CrudRepository`, in the contract form of its language (an `extends`,
  `implements` or `:` base, a Go embedded interface, a Rust supertrait, an F#
  `inherit` line, a Python base before `Protocol`, a Ruby superclass). It then
  gets `findById`, `findAll(page)`, `save` and `deleteById` without declaring
  them. `ContractCheck` refuses the base on a type that is not a Store and a
  Repository that names another Entity than its Store (`TY1308`).
- The compiler removes the base and appends the four methods from one
  template row per language. `findById`, `findAll` and `deleteById` are
  bodiless derived queries, so the normal query lowering writes them; `save`
  calls `behaviors.repository(Entity).save(entity)`. No runtime type named
  `Repository` exists: only `types/tachyon-behaviors.d.ts` declares the class,
  so TypeScript and JavaScript type-check `findById(id: Id): Promise<Entity |
  null>` and the other results.
- Query plans gain two operations. `findOne` sends the adapter a `find` with a
  page of one and answers the first row or null; the exact derived name
  `findById` (`find_by_id`) is a `findOne`, and the tail `Id` of `findById`,
  `existsById` and `deleteById` binds the Entity's Id field whatever its name.
  `delete` sends the adapter `{"kind":"delete"}` and answers the number of
  removed rows; `deleteBy…` (`delete_by_…`) derives it. Both take no page and
  no ordering. `findAll` (`find_all`) is a `find` with no predicate whose one
  parameter is the page. Every runtime (JavaScript, Python, Ruby, PHP, Java,
  Groovy, Kotlin, C#, F#, Dart, Swift, Rust, Go) and the built-in adapters
  (managed SQL `DELETE … WHERE`, FYLO, IndexedDB and its in-memory mirror)
  implement them. A user-written `findById` that expected a list now gets one
  record or null.
- Typed entity results in the compiled languages (reading a JSON row into the
  Entity class) are not part of this amendment; results stay the runtime's
  JSON values, as for every other query.

## Amendment (2026-09-30): a service asks for its converter

- A Converter needs no behavior context. A mapping reads only its argument,
  so a Converter is a plain type with no constructor argument, like a
  MapStruct mapper that Spring injects. This replaces the context that the
  first converter amendment gave it: `new ProductConverter(context)`,
  `ProductConverter::new(context)`, `&ProductConverter{behaviors: context}`
  and `Behaviors.construct(ProductConverter, {behaviors: context})` are gone.
  `ty migrate` reports code that still builds a converter with a context
  (`converter.built_by_hand`).
- Every language maps without a context. Java, Groovy and Kotlin already
  called the static `Behaviors.mapValue`; C#, F#, Swift and Dart call the
  static `BehaviorContext.MapValue` / `BehaviorContext.mapValue`; Rust and
  Go call `map_value` and `BehaviorMapValue` on the value. JavaScript,
  Python, PHP and Ruby get a context-free wrapper for a method whose only
  policy is a mapping, as a gateway method already had. A type whose methods
  are all Convert (or Gateway) gets no `behaviors` field and no constructor.
- Yon: `@Converter` is a provider role, like `@Helper` (Spring's
  `@Component`), so a controller or service asks for a converter with
  `@Needs`, an F# constructor parameter, or `#[Needs]` above a Ruby
  `attr_reader`. The container reads a class, so the contract lowering turns
  the interface form into a class in Java, Groovy, Kotlin, C#, PHP, Dart,
  Rust and Go; Python, F# and Swift keep their existing lowering.
- Tac: the injection that builds a Store for a `@Needs` field also builds a
  converter from `client/converters` (a private initial value in Kotlin,
  Swift, Java and C#, `Arc` plus `Default` in Rust, a `New…Converter()`
  call in Go, a private reader in Ruby, a `let` binding for an F#
  constructor parameter). A JavaScript or TypeScript owner resolves a needed
  converter as a provider; Dart builds it with the service.

## Amendment (2026-09-30): a Setup is configured once

- A Setup is a singleton, like a Spring `@Configuration`. Every runtime
  keeps one configuration for each Setup and selection (the sorted profiles
  and the sorted properties of the context options). `Behaviors.create`
  runs `configure` only when no open context holds that configuration yet;
  otherwise the new context uses the same adapters. Each context counts as
  one user, and `dispose` (once, also when the Rust context is dropped)
  releases it. The last release forgets the configuration, so the next
  context runs `configure` again.
- Lifetime: Tac configures a Setup once per page, because a page's clients,
  stores and services keep their contexts open until the owner is disposed.
  Yon configures it once per server process. A test unit disposes its
  context after each case, so every case still starts with a fresh Setup,
  as if each test had Spring's `@DirtiesContext`.
- This replaces "No global mutable container exists" above for Setups only:
  each runtime holds one registry of open Setup configurations (a static in
  its `Behaviors` type, a package variable in Go, a function-local static in
  Rust, a `globalThis` map in JavaScript). It holds only what open contexts
  already use, and the registry is still not a general bean container.
  Contexts, partitions and signals stay separate for each context.
- JavaScript keys a Setup by realm and name. The JavaScript and TypeScript
  service modules of a site share one Setup class: when two or more of
  them hold the same Setup, the build moves its class into the shared
  `.tachyon/behavior-runtime.js` module, and each service module imports it
  by name instead of carrying its own copy. A Setup moves only when every
  copy is identical, it has no annotated method, and its class uses no other
  name of its service module, no `import` and no `eval`; otherwise each
  module keeps its copy, and the realm-and-name key still gives them one
  configuration. One class also means one set of static fields. The
  generated `TacClientConfiguration` of a managed-client owner is
  `moduleOwned`, so it stays one for each module: each owner module has its
  own adapter policies.
- Current limit: in the languages below, each browser service is lowered on
  its own, with its own copy of the runtime glue and of each Setup, client
  and store type. Two such services on one page therefore get two Setup
  configurations and separate adapters; the clients inside one service share
  them. Go and Rust services already build into one page program (ADR 0045
  and the Rust bullet below),
  JavaScript and TypeScript services share one Setup class as described
  above, and the Dart services of a site build into one Dart library (see
  the next bullet). The thread that lowers a language's page as one
  program deletes that language's bullet, flips `scripts/page-program/<language>/expected.json`
  to `"shared"`, and reports the route size before and after (TY1337, ADR
  0100). The baseline sizes were measured on 2026-10-18 in a Linux container
  without Binaryen or TinyGo, so compare a language only with itself on the
  same machine.
  - Swift: one `private enum TachyonService<N>` for each service; the hook is
    `native/page_program/swift_page_program.rs`. Baseline 35 KB of script,
    1519 KB of WebAssembly.
  - Kotlin: one `private object TachyonService<N>` for each service; the hook
    is `native/page_program/kotlin_page_program.rs`. Baseline 36 KB of
    script, 195 KB of WebAssembly.
  - Java: one holder class for each service; the hook is
    `JavaWebServices::services` in `native/java_web_services.rs`. Baseline
    40 KB of script, 86 KB of WebAssembly.
  - C#: one `namespace TachyonService<N>` for each service; the hook is
    `native/page_program/dotnet_page_program.rs`. Baseline 115 KB of script, 1136 KB of WebAssembly.
  - F#: one `namespace rec TachyonService<N>` for each service in the F#
    project; the same hook as C#. Baseline 115 KB of script, 1278 KB of WebAssembly.
- Dart builds every browser service of the site into one library,
  `services.dart`, inside its one Dart program. Each service is still
  prepared and lowered alone first, so its diagnostics name its own file;
  then the build splits each prepared source into its top-level
  declarations, keeps one copy of each declaration that several services
  share (the Setup, client, store and helper types they assemble), and
  lowers the result once, so the library has one behavior runtime, one
  Setup registry and one copy of each type. A name that two services
  declare with different text stops the build and names both files. Each
  service keeps its own bridge class (`__TacDartBridge<Service>`) and its
  own JavaScript module. The page-program fixture went from 37 KB of script
  and 46 KB of WebAssembly to 37 KB and 44 KB (TY1337, same container).
- Rust (2026-10-18): the Rust browser services of a page build into one
  `mod TachyonPage` (`native/page_program/rust_page_program.rs`). Each
  service file comes in once, and each Setup, HTTP client, Store, entity,
  converter and helper that any of them needs comes in once, by its authored
  path. Behavior lowering runs once over the whole page, so the page has one
  behavior runtime and one Setup registry, and a Setup's key names
  `TachyonPage::<Setup>` for every service. Each service keeps only its
  member table, in `TachyonPage::TachyonService<N>`. The one-class rule for
  service files (TY1310) means two service files cannot both declare a
  top-level type or function. The page's `use` lines are merged into one
  set, and a name that two of them bind to different paths stops the build.
  The fixture went from separate to shared in Chromium, Firefox and
  WebKit. Its route went from 127 KB to 125 KB of script for the same fixture (Linux container without Binaryen;
  the wasm is inside `tachyon-rust.js`): the generated Rust source shrank
  from 372 KB to 246 KB, but rustc had already dropped most of the second
  copy. With `remember` and `recall` written as synchronous methods, which
  adds two task members, it is 125 KB too. A synchronous Rust service
  method that returns `BehaviorResult<()>` or `BehaviorResult` of a model is now a task member
  (a promise in the page that rejects with the error) instead of being left
  off the service; the fixture's `remember` and `recall` are synchronous.
- The page-program gate (`scripts/page-program-browser-gate.mjs`,
  `bun run test:page-program-browser`) builds one project for each
  language: two services, `writer` and `reader`, on one page, both needing
  the same Setup, the same HTTP client (bound to the Setup's own
  `pageprogram` adapter) and the same Store. In Chromium, Firefox and WebKit
  it checks that the two services see one Setup configuration and that an
  increment through one client reaches the other (`"shared"`), or that they
  see two (`"separate"`), and that the Store record one service saves is
  found by the other. `TAC_PAGE_PROGRAM_LANGUAGES` and
  `TAC_PAGE_PROGRAM_BROWSERS` narrow a run. The JavaScript, Go, Dart and
  Rust fixtures are `"shared"` (47 KB of script; 37 KB of script and 990 KB
  of WebAssembly; 37 KB of script and 44 KB of WebAssembly; 125 KB of
  script). The C++ fixture
  is marked owed and skipped by default: a C++ page HTTP client does not
  call a Setup's `adapters.http` yet and ignores the `HTTPClient` options
  (CPP3 of the queue plan).
- Every runtime's contract test opens two contexts from one Setup, checks
  that `configure` ran once and that both contexts see the same adapter
  state, disposes one context twice, and checks that `configure` runs again
  only after the last context is disposed.

## Amendment (2026-09-30): implicit typed-model converter plans

Converter contracts may omit Convert when both method types are declared
models. The compiler resolves models before concrete contract rewriting,
infers exact same-name target fields, validates shape compatibility, and emits
bounded recursive projection plans for nested models and lists. Explicit
field entries override only their named targets; JSON-value contracts retain
the explicit-plan requirement. Model tables and realm tags are compiler-owned
metadata and cannot be authored as Convert options.

Native code reads and writes through the model adapter for its realm. The
server runtime qualification covers thirteen languages. Tac qualification
covers JavaScript and TypeScript browser execution and actual WebAssembly
compilation and execution in Dart, Go, Java, Kotlin, C#, F#, Rust, Ruby and
Swift. Ruby uses compiler-owned TacValues model metadata; F# uses generated
record codecs. Native host deployment is outside this qualification. Lua
converters came later, in both realms, with the 2026-09-30 amendment of
ADR 0085.
