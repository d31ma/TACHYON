# ADR 0028: Owned Application Events and Lifecycle

- Status: Accepted, with `EventPublisher` removed and the Spring event model
  adopted by [ADR 0031](0031-realms-mirror-each-other.md); implementation
  qualification in progress
- Date: 2026-09-15
- Amends: ADR 0020/0022 lifecycle conventions, ADR 0027 browser owner API
- Preserves: client-only Tac, REST-only Yon, explicit remote topic/SSE delivery
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Decision

Tachyon replaces the public browser `this.tac` object with method annotations
and named resources that are bound to an owner. PostConstruct and PreDestroy
are instance hooks with zero arguments. EventPublisher publishes the return
value of a method that succeeds. EventListener receives exactly one local
event payload. These modifiers do not change class, filename or folder
naming. The existing role naming rules stay.

Extraction works from source only, has limits, and the language backends
share it. Its metadata is separate from cache/HTTP behavior contexts.
Application events never require an authored Behaviors context, constructor
or Inject list. Unknown forms and unsupported asynchronous declarations fail
before execution. They never silently become annotations that do nothing.
Tachyon adds no Spring runtime and no reflection package.

In browser JS/TS, Resource fields select one of these: element, signal,
cacheManager, resourceLoader or renderScheduler. Resources are:

- injected after construction;
- excluded from RequiredArgsConstructor dependency inference;
- non-enumerable in state snapshots;
- bound to the current owner.

No resource grants native capabilities, server configuration, credentials or
cross-origin transport.

Listeners register before initialization. Tac initializes after its owner DOM
exists. This is a view lifecycle on purpose. It does not copy a Spring bean
constructor literally. Stable component roots keep their owner identity
across rerenders. Replacing an owner runs cleanup and creates a new lifetime.

Native companions carry a fresh activation ID through calls and
publications. Old (stale) owners cannot affect a replacement page. Native
publication queues drain after the current call releases its mutable borrow.
The initial native feature set is synchronous. It adds no background-thread
delivery that lacks an owner.

Yon owns events inside the provider graph of one invocation. It does not own
them globally across requests, workers or processes. Generated containers
register participating listeners, initialize dependencies, dispatch, drain
owned work and clean up in reverse order. The existing supervisor deadline
stays the final limit. An event does not prove a transaction commit, a
successful durable delivery, or a database/cache rollback. A forced
termination cannot guarantee that user cleanup hooks run.

Payloads are finite JSON snapshots. Each payload is limited to 64 KiB
(kibibytes) and cloned for each listener. Retention and replay are
local-memory policies that an application must opt into. Tachyon limits each
of these separately: names, listeners, queued work, recursion and operation
lifetime. Listener failures stay observable. They do not replace a
successful publisher result. Tachyon adds no implicit network forwarding,
cross-tab bus, event broker or durable queue.

## Upstream review

Reviewed Spring Framework source on 2026-09-15:

- [CommonAnnotationBeanPostProcessor](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/annotation/CommonAnnotationBeanPostProcessor.java)
  supports Jakarta PostConstruct, PreDestroy and named Resource injection.
- [EventListener](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/event/EventListener.java)
  separates method listeners, ordering and asynchronous failure behavior.
- [ApplicationEventPublisher](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/ApplicationEventPublisher.java)
  is a publishing interface. It is not an EventPublisher annotation.
- [CacheManager](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/cache/CacheManager.java)
  and [ResourceLoader](https://github.com/spring-projects/spring-framework/blob/main/spring-core/src/main/java/org/springframework/core/io/ResourceLoader.java)
  informed the separation of caching and resource loading. Tachyon's browser
  service methods are separate contracts. They do not implement these Java
  interfaces.

EventPublisher is specific to Tachyon. DOM access and rendering have no
direct Spring equivalent. The implementation was written independently. It
does not copy Spring internals. It does not claim API or performance parity
with Spring.

## Acceptance

Acceptance requires:

- source diagnostics;
- both TypeScript decorator modes;
- browser lifecycle and HMR (hot module replacement) interaction tests;
- native activation and stale-owner tests;
- real Yon execution across all eight languages.

Migrate the runnable website examples and verify them against the new
compiler. Record the actual execution evidence and limitations separately
from this decision. Do not infer release qualification from source staging
or compilation alone.
