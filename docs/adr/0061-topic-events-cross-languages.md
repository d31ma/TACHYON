# ADR 0061: Topic Events Cross Languages

- Status: Accepted; amended 2026-09-28 (topic logs are saved data) and
  2026-10-31 (a topic says who may read its stream)
- Date: 2026-09-27
- Extends: [ADR 0028](0028-owned-application-events-and-lifecycle.md) (Yon
  events were local to one invocation) and
  [ADR 0058](0058-streams-are-a-route-and-a-request.md) (a page reads a topic
  as a stream) and
  [ADR 0068](0068-safety-rules-are-build-errors.md) (every route says who
  may call it)
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
  reconnect delays as `@StreamRequest`. No `@HTTPClient` is needed. Since
  ADR 0082, the same holds for a listener of a WebAssembly service that
  lists its listeners to the page (Swift, Kotlin and Rust).

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

## Amendment (2026-10-31): a topic says who may read its stream

`/.tachyon/topics/<name>` served every topic to anyone who could reach the
server. It never asked the identity provider, so a caller could replay a
topic's log (up to 16 MiB) and read every event that a Yon class published.
Any slug opened a topic, so 32 made-up names, or 128 streams from one client,
locked out every real subscriber. Every other way into the server already
says who may use it (ADR 0068), so the topic stream was the one hole in deny
by default.

### Access markers on a topic model

- A `@Topic` model in `server/` says who may read its stream with the access
  markers of ADR 0068, written next to `@Topic` on the same model:
  - `@Public`: anyone, also a caller who is not signed in;
  - `@SignedIn`: any signed-in user;
  - `@Allow(Role, ...)`: only a signed-in user with one of these roles.
- The marker goes in the same block of lines as `@Topic`, directly above or
  below it and before the model declaration. Each language uses its own
  spelling, the same one as on a controller:

  | Language | Spelling |
  | --- | --- |
  | Java | `@Public`, `@SignedIn`, `@Allow(Admin.class)` |
  | Kotlin | `@Public`, `@SignedIn`, `@Allow(Admin::class)` |
  | Groovy, Dart | `@Public`, `@SignedIn`, `@Allow(Admin)` |
  | Python | `@public`, `@signed_in`, `@allow(Admin)` |
  | JavaScript | ` * @Public`, ` * @SignedIn`, ` * @Allow(Admin)` in the `@typedef` block |
  | TypeScript | `/** @Public */`, `/** @SignedIn */`, `/** @Allow(Admin) */` |
  | Go | `// @Public`, `// @SignedIn`, `// @Allow(Admin)` |
  | Lua | `-- @Public`, `-- @SignedIn`, `-- @Allow(Admin)` |
  | Rust, Ruby | `#[Public]`, `#[SignedIn]`, `#[Allow(Admin)]` |
  | PHP | `#[Public]`, `#[SignedIn]`, `#[Allow(Admin::class)]` |
  | C# | `[Public]`, `[SignedIn]`, `[Allow(typeof(Admin))]` |
  | F# | `[<Public>]`, `[<SignedIn>]`, `[<Allow(typeof<Admin>)>]` |
  | C++ | `struct [[Topic]] [[SignedIn]] OrderPlaced`, `[[Allow(Admin)]]` |

- The rules of ADR 0068 apply unchanged:
  - A server topic without a rule stops the build with `TY1322`.
  - A topic has one rule. `@Public` and `@SignedIn` take no arguments, and
    `@Allow` takes one or more role types from `server/roles/`. A role
    with no type there stops the build with `TY1322`.
  - A `@SignedIn` or `@Allow` topic without an identity provider stops the
    build with `TY1323`.
- Each language declares its own model (see Decision). When several
  languages declare one topic, they all write the same rule, or the build
  stops with `TY1322` and names both files.
- A page cannot grant itself access. An access marker on a `@Topic` model in
  `client/` stops the build with `TY1308`: the rule goes on the server model.

### The server checks every subscription

- The server serves only the topics that a server model declares. Any other
  slug answers 404 and opens nothing, so made-up names cannot use up the
  active-topic capacity.
- Before it opens a topic, the server runs the same guard as for a route:
  a `@Public` topic opens at once and never calls the identity provider. For
  a `@SignedIn` or `@Allow` topic it calls the identity provider with the
  headers and the URL of the request. No identity answers 401, a user without
  one of the roles answers 403, and a failing identity provider answers 500.
  In each case no log line is read and no subscriber slot is taken.
- A page opens a topic stream with the browser's own request, so the stream
  carries the page's cookies. An identity provider that signs users in with
  a cookie therefore guards a page's topic listeners as well.
- One client address holds at most 16 open topic streams, so one client can
  no longer take all 128 subscriber slots. The address is the peer address,
  or the client that a trusted proxy names (`X-Real-IP` after
  `ForwardedHeaders`). Two addresses can still fill a public topic's 32
  streams, and eight the whole server. Limits per identity and caps the
  operator sets are follow-up work; until then this is a known limit of the
  release candidate ([threat model](../THREAT_MODEL.md)).
- Delivery to Yon listeners does not change: it runs inside the server and
  never passes through the topic route.

### OPTIONS answers after the guard

`OPTIONS` on a route path returned the route's verbs and its JSON schema
before any access rule ran. It now answers only with the verbs and the
schema of the routes on that path that the caller may call. When the caller
may call none of them, it answers with the guard's refusal (401, 403 or
500). A CORS preflight still answers first, from `CrossOrigin`, because a
browser sends it without credentials.

### The published API reference lists only public routes

`/api.json` and `/api-docs/` published every controller route's schema as
static files, guarded routes included: the same schema `OPTIONS` now hides.
Static files cannot ask who is calling, so the build now lists only the
routes that `RouteGuard` lets a caller without an identity call, and only
the models those routes name. A signed-in caller reads a guarded route's
schema from `OPTIONS`. A project whose routes are all guarded publishes
neither file. `ty api freeze` and the `TY1338` check still compare every
route in `server/api-baseline.json`, which is not published.

### Consequences

- Every way into the server goes through `RouteGuard` or is a declared
  framework endpoint (live reload, hot updates and diagnostics in watch mode,
  the live-reload script, and static files). A repository policy test keeps
  it that way.
- An existing server `@Topic` model must add a rule. `@Public` keeps the old
  behavior. This is a breaking change for this release candidate.
