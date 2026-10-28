# ADR 0111: Yon Keeps Handler Processes Warm by Default

- Status: Accepted
- Date: 2026-10-27
- Amends: [ADR 0003](0003-supervised-handler-process-protocol.md)
  (supervised handler processes) and
  [ADR 0014](0014-environment-selected-yon-isolation.md)
  (environment-selected isolation)

## Context

Tachyon is serverless-first. A Yon application is meant to run on a platform
such as AWS Lambda or Cloud Run as well as on a server that is always on.

Until now, Yon started one language process for every request and stopped it
after the response. Every request paid for the runtime start and the module
load again: tens of milliseconds for Python and Node.js, and more for the JVM
and .NET. A serverless platform already isolates each instance and keeps warm
instances for later requests, and it bills by duration. So on that platform a
process per request adds a second cold start to every billed request.

A process per request still has real uses. On an always-on server that runs
code it trusts less, a new process for every request gives each request a
clean state. Yon must keep that mode for the operators who want it.

## Decision

### Three isolation modes

`YON_ISOLATION` now selects one of three modes. It stays an environment
variable of the parent process only (ADR 0014). No project file can set it.

| Mode | What runs a request | Fits |
| --- | --- | --- |
| `warm` (default) | A warm handler process that serves many requests, one at a time | Serverless platforms and most servers |
| `process` | A new process for every request, stopped after the response | Always-on servers that need a clean state for every request |
| `firecracker` | The operator's control program for every request (ADR 0014) | Multi-tenant Linux hosts with KVM |

Two environment variables tune the warm mode:

| Variable | Meaning | Default and bounds |
| --- | --- | --- |
| `YON_WARM_MAX_REQUESTS` | Requests one warm process serves before Yon replaces it | `1000`; 1 to 1,000,000 |
| `YON_WARM_IDLE_SECONDS` | Seconds a warm process may wait unused before Yon stops it | `60`; 1 to 3,600 |

Setting either one with `process` or `firecracker` fails with `TY2010`, like
the Firecracker values do with the other modes.

### How a warm process lives

- Yon starts a warm process for a handler file on its first request, not when
  the server starts. A handler that is never called costs nothing.
- A warm process serves one request at a time. When more requests for the same
  handler arrive together, Yon starts more warm processes. The existing
  concurrency limit still bounds them.
- A warm process belongs to one handler snapshot. When the development server
  takes a new snapshot of the project, the old warm processes are no longer
  chosen and stop when they go idle.
- Yon keeps at most as many idle warm processes as the concurrency limit, and
  stops the one that was idle longest when a new one would go over it.
- Yon replaces a warm process after any of these:
  - it crashes or exits;
  - it breaks the protocol, or writes to protocol stdout while it has no
    request;
  - its request times out, is cancelled, or loses its stream subscriber;
  - its stderr goes over the diagnostic limit;
  - one of its relay programs failed, because the failed relay can leave a
    descendant behind and only the process group stop reaps it;
  - it has served `YON_WARM_MAX_REQUESTS` requests.
- Yon stops idle warm processes after `YON_WARM_IDLE_SECONDS`, and stops all
  of them when the server shuts down.

### The warm protocol

The warm protocol is Handler Protocol v1 with three additions. A runner knows
it is warm because Yon starts it with `YON_HANDLER_WARM=1`.

1. **Every request is framed.** In process mode, a compiled ("direct")
   handler reads its request until stdin closes. A warm runner reads one
   length-prefixed request frame at a time instead. A Lua runner keeps its
   hex length prefix (ADR 0003).
2. **The request carries its resource endpoint.** In process mode, Yon
   passes each request's managed resource endpoint and token in
   `YON_RESOURCE_ENDPOINT` and `YON_RESOURCE_TOKEN`. A warm process starts
   once, so the request frame carries them in an optional `resources` field:
   `{"endpoint": "http://127.0.0.1:<port>/", "token": "<64 hex digits>"}`.
   The runner uses them for that request only and removes them before
   handler code can see the frame.
3. **A request ends with a terminal frame, not with an exit.**
   - A request ends with one `response` frame. A direct runner writes the
     same JSON object it writes in process mode, inside one frame.
   - A stream ends with one end frame:
     `{"protocol_version":1,"kind":"end","request_id":"<id>"}`. A stream that
     fails still ends with a `response` frame that holds an error, and Yon
     then replaces the process.
   - After its terminal frame, the runner resets the state of the request and
     waits for the next request frame. When stdin closes, it exits with
     status 0.

Everything else stays as it is in process mode:

- the deadline: a runner measures it from the moment it reads the request
  frame. For the first request of a new process, it measures it from the
  start of the process, so start-up work counts against the deadline as it
  does in process mode;
- the cancellation frame: Yon still sends it and then replaces the process;
- the resource frames of Lua and C++ handlers;
- the exit codes for protocol errors;
- the rule that stdout carries only protocol frames.

### What is never warm

- A socket connection keeps its own process. A connection is long-lived
  already, so it gains nothing from a warm process.
- `firecracker` mode is unchanged. The control program may keep its own warm
  pool of microVMs (ADR 0014).
- A handler language whose runner does not yet speak the warm protocol runs
  in process mode, even when the mode is `warm`.

## Consequences

- Most requests no longer pay for the runtime start. Startup cost now falls on
  the first request for each handler, and on the first request after a
  replacement.
- Module-level state can survive between requests that one warm process
  serves, as it does on a warm serverless instance. A handler must not depend
  on that state, and must not depend on it being reset.
- A warm process can leak memory or file handles across requests. The request
  limit and the idle timeout bound how long such a leak lasts.
- A handler that writes to protocol stdout after its response breaks the next
  request of the same warm process. Yon then replaces the process. In process
  mode, the same handler failed its own request instead.
- Each idle warm process holds one language runtime in memory until the idle
  timeout. A server that must release that memory sooner can lower
  `YON_WARM_IDLE_SECONDS` or choose `process`.
- ADR 0003 no longer forbids hidden processes that live between requests.
  It still forbids processes that Yon does not supervise. Every warm process
  has a bounded lifetime, an owner and a reaper.

## Rejected Alternatives

- **Run handlers inside the Yon server process.** The server is written in
  Rust. Embedding Node.js, Python, the JVM and the other runtimes is the
  choice ADR 0003 rejects.
- **Keep process mode as the default and make warm mode opt-in.** It keeps
  the slower behavior on the platforms Tachyon targets first.
- **Let `tachyon.json` choose the mode.** A project could then weaken the
  operator's isolation choice. ADR 0014 rejects this for the same reason.
- **One warm process per language instead of per handler.** It would load
  every handler of a language into one runtime, so one handler's state and
  failures would reach the others.

## Acceptance Gate

- Environment parsing tests cover the default, each mode, the bounds of both
  warm variables, and warm variables set with another mode.
- Real-process tests show that a warm process serves a second request without
  a new start, that a crash, a timeout, a cancellation and stray output each
  replace it, that a stream ends with its end frame, and that the request
  limit and the idle timeout stop it.
- Real-process tests run in process mode: all of `handler_process.rs`, the
  request-isolation test in `yon_dynamic_annotations.rs` (JavaScript,
  TypeScript, Python and PHP), the generated PHP bootstrap test in
  `handler/source.rs`, and a test that each request gets a new process for
  JavaScript, PHP, Ruby, Go, Kotlin, C#, F# and Lua in the `warm_*` files.
  The other suites run in the default warm mode (amendment below).
- A benchmark records start and request latency for each handler language in
  `warm` and `process` mode.

## Amendment (2026-10-28): what runs in process mode

The gate first said that the existing process-mode tests pass unchanged
with `YON_ISOLATION=process`. No suite reads `YON_ISOLATION`. The suites
build their supervisor from `HandlerSupervisorOptions::default()`, which is
warm, so after this ADR they ran warm, and only the tests the gate now names
set `YonIsolationPolicy::Process`. The gate line now says what runs in
process mode. The gate's runs on macOS, Windows ARM64 and Linux aarch64 and
the benchmark are recorded in `docs/SUPPORT_TIERS.md` ("Warm Yon handlers").

## Amendment (2026-10-28): the default-deadline relay test starts the handler first (RT1)

On WSL,
`relay_boundaries::the_supervisor_default_timeout_limits_relays_without_a_request_deadline`
failed now and then. A new process's first request pays for its start
inside the request's deadline, as the Decision above says (the Java prelude
measures it from JVM start), so a slow Ruby or Java start used up the
test's 8 s default deadline before the handler answered. The test now sends a quick request first, with up to
three tries, and then times the relay that the default deadline must stop
(iteration 9, D30). Start-up time against the deadline stays covered by
`start_up_time_counts_against_the_relay_deadline`. The test passed 10 of 10
runs in a row on WSL and 3 of 3 on Windows ARM64.

The run found an open Windows problem: with Ruby 4.0.6 (`aarch64-mingw-ucrt`) a
warm Ruby handler's second relay never starts, and the supervisor times out
(`TY2110`). It matches the `Process.spawn` hang that WG1 fixed in the Ruby
runner, which a warm handler's second relay still hits. Until a fix in
`ruby_runner.rb`, the Ruby case of this test skips the warm-up on Windows,
so its timed relay stays the handler's first request (`docs/SUPPORT_TIERS.md`,
"Relay deadline test and CI tools").

## Amendment (2026-10-28): warm Ruby relays on Windows (WR1)

The Windows problem above is fixed. `Process.spawn` on Windows puts the relay
pipes over descriptors 0 to 2 while it starts the child, and Ruby's
`IO.select` kept polling the handle it saw first, so the warm runner never
read the next request. `ruby_runner.rb` now reads its frames from a private
copy of standard input and points `STDIN` at the null device. The Ruby case
of the default-deadline relay test starts the handler first on Windows too,
and `warm_ruby::a_warm_ruby_handler_relays_on_every_request` relays on three
requests of one process (`docs/SUPPORT_TIERS.md`, "Warm Ruby relays on
Windows").
