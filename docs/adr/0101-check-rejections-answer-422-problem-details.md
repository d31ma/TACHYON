# ADR 0101: Check Rejections Answer 422 Problem Details

- Status: Accepted
- Date: 2026-10-18
- Amends: [ADR 0092](0092-settings-and-validation-data-live-in-models.md)
  (what an invalid `@Check` argument answers over HTTP)
- Related: [ADR 0058](0058-streams-are-a-route-and-a-request.md) (fixed
  handler error frames that never carry error text)

## Context

`@Checked` declares field rules on a model, and `@Check` validates one
method parameter against that model before the method body runs
(`docs/BEHAVIOR_ANNOTATIONS.md`, "DTO and method validation"). The rules are
`Required`, `NotEmpty`, `Length`, `Min` and `Max`.

Until this decision, a request whose body broke a rule failed like a crash.
Java, Groovy, Kotlin, C#, F#, Dart, PHP and Go handler processes exited with
an error, so the server answered:

```
HTTP/1.1 500 Internal Server Error
content-type: text/plain; charset=utf-8

Handler execution failed. Reference: <id>.
```

JavaScript, TypeScript, Python and Ruby sent a `TY2201` error frame and the
client got a 500 with an empty body. Rust and Go controllers had to map the
error themselves; the website examples answered 400 with
`{"error":"validation"}`.

This is wrong in three ways:

- It blames the server. Monitoring counts these requests as server errors.
- Clients and proxies retry 5xx answers, but the same input fails again.
- The client learns nothing about which field is wrong.

Every runtime also stopped at the first broken rule, so a form with three
mistakes needed three round trips.

## Decision

When a `@Check` argument breaks one or more `@Checked` rules, and the
application does not answer the error itself, Yon answers
`422 Unprocessable Content` with an RFC 9457 problem details body. This
holds for every Yon language that supports `@Check`: JavaScript, TypeScript,
Python, Ruby, PHP, Java, Groovy, Kotlin, C#, F#, Dart, Go and Rust.

The languages only report the violations. The Rust server builds the answer,
so the body is byte-for-byte the same in every language.

### Contract

```
HTTP/1.1 422 Unprocessable Entity
content-type: application/problem+json
x-tachyon-request-id: 4WHBW1MLRC6

{"type":"about:blank","title":"Invalid input","status":422,
 "errors":[{"field":"level","rule":"Required"},{"field":"name","rule":"NotEmpty"}],
 "truncated":false,"reference":"4WHBW1MLRC6"}
```

(The body is one line; it is wrapped here.)

- `type` is always `about:blank`, `title` is always `Invalid input`, and
  `status` is always `422`. The members appear in the order shown.
- `errors` has one entry for each broken rule. `rule` is the annotation name:
  `Required`, `NotEmpty`, `Length`, `Min` or `Max`. A value that is not a
  JSON object, or that is larger than the 1 MiB copy limit, gives the single
  entry `{"field":"","rule":"Checked"}`.
- `field` is a path. A name is `name`, a nested field is `parent.child`, and
  an array item is `items[2]`, so `items[2].name` is the name of the third
  item. The empty path is the whole value. Today `@Check` validates direct
  fields only, so paths are top-level names; the grammar is ready for nested
  validation without a contract change.
- Entries are sorted by path, comparing segment by segment, with array
  indexes compared as numbers (`items[2]` before `items[10]`). Entries for
  the same path follow the rule order `Checked`, `Required`, `NotEmpty`,
  `Length`, `Min`, `Max`. Repeated entries are removed.
- `errors` holds at most 32 entries. `truncated` is `true` when more rules
  were broken, and `false` otherwise. It is always present.
- `reference` is the request id. It is the same value as the
  `x-tachyon-request-id` header and the id in the server logs. It replaces
  the old `Reference: <id>` text.
- The answer keeps the server's defensive headers, including
  `cache-control: no-store`.

Each runtime checks every rule of every field. A field can break more than
one rule: an empty `name` with `NotEmpty` and `Length({"min":2,...})` gives
two entries.

### Handler protocol

The handler protocol v1 error object gains an optional `violations` array.
The change is additive: a frame without it means what it meant before.

```json
{"protocol_version":1,"kind":"response","request_id":"r1","status":422,"headers":{},
 "error":{"code":"TY2205","message":"Input breaks a Checked rule.","retryable":false,
          "violations":[{"field":"name","rule":"NotEmpty"}]}}
```

- `TY2205` is the only code that may carry `violations`, it must carry at
  least one, and its status must be 422.
- A frame may carry at most 1,024 violations. A field path has at most 512
  bytes and must match the grammar above. `rule` must be one of the six
  names. The server refuses any other frame as a protocol failure, which
  answers 500.
- Languages that write a direct response (Java, Groovy, Kotlin, C#, F#,
  Dart, PHP, Go, Rust) write `{"status":422,"violations":[...]}`. The server
  turns it into the same `TY2205` response.

### What does not change

- An application `@ErrorHandler` that catches the validation error still
  wins, and its answer is sent as before. The error it receives still has the
  code `validation` and the path of the first broken rule, such as
  `name.notBlank`.
- Application code that calls `Behaviors.validate(...)` itself keeps today's
  behaviour: it raises the first violation, and an uncaught one is still a
  500. Only the guard that `@Check` generates reports a 422.
- An unknown schema name (`$.schema`) is a server defect and stays a 500.
- Tac (browser) validation is unchanged. A Tac client that calls a Yon route
  now sees HTTP 422 instead of 500.
- `@Check` on a streaming route, a socket route, a worker or a Gateway call
  is out of scope; those failures keep their existing fixed errors.

## Security notes

- The body never contains a submitted value, a value's type, a rule's
  bounds, or error text from the handler. It names fields and rules only.
  Field names come from the `@Checked` model, which the build already
  exposes in the generated schema.
- The server validates every path and rule from the handler and refuses
  anything else, so a handler cannot use the problem body to echo request
  data or inject text.
- The 32-entry cap and the 1,024-entry frame limit keep the answer small, so
  a hostile payload cannot make the response large. The schema limit of 64
  constrained fields bounds the work in each runtime.
- The `reference` is the same opaque id that the response header already
  carries. It reveals nothing new.

## Consequences

- Clients, monitoring and retry policies see a client error, not a server
  error. Forms can mark every wrong field after one request.
- This is a breaking change to a public response. Clients that matched the
  500 text, or the 400 that the Go and Rust examples answered, must change.
- Go and Rust controllers return the check error instead of mapping it
  themselves, so the framework answers 422.
- Every Yon runtime changed, so a cross-language test sends the same invalid
  input to every available language and asserts the same body.

## Alternatives considered

- **Keep 500.** It needs no change, but it keeps every problem described in
  the context: it blames the server, invites retries and tells the client
  nothing. Rejected.
- **A generic 400 Bad Request.** 400 is also a client error, but RFC 9110
  uses it for a request the server cannot parse. Here the JSON parses and
  only its content breaks the model's rules, which is what 422 means. A
  plain 400 also gives the client no field detail, and the Go and Rust
  examples already used 400 with a body of their own, so the languages
  would still differ. Rejected.
- **Let each language write the problem body.** It needs no protocol change,
  but thirteen JSON writers would differ in member order, escaping and
  limits, and a runtime bug could echo values. Rejected in favour of one
  writer in the server.
