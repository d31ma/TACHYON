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
3. **Go workers build what they need.** A worker is one `Handler` type in
   `server/workers/`. Yon does not inject services into a Go worker (C++
   workers take `[[Needs]]`, ADR 0099 and its C++ worker plan). A Go worker
   creates the values it needs in its own method, or calls a route of its
   own application. Injection into Go workers needs its own change to the
   worker plan and the Go container.
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
- Points 2 and 3 are limits, not features. Apps that need saved state on an
  injected service, or services in a Go worker, use the workarounds above
  until a later change adds them.

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
| Rust linked into a native app | yes, through `Arc<T>` | **not saved**: see below |

Native targets (macOS, Windows, Linux, iOS, Android) run Go, Java, F#,
Python and C++ services as WebAssembly in the platform web view (ADR 0018),
with the same page code, so they get the same result. A Rust service linked
into a native app can hold another Rust service through `Arc<T>`, but the
native companion answers only the services that a view uses, so the page
cannot reach a service that only another Rust service holds. Its saved fields
are not saved. Put them on a service that a view uses, as before, until the
native Rust page graph exposes needed services.

Point 2 of the Context is no longer a limit, except for that native Rust
case.

