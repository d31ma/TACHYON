# ADR 0032: FYLO Is a Datasource

- Status: Accepted; qualification in progress
- Date: 2026-09-15
- Amended: 2026-10-04 (documents, collections, scroll and queue for HEIMDALL)
- Amended: 2026-10-31 (raw SQL parameter contract)
- Amendment proposed: 2026-11-01 (FYLO in-process; a cancelled call is
  abandoned, not killed). Draft, not yet accepted.
- Extends: managed storage, see [MANAGED_STORAGE](../MANAGED_STORAGE.md)

## Context

FYLO (fylo.del.ma) is a document store that ships as a single binary:

- It stores one JSON file per document.
- It uses zero-payload prefix indexes.
- It has no server and no network protocol.

Its language shims start the `fylo` binary as a process. They talk to it with a
bounded NDJSON (newline-delimited JSON) machine protocol.

FYLO supports a subset of SQL. That subset returns documents keyed by id. It
has no aggregate functions, and it always assigns TTID identifiers (FYLO's own
identifier format). So it does not fit the SQL dialect path that PostgreSQL,
MySQL, SQLite and SQL Server share.

## Decision

- `yon.datasource.driver` accepts `fylo`. The URL is `fylo:///absolute/root`,
  and the root must already exist. The driver accepts no credentials and no
  TLS keys.
- The server owns one long-lived `fylo` subprocess for its whole lifetime:
  - It finds the binary on `PATH`, exactly as it finds language toolchains.
  - It performs a handshake with it at startup.
  - It closes it together with the server.

  The startup, admission and operation deadlines are the managed storage
  bounds.
- CrudRepository operations map to document operations:

  | Repository call | FYLO operation |
  | --- | --- |
  | `save` without an identifier | `putData`. It returns the entity with its TTID. |
  | `save` with an existing identifier | `patchDoc` |
  | `delete` | `delDoc` |
  | `find` and derived `query` calls | `findDocs` with `$ops` predicates and page limits. Ordering happens in the server process. |
  | `count` and `exists` | Count the matching identifiers. |

  An entity's id field holds the TTID.
- Raw `dataSource.query(sql, parameters)` is supported, under the parameter
  contract in the 2026-10-31 amendment. It returns the id-keyed result as rows
  that carry the id field.
- A `save` that names an identifier that FYLO does not know fails. FYLO
  assigns identifiers; applications do not.

## Consequences

- Repositories written for SQL datasources work unchanged on FYLO. The
  exception is raw SQL features that FYLO does not have.
- `ty doctor` reports the `fylo` binary when a project configures the driver.

## Amendment (2026-10-04): documents, collections, scroll and queue

HEIMDALL, a Go application with 17 collections named `hd-*`, needs more than
the CrudRepository mapping. FYLO already has the operations; Tachyon did not
reach them.

Decisions:

- **Collection names follow FYLO's grammar**: 2 to 64 lowercase letters,
  digits and hyphens, starting and ending with a letter or digit, and not a
  FYLO reserved word. The server checks the name before every FYLO call, for
  Store tables too. Names never become paths in Tachyon; FYLO maps them below
  its canonical root. `..`, separators, `_` and upper case are refused
  (`TY2160`).
- **New commands on the existing resource channel**, shaped like the Redis
  commands: `{"operation":"document","command":...}` for `findById`
  (`getLatest`), `insert` (`putData`), `patch`, `deleteById` (`delDoc`),
  `scroll`, `createCollection`, `dropCollection` and `collectionExists`
  (`inspectCollection`), and `{"operation":"queue","command":...}` for
  `publish`, `claim`, `ack` and `nack`. The SQL drivers refuse them. Every Yon
  language with a DataSource gets the same methods on DataSource; no new
  injected type is added. Lua has no resources (ADR 0085). Go decodes into
  models: `FindById` and `Patch` take a target, `Scroll` answers a
  `DocumentWindow` and `Claim` answers `QueueDelivery` values.
- **Patch** follows `Update.set` of Spring Data MongoDB: the named top-level
  fields are set, the others stay, null is stored, nested objects are
  replaced and the id cannot change. FYLO's `patchDoc` merges in one request;
  the server also holds the subprocess for the `getDoc` that reads the result,
  so concurrent patches cannot interleave.
- **Scroll keeps its snapshot in the server.** FYLO 26.33 offers cursor
  pagination, but each continuation page runs the whole query again (about
  5 seconds per page at 10,000 documents), and `getDoc` reloads the collection
  index (about 30 ms per call at 10,000 documents). Paging past 10,000
  documents through either would pass every deadline. So the first page runs
  one `findDocs` for all matches (bounded by the 8 MiB FYLO frame), sorts them
  by id and keeps them for 15 minutes under a random 128-bit token; at most 8
  scrolls stay open and the oldest is dropped first. The cursor is the token
  and a position, so reading a cursor again reads the same page. This is the
  Spring Data scroll API (`Window`) with a snapshot at the first page. The
  first page has a 30-second deadline instead of 5 seconds, in the server and
  in the language clients. When FYLO pages without running its query again,
  the snapshot can move back into FYLO without a change to the API.
- **The queue is FYLO's** (FYLO ADR 0009): at-least-once delivery, consumer
  groups, leases, idempotent publication and dead letters. Tachyon adds no
  second queue. Statistics, lease extension and dead-letter reads stay out
  until an application needs them.
- **A cancelled FYLO call restarts the subprocess.** The server takes the
  child out of its slot for the length of a request. When a deadline drops the
  request, the child is killed with it and the next call starts a new one, so
  a late answer is never read as the answer to the next request. Open scroll
  snapshots are not affected; FYLO cursors are no longer used.

## Amendment (2026-10-31): raw SQL parameter contract

The release security review (SEC-3) found that a placeholder written inside
quotes, such as `WHERE name = '$1'` or `WHERE name = "$1"`, let a value close
the string and add its own predicates: `x" OR name != "` returned every
document.

FYLO's machine protocol version 1 has no bound parameters: `executeSQL` takes
only `sql` and `access`. The Security Architect rule is that values never
become query text. FYLO cannot honour that rule yet, so the server keeps
values out of the statement's structure instead:

- **Placeholders are values, never text inside quotes.** The server reads the
  statement with FYLO's own lexical rules: `'` and `"` both open strings, a
  doubled quote is the only escape, and there are no backslash escapes and no
  comments. `$n` is accepted only outside quotes. A `$` followed by a digit
  inside quotes is refused, and so are an unterminated string, a `;`, a NUL, a
  `$` without digits, and an index outside the parameter array (`TY2160`).
- **Each value becomes one self-contained literal token**, with a space on
  each side, so it can never join a neighbouring token:
  - A string becomes `'…'` with each `'` doubled. Backslashes, `"`, Unicode
    quotes such as `’` and `＇`, `;`, `--` and `$n` inside a value stay
    characters of that one string.
  - `true`, `false` and `NULL` are written as keywords.
  - A number is written only as non-negative digits with an optional decimal
    point. FYLO drops a leading `-` and reads `1e2` as `1`, so negative numbers
    and exponents are refused instead of changing the query silently.
- **Identifiers are never parameters.** FYLO accepts only bare identifier
  tokens for collections and fields, and a string literal cannot stand in for
  one. Repository, document and queue commands never build SQL: they send
  structured FYLO operations whose collection names pass `CollectionName` and
  whose field names pass the simple-identifier check.
- Regression tests (`storage::tests::fylo_sql`) check that each injection
  payload ends up as exactly one string token and that the rest of the
  statement is unchanged. They also run the payloads against the `fylo`
  binary, which must answer with only the exact match.

The storage deadlines (10 seconds to connect, 5 seconds per operation and 30
seconds for a first scroll page) stay as they are. Tests that drive the `fylo`
binary connect with every deadline multiplied by six, so a slow runner cannot
fail them. The Windows lane runs the x64 binary under ARM emulation, where one
round trip took 23 seconds.

When FYLO adds bound parameters to `executeSQL`, the server passes the values
through as data and the literal rendering goes away. That needs a FYLO
protocol change.

## Amendment (proposed 2026-11-01): FYLO in-process

Status: **Proposed** (draft). It follows FYLO ADR 0019 (proposed), "FYLO
through Turso's engine", which makes the `fylo` Rust crate FYLO's native API
and retires `fylo exec --loop` and machine protocol version 1 as public
contracts.

Context: FYLO's next release has no listener, synchronous writes and no
backward compatibility. Its native API becomes a Rust crate that opens a root
in the host's own process. Yon is Rust, so it can call FYLO directly instead
of through a subprocess and NDJSON.

Decisions:

- **The FYLO driver runs FYLO in Yon's process.** `FyloStore` opens the root
  with `fylo::Root::open` at startup, within the startup deadline, and holds
  the root's exclusive lease until the server closes. That is one owner per
  root, as the single subprocess was. The URL, the canonical-root and
  symlink rules, the collection-name check, the resource-channel commands and
  every language's DataSource API stay as they are.
- **Engine calls are blocking, so they run on Tokio's blocking pool**
  (`spawn_blocking`). Calls run one at a time, in arrival order, as they did
  through the subprocess; a patch and the `getDoc` that reads its result still
  run back to back under one hold of the store.
- **A cancelled call is abandoned, not killed.** An in-process call cannot be
  stopped once it runs:
  - When a deadline drops the request, the caller gets the deadline failure at
    once. The engine call keeps running to its end on the blocking pool, and
    its result is dropped there; it is never delivered to another request,
    because each call returns through its own task handle and nothing else
    reads it.
  - The call keeps its managed-storage admission permit until it finishes, so
    abandoned calls still count against the 16 admitted operations, and a
    stuck engine fills admission instead of piling up blocking threads.
  - The next call starts only after the abandoned one finishes, because calls
    run one at a time.
  - As before, a cancellation does not prove that a mutation did not commit
    ("writes may have committed").
  - A panic in the engine is no longer contained in a child. TACHYON's
    release profile sets `panic = "abort"`, so it ends Yon's process. In a
    build that unwinds, the blocking task reports it and the request fails
    with the subprocess-failure message.
- **Closing the server releases the lease** when no abandoned call is still
  running, and otherwise when the last one finishes: whichever holds the root
  last closes it, with FYLO's checked release. A closed datasource takes no
  new calls.
- **The `fylo` binary is no longer needed at run time.** A build with the
  in-process driver makes `ty doctor` drop the "fylo datasource" toolchain
  requirement. The Linux and Windows CI lanes compile the crate instead of
  installing a pinned binary for the storage tests.
- **Raw SQL moves to FYLO's Turso SQL layer** (FYLO ADR 0019 section 3) in a
  later phase, with bound parameters, which retires the 2026-10-31 literal
  rendering. Until that layer exists, raw SQL on the in-process driver fails
  with `TY2160` ("FYLO SQL is not available in the in-process datasource
  yet").
- **Transactions** stay refused until FYLO's single-collection transactions
  (`EFYLO_TXN_SCOPE` rules) land.

Consequences:

- Yon loses process isolation for FYLO: a fault in the engine is a fault in
  Yon's process, and memory the engine uses is Yon's memory.
- FYLO's crates join TACHYON's `Cargo.lock` and `cargo deny` review behind
  the `storage-fylo-in-process` feature, and `turso_core` joins them in the SQL
  phase. FYLO's crates enable `serde_json`'s `preserve_order` feature, which
  then applies to the whole TACHYON build that enables the driver. TACHYON's
  emitters rely on `serde_json` maps keeping keys in sorted order, so in the
  phase 1 spike 27 compiler, polyglot and migrate tests failed with the
  feature on only because keys came out in insertion order. The driver can
  become the default only when FYLO's crates stop enabling `preserve_order`,
  or TACHYON stops depending on the map order.
- The phase 1 spike (2026-11-01) keeps the subprocess as the default and adds
  the in-process driver behind the `storage-fylo-in-process` feature. The
  feature becomes the default, and the subprocess is removed, only when the
  FYLO release that ships the crate does.
- The spike takes the `fylo` crate as a git dependency on FYLO's phase 1
  commit (`c08f834`), and `deny.toml` allows the `d31ma/FYLO` git source.
  Cargo resolves optional dependencies too, so every TACHYON build, with or
  without the feature, fetches that commit. A FYLO release tag or a registry
  version replaces the commit before this merges into `develop`.
- In the phase 1 benchmark, with the same FYLO engine on both sides and
  optimized dependencies, in-process calls took 0 to 25 percent less time at
  the median than the subprocess. TACHYON's own share was about 20 to 50
  microseconds per call; the rest was the engine.
- The canonical gate builds with `--all-features`, so it runs the full suite
  with the in-process driver and with `preserve_order`. A build without the
  feature checks the subprocess driver.

