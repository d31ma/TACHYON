# ADR 0032: FYLO Is a Datasource

- Status: Accepted; qualification in progress
- Date: 2026-09-15
- Amended: 2026-10-04 (documents, collections, scroll and queue for HEIMDALL)
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
- Raw `dataSource.query(sql, parameters)` is supported. The server replaces
  each parameter with an escaped literal before it calls `executeSQL`. It
  returns the id-keyed result as rows that carry the id field.
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
