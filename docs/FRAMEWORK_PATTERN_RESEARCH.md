# Per-Language Framework and Pattern Research

Tachyon uses the annotation names of Spring Boot on purpose. Spring Boot is the
most used Java framework. It is based on annotations, and most back-end
developers already know its names (see
[SPRING_ANNOTATION_REVIEW.md](SPRING_ANNOTATION_REVIEW.md)).

That decision looked at one language only. This document asks the same
question for the other eight languages that Tachyon supports:

**In each language, what is the most used framework, and what is the most used
design pattern?**

The goal is that future decisions about parity (matching features across
languages) rest on evidence. They should not depend on which ecosystem the
author happens to know.

Nothing here is a commitment. It is a survey with a recommendation at the end.

## Method and its limits

This document measures popularity in three ways, because each measure alone is
misleading:

1. **Stack Overflow Developer Survey 2025** (about 49,000 respondents). It is
   the only source across languages that asks people what they *use*. The
   percentages below are "all respondents". So they are shares of all
   developers, not of the developers of that language. A language with fewer
   developers gets lower numbers.
2. **Registry download counts** from `crates.io`, `npm`, `PyPI` and
   `Packagist`, read in September 2026. These count machines, not people. CI
   downloads a package again and again. Indirect (transitive) dependencies
   also raise the count of a library that many other libraries depend on.
3. **Vendor direction**: what Apple, Google, JetBrains and Microsoft ship and
   recommend. Vendor direction usually appears in the surveys one or two years
   later. For a framework that is a platform default, it matters more than the
   other two measures.

When the three measures disagree, this document states the disagreement and
does not try to resolve it.

No registry measures design patterns. So the pattern choices rest on
documentation, framework design and how much the community writes about them.
They are the less certain half of this document. The document marks them when
people disagree about them.

## Summary

| Language | Dominant framework | Evidence | Dominant pattern | Declarative surface |
| --- | --- | --- | --- | --- |
| Java | **Spring Boot** | 14.7% of all respondents | Dependency injection / IoC | Annotations — *Tachyon's model* |
| C# | **ASP.NET Core** | 19.7%, plus 14.2% on legacy ASP.NET | DI (built in) + Mediator/CQRS | Attributes |
| TypeScript | **Angular** (front) / **NestJS** (server) | 18.2% / 6.7% | DI with decorators | Decorators — closest analogue to Spring |
| JavaScript | **Express** on **Node.js** | 48.7% / 19.9% | Middleware chain | None; plain functions |
| Python | **FastAPI** (rising) / **Django** (complete) | 14.8% / 12.6% | `Depends()` DI / MTV + Active Record | Type hints and default values |
| PHP | **Laravel** | 8.9%; 15.2M monthly Packagist | Facade over a service container; Active Record | Attributes (newer), container binding |
| Kotlin | **Jetpack Compose** (Android) / **Spring Boot** or **Ktor** (server) | Platform default; contested on server | MVVM + Repository | Compose: none. Hilt: annotations |
| Rust | **Axum** | 117M/90d vs Actix Web's 10.6M | Newtype, typestate, extractors | None; the type system *is* the surface |
| Swift | **SwiftUI** (client) / **Vapor** (server) | Platform default | MVVM (MVVM-C at scale) | Property wrappers and macros |
| Dart *(proposed)* | **Flutter** (client) / **Shelf** (server substrate) | 9.4% in 2024, ahead of React Native's 8.4% | MVVM + Repository (Flutter's own guide); BLoC / Riverpod for state | Annotations, all compile-time via `build_runner` |

Terms used in this table and below:

- DI: dependency injection. A framework creates objects and passes them their
  dependencies.
- IoC: inversion of control. The framework, not your code, creates and connects
  the objects.
- CQRS: command query responsibility segregation. Commands (writes) and
  queries (reads) are separate objects.
- MVVM: model–view–viewmodel. MTV: model–template–view. MVC:
  model–view–controller.
- Declarative surface: how a developer declares framework behavior, for
  example with annotations or decorators.

The most useful finding: **dependency injection is the most used pattern in
five of the nine ecosystems**: Java, C#, TypeScript, Python (through FastAPI)
and PHP. Two more ecosystems offer it but do not usually use it: Kotlin through
Hilt, and Swift through Factory. Only Rust rejects it completely. Rust rejects
it for one reason, the lack of reflection, and Tachyon already handles that
limit.

So Tachyon's `@Autowired` / `@Qualifier` / `@Service` names are not a Java idea
forced on eight other languages. They are what most ecosystems already do,
written with Java's names.

## Java — Spring Boot

**Framework.** Spring Boot, 14.7%. Tachyon already follows it.

**Pattern.** Dependency injection through an IoC container. Repository, MVC and
AOP-style (aspect-oriented programming) cross-cutting concerns are built on top
of it. Each of these has a Tachyon counterpart today.

## C# — ASP.NET Core

**Framework.** ASP.NET Core, 19.7%. This is about equal to Express (19.9%) at
the top of the server-framework list, and well ahead of Spring Boot's 14.7%.
Legacy ASP.NET adds another 14.2%. So the .NET server stack as a whole is the
most used one in the survey.

**Pattern.** In ASP.NET Core, DI is not an extra pattern that you add. It is
built into the host, and everything else is built on top of it. So the default
C# setup is the closest to Spring's. Two conventions sit above DI:

- **Repository + Unit of Work** over EF Core. This is common. But many .NET
  developers argue against it: EF Core's `DbContext` already *is* a unit of
  work and a repository, so wrapping it again adds nothing.
- **Mediator/CQRS**, almost always through MediatR. MediatR registers handlers
  by scanning the assembly. Commands and queries are separate. Cross-cutting
  concerns (logging, validation, authorization) are written once as pipeline
  behaviors instead of being repeated across services.

**Tachyon's position.** Tachyon covers the DI half. It does not cover the
Mediator half. This is the most interesting gap in this document; see the
recommendation.

## TypeScript — Angular and NestJS

**Framework.** In TypeScript, the answer depends on the tier:

- Angular (18.2%) is the most used TypeScript-first front-end framework.
- NestJS (6.7%) is the most used TypeScript-first server framework.

`@nestjs/core` has 12.9M npm downloads a week. That is about ten times fewer
than Express's 121M. But Express is not really a TypeScript framework.

**Pattern.** Both frameworks use DI with decorators, and NestJS openly copies
Angular's design:

- `@Injectable()` marks a provider.
- `@Module()` limits visibility.
- `@Controller()` binds routes.
- `@Inject()` names a token when the field's type cannot supply one.

These are `@Service`, `@Autowired` and `@Qualifier` with different names.

So TypeScript is the **strongest independent confirmation of Tachyon's
design**. The most popular TypeScript server framework reached Spring's shape
without being Java.

Its most common criticism also teaches something. NestJS needs a lot of
decorator code. Each endpoint needs a separate controller, service, DTO (data
transfer object) and module file. New developers must learn a module graph.
Tachyon's role-folder layout (ADR 0031) removes the module graph. Tachyon has
the same cost in file count.

## JavaScript — Express on Node.js

**Framework.** Node.js 48.7%, Express 19.9%, 121M npm downloads a week.
Next.js (20.8%) is now ahead of Express in the survey. But Next.js is a
full-stack React framework, not a server framework.

**Pattern.** The **middleware chain**. This is the chain of responsibility
pattern: a mutable request object passes through functions, and each one calls
`next()` to continue. There is no DI, no container and no declarative
vocabulary. Code connects its parts with `require`/`import` and closures.

**Tachyon's position.** Tachyon's JS/TS realms use decorators. So they look
like NestJS, not like Express. That is the right choice, because Express has no
declarative surface to match. But note this clearly: a developer who knows only
Express will find Tachyon's annotations new. A NestJS or Angular developer will
find them familiar.

## Python — FastAPI and Django

**Framework.** People disagree here, and this row changes the fastest. In the
survey, FastAPI (14.8%) is now ahead of Django (12.6%) and Flask (14.4%). Stack
Overflow named FastAPI's gain of 5 points in one year as the largest change
among web frameworks. Django is still the more complete framework. PyPI shows
about 398M FastAPI downloads a month.

**Pattern.** The two frameworks give different answers, so the row is split:

- **FastAPI: dependency injection**, through `Depends()`. A dependency is a
  callable declared as the default value of a parameter. FastAPI resolves it
  and caches it for each request. Database sessions, authentication and
  settings all arrive this way. This is DI without a container and without
  annotations. The *function signature is where you declare dependencies*.
- **Django: MTV with an Active Record ORM** (object-relational mapper). Each
  model stores and loads itself. There is no repository layer and no DI
  container. Projects that want one use `dependency-injector` or `injector`.

**Tachyon's position.** Tachyon's Python realm uses `@Autowired` on fields. It
also accepts snake_case aliases (`@required_args_constructor`,
`@http_exchange`). Compared with FastAPI, this is the same pattern, declared in
a different place. Compared with Django, it is a different architecture:
Tachyon's `@Repository` is Data Mapper, not Active Record.

## PHP — Laravel

**Framework.** Laravel, clearly. It has 8.9% of all respondents; PHP as a whole
is declining, which lowers this number. `laravel/framework` has 15.2M
Packagist installs a month.

The obvious comparison, `symfony/symfony` at 273k a month, is **not** fair.
Developers install Symfony as separate components (`symfony/console`,
`symfony/http-foundation`), and Laravel itself uses several of them. Laravel
leads for applications. Symfony leads for components, including inside
Laravel.

**Pattern.** Three patterns, one on top of the other:

- **Service Container (IoC).** The core. It creates classes and their
  dependencies. It can bind a service as a singleton that lives as long as the
  process.
- **Facade.** Laravel's best-known pattern. A facade looks like a static call
  (`Cache::get()`), but it is a proxy that resolves the service through the
  container with `__callStatic`. It is short to write, and Laravel developers
  name it first.
- **Eloquent = Active Record.** Each model is a table and each instance is a
  row. Business logic and storage live in one class. In Laravel, a Repository
  is clearly optional. In .NET, developers expect one.

**Tachyon's position.** The container half matches. The Facade half is the
strongest conflict of design ideas in this document. A facade is a service
locator for the whole process: it is mutable, and code reaches it through a
static call. Tachyon's contexts are the opposite by design: each has an
explicit owner, is scoped to a partition, and cannot replace another. A Tachyon
facade would remove that property on purpose. Tachyon should decline it in
writing.

## Kotlin — Compose, and a contested server

**Framework.** On Android: **Jetpack Compose**. It is the platform default, and
few people dispute it. Declarative UI is the main approach, and Compose is
designed for Kotlin.

On the server, the answer is really split:

- **Spring Boot** has the mature ecosystem and full Kotlin support since
  Boot 3 (Kotlin bean DSL, `suspend` controllers).
- **Ktor** is JetBrains' own framework. It is built on coroutines, made of
  plugins, starts cold in under a second, and says openly that it has "no
  magic".

Neither one is the clear leader. Spring Boot wins on ecosystem; Ktor fits
Kotlin's style better. Kotlin Multiplatform is where things are going. It has
been stable since 2023, and Google has supported it since I/O 2024.

**Pattern.** **MVVM with unidirectional data flow**, as Google itself
recommends:

- State lives in the ViewModel.
- `StateFlow` sends state to the UI.
- Events handle one-time actions.

This sits on a **Repository** layer. **Hilt** (based on annotations, resolved
at compile time) is the usual DI choice in production. Screens with complex
state move up to MVI (model–view–intent). Clean Architecture is a way to
arrange layers under either one. Many people note that it is too much for
small apps.

**Tachyon's position.** Kotlin is a Tac companion (Android) and a Yon
language. Tachyon's annotations use the same names as Hilt and Spring, so the
DI half fits. MVVM does not map onto Tachyon at all, because Tachyon's Tac
realm has no ViewModel tier: a view addresses a service directly (ADR 0030).
This is a deliberate difference, not a gap.

## Rust — Axum

**Framework.** **Axum**, clearly:

- 117.0M downloads in the last 90 days, against Actix Web's 10.6M. That is 11
  times more.
- 474M against 82M downloads in total.

Actix Web is still 10–15% faster under load, and keeping it is not a mistake.
Axum is the default for new work for two reasons. The Tokio team maintains it.
It also works with `tower`, which is where most of Rust's middleware lives.

**Pattern.** Rust is the one ecosystem that does not treat DI as a framework
task. The reason is the same one that Tachyon already respects: **Rust has no
reflection**. Four idioms do the work instead:

- **Newtype.** A tuple struct with one field creates a separate type that costs
  nothing at run time. For example, code cannot pass a `UserId` where a
  `ProductId` is expected. Combined with the rule "parse, don't validate",
  people describe it as high value for low cost.
- **Typestate.** Each state of a protocol is a separate type, so an illegal
  sequence does not compile. The standard example is a resource protocol: you
  cannot return a connection to the pool before you commit or roll back,
  because that method does not exist in that state.
- **Builder, often with typestate.** It is as strict as a builder that
  validates, but the check moves from run time to compile time.
- **Extractors.** Axum's replacement for DI. A handler takes any number of
  arguments that implement `FromRequest`/`FromRequestParts`. Trait resolution
  alone resolves them. Shared resources live in one `AppState`. The community
  says this directly: DI "is a term more popular in OOP and not that commonly
  used in Rust", and the extractor pattern replaces it. A container crate
  (`shaku`) exists. Developers use it only when there are too many
  combinations of state structs.

**Tachyon's position.** Tachyon's Rust realm uses `#[Autowired()]` with
`Arc<Provider>` fields. That is DI, which Rust's own ecosystem does not use. It
works. The alternative, handler arguments in the style of extractors, would
split Tachyon's handler contract for one language.

But this is the one row where Tachyon goes against its host ecosystem. The
docs should record that clearly, not hide it.

Note one alignment that happened by chance. Tachyon requires `test*` names for
test cases and a list of cases produced by the compiler, *because* Rust has no
reflection. The same limit produced Rust's own patterns.

## Swift — SwiftUI and Vapor

**Framework.** On the client: **SwiftUI**. In 2026 the question about UIKit is
not whether it survives, but whether Apple still invests in it. New features
appear in SwiftUI first.

On the server: **Vapor**. It is a member of the Swift Server Workgroup. It
offers async IO, JWT/basic/bearer authentication, Redis caching and DI.

**Pattern.** **MVVM**. SwiftUI was designed around it. Mature apps move to
**MVVM-C** (MVVM with coordinators) at about five screens.

The tools just changed to a new generation:

- The Combine-era stack (`ObservableObject` + `@Published` + `@StateObject`)
  is being replaced by the Observation framework. There, a view model is an
  `@Observable` class held in `@State`.
- The leading DI library changed from Swinject to **Factory**. Factory's
  `@Injected` property wrapper is built on Swift Macros.

**Tachyon's position.** Swift is Tac-only (macOS/iOS). Tachyon's `@Autowired`
works like Factory's `@Injected` and like Hilt's annotation. Swift's ecosystem
arrived at the same place: injection at the property level through a
declarative marker. The MVVM note is the same as for Kotlin: Tac has no
ViewModel tier, by design.

## Dart — Flutter *(tenth language: Yon shipped, Tac web pending)*

**Framework.** **Flutter**. It is the reason most developers write Dart at all.
Stack Overflow's 2024 survey put Flutter at 9.4% of all respondents, ahead of
React Native's 8.4%. The 2025 survey removed that question, so 2024 is the
latest clean comparison. Dart itself was near 6%.

Server-side Dart is small. `shelf` shows 9.6M downloads a month on pub.dev.
But most of those come from the SDK's own tools, which depend on it
indirectly. The application frameworks built on it are about a hundred times
smaller: Dart Frog (110k a month) and Serverpod (89k a month).

**Pattern.** Flutter's own architecture guide recommends **MVVM over a data
layer of repositories and services**. This is vendor direction, so it settles
the question more than community debate does.

State management is where the community splits. pub.dev shows three strong
options and no winner:

- Riverpod has the most downloads (3.2M a month).
- Provider has the most likes (11,004).
- flutter_bloc (1.9M a month, 8,077 likes) is the one that enterprise teams
  name.

For dependency injection, developers use a service locator, **get_it** (2.0M a
month). **injectable** (330k a month) generates its registrations from
annotations.

**Declarative surface — the decisive fact.** Flutter builds are AOT
(ahead-of-time compiled) and ship without `dart:mirrors`. So no Dart annotation
does anything at run time. Every annotation library in the ecosystem is a
*code generator* that `build_runner` runs:

- `json_serializable` and `freezed` (3.6M and 4.0M a month) for data classes;
- `injectable` for DI;
- `retrofit` for HTTP clients;
- `mockito`'s `@GenerateMocks` for test doubles.

That is exactly Tachyon's model: a build step reads the annotations and turns
them into plain code. It is also the reason Tachyon's approach works for Rust.
Dart developers already expect an annotation to be an instruction at build
time, not a hook at run time.

The names map closely. Tachyon's spelling with parentheses is already normal
Dart, where an annotation is a call to a const constructor:

| Dart ecosystem | Tachyon |
| --- | --- |
| injectable `@injectable`, `@singleton`, `@lazySingleton` | `@Service()`, `@Scope(...)` |
| injectable `@Named('x')` | `@Qualifier("x")` |
| injectable `@Environment('dev')` | `@Profile("dev")` |
| retrofit `@RestApi()` + `@GET`/`@POST`, `@Path`, `@Query`, `@Body` | `@Client()` + `@HttpExchange(...)` |
| floor `@Entity`, `@dao`, `@Query` | `@Entity`, `@Repository`, `@Query` |
| json_serializable / freezed | partly `@Mapper` / `@Mapping`; no equivalent for the rest |
| mockito `@GenerateMocks` | `@MockBean` |

**Where it would run — the open question.** Every other companion has a clear
native host:

- Kotlin links into Android.
- Swift links into Apple's platforms.
- C# links into Windows.
- Rust links into all of them through the C ABI (application binary
  interface).

Dart has no such host in Tachyon. Flutter is its own engine, not something that
a WebView host can embed. So Dart fits in these places:

1. **Yon**, as a handler language next to Java and Python, with `dart run` or
   `dart compile exe`. It reuses the handler supervisor. The toolchain is
   already on the ANNEX volume (Dart 3.13.1).
2. **Tac on the web**, by compiling a browser service with `dart compile js` or
   `dart compile wasm`. `env.sh` and `migrate.rs` show that a Dart-to-wasm
   companion existed once and was retired. So this would bring back an old
   design. Read its history first.
3. Tac native targets do not fit well, because there is no host to link into.

**Status (2026-09-23).** Option 1 shipped. Dart is a Yon language with the full
annotation vocabulary, including streaming. Handlers compile with
`dart compile aot-snapshot` and run under `dartaotruntime`, not with
`dart compile exe`. The reason: macOS checks every newly linked executable the
first time it starts, while a snapshot is data that one trusted runtime runs.

Option 2 shipped too, with `dart compile js` instead of wasm. A Dart browser
service compiles to a JavaScript module behind a generated bridge (ADR 0033).
On 2026-09-28 the program moved to `dart compile wasm` (WasmGC). Each service
keeps its JavaScript module and bridge (ADR 0033 amendment).

## What this means for Tachyon

**Confirmations.** Four independent ecosystems arrive at annotated or declared
dependency injection:

- Java/Spring;
- C#/ASP.NET Core;
- TypeScript/NestJS-Angular;
- Python/FastAPI.

Two more adopt it through their leading DI library: Kotlin/Hilt and
Swift/Factory. So the names Tachyon chose are what most ecosystems use, not
something only Java does. `@Repository` as Data Mapper matches .NET and
Spring. It differs from Django and Laravel, which both use Active Record.

**The one real gap: Mediator/CQRS.** Reasons it matters:

- It is the most used *architectural* pattern above DI in modern .NET.
- It has a Spring counterpart that Tachyon's current users know
  (`ApplicationEventPublisher`, which Tachyon already has).
- Its main benefit is that cross-cutting concerns run once in a pipeline
  instead of being repeated across services. That fits a compiler that already
  writes method wrappers for `@Cacheable`, `@Retryable`, `@Transactional` and
  `@ConcurrencyLimit`.

Tachyon's existing event vocabulary covers the notification half. The
request/response half (one command, one handler, a typed result) does not
exist. If anything on this list is worth building, it is this.

**Two to decline in writing.**

- **Facade** (Laravel). A service locator for the whole process, reached
  through a static call, is the direct opposite of Tachyon's contexts. Those
  have an explicit owner and are scoped to a partition.
- **Active Record** (Django, Laravel). Tachyon's `@Entity`/`@Repository` split
  is Data Mapper. A model that stores itself cannot respect the partition and
  authorization boundary that a Tachyon query goes through.

**One to record as a known divergence.** Rust's ecosystem answers DI with
extractors and the type system, not with injected fields. Tachyon uses injected
fields in Rust so that all languages work the same way. The trade-off is
reasonable: one handler contract across nine languages. But it is still a
trade-off, and the docs should record the reason. It should not live only in
someone's memory.

**One non-goal.** MVVM is the most used client pattern in both mobile
ecosystems that Tachyon targets (Kotlin/Android, Swift/Apple). Tachyon's Tac
realm has no ViewModel tier on purpose: a view addresses a service (ADR 0030).
This is a design decision, not something missing. Say so wherever Tac is
compared with native app architecture.

## Sources

- [Stack Overflow Developer Survey 2025 — Technology](https://survey.stackoverflow.co/2025/technology)
- [crates.io — axum](https://crates.io/crates/axum) · [actix-web](https://crates.io/crates/actix-web)
- [npm registry download counts](https://api.npmjs.org/downloads/point/last-week/express) · [PyPI Stats](https://pypistats.org/) · [Packagist](https://packagist.org/packages/laravel/framework)
- [Axum vs Actix-web 2026](https://rustify.rs/articles/rust-axum-vs-actix-web-2026) · [Rust Design Patterns — Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) · [Microsoft Rust Training — Newtype and Typestate](https://microsoft.github.io/RustTraining/rust-patterns-book/ch03-the-newtype-and-type-state-patterns.html) · [axum::extract](https://docs.rs/axum/latest/axum/extract/index.html) · [DI strategies in Axum and Actix Web](https://leapcell.io/blog/dependency-injection-strategies-in-axum-and-actix-web)
- [NestJS custom providers](https://docs.nestjs.com/fundamentals/custom-providers) · [FastAPI dependencies](https://fastapi.tiangolo.com/tutorial/dependencies/)
- [Laravel facades](https://laravel.com/docs/13.x/facades) · [Active Record in Laravel](https://dev.to/diego_chavez_dc/active-record-laravel-design-pattern-524c)
- [CQRS with MediatR in ASP.NET Core](https://codewithmukesh.com/blog/cqrs-and-mediatr-in-aspnet-core/) · [MediatR and CQRS](https://code-maze.com/cqrs-mediatr-in-aspnet-core/)
- [Kotlin Multiplatform cross-platform frameworks](https://kotlinlang.org/docs/multiplatform/cross-platform-frameworks.html) · [Spring Boot vs Ktor](https://www.dhiwise.com/post/kotlin-spring-boot-vs-ktor-which-one-should-you-choose) · [Android architecture patterns in 2026](https://singhangad.in/blog/android-architecture-patterns/)
- [pub.dev package scores](https://pub.dev/packages/get_it/score) · [Flutter vs React Native statistics](https://quashbugs.com/blog/flutter-vs-react-native-statistics)
- [MVVM in SwiftUI](https://www.avanderlee.com/swiftui/mvvm-architectural-coding-pattern-to-structure-views/) · [iOS app architecture in 2026](https://www.forasoft.com/blog/article/advanced-ios-app-architecture-explained-on-mvvm-977) · [Vapor](https://en.wikipedia.org/wiki/Vapor_(web_framework))
