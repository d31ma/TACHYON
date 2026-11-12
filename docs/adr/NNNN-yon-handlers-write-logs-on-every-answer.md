# ADR NNNN: Yon Handlers Write Logs on Every Answer

- Status: Accepted
- Date: 2026-11-11
- Builds on: [ADR 0003](0003-supervised-handler-process-protocol.md) (the
  supervised handler process and its bounded stderr) and
  [ADR 0122](0122-ty-test-and-ty-start-agree-on-yon-handler-code.md)
  (`ty test` and `ty start` run the same handler code)

## Context

A Yon handler runs in a child process. Tachyon keeps up to 64 KiB of the
child's standard error for each call, but it shows that output only in two
cases: in the failure message when the handler exits with an error, and as
one `handler.stderr` event when the operator sets `TAC_HANDLER_STDERR=1`.

So a handler that answers normally, or answers with a handled error, cannot
write an operational log line such as "the payment provider took 2.3 s".
Mesail worked around this with the access log of its identity server.

`TAC_HANDLER_STDERR` is not enough:

- It is off by default. Its documentation tells operators to turn it on only
  where the handler's raw output may be logged, because existing handlers may
  print debugging text there. Turning it on by default would change what an
  existing project writes to its logs.
- Its event names the request ID only, not the route or the handler file.
- It is one block of text with no level, so a log system cannot filter it.

Tachyon's own run log is the standard error of `ty`: one JSON object per
line, with sorted keys (`http.access`, `handler.invocation_failed`,
`handler.relay_failed`, `handler.stderr`). Handler logs belong there too.

## Options

- **(a) Forward all handler stderr on every answer.** No new API, but raw
  output has no level or fields, and turning it on by default would log text
  that existing handlers never meant as logs.
- **(b) A small log API in every Yon language.** The handler says plainly
  that a line is a log line, with a level and fields. It needs code in each
  language's runtime.

## Decision

We combine them: (b) is the new, default-on path, and (a) stays the opt-in
path for raw output.

### 1. `YonLog` in every Yon language

Every Yon handler can call `YonLog` with four levels, `debug`, `info`, `warn`
and `error`. Each call takes a message and optional fields (a map from names
to JSON values). The name follows the other framework helpers (`YonHttp`,
`YonTopic`, `YonGateway`). The spelling follows each language:

| Language | Call |
| --- | --- |
| JavaScript, TypeScript | `YonLog.info('message', { key: value })` |
| Python | `YonLog.info("message", {"key": value})` |
| Ruby | `YonLog.info("message", { "key" => value })` |
| PHP | `YonLog::info('message', ['key' => $value]);` |
| Java, Groovy | `YonLog.info("message", Map.of("key", value));` |
| Kotlin | `YonLog.info("message", mapOf("key" to value))` |
| C#, F# | `YonLog.Info("message", new Dictionary<string, object?> { ["key"] = value });` |
| Go | `YonLog.Info("message", map[string]any{"key": value})` (fields optional) |
| Rust | `YonLog::info("message", Json::map([("key", value)]));` (`()` for no fields) |
| Dart | `YonLog.info('message', {'key': value});` |
| Lua | `YonLog.info("message", { key = value })` |
| C++ | `tachyon::yon::YonLog::info("message", {{"key", value}});` |

The call writes one line to the handler's standard error:
`{"event":"yon.log","level":"info","message":"...","fields":{...}}`, and
flushes it. Writing it does not change the handler's answer.

### 2. Tachyon re-emits log lines as `handler.log`

After each call, Tachyon reads the kept standard error. Each line that is a
JSON object with `"event":"yon.log"` becomes one line on `ty`'s standard
error:

```json
{"event":"handler.log","fields":{"upstream_ms":2310},"handler":"server/controllers/OrdersController.java","level":"warn","message":"payment provider slow","request_id":"0J8Y...","route":"/orders","truncated":false}
```

- `request_id` is the same ID that the client sees in
  `x-tachyon-request-id` and that `http.access` and
  `handler.invocation_failed` carry. `route` and `handler` name the route and
  the handler file, as in `handler.invocation_failed`.
- This works for route calls, middleware, the identity provider, workers,
  gateways, streams and sockets, in a fresh process and in a warm process. A
  stream or a socket reports its log lines when it ends.
- It works for a call that answers normally, a call that answers a handled
  error, a call that fails, and a call that times out or is cancelled
  (Tachyon reads what the handler wrote before it was stopped).
- `ty test` runs test units through the same handler supervisor, so a test
  that calls a method that logs prints the same `handler.log` lines.
- Nothing changes in what the client receives.

### 3. Levels and limits

- `TAC_HANDLER_LOG_LEVEL` sets the lowest level that is written: `debug`,
  `info` (the default), `warn`, `error`, or `off`. Any other value keeps the
  default.
- At most 100 log lines per call are written. When more arrive, or when the
  handler's standard error went past the 64 KiB kept for one call, Tachyon
  writes one `handler.log_dropped` line with the same `request_id`, `route`
  and `handler`, the number of lines `dropped`, and `output_cut: true` when
  the middle of the output was cut.
- A message keeps at most 2,000 characters. Control characters other than
  new line and tab are removed. Fields larger than 8 KiB as JSON are left out.
  Either cut marks the line `"truncated": true`. `fields` is always an
  object, empty when the handler gave none.
- A line that says `"event":"yon.log"` but has no known level, no string
  message, or fields that are not an object is not a log line. It stays
  ordinary standard error.
- Tachyon writes every line itself as new JSON, so a handler cannot forge
  another event or split a line.

### 4. Raw standard error stays opt-in

`TAC_HANDLER_STDERR=1` keeps its meaning, now for every kind of call. Its
`handler.stderr` event also names the `route` and the `handler`, and it leaves
out the `yon.log` lines, which are already written as `handler.log`.

### 5. Logs are not a secret channel

A log line goes to the operator's logs. Applications must not log secrets,
tokens, passwords or personal data that the logs may not hold. Tachyon does
not scan log lines for secrets.

## Consequences

- A Yon application can write operational logs on every answer, with a level
  and fields, traced by request ID to the access log and to failures.
- Existing projects write nothing new until a handler calls `YonLog`.
- A warm process keeps the standard error of one call at a time. A line that
  a background thread writes after the handler has answered is not kept.
- Log lines of a long socket appear when the socket closes, and at most 100
  per socket.
- There is no rate limit across calls. The per-call limits and the number of
  handlers that may run at once bound the volume.

## Implementation Notes

- Parsing and limits: `handler/process/handler_log_records.rs`
  (`HandlerLogRecords`), the level setting in `handler_log_level.rs`, and the
  call's names in `handler_call_label.rs` (`HandlerCallLabel`).
  `Drained::surface_events` and `Drained::report_settled` write the events.
- Language runtimes: `handler/preludes/yon.{java,kt,cs,php,rs,dart,go,lua}`,
  `handler/adapters/{javascript_runner.mjs,python_runner.py,ruby_runner.rb}`
  and `yon_cpp/tachyon/yon_log.hpp`.
- Tests: the unit tests of `HandlerLogRecords` and the language cases in
  `tests/it/handler_logs.rs`.
