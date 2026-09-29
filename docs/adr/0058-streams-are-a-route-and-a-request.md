# ADR 0058: Streams Are a Route and a Request

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0048](0048-yon-routes-come-from-annotations.md) (a streaming
  route has its own route annotation) and
  [ADR 0017](0017-mandatory-yon-layers-relay-and-streaming.md) (how a stream
  ends)
- Extends: [ADR 0013](0013-semantic-hot-updates.md) (server-sent events, not
  WebSockets, now also carry application streams to the browser)
- Preserves: [ADR 0055](0055-annotation-arguments-use-types.md) (typed events)
  and [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule table)

## Context

A Yon stream was a `@GetRoute` method that was a generator, with an optional
`@Stream` marker. A generator under any verb streamed, so the method name
(`getTicks`) did not say that it streamed, and two spellings meant the same
thing. The browser side had no annotation: every page wrote about forty lines
of `EventSource` code for opening, closing, timeouts and checking messages.

Spring gives each outgoing call a declarative method (`@GetExchange`) and
each incoming verb one mapping. WebSockets stay out of scope: SSE is the one
push channel (ADR 0013), and a WebSocket decision gets its own ADR.

## Decision

### StreamRoute (Yon)

- `@StreamRoute(path?)` replaces `@GetRoute` with `@Stream`. It means the
  method answers GET requests with a stream of server-sent events. The method
  is a generator in the language's own syntax, as before.
- The verb-prefix rule applies: the name starts with `stream` in the
  language's casing (`streamTicks`, `StreamTicks`, `stream_ticks`).
- A generator under `@GetRoute` or another verb route is refused with a hint.
  `@Stream` is retired in the rule table and refused with its replacement.
  `ty migrate fix --write` turns `@GetRoute` with `@Stream` into
  `@StreamRoute` and renames a `get` method to `stream` when the old name is
  used nowhere else in the file; every other case is a manual step.
- The same eight languages stream (JavaScript, TypeScript, Python, PHP,
  Kotlin, C#, Dart and Ruby). Java, Rust and Go refuse `@StreamRoute`.
- When the method finishes normally, the server sends `event: end`. A failed
  stream still ends with `event: error`.
- `/api.json` marks the route with `"stream": true` and
  `"produces": "text/event-stream"`.

### StreamRequest (Tac)

- `@StreamRequest(path?)` goes on a method of an `@HTTPClient` class. The path
  is added to the class path. The method takes exactly one parameter, and the
  parameter's model type (JSDoc `@param {Tick} tick` or `tick: Tick`) says
  what each message holds.
- One rule decides where a message goes: Tac checks it against the model
  (the same description `@OnEvent` uses), passes it to the method, and then
  publishes it as that model's typed event. A message that does not fit is
  dropped and counts as a listener failure. Every `@OnEvent(Tick)` listener,
  in any Tac language, receives the message through the page's event bus.
- The browser event bus owns the `EventSource`. It opens the stream when the
  client is first used, which is when a component or service that needs the
  client starts. It closes the stream when that owner goes away or the server
  sends `end`. After an error it reconnects after 1, 2, 4, 8 and 16 seconds
  and then stops; a message resets the count. Sixty seconds without a message
  counts as an error.
- `@StreamRequest` is written in JavaScript or TypeScript, because the
  browser's JavaScript holds the connection. Rust, Go, C#, Dart, Swift,
  Kotlin and Ruby services receive the messages with `@OnEvent`; the rule
  table refuses `@StreamRequest` there with that advice. An `EventSource`
  sends no headers or body, so `@StreamRequest` takes no parameter markers.
- The rule table refuses `@StreamRequest` outside an `@HTTPClient` class and
  together with another request or route annotation. A method with more or
  fewer than one parameter, or without a model type, is refused when the
  page is built.

## Consequences

The realtime example has no hand-written `EventSource`: a two-line client and
an `@OnEvent` listener replace it. The browser event bus
(`template/application-events.js`) grew from 11,104 to about 12,300 bytes;
`scripts/application-events-test.mjs` checks the new budget and the stream
behaviour with a fake `EventSource`. `tac-client.js` did not change. Existing
controllers that stream must be renamed; the migration fix does this for the
common case.

## Amendment (2026-09-29)

The `@StreamRequest` method belongs to an `HTTPClient` contract (ADR 0048
amendment). In JavaScript and TypeScript the contract is a class whose
methods have empty bodies, so the stream method keeps its empty body and
Tachyon writes it.
