# ADR 0122: ty test and ty start Agree on Yon Handler Code

- Status: Accepted
- Date: 2026-11-11
- Amends: [ADR 0051](0051-controller-error-handlers.md) (which handlers
  compose `server/errors`) and
  [ADR 0095](0095-native-go-packages-live-in-server-packages.md) (Go types from
  `server/packages` in generated handler code)
- Preserves: [ADR 0048](0048-yon-routes-come-from-annotations.md) (request
  checks at the HTTP boundary) and the test-unit contract of `server/tests`

## Context

A team that wrote Go Yon services found three places where `ty test` passed
code that `ty start` or `ty bundle` then refused, or where the rule was not
written down:

1. A request-body field of a Go model without `,omitempty` in its `json` tag
   is required. `ty start` answers 400 with "'body.<field>' is required" when
   a caller leaves it out. `ty test` never showed this.
2. A Go `@FromBody` type or `@HandlesError` error type from another Go package,
   such as `core.Order` from `server/packages`, passed `ty test`. `ty start`
   and `ty bundle` failed to compile with "undefined: core".
3. An identity provider or a worker that used an error type from
   `server/errors/` failed to compile (`TY2004`). Only route controllers and
   test units composed `server/errors`.

## Decision

### 1. Required body fields: the rule stays at the HTTP boundary

- A test case calls the controller method directly, with a value of the
  language. A Go struct, a Java record or a Python object always has every
  field, so in a test nothing can be "missing". The check needs the JSON text
  that a caller sent, and only the HTTP boundary has it.
- So `ty test` does not make the request checks of `ty start` (missing
  required fields, undeclared fields, access rules), in any Yon language. We
  do not add an HTTP test client in this change: it would be a new testing
  surface in 14 languages.
- The rule is now written down in the docs (Route contracts and Yon test
  units). A field is optional when its type allows null in the language's
  own spelling (`*T` in Go, `T?`, `?T`, `Option<T>`, `Optional<T>`,
  `T | None`, `std::optional<T>`), when it has a default value, in
  TypeScript when it is written `name?:`, in Python with `NotRequired` or
  `total=False`, and in Go when its `json` tag has `,omitempty`. Every other
  field is required. `OPTIONS` and `/api.json` list the required fields.

### 2. Go types from other packages work in generated code

- Tachyon writes Go code for a handler in files of their own: the route
  calls (`routes.go`, which used to be part of `container.go`), the error
  matching (`errors.go`) and the socket endpoint (`container.go`). Go imports
  belong to one file, so a type such as `core.Order` was unknown there.
- `GoImports` reads the import declarations of the handler's Go files and
  copies into each generated file only the imports that its type names need:
  the `@FromBody` and other route parameter types, the concrete
  `@HandlesError` error types and the socket message type. An import without
  a name is named after the last part of its path.
- A type whose package no file imports, and two files that import different
  packages under one name, stop the build with `TY2015` and a message that
  names the package. The fix is to import the package, or to give one import
  another name.
- Other Yon languages append their generated code to the one composed file,
  where the authored imports are joined, so they did not have this gap.

### 3. Every Yon server handler composes the error types

- Every composed Yon handler under `server/` composes the plain error types
  of `server/errors/`: route controllers, socket controllers, the identity
  provider, gateways and workers, with or without `Needs`. A test unit
  already did. So a type that `ty test` sees is also there at `ty start`.
- Error handlers (`<Name>ErrorHandler`) still wrap only the methods of route
  controllers, as ADR 0051 says. A worker or an identity provider gets the
  error types, not the error handlers.
- Middleware is not composed, so it is unchanged. C++ already places every
  error type in each program (ADR 0099). Error types can stay in
  `server/errors/`; moving them to `server/shared/` is no longer needed.

## Consequences

- A Go service can keep its body and error types in `server/packages` and
  its error types in `server/errors`, and `ty test`, `ty start` and
  `ty bundle` agree.
- A worker or identity provider in a project with `server/errors/` now gets a
  composed source even when it has no other role files. The error types are
  plain classes, so this adds code but no behavior.
- Request-body rules are still not checked by `ty test`. Authors read them in
  the route schema and check them with a real request.

## Implementation Notes

- `annotation_modules.rs`: `Discovery::search_server_errors` searches
  `server/errors` for every entry in the server realm. The flag that brings
  in error handlers is now named `include_error_handlers`.
- `yon/go_imports.rs` (`GoImports::for_types`), used by
  `yon/route_calls/go.rs`, `yon/error_calls/go.rs` and `yon/go_container.rs`.
- Tests: `annotation_modules::error_tests` (worker, identity provider, socket
  and client cases), `yon::go_imports::tests`, `it::go_package_types`, and
  the Go cases of `it::behavior_go` and `it::route_access`, which now use a
  `server/errors` type.
