# ADR 0026: JSON Results and Separated Configuration

- Status: Accepted
- Follow-up: ADR 0027 adds scoped role discovery, managed Yon resources and
  browser Client injection; the separation of public configuration and secrets
  remains unchanged.
- Amended by: [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md),
  which moves Tac's public configuration from `application.json` to the web
  app manifest and publishes it at `/manifest.webmanifest`
- Date: 2026-09-14
- Amends: ADR 0021 response authoring and ADR 0023 configuration v1
- Preserves: REST-only Yon, client-only Tac, bounded supervision and explicit module ownership

## Context

Ordinary REST handlers should return application data. They should not need
to build a transport wrapper. Non-secret settings belong in JSON that people
can review. Runtime secrets are separate inputs. A client build never needs
to open them.

## Decision

Ordinary Yon results are normalized to JSON with status 200 and Content-Type
application/json in all eight languages. Implementations prefer standard JSON
facilities. Self-contained adapters provide equivalent parse/stringify
operations. Malformed input fails. It does not silently become an empty
object. Strings stay JSON strings. Explicit response helpers keep status,
headers, redirects, raw HTML and relay behavior. A list alone does not imply
streaming.

Application Configuration v2 has only two root namespaces: tac and yon. Key
segments use lowercase ASCII letters and digits. They form nested
configuration with limits. Every underscore in an application name is a path
separator: YON_CATALOG_PAGE_SIZE means yon.catalog.page.size. Arrays are
values. They are not paths indexed by underscores. These cases fail closed
(Tachyon rejects them):

- duplicate keys;
- empty segments;
- a key that is a scalar in one place and an object in another.

Non-secret settings belong in application.json and an optional
application-<profile>.json. For Yon, the parent YON_PROFILE selects the
profile. Objects merge recursively. Arrays replace. Explicit CLI endpoint
arguments win. Parent environment application values no longer override
settings or supply secrets. Operator runtime/isolation policy stays separate
and unchanged.

Only the .env file at the project root provides application secrets. Literal
YON_ assignments, with limits, map to private paths. Tachyon handles them
without shell execution, without interpolation and without changing the
process environment. Secrets cannot overwrite configuration values. JSON
files reject recognized credential fields and URLs that contain credentials.
Application secrets with unusual names must still go in .env. Tachyon rejects
TAC_ secrets, because client code cannot keep credentials private. This
includes native packages. Production may mount a .env that a secret manager
manages.

At server startup, Tachyon captures one private resolved snapshot for
handlers, middleware and workers. It passes the snapshot through the reserved
supervisor output YON_APPLICATION_JSON. Diagnostics and Debug output omit the
values. Reads have limits, are non-following (they do not follow links) and
are relative to a capability. Changes to Yon settings and .env require a
restart. These inputs cannot select runtime executables, isolation or
environment authority.

Discovery captures only the base public tac namespace. Web builds publish it
as /.tachyon/application.json. Staged native bundles also carry it. Builds do
not open .env or a Yon profile for it. Builds never publish yon. Binding stays
explicit. There is no Tac DI (dependency injection) container and no
automatic adapter initialization. During development, changes to
application.json reload Tac's public projection. They do not replace the
running private Yon snapshot.

## Migration and Consequences

- Move the old server/datasource/redis sections under yon. Convert flat
  prefixed properties into nested tac/yon paths. Tachyon rejects old
  authoring and gives guidance. Configuration v1 stays a historical schema;
  v2 is current.
- Move secrets to an ignored .env. Remove JSON placeholders and parent
  application environment overrides. Commit only non-secret templates.
- Value uses canonical dotted paths that start with the framework namespace.
  Property/conditional annotations keep prefixed names. They bind snapshots
  that the application supplies explicitly.
- Response helpers stay optional transport APIs. They are separate from JSON
  codecs.
- Role annotations still validate original paths before lowering. This ADR
  does not replace module manifests with automatic registration.

## Acceptance

Real-runtime tests cover direct JSON values, explicit responses and malformed
input. Configuration tests cover namespaces, profiles, redaction, collisions,
safe paths and immutable snapshots. Artifact tests prove that Tachyon
publishes only the captured tac configuration, even when .env cannot be
opened. Website examples cover eight Yon languages and public Tac binding.
Native source staging does not prove that native companions execute.
