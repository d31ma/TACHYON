# CLI Signal Lifecycle

Tachyon has three long-running commands: `serve` (`dev`), `preview`, and
`bundle --watch`. Before they report that they are ready, they install the
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

This first request does not have one shared time limit for all three
commands:

- The development server controls its internal watcher, workers, responses,
  and handler completion, and it sets time limits on them.
- Preview may still wait while Axum closes its connections cooperatively.
- `bundle --watch` may finish or observe its synchronous fingerprint pass or
  the current build, which has a time limit, before it returns to the watch
  loop.

Operators and process supervisors must set their own grace period. If they
need to force the process to stop, they send a second supported signal.

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
