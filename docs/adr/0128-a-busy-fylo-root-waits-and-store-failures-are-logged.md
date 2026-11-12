# ADR 0128: A Busy FYLO Root Waits, and Store Failures Are Logged

- Status: Accepted
- Date: 2026-11-12
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
