# ADR 0105: Swift and Rust Browser Listeners May Be Async

- Status: Accepted
- Date: 2026-10-19
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (an async listener in
  a Swift or Rust browser service)
- Works with: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  [ADR 0082](0082-swift-services-reach-the-web-as-webassembly.md),
  [ADR 0022](0022-shared-behavior-adapters.md) (one Setup per page program)

## Context

A Swift or Rust service in the browser answers its HTTP clients and stores
only asynchronously. Its methods may be `async`, and the page runs each one
as a task of the service. Its `@OnEvent` methods could not be `async`: the
compiler refused every async native event method before the toolchain ran.

So a Swift or Rust listener could not read through a client. The cookbook
list could only copy the name from the event, while the Dart list reloads
every saved user through its client.

## Decision

- An `@OnEvent` method of a Swift or Rust service that runs in the browser
  may be `async`. A Rust listener may also return `BehaviorResult<()>`.
- The compiler gives such a listener a task entry. The service's event
  operation starts the task and answers `{"task": id}`, as an async method
  call does.
- The page sends the event of a task listener through the service's task
  queue. The listener tasks of one service run one after another, in the
  order their events arrive. A Rust listener task holds the service, so the
  page shows the fields it read before the task started, as it does for a
  Rust async method.
- The listener's promise goes back to the page publisher, which marks the
  page changed when the task ends and counts a failed task as a failed
  listener.
- Removing the service cancels the running task and the waiting ones.
- An async listener cannot return an event. It publishes through
  `applicationEventPublisher`. The compiler refuses a returned model.
- A Swift or Rust listener of a native app stays synchronous. Kotlin, C# and
  F# browser listeners stay synchronous too.

## Consequences

- The Swift and Rust cookbook lists reload every saved user through their
  own `UsersClient`, from the shared UsersSetup adapter, like the Dart list.
- `scripts/async-listener-browser-gate.mjs` builds the Swift and Rust
  cookbook example and checks in a browser that the list shows every saved
  name.
