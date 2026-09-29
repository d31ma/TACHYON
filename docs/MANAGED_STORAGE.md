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

- The URL is `fylo:///absolute/root`.
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
- FYLO: one subprocess, 1 MiB requests, 8 MiB responses. Each query scans,
  orders or pages at most 10,000 matching documents.
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

FYLO:

- On 2026-09-15, the FYLO driver passed its storage unit contract. It also
  passed a Yon route against a real `fylo` binary. The route tested save,
  patch, find, count, exists, `in` predicates, raw SQL and delete.
- `ty doctor` reported the `fylo` requirement for a project that configures
  the driver.
- Qualification across the compiled Yon languages is still pending. See
  HANDOFF.md.

SQL Server:

- SQL Server has an implemented TDS adapter. This work did not qualify it by
  running it.
- Tachyon does not accept its vendor license automatically. The Docker matrix
  does not claim SQL Server coverage.
- Before you raise its support level, run the ignored
  `storage::tests::sql_real_repository_nullable_columns` contract against a
  SQL Server instance that you provision separately.

Qualification of the whole workspace and of native targets is separate from
these driver checks.
