# CLI Signal Lifecycle

Tachyon has four long-running commands: `serve` (`dev`), `start`, `preview`,
and `bundle --watch`. Before they report that they are ready, they install the
operating-system signal handlers that they support. They write lifecycle
events to standard error as compact JSON lines. Normal readiness output stays
on standard output.

The installed signals are:

- on macOS and Linux: `SIGINT` and `SIGTERM`;
- on Windows: `CTRL_C` and `CTRL_BREAK`.

This interface cannot observe Windows `TerminateProcess`, Unix `SIGKILL`,
termination because the system ran out of memory, or host shutdown.

## Events

Every event has `event_version: 1`, the canonical `command`, and the host
`platform`. At startup, Tachyon reports the handlers it installed:

```json
{"event":"runtime.signal_handlers_ready","event_version":1,"command":"serve","platform":"macos","installed":["SIGINT","SIGTERM"],"unavailable":[]}
```

If registration fails, Tachyon reports a warning. To protect privacy, the
warning contains only the error kind and the numeric operating-system error.
A registration failure does not stop the command. Tachyon never treats it as
a received signal.

```json
{"event":"runtime.signal_handler_unavailable","event_version":1,"command":"serve","platform":"macos","signal":"SIGTERM","error_kind":"other","raw_os_error":null}
```

The first signal that Tachyon observes gives the reason and requests a
graceful shutdown:

```json
{"event":"runtime.shutdown_requested","event_version":1,"command":"serve","platform":"macos","reason":"signal","signal":"SIGTERM","signal_code":15}
```

This first request does not have one shared time limit for all four
commands:

- `serve` and `start` drain. They stop accepting connections at once, and
  they close hot-update streams, topic streams, handler streams and
  WebSockets (with close code 1001). Requests that are already running get a
  grace period to finish. When the grace period ends, Tachyon cancels the
  handlers that are still running, and those requests answer
  `503 Service Unavailable` with `Retry-After: 1`. The workers, the watcher
  and the handler processes then get at most 3 more seconds to stop.
- Preview may still wait while Axum closes its connections cooperatively.
- `bundle --watch` may finish or observe its synchronous fingerprint pass or
  the current build, which has a time limit, before it returns to the watch
  loop.

The drain grace period is 10 seconds. Set `TAC_SHUTDOWN_GRACE_SECONDS` to
change it: `0` cancels running requests at once, and values above 300 count
as 300. A value that is not a whole number keeps the default. A process
supervisor should wait at least the grace period plus 4 seconds before it
forces the process to stop, for example with Kubernetes'
`terminationGracePeriodSeconds`. To force the process to stop sooner, send a
second supported signal.

If a second signal arrives while graceful shutdown is still running, Tachyon
stops immediately. It first writes a `runtime.shutdown_forced` event. Then it
exits with:

- `130` for SIGINT or CTRL-C;
- `143` for SIGTERM;
- `131` for CTRL-BREAK.

The operating system may merge identical signals that arrive before Tachyon
reads the first notification.

Lifecycle events never contain project paths, process or parent identifiers,
arguments, environment values, requests, credentials, or authored output. A
supervisor that captures the standard-error stream should add its own
timestamp and process identity.
