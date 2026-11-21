# ADR 0128: A Busy FYLO Root Waits, and Store Failures Are Logged

- Status: Accepted
- Date: 2026-11-12
- Amended: 2026-11-12 (a stopped FYLO process is waited for before the next
  one starts; the store-call deadline is a setting)
- Amended: 2026-11-12 (one store deadline per answer; the deadline failure
  always names the FYLO operation; a Linux frozen-disk proof)
- Amended: 2026-11-21 (start-up requests do not use up the deadline; a
  stopped FYLO process gets its input closed before a kill; slow store calls
  are logged)
- Amends: [ADR 0032](0032-fylo-datasource.md) (FYLO is a datasource)
- Builds on: [ADR 0127](0127-yon-handlers-write-logs-on-every-answer.md)
  (the JSON lines on the standard error of `ty`)

## Context

Mesail runs Tachyon servers as Lambda functions in Floci, a local AWS Lambda
emulator. Its store failed for 4 to 47 seconds at a time.

The FYLO team found the cause. Every `fylo exec --loop` process takes the
FYLO root on its first request after the handshake and keeps it until it
exits. Each Lambda container runs its own Tachyon server with its own loop on
the same root. When a second container of the same function runs (scale-out,
a timeout replacement, or an old container after a redeploy), FYLO answers
every request of the second container with `EROOTLOCKED` in about 0.1 ms,
until the first container's process exits. The same can happen on AWS Lambda.

Two things made this hard to see and to survive:

- The resource endpoint turned every storage failure into HTTP 400 and
  dropped the diagnostic. The handler saw only "Yon managed resource
  operation failed.", and the operator saw nothing in the log of `ty`. The
  FYLO code `EROOTLOCKED` was lost.
- A root that is busy for a moment (for example while an old container
  stops) failed every call at once.

## Decision

1. **Log the cause.** When a storage command of a Yon handler fails, the
   supervisor writes one `handler.storage_failed` JSON line to the standard
   error of `ty`, with the `request_id` and each diagnostic's `code` and
   `message`. For FYLO the message names FYLO's error code, for example
   `TY2160` "FYLO operation failed (EROOTLOCKED)." The line holds no command,
   stored data, SQL text, capability token or FYLO's own message (which names
   the root path and a process ID). It is always on: storage failures are
   rare and an operator needs them. The handler's answer does not change.
2. **Treat a busy root as busy, not broken.** When FYLO answers
   `EROOTLOCKED`, the FYLO datasource waits and sends the same request again.
   The first pause is 10 ms, each next pause is twice as long, at most
   200 ms, and the wait ends after 2 seconds in all. Then the call fails
   exactly as before. These limits are fixed constants of `RootLockWait`; no
   setting changes them. Every other FYLO error fails at once. Sending the
   request again is safe because FYLO refuses it before it runs it.

## Consequences

- An operator sees `EROOTLOCKED` behind each failed store call and can find a
  second owner of the root.
- A root that is released within 2 seconds no longer fails the call. A root
  that stays held makes each request wait 2 seconds before it fails, inside
  the 5-second storage operation deadline. That is slower than before, but the
  answer is the same.
- The wait does not fix two long-lived owners of one root. A deployment must
  still give each FYLO root one owner (one container per function). A plan
  for serverless deployments to use FYLO's per-invocation lease (FYLO ADR
  0011) instead of one long-lived loop per container is a separate decision.
- Storage failures of every driver are logged, not only FYLO's. Their
  messages are Tachyon's own fixed text.

## Amendment (2026-11-12): wait for the old process to exit

### Context

Mesail found a second cause of `EROOTLOCKED` (its slice K8, proven with a
stalled `fylo` on Tachyon 26.46.02 and FYLO 26.45.07). Every store call has a
deadline (5 seconds). The FYLO datasource took its `fylo exec --loop` process
out of its slot for the call. When the deadline fired, the call was dropped
with the process, `kill_on_drop` sent a kill, and nothing waited for the
process to exit. The next call started a new `fylo` at once. A process that
is in a slow disk write cannot die until the write returns, so the old
process still held the root lease. The new process then answered
`EROOTLOCKED` to every call until the old one exited: more than 10 seconds
were measured, much longer than the 2-second wait above. A broken pipe
(`EPROTOCOL`) dropped the process the same way.

### Decision

3. **Never start a second `fylo` for a root while the first may be alive.**
   The process stays in its slot for the whole request. A `RequestWatch`
   marks the request; when the call is dropped before the answer (the
   deadline), or the answer shows a broken pipe, the watch sends the kill at
   once and records the FYLO operation and the time of the kill. The next
   call first waits for that process to exit, then starts a new one. That
   wait is bounded by the next call's own deadline: a call that cannot get
   the root in time fails with the deadline error, not `EROOTLOCKED`, and
   the process stays in the slot for the call after it.
4. **Name the operation, and log the exit.** A deadline that stops the FYLO
   process fails the call with `TY2160` "Managed storage operation deadline
   exceeded during FYLO putData; writes may have committed. ..." (with the
   real operation), which the supervisor writes as `handler.storage_failed`.
   When the old process has exited, the datasource writes one line
   `{"event":"storage.fylo_process_stopped","operation":"putData","exit_ms":N}`
   to the standard error of `ty`: `exit_ms` is the time from the kill to the
   exit. The exit is known only after the failed call has answered, so it is
   its own line, with no request id.
5. **The store-call deadline is a setting.** The FYLO datasource takes
   `timeout`, in milliseconds, a whole number from 1000 to 60000 (default
   5000). (It was first named `operationTimeoutMs`; `application.json` keys
   may hold only lowercase letters and digits, `TY2010`, so it is `timeout`.)
   It replaces the 5-second operation deadline for that datasource. The
   first page of a scroll keeps its 30 seconds.

### Consequences

- After a stall, the next call waits for the old process instead of
  answering `EROOTLOCKED` for the rest of the stall. The wait is spent inside
  that call's deadline, so a long stall still fails calls, but with the
  deadline error.
- A process that never exits keeps every later call waiting until its
  deadline. This is the honest answer: the root is not free.
- A process killed by a deadline is reaped by the next call or when the
  server closes. Until then it is a finished process that holds nothing.
- Evidence: `storage::tests::fylo_root_lock::a_fylo_stalled_past_the_deadline_is_stopped_before_the_next_call_starts_another`
  pauses the `fylo` process with `SIGSTOP` (Unix only), lets a 1-second
  `timeout` pass, and checks the failure names `putData`, that the
  old process has exited, and that three more calls answer. A delayed exit
  after the kill (a write stuck in the kernel) cannot be made on purpose in a
  portable test; the order "wait for the exit, then start" is in
  `FyloStore::ready_process`.

## Amendment 2 (2026-11-12): one store deadline per answer

### Context

Mesail ran Tachyon develop `ff94a61dd` with a held FYLO root. Every HTTP read
then answered 503 only after 11 to 16 seconds: one read made two or three
store calls (its own `collectionExists`, then `findDocs`), and each call got
its own 5-second deadline or 2-second `EROOTLOCKED` wait. The deadline
failure also still read "Managed storage operation deadline exceeded; writes
may have committed." without the FYLO operation. During a stall most calls
time out while they wait for the FYLO process lock that the stuck call holds,
and the operation name was read through that same lock, which was busy.
Finally, on macOS a killed process ends at once even when it is paused, so no
test there could prove that the next call waits for the old process.

### Decision

6. **One store deadline per answer.** All store calls that a handler makes
   while it answers one invocation share one deadline. It starts at the
   invocation's first store call and lasts `timeout` (default
   5 seconds). The wait for an old FYLO process to exit, the wait for the
   FYLO process lock and the 2-second `EROOTLOCKED` wait all count against
   it. A store call that starts after that deadline fails at once, before
   anything is sent to FYLO, with the deadline error.
   - One invocation is one handler run that answers once: an HTTP route,
     `ty handler invoke`, a `ty test` call, a worker run.
   - A stream and a socket live for a long time, so their store calls keep
     one deadline each, as before (`ResourceAccess::time_store_calls_one_by_one`).
   - The first page of a scroll keeps its own 30 seconds and does not use
     the shared deadline, because it reads every match once.
   - Calls made outside a handler (`ManagedStorage` used directly) keep one
     deadline each.
7. **The failure always names the operation.** The FYLO datasource keeps the
   name of the FYLO operation in progress in its own small record
   (`BusyOperation`), outside the process lock. It is set before each FYLO
   request and cleared when the answer arrives; a request dropped by its
   deadline leaves it set until the process has been stopped. Every deadline
   failure while FYLO is busy reads "Managed storage operation deadline
   exceeded during FYLO <operation>; writes may have committed. Tachyon stops
   that FYLO process, and the next call waits until it has exited.", also for
   calls that only waited behind it.
8. **`collectionExists` stays.** Tachyon itself sends `inspectCollection`
   only for `dataSource.collectionExists(name)`. Repository and document
   reads send only `findDocs` or `getLatest`. So the extra call in Mesail's
   reads is the application's own; an application should create its
   collections once (for example at startup) and not check them on every
   read. Tachyon does not cache the answer, because another server may drop
   a collection at any time.

### Consequences

- During a stall one HTTP answer fails after about one deadline, not one per
  store call (`handler::resources::invocation_storage::tests::store_calls_of_one_answer_share_one_deadline_while_the_root_is_held`:
  three calls against a held root, `timeout` 1000, fail in about
  1 second, all naming `inspectCollection`).
- A handler that makes many store calls must finish all of them within
  `timeout`; raise the setting for handlers that need more.
- Evidence for the wait on Linux:
  `storage::tests::fylo_frozen_disk::a_fylo_frozen_in_a_disk_write_is_waited_for_and_no_second_fylo_starts`
  (Linux only, ignored by default; it needs passwordless `sudo`, a loop
  device, `mkfs.ext4` and `fsfreeze`, and runs in the privileged Linux CI
  container with `--run-ignored only`). It puts the FYLO root on a
  loop-mounted ext4 image, freezes it with `fsfreeze -f`, lets a write pass
  its deadline, and checks that the killed `fylo` is still in uninterruptible
  sleep (state `D`), that the next call fails with the deadline error and not
  `EROOTLOCKED`, that no second `fylo` starts, and that after
  `fsfreeze -u` the waiting call answers and later calls work.

## Amendment 3 (2026-11-21): start-up time, a gentle stop, and slow calls

### Context

Mesail's first directory seed on Floci answered 503 three times in a row,
each time on a new Lambda container, with one store call over its 5-second
deadline (Mesail slice K19). Then the same request took 0.2 seconds. FYLO
repairs a root when a process starts (with `--exclusive-root` or the
`efs-sidecar-v1` layout), before it answers the first request, and repairs a
collection on its first write after a process was stopped in the middle of a
write. There is no known upper bound for that time. Tachyon counted it
against the 5-second deadline of the call that happened to come first. A
repair that needs more than 5 seconds was then stopped by the deadline,
before it could save, and the next process started the same repair again.
Also, Tachyon killed the FYLO process at the deadline, which is what leaves
a write half done. Finally, Tachyon logged only failed store calls, so the
slow calls that pass could not be seen, and nothing showed whether a call
waited for the FYLO process or worked.

### Decision

9. **Start-up requests get their own allowance.** After every start of a
   FYLO process (the first one and each new one after a stop), the first
   request for each collection or queue topic is a start-up request. Each
   handler answer has a start-up allowance, the datasource setting
   `startup` in milliseconds (1000 to 120000, default 20000; streams and
   sockets get it per call, like their deadline). While a start-up request
   runs, the deadline of the call is the normal deadline plus what is left of
   the allowance. When it ends, the deadline of the call, and the shared
   deadline of the handler answer, move later by the time it took, at most by
   what was left; that much of the allowance is used up. So start-up time does
   not use up the time of the calls after it, a repair that fits the
   allowance finishes and is saved instead of starting again, and one answer
   that touches many new collections still ends within `timeout` plus
   `startup`. The default is 20 seconds because Mesail measured first store
   calls of 5.1 and 9.3 seconds on new Floci containers, and 10 seconds would
   leave too little margin. `timeout` plus `startup` must stay below the
   request limit of the platform (for example a Lambda function's timeout),
   or the platform ends the request first.
   A call that waits for the FYLO process lock while another answer's
   start-up request runs (Mesail saw a GET wait 4.8 seconds behind a slow
   first write) keeps its time the same way: while a start-up request runs
   anywhere in the store, its deadline stays open by what is left of its own
   answer's allowance, and when it gets the process, its deadline moves by
   the start-up time it waited behind, at most by that allowance. A wait
   behind a call that is not a start-up request still counts, so a stuck
   call still fails the calls behind it at their deadlines. Every start counts, not only a start after a stop,
   because a new container cannot know that the container before it was
   stopped. Reads count too: FYLO's materialization timing is not measured.
   An `EROOTLOCKED` answer returns in about 0.1 ms, so the start-up limit
   adds almost nothing to it, and the 2-second wait for a busy root (its
   pauses) still counts against the deadline. A request that FYLO answered
   with `EROOTLOCKED`, or that broke the pipe, does not end the start-up for
   its collection.
10. **Close the input first, kill later.** When the deadline drops a FYLO
   request, or the pipe breaks, Tachyon closes the input of `fylo exec
   --loop` at once and sends no kill. FYLO then finishes the request in
   progress, answers it and exits. The next call waits for the exit, and
   sends a kill only when the process has not exited 1 second after the
   input was closed (FYLO 26.45.07 handles no signals, so TERM would be a
   kill too). The `storage.fylo_process_stopped` line gains
   `"stop":"closed"` or `"stop":"killed"`; `exit_ms` now counts from
   closing the input.
12. **Stop FYLO gently when `ty` stops.** Floci stops an old Lambda
   container with `docker stop --time 10`: SIGTERM to `ty`, then SIGKILL to
   everything 10 seconds later, which could cut a FYLO write in half. `ty`
   already turns SIGTERM and SIGINT (Ctrl-C and Ctrl-Break on Windows) into
   its graceful stop: running requests finish (`TAC_SHUTDOWN_GRACE_SECONDS`,
   default 10), then the supervisor closes its resources within 3 seconds.
   No second signal handler is added. Closing the FYLO datasource now closes
   the input of the live FYLO process at once, also while a store call holds
   the process, then waits for that call at most 1 second, waits for the
   exit only for what is left of that second, kills the process if it is
   still running, and writes `storage.fylo_process_stopped` with
   `"operation":"shutdown"`. So the whole FYLO stop takes about 1 second at
   most, also when FYLO is stuck. When the platform's kill comes sooner than
   the request drain plus this step, `TAC_SHUTDOWN_GRACE_SECONDS` must be set
   lower. On Windows, a console close, logoff or shutdown event starts the
   same stop as Ctrl-C and Ctrl-Break (tokio keeps the handler running, so
   the process lives until Windows' own limit of about 5 seconds). Known
   limits: Windows ends the process at that limit, so a long request drain
   may not reach the FYLO step; logoff and shutdown events reach only
   console processes without window libraries; and a hard end
   (`TerminateProcess`, `taskkill /F`, SIGKILL) reaches no handler. FYLO
   then sees its input close when `ty` ends and still finishes its request.
13. **Keep the end of FYLO's error output.** FYLO's standard error was
   thrown away, so a FYLO crash left no reason. Tachyon now reads it on its
   own task, so FYLO never waits, and keeps only the last 20 lines or 4 KiB,
   whichever is smaller, each line cut at 512 bytes. Only when a process ends
   badly (killed, or a failed exit, which covers a crash and a broken pipe),
   it writes one `storage.fylo_stderr` line with `operation` and `lines`,
   next to `storage.fylo_process_stopped`. Decision 1 kept FYLO's own text
   out of the logs because it names the root path; so the root path is
   replaced with `<root>`, and the text never reaches a handler answer or
   `handler.storage_failed`. Process IDs in it stay: they help an operator
   match the lines with the system's own logs and name no data.
11. **Log slow store calls.** When a storage command of a Yon handler takes
   `datasource.slowcall` milliseconds or more (1 to 60000, default 1000),
   the supervisor writes one `handler.storage_slow` line to the standard
   error of `ty`, with `request_id`, `datasource` (the driver, or `redis`),
   `operation` (a fixed name such as `document.insert`, or `unknown`),
   `collection` (the table of a repository command, which comes from the
   application's model; null for document, queue and SQL commands, whose
   names handler code may build at run time from user input), `wait_ms`,
   `startup_ms`, `call_ms` and `result` (`ok` or the diagnostic code).
   `wait_ms` is the time spent waiting for the FYLO process lock, for an old
   process to exit, for a busy root and in start-up requests, also when the
   deadline cut the call off; `startup_ms` is the part of it spent in
   start-up requests; `call_ms` is the rest. `datasource.logcalls: true` writes a
   `handler.storage_call` line with the same fields for every other call, for
   debugging. The line never holds stored data, SQL text, Redis keys or the
   token. The two keys apply to every driver and are removed before the
   driver reads its settings.

### Consequences

- A FYLO repair that fits the start-up allowance no longer fails the first
  calls of a new process again and again. A longer repair still fails, and
  its time now shows as `startup_ms` in `handler.storage_slow`.
- A handler answer whose store calls include start-up requests may take up
  to `timeout` plus `startup` in all.
- A FYLO process stopped by a deadline usually finishes its write and exits
  in milliseconds, so the next process has nothing to repair. A process
  stuck in the kernel is still killed after 1 second, and the next call
  still waits for it within its own deadline (amendment 1). The next call
  after a stop may now wait up to 1 second longer before the kill.
- After `docker stop` (SIGTERM), FYLO finishes its write and exits before
  `ty` does, if the request drain ends in time.
- Evidence (macOS, FYLO 26.45.07; the same tests run in the Linux and
  Windows lanes):
  - `storage::tests::fylo_startup::closing_the_store_stops_an_idle_fylo_at_once`,
    `..::closing_the_store_during_a_write_lets_the_write_finish` and
    `..::closing_the_store_waits_for_a_stuck_fylo_only_a_short_time` (Unix).
  - `fylo_store_timing::sigterm_closes_an_idle_fylo_and_ty_exits` and
    `sigterm_during_a_write_keeps_the_write_and_ty_exits` (`ty start`, Unix):
    the write is kept, `ty` exits, and the stop is logged as `closed`.
    Windows has no test that sends Ctrl-C or a close event to a child
    process; `shutdown::tests::windows_registers_console_close_logoff_and_shutdown_too`
    checks that all five events are registered.
  - `storage::fylo::stderr_tail::tests` (bounds, `<root>`, and a process
    that writes to its standard error and is then killed, Unix).
  - `storage::fylo::fylo_process::tests::closing_the_input_lets_fylo_finish_the_write_and_exit`
    (every OS): a write dropped before its answer is kept, and FYLO exits
    with `stop` `closed` in under 1 second.
  - `storage::fylo::fylo_process::tests::a_fylo_that_does_not_exit_is_killed_after_one_second`
    (Unix, `SIGSTOP`).
  - `storage::tests::fylo_startup::a_first_request_slower_than_the_deadline_finishes_and_the_same_fylo_stays`
    (Unix): with `timeout` 1000, FYLO paused for 1.5 seconds before the first
    request answers it, and the same FYLO process serves the next calls;
    `..::a_later_request_to_the_same_collection_keeps_the_normal_deadline`
    shows that only the first request gets the start-up allowance.
  - `handler::resources::invocation_storage::tests::one_answer_gets_at_most_the_startup_setting_of_extra_time`
    (Unix): with `startup` 1000, two paused start-up requests in one answer;
    the second fails at the cap.
  - `storage::tests::fylo_startup::a_first_write_to_a_second_collection_after_a_start_gets_the_allowance`
    (Unix, a document insert, as in Mesail's seed) and
    `handler::resources::invocation_storage::tests::an_answer_that_waits_behind_another_answers_startup_keeps_its_time`
    (Unix); `storage::store_call::tests` show that a wait behind a slow call
    that is not a start-up request still times out.
  - `handler::resources::invocation_storage::tests::startup_time_does_not_use_up_the_deadline_of_the_answer`
    (Unix).
  - `fylo_store_timing::ty_start_logs_a_slow_store_call_with_its_wait_time`
    and `the_lambda_setup_of_mesail_logs_the_same_slow_line` (`ty start`,
    the second with `YON_PROFILE=lambda`, an `application-lambda.json` and
    the datasource in `.env`, as Mesail's Lambda image runs it): a held root
    gives one `handler.storage_slow` line with `wait_ms` of 1.5 seconds or
    more and `result` `TY2160`.
- For the lease mode `call` (one `fylo` per request, a later change), every
  request starts a new process; its first request must be marked with
  `StoreCall::startup` too.
