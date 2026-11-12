# ADR 0129: A FYLO Datasource Can Lease the Root per Call

- Status: Accepted; **not qualified** (see `docs/SUPPORT_TIERS.md`)
- Date: 2026-11-12
- Amends: [ADR 0032](0032-fylo-datasource.md) (FYLO is a datasource)
- Builds on: [ADR 0128](0128-a-busy-fylo-root-waits-and-store-failures-are-logged.md)
  (a busy root waits, and store failures are logged)
- Depends on: FYLO ADR 0011 (bounded serverless invocation) and FYLO ADR 0028
  (`fylo exec --invocation`, FYLO commit `600aca6`)

## Context

Every `fylo exec --loop` process takes the FYLO root on its first request
and keeps it until it exits. A Tachyon server keeps one such process for its
whole life. In AWS Lambda (and in Floci, the local Lambda emulator that
Mesail uses) each container of a function is its own server. A second
container on the same root therefore got `EROOTLOCKED` on every call. ADR 0128
made such a call wait up to 2 seconds and log the cause, but two containers
still cannot share one root.

FYLO's `InvocationRuntime` (FYLO ADR 0011) takes the root for one request,
waits with bounded backoff while another process holds it, and gives it back
before it answers. FYLO ADR 0028 made it reachable from the `fylo` binary as
`fylo exec --invocation`. FYLO says this mode is for local and test use: its
deployment, the locks over NFS (Amazon EFS) and response loss are not
qualified yet.

The user chose option A of the plan: Tachyon runs one `fylo exec
--invocation` process per storage request, behind an explicit setting,
marked "not qualified", and refused on a network file system.

## Decision

1. **An explicit setting.** The FYLO datasource takes `"lease"`:
   `"server"` (the default, unchanged) or `"call"`. Nothing is guessed from
   `AWS_LAMBDA_*` variables; the deployment configuration says it. Any other
   value stops the server at startup with `TY2160`.
2. **One process per request.** With `"call"`, every FYLO request runs as
   `fylo exec --invocation --request - --deadline-ms N
   --cleanup-reserve-ms 200 --root R`. `N` is the time left of the storage
   operation (5 seconds, or 30 for the first page of a scroll) less 100 ms
   to read the answer. The reserve (200 ms) and the margin (100 ms) are fixed
   constants of `CallLease`. When the time left is not more than the reserve,
   the request fails with `EDEADLINE` before FYLO starts. A storage operation
   that makes several FYLO requests (a patch: `patchDoc`, then `getDoc`)
   runs one process per request, one after another, inside the same
   deadline. Requests of one server still take turns behind one lock, so
   patch combining stays.
3. **FYLO's wait replaces ours.** The 2-second `RootLockWait` of ADR 0128 is
   not used with `"call"`. FYLO waits for a held root until only the reserve
   is left, then answers `EROOTLOCKED` with no change.
4. **Tachyon makes the document id.** Before `putData`, Tachyon adds a TTID
   it made (FYLO requires a caller id here). A repeated insert with the same
   id is refused as "already exists" and stored once. Tachyon never sends a
   request again in this mode: not inserts, deletes, restores or raw SQL.
5. **The answer frame decides.** A frame on standard output is the answer,
   as for `exec --request`. FYLO's codes (`EROOTLOCKED`, `EDEADLINE`,
   `EBADREQUEST`, `EUNSUPPORTEDOP`, `EQUERYLOOPREQUIRED`, `EINVOCATION`,
   `EFRAME_REQUEST_TOO_LARGE` and the operation's own) become `TY2160`
   "FYLO operation failed (CODE).", which ADR 0128 logs as
   `handler.storage_failed`. A FYLO that exits without a frame (a host
   setting error goes to standard error with exit 1) fails the call with
   `TY2160` "FYLO exited without an answer frame (ENOFRAME)". FYLO's
   standard error is not read, because it names the root path.
6. **Refused operations cannot be reached.** FYLO refuses paged queries and
   its version and repository operations in this mode. Tachyon pages,
   orders and scrolls in its own memory and has no version operations, so no
   DataSource or repository method sends them. Should one ever be sent, it
   fails with `TY2160` and FYLO's code. No startup check is needed for them.
7. **Startup checks, fail closed.**
   - The `fylo` binary must have the mode. FYLO v26.45.07 and older ignore
     unknown options, so they run `exec --invocation` as a plain one-shot
     request, with no wait and no lease. The server sends a handshake with
     `--cleanup-reserve-ms 0`: a FYLO with the mode refuses it with
     `EBADREQUEST` before it touches the root; any other answer stops the
     server with `TY2160`.
   - Not on Windows. FYLO's own test of two `exec --invocation` processes
     on one root fails there in 2 of 3 runs (one waits about 9.8 s, then
     answers `EROOTLOCKED`). The server refuses `"call"` on Windows with
     `TY2160` until FYLO sends a fix.
   - The root must be on a local disk. Linux reads `/proc/self/mounts` and
     refuses NFS (EFS included), CIFS/SMB and 9p. macOS runs `/sbin/mount`
     and refuses a mount without the `local` flag. When the mount table
     cannot be read, the lease is refused. The
     workspace forbids unsafe code, so Tachyon reads the mount table instead
     of calling `statfs`. FYLO itself accepted an NFS root on macOS when
     this was tested, so the check is Tachyon's.

## Consequences

- Two containers of one function on one local root take turns instead of
  failing. The tests run two `"call"` stores on one root with 48 inserts at
  once and see no `EROOTLOCKED`.
- Each request pays a process start, FYLO's recovery and a log checkpoint.
  On a shared Mac mini: insert p50 63 ms and read p50 25 ms with `"call"`,
  against 23 ms and 0.7 ms with `"server"`. Handlers with many storage calls
  get slower.
- Between the `patchDoc` and the `getDoc` of one patch, another container
  may write. The answer can already hold that change; no patch loses fields.
- A failure after FYLO admitted a request can follow a committed change.
  Only an insert is safe to repeat (it is refused as "already exists").
- Two TTIDs made in the same 100 ns by two containers would collide. The
  second insert then fails with "already exists"; nothing is overwritten.
- A scroll cursor works only in the container that made it, as before.
- The mode stays "not qualified" until FYLO qualifies ADR 0011 for
  deployment on Linux. It stays refused on NFS (EFS) until FYLO closes its
  EFS gate (FYLO ADR 0010 profile), and refused on Windows until FYLO
  qualifies two invocation processes on one root there.
- FYLO ADR 0029 (FYLO `22c127e`) makes FYLO itself refuse a network root
  with `EREMOTEROOT`, in every mode. Tachyon maps it to `TY2160` "FYLO
  refuses a root on a network file system (EREMOTEROOT); ...". Tachyon's own
  mount-table check stays for a FYLO without that refusal.

## Evidence

- `crates/tachyon-core/src/storage/tests/fylo_call_lease.rs`: two stores on
  one root, a root held by `exec --loop` (waits until the reserve and fails
  with `EROOTLOCKED`, nothing changed), a root released inside the deadline,
  the TTID insert and a repeated insert, every document, queue and raw SQL
  command, an unknown lease, and the ignored latency benchmark.
- Unit tests in `storage/fylo/call_lease.rs` (deadline, reserve, id, missing
  frame, the startup probe), `fylo_lease.rs` and `mount_entry.rs` (Linux and
  macOS mount lines).
- `TAC_STORAGE_TEST_LEASE=call` makes every FYLO test store use the call
  lease, so the FYLO filtersets run once per mode.
- Built and tested against FYLO `600aca6` built from source, before a FYLO
  release contained it; see the CHANGELOG entry for the runs.
