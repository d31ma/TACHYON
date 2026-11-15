# ADR 0130: A Production Server Asks Before a FYLO Root Uses the System Disk

- Status: Accepted
- Date: 2026-11-15
- Amends: [ADR 0032](0032-fylo-datasource.md) (FYLO is a datasource)
- Depends on: FYLO ADR 0032 (a FYLO server keeps its data off the system
  disk)

## Context

FYLO ADR 0032 makes FYLO refuse a database root on the system disk (the disk
that holds the operating system) when FYLO runs as a server or a
command-line database. The refusal is the error code `ESYSTEMDISK`. FYLO
allows such a root when it is asked to: the option `--allow-system-disk`,
the environment variable `FYLO_ALLOW_SYSTEM_DISK=1` or the FYLO setting
`fylo.allowSystemDisk`. A root inside an app's own data folder is allowed
with `--app-id`. On Linux a root on `tmpfs` is refused too.

Tachyon runs FYLO only inside `ty`, on the machine where `ty` runs: `ty
start`, `ty dev`, `ty preview` (also for a native target, whose app reaches
the development server on `localhost`) and `ty handler invoke`. The root is
always the absolute path in `server/application.json`. No Tachyon native
host (macOS, iOS, Android, Windows, Linux) runs FYLO: a native app reaches
Yon only through a declared origin (ADR 0103), and its own data folder holds
the web view's data, not FYLO's.

Development machines, tests and CI keep FYLO roots on the system disk: the
OS temporary folder of every test, and the project folder of a laptop.

The user chose "Servers only": the rule protects FYLO servers; tests and
laptops use the opt-out.

## Decision

1. **A datasource setting.** The FYLO datasource takes `"allowsystemdisk"`,
   `true` or `false` (the default). With `true`, Tachyon starts every `fylo`
   process with `--allow-system-disk`. Any other value stops the server at
   startup with `TY2160`.
2. **Development allows it on its own.** `ty dev`, `ty preview`, `ty serve`
   and `ty handler invoke` add `"allowsystemdisk": true` to a FYLO datasource
   that does not set it. An explicit `false` is kept.
3. **Production asks.** `ty start` adds nothing. A server whose root is on
   the system disk must set `"allowsystemdisk": true`, or run `ty start` with
   `FYLO_ALLOW_SYSTEM_DISK=1` in its environment (the `fylo` child process
   inherits it).
4. **A clear refusal.** FYLO's `ESYSTEMDISK` becomes `TY2160` "FYLO refuses
   a root on the system disk (ESYSTEMDISK); put the root on another local
   disk, or allow the system disk with "allowsystemdisk": true on the
   datasource in server/application.json or with FYLO_ALLOW_SYSTEM_DISK=1 in
   the environment of ty start." ADR 0128 logs it as
   `handler.storage_failed`. FYLO sends `ESYSTEMDISK` in an answer frame:
   with the default lease on the first request that opens the root (the
   handshake answers normally), and with `"lease": "call"` in the
   invocation's one frame. The message names `FYLO_ALLOW_SYSTEM_DISK`, so the
   repository policy, which refuses the removed `FYLO_` environment namespace
   in code, allows that one exact name and nothing else: it is FYLO's own
   opt-out variable (FYLO ADR 0032), not a Tachyon variable. A policy test
   checks that any other `FYLO_` name is still refused.
5. **Tests and CI pass the option themselves.** Tachyon's test harness puts
   `"allowsystemdisk": true` in its FYLO datasources and passes
   `--allow-system-disk` wherever it runs `fylo` directly, as do
   `scripts/ci/install-fylo.mjs` and `scripts/test-managed-storage.mjs`. CI
   lanes need no environment variable.
6. **Native hosts.** No change today. A future FYLO store inside a native
   app keeps its root in the app's own data folder and passes FYLO's
   app-owned mode (`--app-id <bundle id>`), never the system-disk opt-out.

## Consequences

- **Breaking:** a project that runs `ty start` with a FYLO root on the
  system disk (a laptop, or a server with one disk) stops at its first
  storage call until it sets the setting or the variable. The CHANGELOG lists
  it under Breaking.
- `ty dev` and the tests keep working on any disk with no change.
- A FYLO without FYLO ADR 0032 does not know `--allow-system-disk`. Tachyon
  pins a FYLO release that has it (`scripts/ci/install-fylo.mjs`).
- `"allowsystemdisk"` never admits a root on a network file system; FYLO's
  `EREMOTEROOT` stays.

## Evidence

- `crates/tachyon-core/src/storage/tests/fylo_system_disk.rs`: the
  development default, production unchanged, an explicit setting kept, other
  drivers unchanged, and, against a FYLO with FYLO ADR 0032, a root refused
  without the setting (the message names both ways) and used with it.
- Unit tests in `storage/fylo/system_disk.rs` (the setting and the message)
  and `storage/fylo/fylo_root.rs` (the `fylo` arguments).
- `crates/tachyon-contracts/tests/repository_policy.rs`: the exact name
  `FYLO_ALLOW_SYSTEM_DISK` is allowed, and `FYLO_ROOT` or a longer name that
  starts with the allowed one is still refused.
