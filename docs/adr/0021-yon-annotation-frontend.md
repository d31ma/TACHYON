# ADR 0021: Yon Annotation Frontend and Constructor Injection

- Status: Accepted (controller/provider foundation)
- Date: 2026-09-11
- Amended: 2026-09-12 (HTTP verb-named methods replace redundant method annotations)
- Supersedes: the prohibition on Yon annotations in ADR 0020
- Preserves: ADR 0016, Tac's client-only runtime, and plain Handler endpoints
- Follow-up: [ADR 0022](0022-shared-behavior-adapters.md) adds optional behavior annotations to all supported Tac and Yon languages
- Amended by: [ADR 0048](0048-yon-routes-come-from-annotations.md): Yon routes come from annotations in `server/controllers/` (`RequestRoute`, `GetRoute`, ...), not from `server/routes/` folders; clients use `HTTPClient` and `GetRequest`, ...; `OPTIONS.schema.json` is replaced by schemas built from models

- Follow-up: [ADR 0025](0025-required-constructor-inference.md) adds bounded required-field constructor inference in Yon
- Amended by: [ADR 0044](0044-an-annotated-file-holds-one-class.md): each provider now lives in its own file, and "the same source file" below means the unit that discovery composes from those files

## Context

Yon applications benefit from the controller and provider vocabulary that
developers know from NestJS and Spring. But importing the runtime of either
framework would tie Yon to one language. It would also duplicate Yon's own
routing and process ownership.

Yon runs one bounded process per invocation. A dependency container for the
whole server would need a different lifecycle and a different isolation
contract.

## Decision

### Annotation frontend

Tachyon adds an annotation frontend to the eight existing Yon adapters. The
frontend reads annotations but never executes them.

The recognized declarations produce a deterministic provider and route plan
that does not depend on a language. Before the source runs, Tachyon turns
that plan into the existing Handler ABI (application binary interface). It
uses direct constructor calls and calls to named controller methods for this.

None of these are required:

- a runtime annotation package;
- Java DI (dependency injection) based on reflection;
- a Rust procedural macro;
- the JavaScript reflect-metadata dependency.

### Routes and methods

Filesystem paths remain the only way to register a route path. A controller
exposes instance methods named after uppercase HTTP verbs: `GET`, `POST`,
`PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD`. It does not declare a second URL
hierarchy or method decorators.

- Tachyon rejects HTTP markers with migration guidance. The method name alone
  decides dispatch.
- Helpers and provider methods do not register routes.
- JSON, explicit response descriptors, opaque `text/html`, relay, cancellation
  and streaming still follow their existing contracts.
- Builds never invoke controllers.

### Providers and scopes

The foundation supports:

- providers declared in the same source file (since ADR 0044, the same
  composed unit: each provider has its own file in its role folder);
- explicit positional constructor dependencies;
- the singleton, request, and transient scopes.

Tachyon caches singleton and request providers only inside a container that
lives for one invocation. It constructs a transient provider for each
dependency resolution. No instance or mutable state survives from one request
to the next.

Tachyon rejects a singleton that depends on a request provider, directly or
indirectly. This rule prevents a captive dependency if containers later live
longer. (A captive dependency is a short-lived object held by a longer-lived
object.)

### Validation and limits

Tachyon fails before invocation for:

- missing tokens;
- duplicate registrations;
- cycles;
- unsupported annotations and declaration forms.

Tachyon bounds the source size, the provider count, the dependency count, and
the identifier size. It removes the annotations it owns but keeps source line
positions. Generated dynamic entries are immutable files placed next to the
captured source. So ordinary relative imports still resolve.

### Plain handlers

Existing plain Handler code remains supported, without annotations or DI.
Do not mix an annotated controller and a plain Handler in one source.

## Upstream Design Review

We reviewed the public NestJS and Spring Framework repositories on 2026-09-11.

At review time, the repository heads were NestJS
`a3a31b96436a0c5fc33266a19c02cfa4fd2c03c0` and Spring Framework
`a099debca9e59eb562cb3298690423dbb9d0127e`.

- [NestJS injectable metadata](https://github.com/nestjs/nest/blob/master/packages/common/decorators/core/injectable.decorator.ts)
  keeps the provider declaration separate from instance construction.
- [NestJS injector](https://github.com/nestjs/nest/blob/master/packages/core/injector/injector.ts)
  treats contextual instances, transient consumers, dependency lookup, and
  cycle failures as separate cases.
- [NestJS route execution](https://github.com/nestjs/nest/blob/master/packages/core/router/router-execution-context.ts)
  shows separate boundaries for guards, arguments, interceptors, and responses.
- [Spring singleton registry](https://github.com/spring-projects/spring-framework/blob/main/spring-beans/src/main/java/org/springframework/beans/factory/support/DefaultSingletonBeanRegistry.java)
  makes instance ownership and lifecycle explicit.
- [Spring AOT instance supplier](https://github.com/spring-projects/spring-framework/blob/main/spring-beans/src/main/java/org/springframework/beans/factory/aot/BeanInstanceSupplier.java)
  (AOT means ahead-of-time) guided the split between dependency resolution and
  generated creation.

This is an independent implementation. It does not copy framework internals.
Yon chooses an explicit, bounded graph built at build time. It does not import
the full runtime of either framework. This choice fits Yon's architecture. It
is not evidence that Yon performs better than either upstream framework.

## Deliberately Not Shipped by This Slice

This part of the work does not include:

- implicit discovery of annotated providers across files, and module
  visibility (ADR 0024 adds explicit, bounded same-language module lists);
- interface or string injection tokens (ADR 0025 now specifies bounded
  required-field inference, so authors do not need to write Inject lists);
- parameter decorators, guards, interceptors, filters, and lifecycle
  callbacks;
- async provider factories, lazy proxies, and forward references that break
  cycles;
- persistent worker containers or singletons shared across requests;
- arbitrary decorator expressions, registration through inheritance, or
  macros.

Ordinary imported helper modules and the existing root middleware remain
available. Future provider-module or hook APIs need explicit behavior and
evidence for each language and runtime. Documentation must not advertise them
as implemented here.

## Acceptance

These checks establish the implemented boundary:

- parser diagnostics;
- real handler execution in all eight language adapters;
- scope identity;
- stream dispatch;
- response preservation;
- the canonical Rust gate.

Documentation separates this foundation from a full application container
equal to NestJS or Spring.
