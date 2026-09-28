# Local Application Events

Status: historical proposal.

- [ADR 0028](adr/0028-owned-application-events-and-lifecycle.md) (architecture
  decision record 0028) superseded its implementation contract.
- [ADR 0031](adr/0031-realms-mirror-each-other.md) then replaced that
  contract. ADR 0031 removes `@EventPublisher`. It adopts Spring's injected
  `ApplicationEventPublisher`, listeners for several events, `condition`,
  `id`, `@Order` and `@Async`.

[APPLICATION_EVENTS.md](APPLICATION_EVENTS.md) describes the API that runs
today and its limits. Read that page, not this one, to write events.
Qualification evidence belongs in HANDOFF.md.

The user confirmed the scope on 2026-09-15: local application events only.

We keep the proposal text below as a record of the original reasoning. Where
it describes a publisher annotation, read "the injected publisher" instead.

## Authoring

Publish through the injected publisher. Listen with a method annotation. This
works the same way in Tac and Yon:

```typescript
applicationEventPublisher!: ApplicationEventPublisher;

async changeCatalog(input) {
  const catalog = await this.catalogService.update(input);
  this.applicationEventPublisher.publishEvent('catalog.changed', catalog);
  return catalog;
}

@EventListener({"event":"catalog.changed","replay":false})
async onCatalogChanged(value) {
  this.catalog = value;
}
```

Rules:

- The publisher is an injected field, not an annotation. Publication is a
  method call. So if an operation throws or rejects first, the call never
  happens.
- A listener has exactly one parameter, the payload.
- Event names and options are literal metadata with limits. Tachyon validates
  them before execution.
- Tachyon rejects generators. It also rejects conflicting event markers on
  one method.
- These markers change behavior. They are not class roles, so they add no
  folder or filename suffix.

Spring provides `EventListener` and an `ApplicationEventPublisher` interface.
This proposal introduced an `EventPublisher` method annotation that only
Tachyon had. ADR 0031 removed it and uses Spring's injected publisher instead.
Reference:
[Spring application events](https://docs.spring.io/spring-framework/reference/core/beans/context-introduction.html).

## Ownership and delivery

- Tac browser events share the local bus of the active document. Register
  listeners before mount. Remove them when the owner is disposed. Render after
  listeners change.
- Yon events belong to the provider graph of one invocation. Construct the
  listener providers that take part once, before dispatch. Use the existing
  dependency validation. This design does not imply delivery across
  requests, across workers, across the whole process, or to durable storage.
- Native Tac events keep explicit context ownership. Host messages need a
  route and owner identity before Tachyon can accept these markers. After
  navigation, old owners must not publish or receive events.
- Registration order is deterministic. Synchronous delivery has a recursion
  limit. Asynchronous work needs limited admission, cancellation, and a
  defined policy for completion and errors. Detached promises are not enough.
- Before Yon sends its response, it finishes the asynchronous listener work it
  owns. This happens within the existing invocation deadline.
- Native listeners bind before Tachyon delivers queued host publications.
  Rust listener captures must not create strong reference cycles between the
  owner and the bus.
- Payloads must be finite JSON. Every payload is limited to 64 KiB. Each
  listener gets its own copy of the value. Existing topic and listener limits
  still apply where relevant.
- Retention is off by default. Replay is off by default, and a listener must
  request it explicitly. Retention uses local memory. It is never durable
  storage.
- A successful publication does not mean that an application transaction
  committed. It also does not mean that every listener succeeded. Listener
  failures must stay observable. They must not change the result of a
  publisher that already succeeded.

## Migration and acceptance

Replace public local use of `this.tac.publish`, `this.tac.subscribe` and
`this.tac.retained` with the new declarations. Do this only when their
executable language adapters and their lifecycle tests are complete. Existing signals
that depend on retention or replay must select those policies explicitly. Do
not add markers that Tachyon accepts but that have no runtime behavior.

These stay unchanged:

- the existing Yon topic publisher;
- topic logs;
- SSE (server-sent events) subscribers;
- cursor replay;
- network delivery from Yon to Tac.

DOM event listeners are also not affected. Tachyon adds no automatic
forwarding from local events to the network.

Required execution evidence covers these languages in their supported
environments:

- Yon: JS/TS/Python/PHP/Java/Kotlin/C#/Rust;
- Tac: JS/TS/Swift/Kotlin/C#/Rust.

The evidence covers:

- publication that succeeds and publication that fails;
- listener order;
- payload isolation;
- retention and replay;
- duplicate registration;
- capacity and recursion limits;
- async cancellation;
- context disposal;
- native navigation;
- isolation between Yon invocations.

This proposal does not change the separately approved Client migration.
