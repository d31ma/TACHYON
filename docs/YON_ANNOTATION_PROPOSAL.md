# Proposal: Yon Behavior Annotations

- Status: Adapter ownership accepted; polyglot Tac/Yon implementation in [the behavior contract](BEHAVIOR_ANNOTATIONS.md)
- Date: 2026-09-12
- Decision: application-selected adapters across every supported source language; additional Spring-inspired annotations remain recommendations
- Implemented authoring contract: [Yon annotations](YON_ANNOTATIONS.md)

## Intent

Routing stays based on conventions:

- Controller methods named `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`,
  and `HEAD` are endpoints.
- These endpoints need no HTTP verb decorators.
- Use annotations only for behavior that the method name cannot express.
- Tac supports optional behavior annotations. Tac still renders on the client.
- Yon never renders a template.

The vocabulary and the open questions below are the original design record.
`BEHAVIOR_ANNOTATIONS.md` is the normative source for the implemented API and
its limits.

The requested additions are real execution features. They are not only
decorative metadata. Do not accept a marker and then silently remove it. First,
implement and test its complete behavior and failure contract in every language
adapter that claims support.

## Proposed Vocabulary

| Marker | Proposed responsibility | Boundary to settle before implementation |
| --- | --- | --- |
| `Cache` | Declares a named cache policy: namespace, lifetime, capacity and provider. | Choose the cache provider. Decide where state that lasts across requests lives. It is not another name for HTTP `Cache-Control`. |
| `Cacheable` | Returns an existing method result for the declared key. Otherwise it runs the method and stores a successful result. | Define the key inputs, partitioning by tenant or user, serialization, TTL (time to live), invalidation and failure policy. |
| `Pageable` | Validates and normalizes pagination and sorting input before the endpoint uses it. | Choose cursor or offset semantics. Choose the pagination value the request sees. Require page-size bounds and a list of allowed sort fields. It does not fetch an unbounded collection and then cut it. |
| `Entity` | Marks a persistent application model for a selected persistence adapter. | Define model and field types, identifiers and serialization. Do not register the model as a DI (dependency injection) service. |
| `Table` | Maps an entity to a database table with an explicit name. | Choose the database or mapping adapter. No implicit table creation, no migration and no destructive DDL (data definition language). |
| `RequiredArgsConstructor` | Generates a constructor that assigns the explicitly required instance fields in declaration order. | Define the required-field syntax for each language, and how it works with explicit `Inject` tokens. Do not guess dynamic JS/Python/PHP types. Do not silently overwrite an existing constructor. |
| `Configuration` | Declares a class for configuration and provider registration. | Keep provider factories separate from typed environment binding. Define when values are read and when factories run. Only environment names owned by the project (`TAC_`/`YON_`) are allowed. |
| `HttpExchange` | Generates an outbound HTTP client call. It is not an inbound REST route. | Define the transport adapter, request and response mapping, timeouts, cancellation, size limits, redirects and credential handling. |

`CachePut` (run the method and update the cache) and `CacheEvict` (explicit
invalidation) are candidate companion annotations. They are not implemented
additions. A cache feature must ship a usable invalidation API, even if the
first interface is imperative.

## Constraints

Discovery and builds:

- The framework validates all metadata without running application code during
  discovery or bundling.
- Builds make no database calls and no network calls.

Process model:

- Yon starts a new supervised process for each invocation.
- So an in-process map is not a cache across requests. Provider `singleton`
  scope cannot be used to pretend that it is.
- Durable or shared storage must have explicit ownership.

Cache safety:

- A cache hit must not skip request validation, authentication, authorization
  or tenant isolation.
- At first, errors and streaming results are not cached.
- Do not share personalized results under keys built only from the URL or only
  from the arguments.

Bounds:

- Bounds cover cache entry count and bytes, individual values, key length, TTL,
  pagination size and offset, outbound bodies and retries.
- The policy for a cache outage is explicit. A fallback must not silently repeat
  unsafe side effects.

Keys and queries:

- Cache keys are structured data or bounded field selections. They are not
  arbitrary annotation expressions that the framework evaluates.
- Parameterized queries are required. Request input cannot become a table or
  column identifier or an SQL fragment.

Constructor generation:

- It keeps source diagnostics and is deterministic.
- Each language must compile and run the generated constructors before the
  framework advertises support. Ambient TypeScript declarations alone do not
  prove support.

Configuration secrets:

- The framework reads them at the invocation or startup boundaries, as
  specified.
- They never appear in generated bundles, manifests, cache keys or diagnostics.
- An annotation cannot extend the allowlist of the child environment.

Outbound HTTP:

- It inherits the deadline and cancellation of the invocation.
- Redirects cannot forward credentials to another origin.
- Retries of non-idempotent requests are off, unless they are explicitly
  configured with an idempotency contract.
- URLs that an endpoint provides do not skip the deployment's egress (outbound
  traffic) policy.

## Implementation Decision Needed

The current framework has caches for compiled artifacts and Tac browser caches.
Neither one is a cache for application method results. The framework has no
entity mapper, no database adapter and no declarative outbound HTTP client that
these markers could attach to.

Recommended direction:

- Yon owns annotation parsing, generated calls, validation and lifecycle
  integration.
- Adapters that the application selects own storage, queries and transport.

This avoids choosing a database implicitly. It also avoids forcing the library
of one language onto all eight language adapters. A first-party backend is also
possible, but it needs its own decisions about dependencies, deployment and
support.

After that choice, deliver separate tested slices instead of one release that
only adds markers. The order is:

1. required-field construction and configuration
2. outbound HTTP
3. pagination
4. cache policy, results and invalidation
5. entity and table persistence

Each slice needs:

- positive and negative compiler fixtures
- real runtime tests
- failure and cancellation tests where relevant
- documentation
- bounded CI gates

## Upstream References

- [Spring cache annotations](https://docs.spring.io/spring-framework/reference/integration/cache/annotations.html)
  separate result reuse, updates, eviction and shared cache configuration.
  Yon's proposed `Cache` name does not claim compatibility with the Spring API.
- [Spring HTTP interfaces](https://docs.spring.io/spring-framework/reference/integration/rest-clients.html#rest-http-interface)
  inform the separation between outbound client declarations and inbound
  routing.
- [Spring Data Pageable](https://docs.spring.io/spring-data/commons/docs/current/api/org/springframework/data/domain/Pageable.html)
  is a pagination interface, not a `Pageable` annotation. The proposed Yon
  marker would normalize input. A separate value would carry the resulting page
  request.
- [Jakarta Persistence Entity](https://jakarta.ee/specifications/persistence/3.2/apidocs/jakarta.persistence/jakarta/persistence/entity)
  describes persistence identity. It is not the same as a DI provider role.
- [Lombok constructor generation](https://projectlombok.org/features/constructor)
  informs how required fields are assigned. Yon needs explicit rules for each
  language. It must not assume that Java's `final`/nullability model exists in
  every supported language.
