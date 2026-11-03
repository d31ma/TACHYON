# ADR 0042: A Gateway Method Calls Another Language

- Status: Accepted
- Date: 2026-09-24
- Amends: [ADR 0017](0017-mandatory-yon-layers-relay-and-streaming.md) (other
  languages are reached only through relay, and what a relay program inherits)
  and [ADR 0027](0027-scoped-discovery-and-managed-resources.md) (the managed
  resource endpoint gains one operation)
- Preserves: `relay(command, request)` as the way to forward a whole request,
  Handler Protocol v1 for handlers, deadlines, bounded output and
  process-group cleanup

## Context

A Yon service sometimes needs code that is written in another language. For
example, a Python service needs a Java class, or a JavaScript service needs a
Ruby program.

Today the only way is `relay(command, request)`. It forwards a whole HTTP
request from a Handler method and returns an HTTP response. It cannot call one
method with arguments and get a value back. It also cannot run a class in
another Yon language, because each entry assembles only the role files of its
own language.

`relay()` also has two faults:

- A relay program inherits `YON_RESOURCE_ENDPOINT`, `YON_RESOURCE_TOKEN` and
  `YON_APPLICATION_JSON`. So an outside program receives the invocation's
  storage capability and the private application configuration.
- A relay's timeout is the full request deadline, counted from when the relay
  starts, not the time that is left.

## Decision

- `@Gateway` declares a method that another language answers. The method has
  no body, like `@HttpExchange`. It always takes a JSON object with one of two
  targets:
  - `{"class":"server/services/ReportService.java","method":"summarize"}`
    names a role file in another Yon language. `method` is optional and
    defaults to the annotated method's name.
  - `{"command":["ruby","server/delegates/report.rb"]}` names a program. The
    first entry is a program name on the supervisor's `PATH`. Every entry that
    contains a slash is a file under `server/delegates/` in the captured
    snapshot. No shell runs, and call arguments never go on the command line.
- The method's positional arguments (at most 32) are sent as bounded JSON. The
  other side's return value becomes the method's result.
- A class target runs as a generated unit in its own language. The unit
  resolves the class with its own dependencies and calls only the methods that
  callers declared. It speaks Handler Protocol v1 with the operation
  `gateway.invoke`.
- A command target reads plain JSON on stdin, `{"method": "...", "arguments":
  [...]}`, and writes the bare JSON result on stdout. It exits with status 0 on
  success. Any other exit status is a failure.
- A gateway target cannot declare a gateway itself. Only one hop is allowed,
  so cycles cannot happen.
- The build checks every target and fails with TY1312 when:
  - the target is missing, outside `server/`, or not in the snapshot;
  - the target is in the caller's language (inject it instead);
  - the target's language is not a Yon language;
  - the method is missing or not a concrete instance method;
  - the argument counts differ;
  - a typed parameter or result is not the language's JSON value type;
  - the target declares a gateway itself.
  A misused annotation fails with TY1308: in Tac, on an HTTP verb method, on a
  stream, or together with another method annotation.
- The caller asks the supervisor through its per-call resource endpoint and
  sends only a key that the build declared. The supervisor starts a new
  process for each call.
- Limits: 32 arguments; 1 MiB for the arguments and 1 MiB for the result,
  depth 32, 10,000 nodes; 64 calls and 4 concurrent calls per invocation; a
  gateway process pool separate from the handler pool; and the caller's
  remaining deadline minus the cancellation grace.
- A failure reaches the caller as a BehaviorError (`invalid`, `timeout`,
  `capacity` or `adapter`) with a fixed message. The target's own messages and
  stderr never reach the caller. The supervisor logs only a
  `handler.gateway_failed` category.
- Outside programs are less trusted than project code:
  - A command target, and every `relay()` program, gets only the baseline
    environment plus the explicit allowlist. It never receives
    `YON_APPLICATION_JSON` or the `YON_RESOURCE_*` values.
  - A `relay()` program's deadline is the time left in the request.
  - A class target is project code and runs like any route, with the same
    environment policy and its own managed-storage access.
- Gateways are refused under Firecracker isolation, like managed storage,
  because resource capabilities are not passed into the microVM.

## Consequences

- A service calls another language like an ordinary method. `relay()` stays
  the way to forward a whole request.
- Each call starts a process. JVM and .NET targets pay their start-up time on
  every call.
- `server/gateways/` is reserved for generated units (TY1310).
- A relay program that read `YON_APPLICATION_JSON` no longer receives it. It
  must get its configuration from its own files or explicit environment
  variables.

## Implementation notes (2026-09-25)

These details were settled while implementing the decision:

- The supervisor writes the time left, minus the cancellation grace, into
  `deadline_ms` just before it writes the request. So a runner can still
  answer its own relay or gateway timeout before the supervisor cancels, and
  handlers see the remaining time. Each runner measures that budget from its
  own start (JavaScript from process start, Java and Kotlin from JVM start)
  and keeps it private.
- The 1 MiB limits apply to the whole request and answer on the resource
  endpoint, including their small `{"operation","key","arguments"}` and
  `{"result"}` envelopes. The endpoint always answers HTTP 200 with
  `{"result": value}` or `{"error": kind}`.
- One hop is enforced twice: a target class that declares a gateway fails
  with `TY1312`, and a generated unit runs without a gateway table, so a
  gateway reached from inside it fails with `adapter`.
- A command target does not receive `YON_HANDLER_PROTOCOL`; it speaks plain
  JSON. A relay program still receives it.
- A target language whose composed role files use local events or lifecycle
  hooks cannot host a generated unit yet (`TY1312`). A private or `suspend`
  JVM target method fails when the unit compiles, not with `TY1312`.
- Overloaded targets select one concrete JSON-valued signature with the
  declared argument count. Ambiguous matches and callers requesting different
  argument counts for the same target method fail with `TY1312`. Rust targets
  use `&self`; consuming or mutably borrowing the shared provider is refused.
- Reserved environment names stay excluded even when explicitly allowlisted,
  using case-insensitive comparison. PHP captures its relay context in a
  prepended runtime, before application initialization.
- Endpoint shutdown cancels gateway operations and joins their process cleanup.
  A deadline never aborts an operation while it is terminating and reaping its
  process group. Command stderr overflow is an `adapter` failure.

## Implementation notes (2026-09-25, security review)

- Removing environment values prevents accidental inheritance only. It is not
  a sandbox: an outside program runs as the same operating-system user and can
  read its parent's environment (for example `/proc/<parent pid>/environ`) and
  the project's files. A relay program gets the baseline, the allowlist,
  `YON_HANDLER_PROTOCOL` and its handler's `NO_PROXY` values; a command
  target gets only the baseline and the allowlist. The Windows baseline
  (`PATH`, `PATHEXT`, `SYSTEMROOT`, `WINDIR`, `COMSPEC`, `TEMP`, `TMP`)
  differs from the Unix one (`PATH`, `LANG`, `LC_ALL`, `TMPDIR`).
- On Unix a PHP relay maps every open descriptor above 2 to `/dev/null`, so a
  relay program cannot write into a stream the handler opened, such as
  `php://stdout`. PHP handlers send warnings to stderr.
- Every runner connects directly to the loopback endpoint and ignores proxy
  variables (Bun honors `HTTP_PROXY` in `fetch` and `node:http`, so the
  JavaScript runner uses a plain `node:net` socket; Dart sets `findProxy` to
  `DIRECT`). The supervisor also adds `127.0.0.1` to `NO_PROXY` and
  `no_proxy` for a handler with an endpoint.
- Every relay failure reaches the handler as HTTP 502 with the one message
  `Component invocation failed.` in all nine languages. JSON output that is
  not an object is a `protocol` failure.
- C# measures its relay budget from the operating system's process start time
  (`Process.StartTime`), so runtime start-up and module initializers count.
  That time comes from the wall clock with a coarse resolution, and the short
  gap between the supervisor's spawn call and process creation is not
  measured; the supervisor's own deadline still bounds the invocation.
- Python keeps only the last `def` of a name, so a Python target is checked
  against its last definition. A generated unit is never started after its
  deadline: a passed deadline wins over a free permit, and the supervisor
  checks the time left again just before it spawns.
- `ty test` preflights the programs that gateways name and the languages of
  their class targets, like the server does.
- On Windows, the preflight looks for a program the way Rust's spawn does: a
  name without a dot gets `.exe` appended, and `.cmd` or `.bat` files do not
  count. A command entry with a drive prefix such as `C:x` counts as a path,
  so it fails with `TY1312` (or `TY1308` as the program) because it is not a
  file under `server/delegates/`.
