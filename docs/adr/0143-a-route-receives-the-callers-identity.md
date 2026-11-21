# ADR 0143: A Route Receives the Caller's Identity

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0068](0068-safety-rules-are-build-errors.md) (the server
  keeps the whole object that `identify` returned, and a route method can
  receive it)
- Requested by: Mesail ADR 0009 (CADUCEUS `f0809fc`), request Y2

## Context

Before a `@SignedIn` or `@Allow` route, the server calls the one
`@IdentityProvider` of the app (ADR 0068). `identify` returns the user and
the names of their roles, for example `{"user": "ada", "roles": ["Admin"]}`.
The server read only `roles` and threw the rest away.

A route method that needs to know who calls it, for example to save a
message under the caller's name, had to read the token again and repeat the
work of `identify`. Mesail asked for the caller's identity as a normal
route argument.

## Decision

### The marker

`FromIdentity` is a route parameter marker, next to `FromPath`,
`FromQuery`, `FromBody`, `FromHeader` and `FromCookie` (ADR 0047, 0055).
Each language writes it like its other parameter markers:

| Language | Spelling |
| --- | --- |
| JavaScript, TypeScript, Java, Groovy, Kotlin, Dart | `@FromIdentity` |
| Python | `Annotated[Identity, from_identity]` |
| Go, Lua | `// @FromIdentity` or `-- @FromIdentity` on the line before the parameter |
| Ruby, PHP, Rust | `#[FromIdentity]` |
| C# | `[FromIdentity]` |
| F# | `[<FromIdentity>]` |
| C++ | `[[FromIdentity]]` |

The parameter type is an ordinary model in `server/models/`, data only.
Its fields are the fields that `identify` returns: at least `user` and
`roles`, and any other fields the provider adds. The model must name every
field the provider returns, as a body model must.

```go
// @PostRoute
// @SignedIn
func (c *MessagesController) PostMessage(
	// @FromIdentity
	caller Identity,
	// @FromBody
	body NewMessage,
) (Message, error) {
	return c.messages.Post(caller, body)
}
```

### Build rules

- One `@FromIdentity` parameter per method at most (TY2015).
- Only on a request route (`@GetRoute`, `@PostRoute`, `@PutRoute`,
  `@PatchRoute`, `@DeleteRoute`) or a stream route (`@StreamRoute`). An
  identity provider, a worker, a socket method or a gateway method with
  `@FromIdentity` stops the build (TY2015).
- Not on a route whose rule is `@Public`. The rule on the method wins over
  the rule on the class, as always. A `@Public` route does not ask the
  identity provider, so it has no identity to give (TY1322).
- The type must be a model, not text, a number or a list.

### The server

- The identity provider is still called once per request, only for
  `@SignedIn` and `@Allow` routes. Y2 adds no call.
- The server keeps the whole JSON object that `identify` returned, with its
  keys sorted, for the length of the request.
- When the route takes a `@FromIdentity` parameter, the server checks that
  object against the parameter's model, exactly like a body, and passes it
  as that argument. Each language turns it into the model the same way it
  turns a `@FromBody` argument into its model.
- If the object does not fit the model (a field is missing, has the wrong
  type, or is not declared), that is a defect on the server side, not a bad
  request. The server answers the fixed 500
  `Handler execution failed. Reference: <id>.` with the
  `x-tachyon-request-id` header, never a 400, and the route method does not
  run. The answer carries the built-in case `UnexpectedError`
  ([ADR 0142](0142-an-error-handler-shapes-the-answers-tachyon-makes-itself.md)),
  so an app can shape its body. The server writes one line on stderr:
  `{"event":"route.identity_mismatch","request_id":...,"route":...}`.

### Where the identity never goes

The identity's values never appear:

- in `/api.json`, an `OPTIONS` answer or the route schema: the parameter is
  left out, like the native request parameter;
- in any log line, including `route.identity_mismatch`;
- in the `/.tachyon/error` request that asks an error handler for a body;
- in the 500 answer.

`ty handler invoke` does not run the identity provider, so it has no
identity to give: a route method with a `@FromIdentity` parameter answers
400 with the part `identity` there and does not run.

## Consequences

- A route method gets the caller's user, roles and any extra fields as a
  typed model, with no second token check.
- The identity model and the identity provider must agree. A provider that
  returns a field the model does not declare makes every route with
  `@FromIdentity` answer 500 until the model is fixed. The log line names
  the route, so the cause is easy to find.
- `RouteGuard::admission` returns the refusal or the identity. The
  repository policy test checks that `serve_allowed_route` is only called
  after it, as it checked `RouteGuard::refusal` before.

## Out of scope

- Socket routes: `OnOpen(session)` receives no identity.
- An optional identity on a `@Public` route. That would need the server to
  call the identity provider on public routes, which it does not do.
