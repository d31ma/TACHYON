# Phase 2 Yon Handler Boundary

> Current Yon addition (2026-09-11): ADR 0021 (ADR means Architecture Decision
> Record) adds two optional features across the eight owned adapters:
> compile-time annotated controllers and constructor injection that is local to
> one source file. The plain Handler is still supported. See
> `YON_ANNOTATIONS.md` for the grammar, scopes, bounds, and the deferred APIs
> for cross-file use and hooks.

> Historical milestone: this document records the original Phase 2 gate for
> JavaScript and Python. ADR 0020 replaces its scope for handler authoring and
> streaming. ADR 0020 adds a plain Handler convention, eight owned Yon
> languages, explicit relay, and multi-frame streaming with native generators.
> For current authoring behavior, use the README and ADR 0020.

## Outcome

A developer can call a JavaScript or Python Yon handler through the compiled
Rust `ty` executable. For each invocation, Tachyon:

1. starts one isolated child process directly and supervises it;
2. communicates with it only through Handler Protocol v1;
3. returns a validated response or a stable diagnostic.

Phase 2 sets up execution and isolation. It does not inject handler values into
HTML. It does not merge route context. Under ADR 0016, Yon is a REST-only
boundary.

## Public Workflow

```text
ty handler invoke server/routes/products/yon.js \
  --project . \
  --route /products \
  --method GET
```

The command writes one Handler Protocol v1 response as JSON to stdout.
Diagnostics go to stderr. They use the global diagnostic format, either human
or JSON.

The invocation accepts:

- a `yon.js` or `yon.py` source path, relative to the project;
- an explicit route and HTTP method;
- an optional UTF-8 request body, and repeated `name=value` headers;
- an optional request ID;
- a bounded timeout from 1 through 300,000 milliseconds;
- repeated `--allow-env NAME` entries;
- explicit overrides for the JavaScript or Python runtime executable;
- an optional concrete `--target` such as `/items/42?limit=3`, used instead of
  `--route`.

For an annotated controller, the command binds `@FromPath`, `@FromQuery`,
`@FromHeader`, `@FromCookie` and `@FromBody` arguments with the same code as
the server. The target (or the route when there is no target) chooses the
controller route and supplies path and query values. `--header` and `--body`
supply the rest. A value that does not match its model gets the same `400`
JSON answer as the server, and the controller does not run. A target that no
route of the controller answers is a `TY2005` diagnostic. A plain `Handler`
gets the request without arguments, as before.

`ty handler invoke` is a local operator tool. It does not run the identity
provider, the route access rules (`@Public`, `@SignedIn`, `@Allow`) or the
root middleware, so it can call routes that the server would refuse.

Ctrl-C cancels the invocation. It uses the same cancellation path as the
library API.

## Handler Authoring Contract

A JavaScript handler exports a `Handler` class from an ESM (ECMAScript module)
file. The export can be named or default:

```javascript
export class Handler {
  static async GET(request) {
    return { products: [] }
  }
}
```

A Python handler uses a `Handler` class:

```python
class Handler:
    @staticmethod
    async def GET(request):
        return {"products": []}
```

Rules for the handler method:

- Tachyon selects the uppercase static method that matches the HTTP method.
- The method receives the validated request envelope. In JavaScript it is an
  object. In Python it is a dictionary.
- Both synchronous and asynchronous methods are accepted.
- An ordinary return value must be JSON-serializable. It becomes a UTF-8 JSON
  response body with status 200 and
  `content-type: application/json; charset=utf-8`.
- Instead, a handler may return an explicit `{status, headers, body}`
  descriptor.
- Tachyon passes a `text/html` body through unchanged. It never treats the body
  as a Tachyon template.

These failures become a bounded Handler Protocol error response:

- a missing class or method;
- an import failure;
- a thrown exception;
- a result that cannot be serialized.

Tachyon redirects the application's console or print output to bounded stderr.
This output never shares stdout with the protocol.

## Source and Route Discovery

- `server/routes/**/yon.js` selects adapter `javascript.v1`.
- `server/routes/**/yon.py` selects adapter `python.v1`.
- Both files may exist at the same route. Tachyon orders them by their portable
  source path.
- A route with only a handler is an API route.
- A handler may share a route with a Tac page. For the methods the handler owns,
  the handler runs first.
- Tachyon still rejects other `yon.*` companions until their adapter phase.
- Handler files and their parent directories must be regular paths inside the
  project. They must not be symbolic links.
- Handler source is UTF-8, contains no NUL bytes, and is at most 1 MiB.

Route Manifest v1 records the discovered handlers. A route with only a handler
advertises the HTTP method set of the protocol. A route with only a view keeps
GET and HEAD.

## Process and Protocol Contract

- Tachyon passes the executable and each argument directly. It does not use a
  shell.
- Each frame is a four-byte unsigned big-endian length, followed by one UTF-8
  JSON envelope.
- A complete frame, including the JSON, cannot be larger than 16 MiB.
- Tachyon accepts exactly one request and at most one matching response.
- These are protocol violations: extra stdout after the response, malformed
  JSON, invalid response fields, a request ID that does not match, output that
  is too large, or an early exit.
- Tachyon reads stderr all the time. It keeps up to 64 KiB and reports it only
  in bounded diagnostics.
- The default deadline is 30 seconds. The deadline includes time waiting in the
  concurrency queue, process startup, handler work, framing, and process exit.
- On cancellation, Tachyon writes a protocol cancellation frame and waits a
  100 ms grace period. Then it kills the child and reaps it.
- After a timeout, cancellation, crash, or protocol failure, Tachyon never
  reuses the process. The next invocation starts a clean child.
- By default, a supervisor runs at most 16 processes at the same time.

## Environment Policy

- Tachyon clears the child environment before launch.
- The child gets only two kinds of variables: the minimum runtime variables the
  host platform needs, and names that the developer explicitly allowlists.
- An allowlist name that is invalid or not Unicode causes a failure. Tachyon
  does not continue with it.
- Tachyon never logs environment values.

Each adapter looks up its runtime explicitly. If the runtime is missing,
Tachyon reports a diagnostic that tells the user what to do. It does not fall
back to another language or to a shell command.

## Stable Failure Families

- `TY2001`–`TY2009`: invalid handler source, request, runtime, or environment.
- `TY2101`–`TY2109`: failures in spawn, framing, protocol, exit, or stderr.
- `TY2110`: deadline exceeded.
- `TY2111`: invocation cancelled.
- `TY2112`: a required Yon runtime executable is missing. This can happen
  during the readiness check, or in the race when the process is spawned after
  readiness. Tachyon hides (redacts) configured executable paths.
- `TY2201`–`TY2209`: handler authoring or execution failures that the adapter
  reports. `TY2205` ("Input breaks a Checked rule.") carries the broken rules
  of a `@Check` parameter in the optional `violations` member of the v1 error
  object; the server answers `422` with problem details (ADR 0101).

Messages are bounded. They may include a source path relative to the project
and shortened stderr. They never include environment values or an unbounded
stack trace.

## Acceptance Criteria

- Typed Rust envelopes serialize to every Handler Protocol v1 wire shape.
- JavaScript and Python pass one shared behavior corpus with real runtimes.
- The compiled `ty` binary calls both languages successfully.
- These round-trip consistently: requests, response bodies, headers, request
  IDs, sync and async methods, and Unicode.
- These fail safely: missing runtimes, classes or methods; exceptions;
  malformed, too large or trailing protocol output; mismatched IDs; source paths
  that escape the project; symbolic links; and floods of stderr.
- On a deadline or explicit cancellation, Tachyon sends a cancel frame, kills
  the process when needed, and reaps it.
- A crash stays isolated, and the next invocation succeeds.
- A secret test variable (canary) is not present unless its name is explicitly
  allowed.
- These gates pass: workspace quality, coverage, supply chain, legacy
  compatibility, macOS, Linux container, Windows buildability, and native CI.

## Out of Scope

These belong to later phases:

- handler pooling
- streaming frames
- TypeScript and additional languages
- extracting or merging route context
- HTML injection
- control tags, components, bindings, and islands
- production HTTP routing
- runtime installation
- CPU and memory sandboxing at the OS level
- JavaScript dependency imports
- filesystem and network capabilities
- native rendering
