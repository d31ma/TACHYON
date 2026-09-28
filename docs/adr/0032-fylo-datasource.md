# ADR 0032: FYLO Is a Datasource

- Status: Accepted; qualification in progress
- Date: 2026-09-15
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
