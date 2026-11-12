# ADR 0128: A Busy FYLO Root Waits, and Store Failures Are Logged

- Status: Accepted
- Date: 2026-11-12
- Amended: 2026-11-12 (a stopped FYLO process is waited for before the next
  one starts; the store-call deadline is a setting)
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
   `operationTimeoutMs`, a whole number from 1000 to 60000 (default 5000).
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
  `operationTimeoutMs` pass, and checks the failure names `putData`, that the
  old process has exited, and that three more calls answer. A delayed exit
  after the kill (a write stuck in the kernel) cannot be made on purpose in a
  portable test; the order "wait for the exit, then start" is in
  `FyloStore::ready_process`.
