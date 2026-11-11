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
2. **Saved state belongs to a service a view uses.** The page saves the
   stored fields of the service objects it holds. A Go service that only
   another Go service injects lives inside the WebAssembly module, so the
   page never sees it and its stored fields are not saved. Until a later ADR
   moves saving into the Go runtime, put `// @SessionState` and
   `// @LocalState` fields on a service that a view uses, and pass the values
   to the injected service through its methods. The build does not refuse a
   stored field on an injected-only service yet.
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
