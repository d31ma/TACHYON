# ADR 0119: Go Edge Contracts for Tac and Yon

- Status: Accepted
- Date: 2026-11-10
- Amends: [ADR 0063](0063-services-and-controllers-are-cohesive.md)
  (the Go error check), and records limits of
  [ADR 0090](0090-compiled-dart-http-clients-await-the-browser-host.md),
  [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md),
  [ADR 0045](0045-go-is-a-service-language.md) and
  [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md)

## Context

The Mesail rewrite writes its Tac services and Yon handlers in Go. It met
five edges where Go code behaved in a way the docs did not say:

1. A browser HTTP request in an `// @OnStart` method failed with only
   "The browser HTTP request failed."
2. A `// @SessionState` or `// @LocalState` field was saved on a service that
   a view uses, but not on a service that only another Go service injects.
3. A `// @Schedule` worker in `server/workers/` received no injected
   services.
4. TY1314 refused the plain Go error check `if err != nil { return ..., err }`
   in a public method, although Rust's `?` operator does the same job and is
   allowed.
5. `YON_APPLICATION_JSON` holds `{"yon":{...}}`, while
   `server/application.json` has no `yon` key.

## Decision

1. **No waiting in `OnStart`.** This stays as ADR 0090 decided: `OnStart`,
   getters and setters run without a task, so a browser HTTP request or a
   socket send there cannot wait, in Go and in the other languages. The Go
   runtime now says so. The failure keeps the code `HTTP_REQUEST` and its
   message is "A browser HTTP request or socket send cannot wait in OnStart,
   a getter or a setter. Publish an event from OnStart and make the request
   in an @OnEvent method." An `// @OnEvent` method runs as a task, so it can
   wait; add `// @InBackground` when the publisher should not wait for it.
2. **Saved state works on a needed service.** Replaced by the section
   "Saved fields on needed services" below.
3. **Workers get injected services.** Replaced on 2026-11-11 by the section
   "Workers get injected services in every Yon language" below.
4. **The Go error check is not a decision.** ADR 0063 now allows, in Go
   only, `if err != nil {` right after the statement that assigns `err`,
   whose body is one `return` that ends with `err` or
   `fmt.Errorf("<text with %w>", err)`, with no `else`. Every other shape
   still counts.
5. **The snapshot has a `yon` root.** The loader puts the whole file under
   the `yon` namespace (ADR 0034) and merges the profile and `.env` secrets.
   Binding calls (`BehaviorsProperties[T]`, `BehaviorsValues[T]` and their
   relatives in the other languages) take the whole snapshot. Code that reads
   a value by hand reads it below `yon`. `docs/APPLICATION_CONFIGURATION.md`
   and the configuration guide now say so.

## Consequences

- Go code reads like the other languages: the error check that replaces `?`
  no longer needs a private helper.
- A Go `OnStart` that needs data starts the request through an event; the
  error now names that pattern.
- Points 2 and 3 of the Context are no longer limits: see the sections
  below (one native Rust case stays, as the saved-fields section says).

## Saved fields on needed services

This section replaces decision 2 (2026-11-11).

A `@SessionState` or `@LocalState` field is saved and restored on a service
that only another service needs, in every Tac language, as on a service that
a view uses. The value goes to the same key in the same storage:
`tac:service:<instance>:<page path>:<field>`, in `sessionStorage` or
`localStorage`, as JSON of up to 64 KiB. A service that a view uses and that
another service also needs reads and writes the same keys, so the two never
disagree.

How it works:

- JavaScript, TypeScript and Dart already had it: the page fills a needed
  field with its own singleton of that service, and binds its saved fields.
- A WebAssembly service keeps the services it needs inside its module. Its
  `init` answer now lists them in `needs`: the instance names of the shared
  services that it holds through Needs, directly or through other services.
- The page plan has a `saved` map: for each service that is not JavaScript,
  TypeScript or Dart and has saved fields, its instance name and the
  `local` or `session` area of each field.
- For each listed name that `saved` names, the page opens one field-only
  activation of that service in the same module (no events, no streams, no
  OnStart again: the module already started it) and binds its saved fields
  like any other service. One activation serves every service of the page
  that needs the same instance. When the last service that needs it is
  released, the page releases it too, so the needed service still stops when
  its last holder stops.
- The page saves the needed service's fields after every call, event and task
  of the service that needs it, the same moment it saves that service's own
  fields. It restores them when it opens the activation, right after the
  module started the services, as for a WebAssembly service that a view uses.
- Nothing new is stored: the same two browser storages, keys and limits.

| Language | Can a browser service need another service? | Saved fields on the needed service |
| --- | --- | --- |
| JavaScript, TypeScript | yes (page singleton) | saved (unchanged) |
| Dart | yes (page singleton, ADR 0033) | saved (unchanged) |
| Go | yes, `// @Needs` | saved (runtime lists `needs`) |
| Java | yes, `@Needs` | saved (runtime lists `needs`) |
| F# | yes, `[<Needs>]` | saved (runtime lists `needs`) |
| Python | yes, `Annotated[T, needs]` | saved (runtime lists `needs`) |
| C++ | yes, `[[Needs]]` | saved (the build lists `needs`) |
| C# | no: TY1308 | not applicable |
| Rust, Kotlin, Swift (WebAssembly) | no: TY1406 | not applicable |
| Ruby, Lua | no: TY2015 | not applicable |
| Rust linked into a native app | yes, through `Arc<T>` | saved (the native page graph lists `needs`): see below |

Native targets (macOS, Windows, Linux, iOS, Android) run Go, Java, F#,
Python and C++ services as WebAssembly in the platform web view (ADR 0018),
with the same page code, so they get the same result.

A Rust service linked into a native app can hold another Rust service through
`Arc<T>` (the native page graph). It works the same way (2026-11-11):

- The `init` answer of a Rust service that a view uses lists in `needs` the
  instance names of the shared services it holds through `Arc`, directly or
  through other services. A prototype service belongs to one owner, so it is
  not listed.
- The page opens the field-only activation through the service that holds
  it: the `init` request names that service in `service` and the held service
  in `held`. Every native host already sends a request to the Rust library by
  its `service` name (the Swift host on macOS and iOS, the companion table on
  Windows and Linux, the Rust service list on Android), so the request reaches
  the library that holds the running service, with no host change.
- The Rust library opens the activation only for a name in that service's
  `needs`, and only on the running node: it never builds the service, and
  OnStart does not run again. The activation has only the saved fields, with
  no methods, events or streams. Closing it does not stop the service.
- `Arc` gives shared access only, so a saved field on such a service is a
  `std::sync::Mutex<T>` or `std::sync::RwLock<T>`, where `T` is a type a
  page field can have. The page reads and restores the value through the
  lock. A plain saved field there stops the build (`TY1406`), because the
  page could not restore it.

Point 2 of the Context is no longer a limit.

## Every Tac language: waiting in OnStart and the early error exit (items 1 and 4)

Amended 2026-11-11. Items 1 and 4 now behave the same way in every Tac
language that can meet them.

**Item 1.** No Tac language waits for a browser HTTP request or a socket
send in a plain `OnStart`, a getter or a setter. Where a language can still
start such a request outside a task, the answer is the same as in Go: the
code `HTTP_REQUEST` and the message "A browser HTTP request or socket send
cannot wait in OnStart, a getter or a setter. Publish an event from OnStart
and make the request in an @OnEvent method." The language's HTTP client and
socket error keep that message. Languages whose `OnStart` may be `async`
wait as before.

**Item 4.** TY1314 allows each language's short early exit that does the job
of Rust's `?` (ADR 0063 lists the exact shapes). Languages that report
errors with exceptions need no exception to the rule.

| Language | Item 1: a request in `OnStart` | Item 4: the early error exit |
| --- | --- | --- |
| JavaScript, TypeScript | An `async` `OnStart` is awaited, so the request waits. No change. | Exceptions; nothing to allow. |
| Rust | An `async` lifecycle method is refused at build, so no request can start. A socket send without an owner fails with `HTTP_REQUEST`. No change. | `?` and `let ... else` were already allowed. |
| Go | Fails with `HTTP_REQUEST` and the message (HTTP and socket). | `if err != nil { return ..., err }` right after `err` is set. |
| C#, F# | `OnStart` is synchronous and an `async` one is refused, so it cannot wait. A request started without waiting faults its own task, which no caller reads. No change. | Exceptions; nothing to allow. |
| Swift | A plain `OnStart` cannot call an `async` client: the compiler refuses it. A request that still runs outside a task fails with `HTTP_REQUEST` and the message (HTTP and socket). | `guard let x = <expression> else { return nil }`. |
| Kotlin | A plain `OnStart` cannot call a `suspend` client: the compiler refuses it. A request that still runs outside a task fails with `HTTP_REQUEST` and the message (HTTP and socket). | `val x = <expression> ?: return null`. |
| Java | Fails with `HTTP_REQUEST` and the message (HTTP and socket). | Exceptions; nothing to allow. |
| Dart | An `async` `OnStart` returns a `Future`, and the page waits for it as for JavaScript. No change. | Exceptions; nothing to allow. |
| Ruby | Fails with `HTTP_REQUEST` and the message (HTTP and socket). | Exceptions; nothing to allow. |
| Lua | Fails with `HTTP_REQUEST` and the message (HTTP and socket). | TY1314 does not scan Lua. |
| Python | An `async` `OnStart` is awaited, so the request waits. No change. | Exceptions; nothing to allow. |
| C++ | `OnStart` is synchronous and a request needs `co_await`, so no request can start. A socket send without an owner fails with `HTTP_REQUEST`. No change. | TY1314 does not scan C++. |

The native hosts keep their own socket answer outside a call: Kotlin and
Swift services that run natively still fail with `HTTP_REQUEST` and "The
socket message was not sent."

Tests: Go `a_go_request_in_on_start_explains_why_it_cannot_wait`, Java
`a_java_request_in_on_start_explains_why_it_cannot_wait`, Ruby
`a_ruby_service_waits_for_the_browser_transport_without_blocking_the_page`,
Lua `a_lua_call_parks_on_browser_http_until_the_answer_arrives`, and
`tac_web::on_start_wait_tests` for the answers of every language and the
Kotlin and Swift readers. The TY1314 shapes are in
`annotation_layout::single_class::cohesion::tests`.

## Workers get injected services in every Yon language (2026-11-11)

This section replaces decision 3. A worker is still one type named `Handler`
in `server/workers/` with a `POST` method and one `Schedule` marker. A worker
may now declare `Needs` fields, in the same spelling as a controller of its
language. Tachyon then builds the worker like a controller: it finds the
providers in `server/services/` and the other role folders under `server/`,
fills the fields, and calls `POST` on the built object on every tick. `POST`
is then an instance method that takes no parameters, as a controller route
method is an instance method. Lifetimes work as they do for a controller: the
worker object lives for one run, and singletons live for the process.

A worker with no `Needs` is unchanged: Tachyon compiles its source as before,
with no container code, and its `POST` may stay static where the language
allows it. C++ workers keep their own plan (ADR 0099), which already took
`[[Needs]]` references.

How it works: when the worker declares a `Needs` marker (or, in F#, takes
primary constructor values), Tachyon renames the code word `Handler` in the
worker to the generated name `YonWork`, treats `YonWork` as a request-scoped
controller with one `POST` route that has no URL, and emits the usual
`Handler` dispatcher that resolves `YonWork` from the container. So no
language runtime changed. A worker that already uses the name `YonWork` fails
with TY2015.

| Language | Needs field in a worker | POST |
| --- | --- | --- |
| JavaScript | `@Needs ledgerService;` (lowerCamel provider name) | `POST() {}` |
| TypeScript | `@Needs ledgerService!: LedgerService;` | `POST(): number {}` |
| Python | `ledger: Annotated[LedgerService, needs]` | `def POST(self)` |
| PHP | `#[Needs] private LedgerService $ledger;` | `public function POST()` |
| Java | `@Needs LedgerService ledger;` | `int POST()` |
| Groovy | `@Needs LedgerService ledger` | `int POST()` |
| Kotlin | `@Needs val ledger: LedgerService` in a `class` (not `object`) | `fun POST()` |
| C# | `[Needs] LedgerService _ledger;` in a non-static class | `public int POST()` |
| F# | `type Handler(ledger: LedgerService) =` (primary constructor values) | `member this.POST()` |
| Rust | `#[Needs] ledger: std::sync::Arc<LedgerService>` | `fn POST(&self)` |
| Dart | `@Needs final LedgerService ledger;` | `int POST()` |
| Go | `// @Needs` above `ledger *LedgerService` | `func (worker *Handler) POST()` |
| Ruby | `#[Needs]` and `# @return [LedgerService]` above `attr_reader :ledger` | `def POST` |
| Lua | `-- @Needs` and `---@type LedgerService` above `ledger = nil,` | `function Handler:POST()` |
| C++ | `[[Needs]] LedgerService& ledger;` (ADR 0099) | `void POST()` |

Swift is a Tac language only, so it has no workers.

Go example:

```go
// @Schedule({"fixedDelaySeconds":300})
type Handler struct {
	// @Needs
	ledger *LedgerService
}

func (worker *Handler) POST() {
	worker.ledger.Settle()
}
```

Java example:

```java
@Schedule({"fixedDelaySeconds":300})
class Handler {
  @Needs
  LedgerService ledger;

  void POST() {
    ledger.settle();
  }
}
```

Python example:

```python
from typing import Annotated


@schedule({"fixedDelaySeconds":300})
class Handler:
    ledger: Annotated[LedgerService, needs]

    def POST(self) -> None:
        self.ledger.settle()
```

The website section "Scheduling: Yon" shows the worker in every language.
