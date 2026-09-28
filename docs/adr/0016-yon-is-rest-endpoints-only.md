# ADR 0016: Yon Is REST Endpoints Only

- Status: Accepted
- Date: 2026-08-10
- Supersedes: the `yon.html`, Yon view-context, and Yon server-rendering
  portions of ADR 0002 and the Phase 1/3 specifications

## Context

Yon owns server routes and supervised handler execution. An experimental
rewrite feature also did the following:

- It treated `server/routes/**/yon.html` as a template.
- It executed colocated `GET` handlers during compilation, through a private
  `view.context` operation.
- It merged the values those handlers returned.
- It rendered the result on the server.

That design broke the endpoint boundary in three ways:

- An HTTP method ran at build time.
- A handler return value had two meanings.
- REST routes took part in a view compiler, although no one had explicitly
  decided what the response should be.

## Decision

Yon is only an HTTP endpoint framework.

- `server/routes/**/yon.*` files are handler sources.
- Handlers execute only for an HTTP request, a worker or middleware lifecycle
  step, or an explicit `ty handler invoke` operation.
- Ordinary JavaScript and Python handler return values become JSON responses.
- A handler may return an explicit `{status, headers, body}` descriptor.
- HTML requires an explicit `Content-Type: text/html` header. Yon sends the
  supplied body unchanged.
- Yon never parses an HTML response. It never interpolates, escapes, expands
  components in, or evaluates structural tags in it.
- Yon rejects `yon.html` with `TY1008` and a migration message that tells the
  developer what to do.
- These no longer exist: the private `view.context` operation, populated
  route-context declarations, the server template evaluator, and build-time
  handler execution.
- Route Manifest v1 keeps its required `context` member with empty export
  lists. It keeps the member only for wire compatibility. The member declares
  no executable behavior.

Tac remains the only view frontend. Tac remains exclusively client-rendered.

## Consequences

- Builds are deterministic and do not execute endpoint code.
- `GET` has one meaning: handling a GET request.
- The application explicitly decides the response content type.
- Application code or another library can generate an HTML response. Tachyon
  treats that response as opaque data and does not look inside it.
- Route Manifest v1 publishes only the deprecated empty context shape. A
  future Route Manifest v2 may remove the member with an explicit migration.
- Earlier Yon-template fixtures and documentation are invalid. They must
  migrate to Tac pages or to explicit Yon HTTP responses.

## Rejected Alternatives

- **Render `yon.html` only when a handler chooses `text/html`**: Yon would
  still have a hidden template engine. A response body and a source file would
  be mixed up.
- **Run `GET` during build only for static routes**: This breaks the meaning of
  HTTP methods. Builds would run application side effects.
- **Keep populated route context as a deprecated behavior**: This would
  advertise and keep build-time handler execution. The empty v1 wire member
  stays only because the published contract cannot change.

## Acceptance Gate

- Discovery rejects `yon.html`.
- Builds do not invoke Yon handlers.
- Ordinary handler values remain JSON responses.
- Explicit `text/html` responses keep their status, headers, and body. There is
  no template processing and no Tac runtime injection.
- No production server renderer or server expression evaluator remains.
- Route Manifest v1 accepts and emits its deprecated empty context shape.
- These gates pass: canonical format, check, Clippy, test, documentation,
  dependency policy, and HTTP dispatch.
