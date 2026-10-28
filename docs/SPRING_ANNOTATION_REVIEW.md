# Spring-Inspired Annotation Review

History of this review:

- 2026-09-12: reviewed against the Spring Framework and Spring Boot
  documentation and their public source repositories.
- 2026-09-15: revised for ADR 0031 (ADR means architecture decision record).

Spring guides the design. Tachyon is not a Spring-compatible container and not
an ORM (object-relational mapper).

- The [behavior guide](BEHAVIOR_ANNOTATIONS.md) records 23 implemented shared
  names.
- Only `Controller` and `Stream` are Yon-only.
- These names are markers in both realms (ADR 0030, ADR 0031): `Service`,
  `Scope`, `Client`, `Component`, `Inject`, bare `Repository`, `Resource`,
  `RequiredArgsConstructor`, `PostConstruct`, `PreDestroy`, `EventListener`,
  `Order` and `Async`.

This review is not evidence that a release is qualified.

In this document, SpEL means the Spring Expression Language.

## Adopted families and deliberate differences

| Annotation family | Shared Tachyon contract | Intentionally excluded |
| --- | --- | --- |
| `CacheEvict`, `CachePut` | Explicit Cacheable target in the same class, mapped arguments, normalized keys, authorization before mutation, cache action after a successful bounded execution. Put always executes. | Expression evaluation, clearing across tenants, eviction before invocation, and atomic database/cache transactions. |
| `Repository`, `Query` | Entity/Table mapping local to the source; empty query bodies; bounded find/count/exists with AND predicates, structured bindings and derived names; required query authorization. | Raw SQL, nested paths, joins, OR, classpath scanning and generated database connectors. Bare Repository is the injectable role, in both realms. |
| `Validated`, `Valid`, `NotNull`, `NotBlank`, `Size`, `Min`, `Max` | Pure JSON schemas for direct fields, UTF-8 Size, explicit Unicode whitespace, validation at the method boundary, and redacted field/rule paths. | Nested validation, groups, reflection, running constructors or getters, and authorization claims on the client side. |
| `ConfigurationProperties` | Explicit binding of a string map, the TAC_/YON_ prefix of the current framework, strict scalar conversion, typed defaults, rejection of unknown keys, and bounded/redacted errors. | Snapshots of the ambient environment, implicit secret access and bean creation. |
| `Profile`, `ConditionalOnProperty` | Explicit inputs taken as a snapshot, OR matching of profiles plus an exact property condition, rejection of inactive items before configure. | Global application contexts, package scanning and automatic fallback profiles. |
| `EventListener`, `Order`, `Async`, `ApplicationEventPublisher` | The Spring shape: an injected publisher with `publishEvent(event, value)`; listeners on one or several events with a compiled `condition`, an `id` and `replay`; ordered lowest first; deferred with `Async`; returned `{event, value}` records are published again. | SpEL conditions (the condition grammar is compiled, bounded and has no evaluator), event class hierarchies, listeners bound to transactions, a process-wide or durable bus, and `Async` in Yon languages whose event methods are synchronous. |
| `Value` | One field bound from the resolved application snapshot: a closed set of section paths plus properties with the framework prefix, in placeholder form or typed-descriptor form, and an explicit `Behaviors.values` call that returns bound JSON. | SpEL and `#{}` expressions, arbitrary property paths, reflective field injection at construction, and selection of a bean provider. |

The original families are still Configuration, RequiredArgsConstructor, Cache,
Cacheable, Pageable, Entity, Table and HttpExchange. All of them use adapters
that the application selects. All of them keep the existing ownership
boundaries for targets and languages.

### Caching

In the Spring caching model, eviction after a successful execution is the
normal case. Spring also separates two things: filling the cache, and skipping
execution on a cache hit. Tachyon adopts these useful rules. It does not import
Spring expression evaluation. It does not import the self-invocation behavior
that is specific to Spring proxies.
[Caching reference](https://docs.spring.io/spring-framework/reference/integration/cache/annotations.html),
[CacheEvict source](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/cache/annotation/CacheEvict.java),
[CachePut source](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/cache/annotation/CachePut.java).

### Configuration

Spring Boot has typed external configuration and conditional configuration.
These are useful models. Tachyon still keeps explicit adapter factories and
strict boundaries for prefixes and secrets. It does not use an ambient
application container.
[External configuration](https://docs.spring.io/spring-boot/reference/features/external-config.html),
[ConfigurationProperties source](https://github.com/spring-projects/spring-boot/blob/main/core/spring-boot/src/main/java/org/springframework/boot/context/properties/ConfigurationProperties.java),
[Profiles](https://docs.spring.io/spring-boot/reference/features/profiles.html),
[ConditionalOnProperty source](https://github.com/spring-projects/spring-boot/blob/main/core/spring-boot-autoconfigure/src/main/java/org/springframework/boot/autoconfigure/condition/ConditionalOnProperty.java).

### Value

The Spring `@Value` resolves `${...}` placeholders and SpEL against the whole
environment at injection time. Tachyon keeps the placeholder spelling, because
`application.json` already uses it. But Tachyon binds the value through an
explicit factory over the private snapshot:

- no expression evaluation;
- no reflection;
- only the set of paths that the configuration contract already defines.

Adding `Value` did not reopen `Bean`. A bound value is data, not a provider.
[Value reference](https://docs.spring.io/spring-framework/reference/core/beans/annotation-config/value-annotations.html).

### Queries

Spring Data supports both derived queries and declared queries. Tachyon keeps
a smaller, read-only grammar with explicit field allowlists and data bindings.
It does not copy the store-specific query language or the proxy model of Spring
Data.
[Spring Data query methods](https://docs.spring.io/spring-data/jpa/reference/repositories/query-methods-details.html).

### Validation and transactions

The validation family comes from Jakarta Bean Validation, which Spring
integrates. Not all of these annotations belong to Spring. Transaction
semantics need real support from a resource manager. Syntax alone cannot
provide them.
[Validation reference](https://docs.spring.io/spring-framework/reference/core/validation/beanvalidation.html),
[Transaction reference](https://docs.spring.io/spring-framework/reference/data-access/transaction/declarative/annotations.html).

## Implemented since this review was written

`Transactional` now has the transaction-capable adapter it needed.

- The marker names an `entities` adapter. The adapter answers begin, commit and
  rollback.
- The managed datasource implements this adapter with a pinned connection.
- At most four such connections are open at once. Each has an idle limit of ten
  seconds.
- It does not make an HTTP call, a cache and a database atomic together. It
  only groups the entity commands that one method issues.

`Scheduled` now declares the timing of a worker beside the worker. It uses the
supervisor that already ran `.tachyonrc` workers. A browser still has no
durable scheduler, so the annotation is Yon-only.

`Retryable` and `ConcurrencyLimit` adopt the resilience annotations of Spring
Framework 7. They use millisecond literals instead of `TimeUnit`. `RestClient`
is available beside `HttpExchange`. It is the imperative form of the same
bounded adapter call.

`Autowired` and `Qualifier` replace the Lombok and JSR-330 spellings that the
framework had borrowed. (JSR-330 is the Java standard for dependency
injection.)

- A field with `@Autowired` joins the generated constructor in declaration
  order.
- Put `@Qualifier('Name')` beside the field to name the provider when the name
  or type of the field does not identify it.

Roles can now be declared abstractly. Spring does the same for a repository or
an HTTP interface. The abstract form is:

| Language | Abstract form |
| --- | --- |
| Java, Kotlin, C#, PHP | an `interface` |
| Swift | a `protocol` |
| Rust | a `trait` |
| Python | a `Protocol` class |

The compiler lowers the abstract form into the concrete type. It writes the
behavior context, the constructor and every body. JavaScript has no method
without a body, and TypeScript cannot decorate an abstract member. So both
languages keep an empty body.

`Mapper` and `Mapping` adopt the vocabulary of MapStruct. MapStruct is not part
of Spring Framework, but every Spring application writes a mapper with it.

- `@Mapping` is the whole method. It is a literal plan of dotted source paths
  and constants over one bounded JSON argument.
- There is no expression language, no `qualifiedByName` and no generated
  implementation class.
- The compiler writes the body itself.

## Names now follow Spring

Every marker that had a Tachyon-only name now has the Spring name:

| Old name | New name |
| --- | --- |
| `Cache` | `CacheConfig` |
| `Pageable` | `PageableDefault` |
| `HTTPExchange` | `HttpExchange` |
| `Component`, then `Helper` | `Delegate` |

The compiler refuses the retired spellings and names the replacement. The
delegate folder is `delegates/`, not `components/`, because
`client/components/` holds Tac views (ADR 0112).

## Do Not Add Merely for Familiarity

- `Bean`, `Primary`: reconsider these only when there is a specified provider
  model across modules. Today, explicit configuration selects adapters without
  ambiguity.
- `PostConstruct`, `PreDestroy` and `EventListener` were deferred earlier. Two
  later decisions replaced that deferral:
  - ADR 0028 added the owned lifecycle and the local-event contract.
  - ADR 0031 removed the Tachyon-specific `EventPublisher` annotation and used
    the injected `ApplicationEventPublisher` of Spring instead.

  These replace the implicit Tac mount and signals. They do not add a second
  lifecycle.
- `PreAuthorize`: checks in the frontend are not a security boundary. Yon
  authorization should stay explicit until its policy and evaluation semantics
  are specified.
- HTTP verb decorators: Yon already dispatches uppercase verb methods through
  filesystem routing. A second way to declare routes would be redundant.

The `Bean` facility of Spring belongs to its container model. Tachyon
deliberately does not copy that model.
[Bean reference](https://docs.spring.io/spring-framework/reference/core/beans/java/bean-annotation.html).

Event naming:

- `EventListener.classes` is spelled `events` here, because Tachyon events are
  names, not classes.
- `condition` keeps the Spring spelling, but it is not SpEL.
- In Spring, `ApplicationEventPublisher` is an interface that a bean injects.
  Tachyon now publishes in exactly the same way.

[Spring application events](https://docs.spring.io/spring-framework/reference/core/beans/context-introduction.html),
[EventListener source](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/event/EventListener.java).

Accurate naming still matters:

- `Entity`, `Table`, `Id`, `Valid` and the constraint annotations are Jakarta
  names. Spring integrates them, but they are not Spring's own.
- `RequiredArgsConstructor` comes from Lombok. `Inject` comes from JSR-330.
  Tachyon gave both an argument list that neither original has. Field-level
  `Autowired` and `Qualifier` replace both.

## Polyglot Implementation Rule

Every accepted addition must first have:

1. one behavior contract that does not depend on a language;
2. then executable fixtures for all supported languages and for both framework
   environments that apply.

A marker that only parses is not implemented support. Document and test these
points explicitly:

- typed native values;
- ownership of cancellation;
- adapter failures;
- synchronous versus asynchronous behavior.
