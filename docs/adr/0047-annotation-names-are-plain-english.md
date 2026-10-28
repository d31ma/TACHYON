# ADR 0047: Annotation Names Are Plain English

- Status: Accepted
- Date: 2026-09-25
- Amends: [ADR 0022](0022-shared-behavior-adapters.md),
  [ADR 0025](0025-required-constructor-inference.md),
  [ADR 0027](0027-scoped-discovery-and-managed-resources.md),
  [ADR 0028](0028-owned-application-events-and-lifecycle.md),
  [ADR 0031](0031-realms-mirror-each-other.md),
  [ADR 0038](0038-resources-are-autowired-by-spring-names.md) (the marker
  names, not the resource names),
  [ADR 0039](0039-a-bare-required-args-constructor-takes-every-field.md),
  [ADR 0044](0044-an-annotated-file-holds-one-class.md),
  [ADR 0045](0045-go-is-a-service-language.md) and
  [ADR 0046](0046-tachyon-annotations-are-global.md): every place where they
  name an annotation or a role folder
- Preserves: what each annotation does, its JSON arguments, the error codes,
  the framework types, and the Spring names of the browser resources
  (`view`, `lifecycle`, `cacheManager`, `resourceLoader`, `taskScheduler`,
  `applicationEventPublisher`)
- Amended by: [ADR 0048](0048-yon-routes-come-from-annotations.md): Yon routes come from annotations in `server/controllers/` (`RequestRoute`, `GetRoute`, ...), not from `server/routes/` folders; clients use `HTTPClient` and `GetRequest`, ...; `OPTIONS.schema.json` is replaced by schemas built from models

## Context

Tachyon copied its annotation names from Spring, Lombok, MapStruct and
JUnit. Those names are clear to a Java developer. They are not clear to a
beginner who reads English as a second language. `Autowired`,
`RequiredArgsConstructor`, `Qualifier`, `Transactional` and
`DirtiesContext` are rare words or jargon. `Service` is a common word and
needs no explanation.

## Decision

- Each annotation has one plain English name. The old name is refused.

  | Old | New |
  | --- | --- |
  | Autowired, Inject | Needs |
  | RequiredArgsConstructor | NeedsAll |
  | Qualifier | Named |
  | Scope | Lifetime |
  | Repository | Store |
  | Mapper, Mapping | Converter, Convert |
  | Configuration | Setup |
  | ConfigurationProperties | Settings |
  | Value | Setting |
  | Profile, ConditionalOnProperty | WhenProfile, WhenSetting |
  | PostConstruct, PreDestroy | OnStart, OnStop |
  | EventListener | OnEvent |
  | Async | InBackground |
  | Scheduled | Schedule |
  | Cacheable, CachePut, CacheEvict, CacheConfig | Cached, UpdateCache, ClearCache, CacheSettings |
  | Retryable | Retry |
  | ConcurrencyLimit | MaxAtOnce |
  | Transactional | AllOrNothing |
  | PageableDefault | PageDefaults |
  | HttpExchange, HTTPExchange, Client, WebCall | HTTPClient (ADR 0048) |
  | GetExchange, PostExchange, PutExchange, PatchExchange, DeleteExchange | GetRequest, PostRequest, PutRequest, PatchRequest, DeleteRequest (ADR 0048) |
  | Validated, Valid | Checked, Check |
  | NotNull, NotBlank, Size | Required, NotEmpty, Length |
  | TachyonTest, TestConfiguration | Test, TestSetup |
  | MockBean, MockitoBean | Fake |
  | ActiveProfiles, TestPropertySource, DirtiesContext | UseProfiles, TestSettings, FreshStart |

  These names do not change: `Service`,
  `Id`, `Query`, `Gateway`, `Description`, `Order`, `Min`,
  `Max`, `Stream`, `Entity`, `Table`, the HTTP verbs and the `Middleware`
  class. ADR 0048 replaces `Controller` with `RequestRoute` and `Client`
  with `HTTPClient`; route methods receive verb annotations.
- Every spelling follows: `@Needs`, `[Needs]`, `#[Needs]`,
  `#[yon::needs]`, and the Python snake case `@needs_all`, `@on_start`,
  `@cache_settings`.
- `Needs` does both old jobs. On a field it is the old `Autowired`. On a
  class, with provider types as arguments, it is the old `Inject`. The
  compiler tells them apart by the declaration that follows the marker.
  `MockBean` and `MockitoBean` were the same marker, so both become `Fake`.
- An old name fails with `TY1308`. The message names the old name and the
  new one, in the author's spelling, for example "'post_construct' was
  renamed to on_start; write on_start".
- Role folders follow their role, so there is one vocabulary.
  `client/` and `server/` hold `stores/`, `setups/` and `converters/`
  instead of `repositories/`, `configurations/` and `mappers/`. A file in
  an old folder fails with `TY1310`, which names the new folder. A role type
  still ends with its role, so `ProductRepository` becomes `ProductStore`,
  `CatalogConfiguration` becomes `CatalogSetup` and `ProductMapper` becomes
  `ProductConverter`. A test setup file is `<Name>TestSetup.<ext>`.
- `ty migrate fix --write` renames the markers in every language, moves the
  files out of the old role folders, and renames the moved role types and
  test setups where they are named, including their lowerCamel and
  snake_case forms. It moves files only from an old role folder to its new
  folder (or from `<Name>TestConfiguration` to `<Name>TestSetup` in
  `tests/`), inside `client/` and `server/`. It never deletes or overwrites
  a file, and it never follows a symbolic link. A move to a name that is
  already taken is a manual step.
- These stay the same: the error codes; the JSON argument keys, such as
  `{"configuration": ...}` in `Test`, `{"value": ...}` in `Lifetime`,
  `havingValue` in `WhenSetting` and `defaultSize` in `PageDefaults`; the
  internal metadata keys and runtime error codes such as
  `inactive-configuration` and the validation paths `notNull`, `notBlank`
  and `size`; and the framework types, such as `BehaviorConfiguration`,
  `BehaviorRepository`, `Behaviors.Configuration` in Java and Kotlin,
  `ApplicationEventPublisher` and `BehaviorContext`.

## Consequences

- This is a breaking change. Old projects fail to build until they run
  `ty migrate fix --write` or rename by hand.
- The runtime error messages that named an old annotation now name the new
  one, for example "Store query was not authorized." and "Declared WebCall
  argument is missing.".
- Some new names are also names in other tools. Tachyon markers are erased
  at compile time, so none of these clash at run time, but they matter to
  readers and editors:
  - Rust's own `#[test]` stays Rust's. An unqualified lowercase `#[test]` is
    never the Tachyon `Test` marker; Tachyon's is `#[Test]` or
    `#[yon::test]`.
  - JUnit's and Swift Testing's `@Test`, and NUnit's `[Test]` and
    `[SetUp]`, go on methods. In a Tachyon file they are now the class
    markers `Test` and `Setup`, so they are refused on a method. JUnit's
    other markers, such as `@BeforeEach` and `@Disabled`, are still ignored.
  - `Named` (JSR-330), `Required` (C# data annotations, Symfony),
    `Converter` and `Convert` (JPA), and `retry` (Python retry libraries)
    have similar meanings elsewhere. Import such a library name under
    another name in a Tachyon file.
  - In TypeScript, the global `Required` annotation and the standard
    `Required<T>` type live in different declaration spaces and do not
    clash. A user class with the same name as an annotation hides the
    global: `@Settings(...) class Settings {}` does not type-check. Name the
    class after its domain, for example `CatalogSettings`.
    `ty migrate fix` reports such a class as a manual step.
- The framework types keep their names for now. Candidates for a later
  decision are `BehaviorConfiguration`, `BehaviorConfigurationSource`,
  `BehaviorRepository`, `BehaviorsRepository`, `BehaviorScopeOptions`,
  `BehaviorValueOptions`, `BehaviorCacheConfigOptions`,
  `BehaviorHttpExchangeOptions`, `ApplicationEventListenerOptions`,
  `BehaviorRequiredArgsConstructor` and `Behaviors.Configuration`.

## Amendment (2026-09-28): Component is now Helper

`Component` was on the list of names that did not change. It is renamed
too, because Tac already uses "component" for a view in
`client/components/`, and one word must not name two different things.

- The annotation `Component` is now `Helper`: `@Helper`, `[Helper]`,
  `#[Helper]`, `// @Helper` in Go and `@helper` in Python.
- The role folder `delegates/` is now `helpers/`, in both realms. This is
  also the folder of the programs that a `Gateway` command runs:
  `server/helpers/`.
- A helper class name ends in `Helper` instead of `Component`.
- The build refuses the old annotation with TY1308 and names `Helper`. It
  refuses a file under `client/delegates/` or `server/delegates/` with
  TY1310 and names the `helpers` folder.
- `ty migrate fix --write` moves every file from `delegates/` to
  `helpers/`, renames each class from `<Name>Component` to `<Name>Helper`
  with its `Needs` fields and types, rewrites the old annotation, and
  rewrites paths such as `server/delegates/report.rb` in the source.

A helper is a class that other classes use for part of their work. Tachyon
creates it and gives it out through `Needs`. Use one when the shared code
needs other classes, saved state, `OnStart` or `OnStop`, or a `Fake` in
tests. Otherwise the code is a utility (static methods only, ADR 0064), a
private method of the one class that uses it, or a service when a view or a
controller calls it. The layer order is view or controller, then service,
then helper, then store or client (ADR 0069).

## Amendment (2026-10-28): Helper is now Delegate

[ADR 0112](0112-a-delegate-takes-on-the-methods-of-its-service.md) renames
`Helper` to `Delegate`. The role folder is `delegates/` again, and a class name
ends in `Delegate`. The build refuses `@Helper` with TY1308 and names
`Delegate`, and it no longer refuses `delegates/`. `helpers/` stays as a folder
that is not a role, for the programs that a `Gateway` command runs and for
behavior-only holders. `ty migrate fix --write` moves each `<Name>Helper` class
from `helpers/` to `delegates/<Name>Delegate` and leaves the other files in
`helpers/` where they are.
