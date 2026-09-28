# ADR 0061: Topic Events Cross Languages

- Status: Accepted; amended 2026-09-28 (topic logs are saved data)
- Date: 2026-09-27
- Extends: [ADR 0028](0028-owned-application-events-and-lifecycle.md) (Yon
  events were local to one invocation) and
  [ADR 0058](0058-streams-are-a-route-and-a-request.md) (a page reads a topic
  as a stream)
- Preserves: [ADR 0055](0055-annotation-arguments-use-types.md) (the event is
  a model, and the model type name is the event name) and
  [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule table)

## Context

`publishEvent(model)` reached only the listeners of one Yon invocation. A
Python controller could not tell a Go service that an order was placed, and a
page had to write an `@HTTPClient` with `@StreamRequest('/order-placed')` to
read the topic log. Spring Cloud Bus solves the same problem for Spring: an
event class that extends `RemoteApplicationEvent` is sent to a broker, every
other service receives it and publishes it locally, and the service that sent
it ignores its own copy.

## Decision

### Topic (a model annotation)

- `@Topic` goes on a model in a models folder (`server/models/` or
  `client/models/`). The spelling follows the language: `@Topic`,
  `#[Topic]` (Rust, PHP, Ruby), `[Topic]` (C#), `// @Topic` (Go) and
  `/** @Topic */` (TypeScript). JavaScript writes ` * @Topic` inside the
  `@typedef` block of a model in a models folder.
- The topic name comes from the model type name only: `OrderPlaced` is the
  topic `order-placed`, stored in `data/topics/order-placed.jsonl` (see the
  amendment) and served at `/.tachyon/topics/order-placed`. The name starts with a capital
  letter and has only letters and digits, so no path can be written into it.
- Each language declares its own model and marks it. A listener hears other
  processes only when its own model has `@Topic`.
- The compiler refuses `@Topic` outside a models folder and `@Topic` that is
  not written above a model.

### Publishing

- `publishEvent(model)` of a `@Topic` model first delivers the event to the
  listeners of the same invocation, as before. The runtime then sends it to
  the server over the invocation's resource channel.
- The server accepts only a `@Topic` model of the publishing file's
  language, only a JSON object, and at most 64 KiB. The log holds at most
  16 MiB. It appends one line to the topic log. A refused publication is a
  publication failure; it never undoes the operation that published it.
- A page's `publishEvent` stays in the page. A page sends an event to the
  server through a Yon route that publishes it. This keeps the topic log
  closed to browsers (fail closed).

### Delivery

- The server calls each `@OnEvent` listener of the topic once, in every Yon
  language, with the operation `event.deliver`. When one listener class is
  part of several controllers, one controller runs it. The listeners that ran
  in the publishing invocation are skipped, so no listener hears an event
  twice.
- Delivery is at most once, as with Spring Cloud Bus's default anonymous
  consumer groups: the server keeps no position for a Yon listener, and a
  listener that fails is not called again. An event that a listener
  republishes travels at most 8 times.
- Java, Kotlin, C#, Dart, Rust and PHP send the event as JSON text
  (`payload_json`), and the server reads it back into a JSON object before
  it checks it. A delivery names the listeners to call (`Type.method`); the
  runtime registers only those listeners, so a listener never hears one
  event twice.
- All eleven Yon languages send and receive Topic events.
- A page's `@OnEvent` listener for a `@Topic` model opens the topic stream
  through the page event bus: replay from the last position, and the same
  reconnect delays as `@StreamRequest`. No `@HTTPClient` is needed.

## Consequences

Yon listeners in different languages hear each other through the existing
durable topic log. Records written into the log by hand still reach pages, but
not Yon listeners, because the server delivers what `publishEvent` sends it.
There is one bus for each server; several servers do not share it.

## Amendment (2026-09-28): topic logs are saved data

The topic logs were in `.tachyon/topics`. Everything else in `.tachyon` is
generated (types, prepared handler files, route contracts, the coverage
record), so deleting the folder to clean up also deleted the app's events.

- The logs live in the folder that `topics.directory` in
  `server/application.json` names (`yon.topics.directory`). The default is
  `data/topics` at the project root. `ty init` does not ignore `data` in
  `.gitignore`; the guides explain backup and when to commit it.
- The value is a plain path inside the project or an absolute path. The
  server refuses to start (`TY1303`) for a path with `..` or `.`, a path
  inside `.tachyon`, an absolute path that passes through a symbolic link,
  and a project folder that a symbolic link leads out of the project. Logs
  are still opened without following a symbolic link.
- On start, when `.tachyon/topics` holds logs and the new folder holds none,
  the server moves them once and prints a notice. When both hold logs, it
  moves nothing and says so. `ty migrate check` reports old logs that are
  still in `.tachyon/topics`.
- The URL stays `/.tachyon/topics/<name>`. It is a server route, not a
  folder, and pages already use it.
- `.tachyon` and `dist/.tachyon` are generated and safe to delete.
