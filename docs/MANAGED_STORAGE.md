# Managed Yon Storage

Yon can initialize one datasource and one Redis connection resource. It reads
them from its private startup configuration.

- Tac never initializes these resources. Tac never receives their credentials.
- If a configured resource cannot connect, the server fails at startup. There
  is no silent in-memory fallback.

## Configuration

Put the non-secret settings in `server/application.json`. The object below is
the `yon` namespace itself:

```json
{
  "datasource": {
    "driver": "postgresql",
    "url": "postgresql://db.example.com:5432/catalog"
  },
  "redis": { "url": "rediss://cache.example.com:6379/0" }
}
```

Put the secrets in the `.env` file at the project root. Replace the example
values. Never commit this file.

```dotenv
YON_DATASOURCE_USERNAME=replace-me
YON_DATASOURCE_PASSWORD=replace-me
YON_REDIS_USERNAME=replace-me
YON_REDIS_PASSWORD=replace-me
```

Credentials:

- You must supply network SQL usernames and passwords explicitly.
- Passwords belong in the root `.env`. The example keeps both credential
  fields there.
- Tachyon rejects driver-specific ambient PostgreSQL configuration. It does not
  read settings from `PG*` variables or from `.pgpass`.

TLS (Transport Layer Security):

- The optional setting `yon.datasource.tls.ca` names an absolute path to a CA
  (certificate authority) certificate. Use it for a private PKI (public key
  infrastructure).
- This setting never turns off identity verification.
- PostgreSQL, MySQL, MariaDB and Redis use bundled WebPKI roots for TLS.
- SQL Server uses the native TLS backend.
- A private SQL CA adds trust. It does not skip certificate checks or hostname
  checks.

| Driver | Non-secret URL | Raw SQL parameters |
|---|---|---|
| `postgresql` | `postgresql://host:5432/catalog` | `$1`, `$2` |
| `mysql` | `mysql://host:3306/catalog` | `?`, `?` |
| `mariadb` | `mysql://host:3306/catalog` | `?`, `?` |
| `sqlite` | `sqlite::memory:` or `sqlite:///absolute/existing.db` | `?`, `?` |
| `sqlserver` | `sqlserver://host:1433/catalog` | `@P1`, `@P2` |
| `fylo` | `fylo:///absolute/root` | `$1`, `$2` |

- SQLite files must already exist. The configuration does not create them.
- In-memory SQLite belongs to this server. Its data is lost on restart.
- For production schema changes, use explicit migrations outside request
  handlers.
- The URL must not contain credentials, fragments or arbitrary URL options.

## FYLO

`fylo` is a document store. It is not a SQL server (ADR 0032; ADR means
architecture decision record).

Configuration rules:

- The URL is `fylo:///absolute/root`. On Windows, write the drive after the
  third slash with forward slashes, as in `fylo:///C:/data/catalog`.
- The root directory must already exist.
- The path must be spelled canonically. FYLO refuses a path that goes through a
  symlink or that uses different letter case.
- FYLO accepts no credentials, no TLS keys and no other settings.

How the server runs FYLO:

- The server finds the `fylo` binary on `PATH`, in the same way it finds
  language toolchains.
- The server owns one long-lived `fylo exec --loop` subprocess for its whole
  lifetime.
- The server and the subprocess speak the bounded NDJSON (newline-delimited
  JSON) machine protocol. The subprocess closes when the server closes.
- The startup, admission and operation deadlines are the managed storage
  bounds listed below.
- When a project configures the driver, `ty doctor` reports the requirement
  "fylo datasource".

```json
{
  "datasource": {
    "driver": "fylo",
    "url": "fylo:///srv/catalog"
  }
}
```

Tachyon maps repository commands to document operations. Because of this, a
repository written for a SQL datasource works without changes:

| Repository operation | FYLO |
| --- | --- |
| `save` without an identifier | `putData`; returns the entity carrying its assigned TTID |
| `save` with an identifier | `patchDoc`, then the stored document |
| `delete` | `delDoc`; a missing document is `false`, not an error |
| `find` and derived queries | `findDocs` with `$ops` predicates |
| `count`, `exists` | `findDocs`, counting matches |

Identifiers:

- FYLO assigns identifiers. Applications do not.
- A `save` that names an identifier that FYLO does not know fails with
  "FYLO assigns identifiers; save a new entity without one."
- The id field of an entity holds the TTID.

Queries:

- `$ops` is an OR of AND-objects.
- `eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `isNull` and `isNotNull` become
  `$eq`/`$ne`/`$lt`/`$lte`/`$gt`/`$gte` predicates.
- `in` creates one predicate for each value.
- FYLO has no `ORDER BY`. The server applies ordering and paging itself, over
  the bounded set of matches. That set holds at most 10,000 documents.
- If the offset and size of a page reach past that bound, the server refuses
  the page. It does not silently cut the results short.
- Count and exists queries cannot order or paginate.

Collection names:

- A collection name has 2 to 64 lowercase letters, digits and hyphens. It
  starts and ends with a letter or a digit. This is FYLO's own grammar, so
  `hd-events` works.
- Tachyon refuses `..`, `/`, `\`, `_`, upper case and the FYLO reserved words
  (`sql`, `as`, `then`, `db`, `engine`, `cache`, `queue`, `startup`, `join`,
  `ready`, `close`) before it calls FYLO (`TY2160`). Store tables follow the
  same rule.

Documents, collections and the queue (FYLO only; the SQL drivers refuse these
commands with `TY2160`). Every Yon language with a DataSource has the methods
below; Python, Ruby and Rust write them in snake case, C# and Go in Pascal case:

| DataSource method | FYLO | Answer |
| --- | --- | --- |
| `findById(collection, id)` | `getLatest` | The newest version, or null. |
| `insert(collection, document)` | `putData` | The document with its TTID; a document that has an id is refused. Go answers the id. |
| `patch(collection, id, fields)` | `patchDoc` + `getDoc` in one step | The patched document, or null. |
| `deleteById(collection, id)` | `delDoc` | `true`, or `false` when it was gone. |
| `scroll(collection, where, cursor, size)` | one `findDocs`, then pages | `{content, nextCursor}`. |
| `createCollection(name)` | `createCollection` | `true`; it already existing is fine. |
| `dropCollection(name)` | `dropCollection` | `true`, or `false` when there was none. |
| `collectionExists(name)` | `inspectCollection` | `true` or `false`. |
| `publish(topic, payload, key)` | `queuePublish` | The message id; the same key and payload answer the first id. |
| `claim(topic, group, maxMessages, leaseMs)` | `queueClaim` | Up to 100 deliveries `{id, receipt, attempt, payload}`. |
| `ack(topic, group, delivery)` | `queueAck` | `true`. |
| `nack(topic, group, delivery, delayMs)` | `queueNack` | `true`. |

Patch:

- `patch` sets only the named top-level fields, like `Update.set` in Spring
  Data MongoDB. Other fields stay. A null value stores null. A nested object
  replaces the old one. The id cannot change.
- The server holds the FYLO subprocess for the change and the read of the
  result. Patches that run at the same time are applied one after another, so
  none loses another's fields.

Scroll:

- One page has 1 to 100 documents (20 by default) in id order, which is the
  order FYLO created them in. `where` is null or an object of field values
  that must all be equal.
- There is no offset and no 10,000 limit. The first page reads every match
  once and keeps the documents in the server for 15 minutes under a random
  cursor (at most 8 open scrolls). Later pages read from that snapshot, so
  inserts and changes do not move documents between pages, and reading the
  same cursor again reads the same page.
- FYLO 26.33 runs its whole query again for every continuation page of its
  own pagination, and `getDoc` reloads the collection index. That is why the
  server keeps the snapshot itself.
- The first page may take up to 30 seconds (the scroll deadline, also in the
  language clients). The matches of one scroll must fit in FYLO's 8 MiB
  response frame. An expired cursor, a cursor from another query and a server
  restart are refused; start again from the first page.

Queue:

- This is FYLO's own durable queue. Tachyon adds no queue of its own and no
  broker. Delivery is at least once: a delivery that is not acknowledged
  before its lease ends is claimed again. Handlers must be safe to run twice.
- `claim` takes a lease of 1 ms to 24 hours (30 seconds by default) and gives
  up after 3 attempts by default; FYLO then keeps a dead letter per group.
  Acknowledging again with the same receipt is fine; a stale receipt is
  refused.
- Topic and group names have 1 to 127 ASCII letters, digits, dots, hyphens
  and underscores, and start with a letter or a digit.
- Statistics, lease extension and dead-letter reads stay with the `fylo`
  tools for now.

Raw SQL:

- Raw `dataSource.query(sql, parameters)` uses the SQL subset of FYLO.
- The server replaces each `$1`, `$2` parameter with an escaped literal before
  it calls `executeSQL`.
- The server runs one statement at a time.
- The result is keyed by id. The server returns it as rows that carry the `id`
  field.
- The FYLO subset has no aggregate functions. Do aggregation in the
  application or with a derived count.

## DataSource injection

Declare a RequestRoute controller in `server/controllers/CatalogController.ts`.
Put the service below in the shared file of the realm,
`server/services/CatalogService.ts` (ADR 0031):

```typescript
@Service
@NeedsAll
class CatalogService {
  dataSource: DataSource;

  findByName(name: string) {
    return this.dataSource.query(
      'SELECT id, name FROM products WHERE name = $1', [name]
    );
  }
}
```

- This service is a managed provider, like a Spring service that asks for a
  `JdbcTemplate`. The example method contains explicit parameterized SQL.
- A Store cannot hold this code: a Store is a contract that names its Entity,
  and Tachyon writes each of its queries (`TY1308`). It takes `fields`,
  pagination options and an optional `setup`, and provides derived query
  methods.
- The entity adapter of a Store can pass the generated command object to
  `dataSource.command(command)`. The entity authorization checks still apply.
- Do not interpolate values into SQL strings.
- The same service runs against the `fylo` driver, except for the raw SQL
  features that FYLO does not have.

Redis:

- `NeedsAll` also injects a `Redis` field (`redis` in
  JavaScript).
- Its API is `get(key)`, `set(key, value, ttlSeconds)` and `delete(key)`.
- Values are JSON. Every write requires an expiry. Keys and values have size
  limits.
- C# also offers the usual PascalCase names as method aliases.
- Rust methods return `Result<Json, YonResourceError>`. Handlers may return
  that result directly.

## Bounds and failure behavior

- Startup: 10 seconds. Storage operations: 5 seconds. At most 16 admitted
  operations.
- SQL pools: at most 8 connections (1 for in-memory SQLite). TDS (Tabular Data
  Stream, the SQL Server protocol) is serialized.
- FYLO: one subprocess, 1 MiB requests, 8 MiB responses. Each Store query
  scans, orders or pages at most 10,000 matching documents; `scroll` has no
  count limit, a 30-second first page and an 8 MiB snapshot. A cancelled or
  timed-out FYLO call restarts the subprocess, so a late answer can never be
  read as the answer to the next call.
- Results: at most 100 rows, 64 columns per row and 1 MiB of serialized JSON.
- SQL values: null, finite numbers, booleans and strings.
  - Date, decimal and JSON types that are specific to one database need
    explicit SQL casts.
  - Return wide integer IDs as text when browser JSON numbers are not precise
    enough.
- Raw queries accept one data statement or schema statement.
  - Tachyon rejects session commands, transaction commands and batches
    separated by semicolons.
  - Use parameters even for text that contains semicolons.
  - Raw nullable parameters may need typed casts that are specific to the
    driver.
- Tachyon discards a connection after it runs raw network SQL. Because of
  this, arbitrary session state cannot leak into another request.
- Public resource errors do not include SQL errors, credentials, query text or
  capability tokens. Check diagnostics on the database side with authorized
  operational access. Do not log credentials or query parameter values.
- `YON_RESOURCE_ENDPOINT` and `YON_RESOURCE_TOKEN` are private outputs of the
  supervisor for one invocation. They are not application settings.
  - Do not write them in `.env`.
  - Do not expose them through an endpoint.
  - Do not copy them into a client bundle.
  - The supervisor replaces inherited values. It revokes access when the
    invocation ends.
- There is no automatic retry, no transaction annotation and no rollback for
  the whole request. A cancellation does not prove that a mutation did not
  commit.

Before you claim support or release readiness, check the execution matrix for
all drivers and the language runtime results in `HANDOFF.md`.

## Qualification evidence

SQL and Redis:

- On 2026-09-15, real SQLite repository injection passed across all eight Yon
  language adapters.
- Separate driver contracts passed against PostgreSQL 18, MySQL 8.4 and
  MariaDB 11.8, with verified TLS and a generated fixture CA. They also passed
  against Redis 8.
- Run `node scripts/test-managed-storage.mjs` to repeat the isolated Docker
  matrix. CI runs this same fixture harness. It does not change application
  databases.
- `node scripts/test-managed-storage.mjs --without-docker` runs the SQLite and
  FYLO contracts, and the Redis contract against a local `redis-server` when
  one is on `PATH`, or else against `garnet-server`. CI runs it on macOS
  (Redis from Homebrew) and on Windows, where Redis ships no build, so the
  Windows job installs Garnet 2.2.4 and runs the Redis contract against it.
  On Windows the FYLO contract also runs the Yon route phase: the shared
  supervised runner reads Windows processes through CIM (WG1).
- On 2026-10-24 that Windows job passed on `windows-latest` (x64, develop
  `34409d70`, [run 40098937901](https://github.com/d31ma/TACHYON/actions/runs/40098937901)):
  25 storage unit tests, the FYLO route phase ("Yon autowired resources and
  raw SQL") against FYLO v26.33.02, and
  `storage::tests::redis_real_json_ttl_and_delete` against Garnet 2.2.4.
- On 2026-10-25 the same harness passed on Windows 11 ARM64 (the Surface,
  commit `661753db`, WG2), with the x64 FYLO running under emulation and
  Garnet 2.2.4 on .NET 10.

FYLO:

- On 2026-09-15, the FYLO driver passed its storage unit contract. It also
  passed a Yon route against a real `fylo` binary. The route tested save,
  patch, find, count, exists, `in` predicates, raw SQL and delete.
- `ty doctor` reported the `fylo` requirement for a project that configures
  the driver.
- On 2026-10-04, the document, collection, scroll and queue commands passed
  their storage unit tests (including 12 concurrent patches and a scroll over
  10,050 documents) and ran through real `ty handler invoke` Go (typed models)
  and Python services. The website example ran the same flow in every Yon
  language that has a DataSource against a real `fylo` binary.
- On Windows, FYLO v26.33.02 starts `powershell.exe` on every write to read
  its own start time for the collection lock, so each write takes about
  0.8 s (FY1, 2026-10-29, measured on the Surface ARM64 with the x64 FYLO).
  The `fylo_datasource` CLI tests do 123 writes in one request: about 97 s
  of FYLO time, against 6 ms a write natively on macOS (0.85 s in all) and
  10 ms under Rosetta (1.3 s), so emulation is not the cause. With
  PowerShell answered instantly the same writes took 7.7 s. The tests give
  Windows a 240 s handler deadline until Tachyon pins a FYLO release with
  FYLO commit `d75e4ba` (on FYLO develop), which finds the identity once.
  CI's Windows native job installs no FYLO, so the tests skip there.

SQL Server:

- On 2026-10-19, the ignored
  `storage::tests::sql_real_repository_nullable_columns` contract passed
  through the TDS adapter against SQL Server 2022 (the digest-pinned
  `mcr.microsoft.com/mssql/server:2022-latest` image) with forced TLS and the
  generated fixture CA. `scripts/test-managed-storage.mjs` starts it with the
  other Docker fixtures, so CI's Linux storage job runs it too.
- On 2026-10-27 (MA10) the same contract passed from macOS, where the TDS
  adapter uses Apple's TLS library (Security.framework), against SQL Server
  2022 in Docker Desktop under Rosetta. It first failed with `TY2160`:
  Apple's TLS library flushes after every TLS record, and the driver sent
  each flush as its own PRELOGIN message, which SQL Server drops. Tachyon
  now sends a handshake flight as one PRELOGIN message on every platform.
  The same day it passed from Windows on ARM64 over SChannel, against that
  container served on the Mac's LAN address.
- `node scripts/test-managed-storage.mjs --serve --drivers=sqlserver` with
  `TAC_STORAGE_PUBLISH_ADDRESS=<LAN address>` serves the container to
  another machine, which runs
  `node scripts/test-managed-storage.mjs --remote=<tachyon-storage-fixtures.json>`.
  This is how a host without Docker or without an SQL Server image (Windows
  on ARM64) runs the contract.
- The harness sets `ACCEPT_EULA=Y` for that throwaway container only, which
  accepts the Developer edition license for the test. Tachyon never accepts a
  vendor license for an application database.

Qualification of the whole workspace and of native targets is separate from
these driver checks.
