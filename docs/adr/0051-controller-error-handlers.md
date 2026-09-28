# ADR 0051: Controllers Share Error Handlers

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0048](0048-yon-routes-come-from-annotations.md) (controller
  composition, response status and published error responses)
- Preserves: [ADR 0044](0044-an-annotated-file-holds-one-class.md) (one type
  per file), the supervised handler protocol, resource limits and middleware
  ownership

## Context

Controllers often need the same answer for the same application error.
Repeating a catch block in every route hides the normal route logic.
Matching errors by a string also repeats a type already present in the code.

## Decision

### Files and declarations

- Put `@ErrorHandler` on a class named `<Name>ErrorHandler`, in the matching
  `server/controllers/<Name>ErrorHandler.<ext>` file. Several handler classes
  may exist. They are providers in the current invocation, not route entries.
- Put `@HandlesError` on an instance method. The method name is free. This
  annotation takes no arguments: the first parameter's declared native error
  type selects the errors it handles. An optional second parameter receives
  the request. JavaScript uses a JSDoc `@param` type, as routing does.
- A custom native error type lives in `server/errors/<Type>.<ext>`, directly
  under that folder. The file holds one matching class or type and its own
  methods or Rust trait implementations. It is not an injectable role and
  carries no Tachyon annotations. The existing one-class, import and
  load-time restrictions apply. Models stay data-only.
- A controller composes the same-language error handlers and error sources.
  It still excludes sibling route controllers. Discovery is bounded and
  does not execute application code or follow symlinks.
- Go uses comment annotations from ADR 0050. Other languages retain their
  current marker spelling. Ruby support is a later queue item.

### Matching and execution

- Matching uses the actual native error object and declared parameter type,
  not a message string. Where inheritance exists, the nearest matching type
  wins. Go uses `errors.As`; Rust matches a controller's `Result` error type
  before response conversion.
- Two methods handling the same error type are refused, including methods
  in different handler classes. Missing or ambiguous error parameter types,
  non-error types, static methods, streams and route markers on an error
  method are refused. Native compilers check native trait or base-type
  compatibility; dynamic languages check error-type compatibility at runtime.
- Interception surrounds the controller method and service calls it makes.
  It does not surround provider construction, lifecycle hooks, middleware,
  workers, gateways, or Tac client code. It does not restart a stream after
  output has begun.
- Handler dependencies come from the controller invocation's existing
  provider container. No process-global handler registry is introduced.
- The order is controller, matching error handler, then middleware `after`.
  Existing cleanup and deadline rules remain in effect.

### Responses and safety

- A handler returns a normal response or a typed model. `@ResponseStatus`
  accepts 400 through 599 on an error method and defaults to 500. The normal
  route annotation still accepts only success statuses.
- Error model responses are checked using the same model rules as successful
  controller responses. The route's published schema lists error types,
  statuses and response shapes. Several types may share a status; the answer
  must fit one of the declared shapes for that status.
- An unmatched error keeps the fixed safe 500 response. If the error handler
  itself throws, it also gets the fixed safe 500 response; handling never
  recurses. Error messages and stacks are not sent to clients unless the
  author explicitly places them in a returned response.
- Foreign names such as `ControllerAdvice`, `RestControllerAdvice` and
  `ExceptionHandler` are refused with guidance to use these two markers.

## Consequences

An author defines the error type once and uses normal language references.
Error handlers remain local to one supervised invocation, while their source
is shared across controllers. The source layout makes the distinction clear:
models are data, errors are native failure types, and handlers choose public
responses. Tac event handling and remote topic streams do not change.

## Implementation Notes

- Go walks the error chain from the outside in with `errors.Unwrap`. The
  first error in the chain that has a concrete handler type wins, so a
  wrapper type beats the type it wraps. When no single chain matches, for
  example with `errors.Join`, `errors.As` decides. Two matches there stay
  unmatched and get the fixed 500. A catch-all `error` parameter runs only
  when no concrete type matches.
- Java, Kotlin, C# and Dart used to put the exception message into their
  500 body. An unhandled controller error now ends the handler process with
  a failure status, so the supervisor answers with the fixed 500, as the
  other languages already did.
- The handled error types that are not error types (`any`, `Object`,
  `dynamic`, `mixed`, plain values and others) are one list in
  `yon/parser/errors.rs`.
