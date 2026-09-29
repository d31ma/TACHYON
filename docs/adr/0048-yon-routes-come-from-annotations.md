# ADR 0048: Yon Routes Come From Annotations

- Status: Accepted, amended by [ADR 0058](0058-streams-are-a-route-and-a-request.md)
  (a streaming route is `@StreamRoute`, and its name starts with `stream`)
- Date: 2026-09-25
- Supersedes: the Yon parts of filesystem routing: `server/routes/`, the
  `yon.<ext>` route files, controllers beside a route, HTTP verb-named
  controller methods, and `OPTIONS.schema.json` files
- Amends: [ADR 0020](0020-decorator-free-authoring-conventions.md),
  [ADR 0021](0021-yon-annotation-frontend.md),
  [ADR 0031](0031-realms-mirror-each-other.md) (Tac pages keep folder
  routing; Yon does not, so the two realms no longer mirror each other here),
  [ADR 0044](0044-an-annotated-file-holds-one-class.md) (route files) and
  [ADR 0047](0047-annotation-names-are-plain-english.md) (the `Client` and
  `WebCall` names)
- Preserves: Tac page routing in `client/pages/`, the handler process
  protocol, workers, root middleware, test units, gateways, and every other
  annotation

## Context

A Yon route was a folder. `server/routes/orders/_id/yon.ts` answered
`/orders/{id}`, and a controller beside it answered each HTTP method with a
method named `GET`, `POST` and so on. A request or response shape lived in a
separate `OPTIONS.schema.json` file of regular expressions.

Spring developers expect the path next to the code, in an annotation. The
folders also split one controller into many files, one per path, and the
schema file repeated types that the code already declares.

Spring's names are well known, but some are hard for a beginner who reads
English as a second language. So Tachyon keeps names close to Spring and
explains each one in plain words: in the editor, in every error message, and
on the website.

## Decision

### Controllers

- Every controller lives in `server/controllers/`, one class per file, named
  `<Name>Controller.<ext>`. Each controller file is its own Yon program: the
  compiler composes it with the shared roles (`services/`, `stores/`,
  `clients/`, and the rest) as before, but never with another controller.
- `RequestRoute("/orders")` on the class marks the controller and gives the
  path that its routes start with. It replaces `Controller`. The path is
  optional; without it, routes start at `/`.
- `GetRoute`, `PostRoute`, `PutRoute`, `PatchRoute` and `DeleteRoute` on a
  method say that the method answers that HTTP method. Each takes an optional
  path, for example `@GetRoute("/{id}")`. The full path is the class path
  followed by the method path.
- A path part is lowercase letters, digits and single hyphens, or a path
  variable in braces, such as `{id}`. A path variable name is lowercase
  letters, digits and underscores. Tac pages keep their `_id` folders; both
  forms name the same route, so a page and a controller can share a path.
- A route method's name is free, but it starts with its HTTP method in the
  language's own casing: `getOrderById`, `GetOrderById` or
  `get_order_by_id` for `@GetRoute`. A method with the wrong start is refused,
  and the message explains what the annotation means.
- `ResponseStatus(201)` on a route method sets the status of a normal (200)
  answer. It takes a success status from 200 to 299.
- `Stream` works as before on a route method. In one controller, the routes
  of one HTTP method all stream or none does.

### Parameters

- Each parameter of a route method carries one marker:
  - `FromPath`: a path variable, such as `{id}`;
  - `FromQuery`: a value from the query string, after `?`;
  - `FromBody`: the request body, read as JSON;
  - `FromHeader`: a request header;
  - `FromCookie`: a cookie.
- A marker may name the value, for example `@FromQuery("page_size")`.
  Without a name it uses the parameter name.
- One parameter may have no marker. It receives the whole request, as a
  controller method did before.
- A `GET` route has no `FromBody` parameter, and a method has at most one.
- Python writes a parameter marker inside the parameter's type, so the file
  stays valid Python: `order: Annotated[Order, from_body]`,
  `trace: Annotated[str, from_header("X-Trace")]`, with
  `from typing import Annotated`. The first `Annotated` argument is the type
  that validation uses. Client parameters work the same way
  (`id: Annotated[str, path_variable]`). The old form `@from_body order` is
  refused with a message that shows the new form, and
  `ty migrate fix --write` converts it. (Amended 2026-09-26: ADR 0056.)

### Validation and the published schema

- The server builds the arguments before the handler runs. It converts path,
  query, header and cookie text to the parameter's type and reads the body as
  JSON. A value that does not fit its type, or a missing required value, is
  answered with 400 and a JSON body that names the part.
- Types come from the method signature: `string`, whole numbers, decimal
  numbers, `boolean`, lists, and models from `server/models/` (ADR 0043).
  JavaScript declares them with JSDoc on the method (`@param {Order} order`,
  `@returns {Receipt}`).
- Model records accept only declared fields, including nested records and
  records in lists. Required fields must be present; optional fields may be
  absent. Their published schemas set `additionalProperties: false`. An
  explicitly untyped (`any`) value remains open. Fields of a parent model
  (`extends`, a Python base class, a Go embedded struct, a TypeScript `&`)
  join the record. A record stays open, with no `additionalProperties: false`,
  when Tachyon cannot see all of its fields: a parent that is not in
  `server/models/`, a PHP trait, a TypeScript index signature, or a field type
  it cannot read. A PHP model whose constructor needs a parameter that is not
  a public property cannot be a request body, because the request cannot fill
  it; the build names that parameter. (Amended 2026-09-26 after review.)
- The answer of a route with a model return type is checked against the
  model. A mismatch is a server error (500) that names the field in the log.
- Like Spring's `@RequestBody` and message converters, `FromBody` hands the
  method an instance of its model (a record, data class, dataclass, struct or
  class, with nested models and lists of models), and a returned model, list
  of models or stream of models is written as JSON with the model's field
  names. Each language uses its own built-in tools: reflection in Java,
  Kotlin and PHP, `System.Text.Json` in C#, `dataclasses` in Python,
  `encoding/json` in Go, and generated code in Rust and Dart. JavaScript and
  TypeScript models are plain objects. A PHP array gets its item type from a
  doc comment, as in PHPStan: `/** @var list<Line> */` on a model property,
  `@param list<Line> $lines` and `@return list<Line>` on a controller method.
  A return type that is not text, a number, true or false, a list, a JSON
  value or a model is refused at build time. (Amended 2026-09-27.)
- The same types produce the published API schema: the server answers
  `OPTIONS` on a controller path with the schema of that path and an `Allow`
  header, and `/api.json` lists every route. `OPTIONS.schema.json` files are
  refused.
- A path that exists but has no route for the HTTP method is answered with
  405 and an `Allow` header. `HEAD` is answered by the `GET` route.

### Clients

- `HTTPClient("/api/orders")` on a class marks a client and gives the path its
  requests start with. It replaces `Client`.
- `GetRequest`, `PostRequest`, `PutRequest`, `PatchRequest` and
  `DeleteRequest` on a method say that calling the method sends that HTTP
  request, with an optional path. They replace `WebCall`.
- Client parameters use Spring's names: `PathVariable`, `RequestParam`,
  `RequestBody` and `RequestHeader`.
- Clients keep their current reach: the browser client transport is
  JavaScript and TypeScript only, and the other languages send requests
  through their configured HTTP adapter as before.

### Old forms

- These are refused with a message that names the new form: a
  `server/routes/` folder, `yon.<ext>` route files, `OPTIONS.schema.json`,
  `Controller`, verb-named controller methods, `Client`, `WebCall`,
  `HttpExchange`, `HTTPExchange`, the `*Exchange` method markers,
  `RequestMapping` and the `*Mapping` method markers.
- `ty migrate fix --write` moves route controllers into
  `server/controllers/`, adds `RequestRoute` and the route markers, renames
  verb-named methods, and converts clients. Where a change is not safe, it
  reports a manual step.

## Consequences

- A reader finds a URL by searching for its path in `server/controllers/`.
- Request and response shapes live in one place: the models the code already
  uses.
- This is a breaking change. Projects stop building until they run
  `ty migrate fix --write` or move their routes by hand.
- The Tac and Yon realms no longer mirror each other in routing: pages stay
  folders, and APIs are annotations.

## Amendment (2026-09-29): an HTTPClient is a contract

`HTTPClient` follows Spring's `@HttpExchange` interface clients: it has no
implementation, and Tachyon generates each request.

- A client is declared in the bodiless form of its language: an `interface`
  in Java, Groovy, Kotlin, C# and PHP; `type XClient interface { … }` in Go;
  an `abstract interface class` in Dart; a `trait` in Rust; a `protocol` in
  Swift; a `Protocol` class with `...` bodies in Python. JavaScript and Ruby
  have no method without a body, and TypeScript accepts annotations only on
  a method with a body, so those languages write a class whose methods are
  empty. F# writes an interface: a type with only `abstract` members, which
  Tachyon turns into a class for the F# behavior bodies of ADR 0073.
- The build fails with `TY1308` (fail closed, rule table of ADR 0056) for a
  class client where the language has a contract form, any method body, a
  field, a constructor, a method without a request or gateway annotation, and
  a client that also carries another role: "An HTTPClient has no
  implementation: Tachyon generates each request. Move the logic into a
  service." The old unmarked descriptor methods that returned
  `{query, headers, body}` are gone; `RestClient` covers requests whose shape
  is known only at call time.
- Before events, lifecycle and injection read the role files, the compiler
  turns each contract into the concrete client it used to be, with the
  behavior context created from a Setup: `{"setup":"CatalogSetup"}`, by
  default the client name with `Setup` instead of `Client`, and the adapter
  name as partition. Gateway-only clients get no context. Tac JavaScript and
  TypeScript clients stay managed by the page.
- `ty migrate fix --write` rewrites a class client whose methods only send
  requests; a client with logic or state is a manual step.
