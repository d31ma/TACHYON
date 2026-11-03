# ADR 0023: Private Application Configuration

Superseded authoring schema and secret sources: [ADR 0026](0026-json-results-and-separated-configuration.md)
defines v2 tac/yon namespaces and separate .env secrets. The historical v1
decision below is not the current configuration guide.

- Status: Accepted
- Date: 2026-09-12
- Preserves: ADR 0014 deployment-selected isolation, ADR 0015 client-only Tac,
  ADR 0016 REST-only Yon, ADR 0022 explicit behavior configuration

## Context

Applications need a predictable place for these settings: the server bind
address, the datasource, Redis and typed application properties. Spring-style
external configuration is useful. But a second parser and an implicit global
container would make Tachyon's bounded, polyglot execution boundary more
complex. Client builds must not depend on production secrets. An application
file cannot change the isolation authority that the operator selected.

## Decision

The project root can have an optional `application.json` and one optional
`application-<profile>.json`. Only `YON_PROFILE` selects the profile file. The
loader:

- reuses the existing JSON dependency;
- rejects duplicate and unknown keys;
- limits file names and input size;
- validates all supported section types.

There is no YAML, JSON5, remote import or expression-language evaluator.

The versioned Application Configuration v1 JSON Schema describes the authoring
structure. Runtime validation also owns duplicate keys, byte limits,
environment expansion and resolved endpoints. Editors associate the schema
externally. Applications do not add an unsupported `$schema` application
property.

The supported sections are `server`, `datasource`, `redis` and `properties`.
`properties` is a map of string values with framework-prefixed names. Objects
merge recursively. For the CLI host/port, precedence goes from highest to
lowest:

1. explicit argument;
2. supported environment override;
3. selected profile;
4. base file;
5. existing default.

Library host/port options stay explicit. Environment placeholders may use only
the current `TAC_`/`YON_` names. They may have optional literal defaults.
Expansion is never recursive. Parent environment overrides and secret values
are data. They are never executable configuration.

Tachyon resolves the configuration once for a server startup or an explicit
handler invocation. The resulting owned snapshot is private to the server and
its supervised invocations. Builds and static preview do not load it or
publish it. Changes require a restart. A client hot rebuild cannot replace the
runtime snapshot.

The supervisor passes the validated, serialized snapshot through a reserved
output, `YON_APPLICATION_JSON`. This output is in the environment of the child
or control program. All owned Yon languages can parse this ordinary JSON. They
need no new language-specific package and no change to the protocol envelope.
Parent input cannot replace the reserved output. General child environment
inheritance stays deny-by-default.

Applications pass `properties` into behavior binding/selection explicitly. They
give datasource/Redis settings to their selected adapters. This decision adds
no automatic driver, connection pool, cache, ORM (object-relational mapper),
transaction manager or reflection scanner. No configuration crosses
automatically into browser or native Tac.

These stay outside application files: isolation selection, runtime executable
paths, environment allowlists and capabilities. Firecracker control programs
must own explicit propagation into the guest. The environment transport does
not prove guest configuration. It also does not prove qualification for
hardware isolation.

## Consequences

- JSON adds no new parser dependency and has deterministic type semantics. But
  users cannot write YAML syntax or comments.
- Startup fails when a selected profile is missing or the configuration is
  invalid. It does not silently fall back or run with a partial configuration.
- Secrets can be injected at runtime without affecting deterministic client
  artifacts. Applications stay responsible for not returning or logging them.
- One private environment transport covers the existing Yon language matrix.
  It adds no ambient behavior binding and no mutable framework globals.
- Deployments restart to apply changes. Hot-reloadable connection pools and
  secret rotation need separate, specified lifecycle work.

## Acceptance

Tests cover:

- merge and precedence;
- malformed, duplicate, unknown and oversized input;
- missing and unsafe profiles;
- placeholder limits and redacted failures;
- direct environment overrides;
- reuse of the immutable snapshot;
- protection of the reserved output;
- access from the supervised runtime;
- absence from published artifacts.

Existing tests for isolation, non-loopback acknowledgement and behavior
properties stay valid.
[Application configuration](../APPLICATION_CONFIGURATION.md) describes the
public workflow.

## Amendment 2026-09-12: `Value`

A shared `Value` annotation binds one field from the resolved snapshot. It
adds no transport and no ambient read. `Behaviors.values` takes the parsed
`YON_APPLICATION_JSON` object that the application already holds. It returns
bound JSON. It uses the property conversion rules that
`ConfigurationProperties` defines.

Paths are the four sections plus `properties.<prefixed name>`. Tac may name
only `properties.TAC_*`, so a client can never declare a datasource secret.
The placeholder spelling stays because the file uses it. A placeholder stays
one path and one literal default. See
[behavior annotations](../BEHAVIOR_ANNOTATIONS.md).
