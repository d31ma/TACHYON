# ADR 0022: Shared Tac and Yon Behavior Adapters

- Status: Accepted; executable polyglot integration
- Date: 2026-09-12
- Supersedes: ADR 0020's blanket prohibition on Tac annotations
- Preserves: ADR 0015 client-only rendering, ADR 0016 REST-only Yon, ADR 0021's HTTP verb method convention
- Amended by: [ADR 0044](0044-an-annotated-file-holds-one-class.md): "source-local" means the unit that discovery composes, and a behavior-only class, such as a `Validated` input, lives in its own file in a role folder
- Amended by: [ADR 0046](0046-tachyon-annotations-are-global.md): the annotations and types are global in every language, and JS/TS refuses an import from `tachyon:behaviors`
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

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
