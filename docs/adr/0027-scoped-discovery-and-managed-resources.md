# ADR 0027: Scoped Annotation Discovery and Managed Yon Resources

- Status: Accepted; implementation qualification in progress
- Date: 2026-09-15
- Amends: ADR 0024 explicit manifests, ADR 0026 explicit resource initialization
- Preserves: client-only Tac, REST-only Yon, private configuration snapshots
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Decision

An entry owns the annotated role files beneath its adjacent plural role
folders. Only the entry's language takes part. Discovery does not cross child
entry boundaries or Tac/Yon boundaries. Discovery reads the retained project
snapshot. It never imports application modules. It validates the original
names and paths. Composition is deterministic and bounded. When definitions
require earlier types, composition follows dependency order. Tachyon rejects
legacy `tac.modules.json` and `yon.modules.json` files and gives migration
guidance. It does not silently ignore them.

Yon resolves the optional `yon.datasource` and `yon.redis` settings at
startup. The private application snapshot supplies credentials only from the
root `.env`. The server owns connections and admission. Language processes
are still local to one invocation and are not pooled. `DataSource` and
`Redis` are reserved provider tokens. `RequiredArgsConstructor` can use them
in all eight Yon languages. Initialization never creates, migrates or drops
an application schema.

SQL adapters cover PostgreSQL, MySQL, MariaDB, SQLite and SQL Server. Redis is
a separate key/value resource, not a SQL dialect. Connections work like
this:

- SQLx owns the SQL connections that it supports.
- Tiberius owns TDS (Tabular Data Stream, the SQL Server protocol).
- Redis uses a multiplexed connection manager.

Network SQL verifies TLS (Transport Layer Security) identity. Parameters stay
bound values. Tachyon validates identifiers separately. It never retries
mutations automatically. A timeout can leave a write committed. So callers
must not treat a timeout as proof of a rollback.

Each supervised process receives a fresh, random resource capability for a
private loopback listener. This listener is not a route on the public Yon
server. The listener has these limits:

- four owned connections;
- bounded HTTP headers and JSON bodies;
- one request per connection;
- the invocation's absolute deadline.

The listener rejects browser Origin headers. It requires its exact Host and
bearer token. Dropping the invocation aborts its listener and all owned
operations. Server shutdown closes resource admission and connections within
its existing deadline. The current Firecracker transport cannot carry this
capability. When managed storage is configured, Firecracker fails closed.

Tac JavaScript/TypeScript `Client` declarations use the existing
`HttpExchange` policies. They use same-origin JSON transport and cancellation
that the mount owns. Native companion `Client` construction is not implied.
Native origins and route disposal need an explicit contract before Tachyon
enables that feature. Native companions keep the existing explicit HTTP
behavior adapters. The user explicitly chose manual native client
configuration for this feature.

`Client` is a Tachyon role marker, not a Spring Framework annotation. Spring
HTTP interfaces use `HttpExchange`. Spring Cloud OpenFeign separately offers
`FeignClient`. The Tachyon role owns discovery, injection and layout.
`HttpExchange` owns operation metadata. Declarative clients replace the
public `this.tac.fetch` application API. Framework-owned transport still uses
browser networking internally. This does not imply automatic native client
configuration or a server-side Tac renderer.

## Consequences

- Nobody needs to maintain manifests anymore. But moving a role file between
  entry scopes changes its visibility, and this is deliberate. There is no
  shared global component scanning.
- Resource fields need no user constructor or `Inject` list. Application
  authentication, authorization and transaction semantics stay the
  application's job. Configuring a database does not authorize a public
  endpoint.
- Existing structured Entity/Repository/Query adapters can use the managed
  datasource command boundary. This is not a new reflection ORM
  (object-relational mapper) or an automatic transactional unit of work.
- Driver integration and runtime qualification stay separate from release
  support. A configured driver name alone does not prove that anyone tested
  the database.

## Reference implementation review

The design follows how Spring separates these concerns: component discovery,
connection details, managed resources, repository query parsing and SQL
dialects. The reviewed sources are:

- Spring Boot `DataSourceAutoConfiguration` and `DataSourceConfiguration` at
  `c01e1b34b61de8e72d4072a8a703fa92300ada89`;
- Spring Data JDBC (Java Database Connectivity) `DialectResolver` at
  `684d2538a84a770281eb5c07c8f7d6c1d812104d`;
- Spring Data Commons `PartTree` at
  `bd0ccbb5125463b50116f52ffef51b7b3951a5a5`.

Spring Boot can recognize a SQLite JDBC driver. This does not mean that
Spring Data JDBC has built-in support for a SQLite dialect. Tachyon owns its
SQLite dialect and tests it explicitly.
