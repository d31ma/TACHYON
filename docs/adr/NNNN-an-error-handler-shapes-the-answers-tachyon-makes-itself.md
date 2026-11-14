# ADR NNNN: An Error Handler Shapes the Answers Tachyon Makes Itself

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0051](0051-controller-error-handlers.md) (error handlers
  now also receive seven built-in error types, and a root-type method no
  longer takes the `@Check` error),
  [ADR 0068](0068-safety-rules-are-build-errors.md) (the 401, 403 and 500
  answers of the identity gate can carry an app body) and
  [ADR 0101](0101-check-rejections-answer-422-problem-details.md) (the 422
  answer can carry an app body)
- Requested by: Mesail ADR 0009 (CADUCEUS `f0809fc`), request Y1

## Context

Tachyon answers some requests itself, before or around the app's code:

| Case | Status | Body |
| --- | --- | --- |
| No identity on a `@SignedIn` or `@Allow` route | 401 | text |
| Signed in, but none of the route's roles | 403 | text |
| The `@IdentityProvider` failed | 500 | text |
| A path, query, header, cookie or body value is missing or has the wrong type, or the body is not JSON or not UTF-8 | 400 | JSON or text |
| The body is larger than `yon.server.maxbody` | 413 | text |
| A `@Check` argument breaks `@Checked` rules | 422 | problem details |
| The route could not answer: an error no method takes, an error inside an error handler, a crash, or an answer that does not fit its declared type | 500 | text, empty or JSON, by language |

An app cannot change these bodies. Mesail's rule is that every error answer
carries a Mesail error number, such as `[MS0510]: This passkey name is
already used.`, so these answers break the rule.

## Decision

### Seven built-in error types

Every Yon language has seven built-in error types. An `@HandlesError`
method that names one of them shapes the body of that answer. Tachyon keeps
the status.

| Type | Status | Fields |
| --- | --- | --- |
| `NotSignedIn` | 401 | `reference` |
| `RoleMissing` | 403 | `reference` |
| `IdentityCheckFailed` | 500 | `reference` |
| `RequestRejected` | 400 | `reference`, `part`, `field`, `message` |
| `BodyTooLarge` | 413 | `reference`, `limitBytes` |
| `CheckRejected` | 422 | `reference`, `violations` (`BrokenRule` items with `field` and `rule`), `truncated` |
| `UnexpectedError` | 500 | `reference` |

- Every type also has `status`. `reference` is the request id, the value of
  the `x-tachyon-request-id` header. Field names follow the language's
  style: `limit_bytes` in Python, Ruby, Rust and Lua, `LimitBytes` in Go and
  C#.
- `part` is `path`, `query`, `header`, `cookie` or `body`. `field` is the
  parameter name, or empty for the whole body. `message` is the server's own
  text, the same text as today's 400 body. `CheckRejected.violations` is the
  list of ADR 0101: sorted, at most 32, field paths and rule names only.
- The types live in the language's lowered Yon unit, and only when a
  `@HandlesError` method names one of them. They are not files in
  `server/errors/`. An app error type in `server/errors/` with one of these
  names, or `BrokenRule`, is refused (TY1310).
- `@ResponseStatus` on a method for a built-in type is refused (TY2015):
  the type fixes the status.
- App code may throw a built-in type itself. It is then matched in process
  like any error type, and the method answers with the type's status.

### How the server asks the handler

1. The server tags each answer of the table above with its case.
2. At the end of a route call, if the answer has a tag and the route's
   language has a `@HandlesError` method for exactly that type, the server
   asks the route's handler process for the body. Otherwise the answer is
   sent unchanged, byte for byte.
3. The question is a normal `route.invoke` request with the reserved route
   `/.tachyon/error` (no app route may start with `/.tachyon`, like the
   identity provider's `/.tachyon/identity`), the route's HTTP
   method, no body, and the case as the only argument:
   `{"type":"CheckRejected","status":422,"reference":"...","violations":[...],"truncated":false}`.
   The generated route function checks for `/.tachyon/error` before it calls any
   route method, builds the built-in error and calls the error handler. The
   handler process builds its providers as for any call, the controller
   included, because the error handler shares the controller's provider
   container (ADR 0051).
4. The server checks the answer against the method's declared return type,
   like every error answer, sets the built-in status, keeps only the body
   and `content-type`, and adds `x-tachyon-request-id` and
   `cache-control: no-store`. Middleware `after` runs as for every answer.

A built-in type reaches only a method that names that exact type. A method
for the language's root error type (`Error`, `Exception`, `error`,
`std::exception`, ...) never receives a built-in error from the server, so
adding a catch-all never changes a 401 or a 422.

### When the error handler fails

If the `/.tachyon/error` call fails in any way (an exception, a crash, the 10 second
deadline, a busy pool, an answer that does not fit its declared type), the
server sends the answer it would have sent without the handler, byte for
byte, and writes one line on stderr:
`{"event":"error_handler.failed","handler":...,"request_id":...,"route":...,"type":...}`.
It never asks again, so a request starts at most one extra handler call.

The `/.tachyon/error` call has its own deadline: 10 seconds, or less when the
server's handler timeout is shorter.

### What a handler may and may not see

May see: the built-in type, its status, the request id, the HTTP method,
the path and query, the request headers without credentials, and the
fields of the table above.

Never sees:

- the identity provider's error text, exit code, output or stack;
- for a 403, the caller's user or roles;
- a body byte, a submitted value or its type;
- the `authorization`, `proxy-authorization` and `cookie` headers, or a
  header that the identity provider reads with `@FromHeader`. A provider
  that reads the whole request may read any header; then only the three
  standard credential headers are removed;
- the message or stack of the error that led to `UnexpectedError`.

### The `@Check` error and root-type methods

Before this decision, the `@Check` guard threw its error inside the
generated `try` of the route, so a root-type method (`Error`, `Exception`,
Go `error`, `std::exception`) caught it and the 422 was lost. From now on a
root-type method does not take the `@Check` guard's error. The 422 answer
follows, and `CheckRejected` can shape it. A method that names the guard's
own error type still takes it, as ADR 0101 says.

### What does not change

- Without a method for a built-in type, every answer is exactly as before.
- Socket upgrade refusals, topic stream refusals, `OPTIONS`, static files,
  middleware `before` answers, 502 (relay), 503 (busy) and 504 (deadline)
  keep their answers. A stream route is covered only before its stream
  starts.
- The handler protocol is unchanged. Rust and C++ add `"unhandled":true`
  to the direct answer of an unhandled error, so the server can tell it from
  a 500 the app chose; the body stays the same.

## Consequences

- Mesail, and any app, can put its own error number on every answer of a
  Yon route.
- A request that fails the gate in an app with a `NotSignedIn` method costs
  two handler calls instead of one (the identity provider, then the error
  handler). Warm handler processes keep this small, and a busy pool skips
  the second call.
- A new answer that Tachyon makes itself in the future needs a new
  built-in type, or it stays unshaped. The seven types are the closed set
  for now.
- Apps with a root-type error method get the 422 back for `@Check` errors.

## Alternatives considered

- **A new protocol operation** (`route.error`) with its own entry branch in
  each runtime. It would skip building the controller, but it needs a new
  branch in twelve runners and preludes. The reserved route uses the
  existing dispatch in every language. Rejected.
- **One type `TachyonError` with a `kind` field.** Less code, but the
  author writes a switch in the handler, which the code rules refuse, and
  the build cannot tell which cases an app shapes. Rejected.
- **Static templates in settings.** No code runs, but Mesail builds the
  message from the error type and from the broken rules. Rejected.
