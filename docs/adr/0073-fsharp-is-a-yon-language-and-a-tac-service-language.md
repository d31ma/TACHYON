# ADR 0073: F# Is a Yon Language and a Tac Service Language

- Status: Accepted
- Date: 2026-09-28 (behavior annotations, events, lifecycle hooks,
  `NeedsAll`, sockets, streams, executed coverage and the test context:
  2026-09-29; Needs properties: 2026-09-29; browser APIs: 2026-09-30)
- Builds on: [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md)
  (the .NET browser runtime), [ADR 0048](0048-yon-routes-come-from-annotations.md)
  (routes), [ADR 0062](0062-yon-uses-native-http-types.md) (native HTTP
  types), [ADR 0044](0044-an-annotated-file-holds-one-class.md) (one class
  per file) and [ADR 0056](0056-every-annotation-checks-its-target.md) (the
  shared rule table)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (the Yon languages)
  and [ADR 0045](0045-go-is-a-service-language.md) (which service languages
  reach the web as WebAssembly)

## Context

C# already runs in both realms on .NET: Yon compiles a C# handler with the
.NET SDK, and Tac runs C# services on the .NET browser runtime. F# is the
other main .NET language. F# has real attributes (`[<Service>]`), a type
system and primary constructors, so it can use Tachyon's annotations as they
are. Writing new parsers, guards and runtimes for F# would repeat most of the
C# work.

## Decision

### Annotations and types

- An F# annotation is an F# attribute, `[<Name>]` or `[<Name(arguments)>]`,
  alone on its line above the type or member. Several attributes may share
  one pair of brackets: `[<RequestRoute("/orders"); Public>]`. Parameter
  attributes stay inside the parameter list: `([<FromBody>] order: Order)`.
  The build removes Tachyon's attributes before F# compiles the file, as it
  does for C#. Other attributes stay.
- A type takes the roles it needs as the parameters of its primary
  constructor: `type OrderService(orderStore: OrderStore) =`. This is
  Spring's single-constructor injection, so F# needs no `Needs` attribute.
  `[<NeedsAll>]` is allowed: in F# the primary constructor already is the
  constructor of every required dependency, so the attribute states what the
  constructor does and changes nothing. `NeedsAll` with a list of fields is
  refused by name. Field injection uses a Needs property (see below).
- Route parameters and return values use the written F# types. Members that
  Tachyon reads (routes, service members, test cases) write their types:
  `member this.GetOrder(id: int) : Order =`. `int`, `int64`, `float`,
  `string`, `bool`, arrays, `list`, `option`, `ResizeArray` and models map to
  the shared route types. System.Text.Json reads and writes F# records,
  lists and options.
- A model in `server/models/` or `client/models/` is an F# record or an enum
  union, one type in each file (ADR 0067). Tachyon places every model in the
  `Models` namespace, so code writes `open Models`.
- An F# file holds its type and nothing else at file level. It has no
  `module` or `namespace` line (Tachyon places every file in one recursive
  namespace, so file order does not matter), and no file-level `let` or `do`,
  which would be a file-level variable. These forms are TY1335.

### How Tachyon reads F#

- Tachyon reads an F# file through a C# view of it: the same types,
  attributes and member signatures, written as C# declarations, with the
  member bodies copied across (`match` reads as `switch`, `elif` as
  `else if`). Every C# rule and guard then runs on F# unchanged: the rule
  table, one class per file, file names, cohesion, duplication, the design
  rules, the safety rules and the route and model readers. Each diagnostic
  points back at the line of the F# file.

### Yon

- F# (`.fs`) is a Yon language: controllers, services and the other roles,
  models, error handlers, root middleware (`middleware.fs`), plain handlers
  (`type Handler` with static members named after HTTP methods) and test
  units.
- A Yon program is three .NET assemblies built by one `dotnet build`: the C#
  protocol runtime of the C# handlers as a library, the author's F# code and
  models as an F# library, and the generated C# container and route code as
  the program. The C# library lets the F# library use its types, so F# code
  reaches `relay`, `DataSource`, `RedisTemplate` and the native HTTP types
  (`HttpRequestMessage`, `HttpResponseMessage`) that C# uses. The build is
  cached like a C# build.
- A test unit is a `server/tests/<Name>Test.fs` type with `[<Test(typeof<
  Setup>)>]`. Each member whose name starts with `test` is a case and returns
  `unit`. The generated driver creates the type for each case and reports it
  to `ty test`.

### Tac

- F# is a Tac service language on the web. `client/services/<Name>Service.fs`
  compiles into an F# library that the .NET browser project of ADR 0037
  references, so a page with C# and F# services loads the .NET runtime once.
  The page reaches the service through the same generated C# member table as
  a C# service. No native host links F#, so every target, Windows included,
  runs F# services in its web view.
- A service's page members are its public properties (`member val Count = 6
  with get, set`, writable with `set`) and its public methods. The value of a
  `member val` may continue on the lines below it, with `with get, set` at
  its end.
- Client test units compile the service and the test for .NET on the build
  machine.
- `ty doctor` reports the .NET SDK for F# handlers, and the .NET WebAssembly
  workload for F# services. `ty init` offers F# for both realms.

### Behavior annotations (added 2026-09-29)

F# classes hold the shared behavior annotations with their Spring meaning:
CacheSettings, Cached, UpdateCache, ClearCache, Retry, MaxAtOnce,
AllOrNothing, Check and Checked, PageDefaults, Entity, Table, Store and
Query, Converter and Convert, Settings, Setting, WhenProfile, WhenSetting,
HTTPClient with its request annotations, Gateway and Schedule.

- Tachyon runs the C# behavior lowering on the C# view of the F# file, so
  F# gets the same checks, the same errors and the same runtime as C#.
- The C# behavior runtime is compiled into a C# library that the F#
  assembly references: the Yon prelude library, or a `FsharpBehaviors`
  project next to the F# services in the WebAssembly build. That library
  cannot see the F# types, so it compares types by name, and a generated
  module initializer in the C# glue (which can see them) registers each
  Setup's `configure` method. In Tac the same initializer connects the page
  storage.
- A member with a body keeps its code: Tachyon renames it to
  `__tachyon_<name>` and adds the real member in an intrinsic type extension
  (`type X with ...`) at the end of the file. The extension calls the
  behavior context and then the renamed member. Line numbers of the author's
  code do not change.
- A member whose body Tachyon writes (Query and derived `findBy` queries,
  Convert, Gateway, request annotations without a body) is an F# `abstract`
  member. In an interface-shaped type the type gets a constructor that takes
  the `BehaviorContext` (or no parameters for gateway-only clients); in a
  class the class keeps its own `behaviors` member. Tachyon replaces each
  abstract line with a concrete member on the same line.
- The behavior context is a `behaviors` member of the class, usually
  `member val behaviors : BehaviorContext = Behaviors.create(...)`, which
  runs once for each new instance (an `OnStart` hook may open it instead).
  Methods return `Task<BehaviorJSON>`; an F#
  lambda for `authorize` is written as `Func<_, _, _>(fun ...)`.

### Events and lifecycle hooks (added 2026-09-29)

F# classes hold `OnEvent` (with `Order`, `InBackground`, conditions, ids,
replay and a returned model that is published again), `OnStart` and
`OnStop`, with the Spring meaning of ADR 0031.

- A class takes its `ApplicationEventPublisher` as a primary constructor
  parameter, in both realms: `type CheckoutService(applicationEventPublisher:
  ApplicationEventPublisher) =`. It publishes a model record with
  `applicationEventPublisher.PublishEvent(order)`.
- A listener takes one model record and returns `unit` or a model record.
  Hooks and listeners are synchronous, as in C#. `InBackground` is allowed
  in both realms: in Tac it only delays delivery, and in Yon (amended
  2026-10-17) the C# event runtime calls it after the other listeners of the
  event and never publishes its returned value again.
- Yon: Tachyon runs the C# event lowering on the C# view of the F# file. The
  generated event container stays in the C# program; the C# event runtime
  moves into the C# library that the F# assembly references, so F# code and
  the container share one publisher. The .NET realm reads the models of both
  C# and F# files, so conditions and listener shapes check F# records.
- Tac: the listener and hook entries go into the generated C# member table,
  as for a C# service. The F# assembly gets its own small
  `ApplicationEventPublisher` next to the behavior runtime; a module
  initializer in the C# glue connects it to the page event bus. The C#
  member table passes a new publisher to the F# constructor. A Tac F#
  service takes no other constructor parameter. F# records join the .NET
  JSON model context, so listeners read and publish them.
- The Yon library makes `DataSource` and `RedisTemplate` public for F#, so
  an F# service takes them as constructor parameters.

### Sockets and streams (added 2026-09-29)

F# controllers hold `SocketRoute` with `OnOpen`, `OnMessage`, `OnClose` and
`OnError` (ADR 0060), and `StreamRoute` (ADR 0058), with the same meaning as
in C#.

- Tachyon runs the C# socket and route lowering on the C# view of the F#
  file, so the C# glue calls the F# members and the C# prelude runs the
  socket or the stream.
- A socket member takes the prelude's `SocketSession` (and an `exn` for
  `OnError`) and returns `unit`. `session.OpenSessions()` returns a
  `ResizeArray<SocketPeer>`. The message is an F# record from
  `server/models/`, and Tachyon checks each message against it.
- A stream member returns `seq<'T>` or `IAsyncEnumerable<'T>`. A `seq { }`
  expression is the F# form of a C# iterator: the C# prelude recognises the
  class that the F# compiler writes for it (a `GeneratedSequenceBase`) and
  sends each value as one server-sent event.

### Executed coverage (added 2026-09-29)

F# classes follow ADR 0070: a public member of an F# role file or a public
static member of an F# utility counts only when a test case called it.

- Tachyon finds the counted members on the C# view of the F# file, so F# and
  C# count the same members, and maps each one back to its F# line.
- In a test build Tachyon adds one line at the start of each counted member:
  `BehaviorCalls.Record("Type.Member")`, at the column of the member body.
  The next line is an F# line directive (`# 12 "server/services/A.fs"`), so
  the author's code keeps its file and line numbers in compiler messages. A
  body on the same line as `=` moves to the next line at the same column.
- Composed F# code starts each role file with a line directive too, and a
  moved `open` line leaves an empty line, so every line of a composed test
  unit keeps its own file and line number.
- The recorder is the one in the C# behavior runtime. Every F# test build
  now compiles that runtime into a C# library that the F# assembly
  references: the Yon prelude library, or a `behaviors` library next to the
  Tac test unit. The test driver clears the recorder before each case and
  reports the recorded names with the case.

### Behavior context in test units (added 2026-09-29)

An F# test unit has a behavior context in each case, as a C# unit has.

- The `TestSetup` class declares
  `static member configure(execution: BehaviorExecution) : BehaviorAdapters`,
  as in C#; `type CatalogTestSetup() = class end` is no longer enough.
- `Test(typeof<CatalogTestSetup>, {"partition":"tests"})` takes the same
  options as in C#. Before each case the driver opens a context with them and
  closes it after the case.
- The case reads the context as `this.behaviors`. Tachyon adds this member
  to the test class in an intrinsic type extension.
- Role files composed into the unit keep their behavior annotations: Tachyon
  lowers them as it does for a route.

### Needs properties (added 2026-09-29)

A settable property with a written type receives a role like a Spring field:
`[<Needs>] member val OrderStore : OrderStore =
Unchecked.defaultof<OrderStore> with get, set`, with an optional
`[<Named("FastStore")>]` that chooses another provider. Constructor
parameters stay the recommended form.

- The C# view reads each Needs property as one more primary constructor
  parameter (its type, or the Named provider), so the provider graph, the
  scope checks and the Tac publisher check see it as they see a constructor
  parameter. The view drops the Needs and Named lines.
- The F# code keeps the author's constructor. The generated C# container (in
  Yon) and member table (in Tac) create the type with its constructor
  parameters and then set each property: `new StockService() { Names =
  resolve_1() }`. So a test sets it the same way: `StockService(Names =
  this.Names)`, where the test unit declares `[<Fake>] member val Names :
  NameService = ... with get, set`.
- A Tac F# service still takes only its `ApplicationEventPublisher`, as a
  constructor parameter or as a Needs property.
- `TY2015` refuses `[<Needs>]` on anything but a settable `member val` with a
  written type, and a type that takes the same role twice: as a constructor
  parameter and as a Needs property, or as two Needs properties.

### Browser APIs (added 2026-09-30)

An F# Tac service uses Tachyon's browser APIs (ADR 0065) and
`WindowControls` (ADR 0087) under the same names as a C# service:
`let clock = Clock()` in the type, then `clock.NowMs()`.

- The C# classes of the browser APIs and their `TacWebHost` move into the C#
  library below the F# library (`FsharpBehaviors`, now written for every page
  with F# services). The C# browser project finds them there through its
  reference, so a page with C# and F# services still has one `TacWebHost`,
  and the page answers both languages through it. A page without F# services
  keeps the classes in its own C# source, as before.
- The built-in browser events (`ConnectivityChanged`, `PositionFound`, …) are
  F# records in the global namespace, compiled before the services:
  `type ConnectivityChanged = { Online: bool }`. `ShareFinished` has no field,
  so it is a class: `type ShareFinished() = class end`. On such a page, C#
  services read the same records; an F# record has the same constructor and
  properties as the C# record it replaces.
- A method without a written return type (`member this.Forget() =`) may
  return `unit`. The view cannot know the type, so the member table calls it
  through `TacMember.Result`, which accepts a method that returns a value and
  one that returns nothing. A method with a written type keeps its direct
  call.
- An F# test unit cannot use the browser APIs yet, like a C# test unit: its
  test program has no page clients. Test such a service in the browser.

### Not yet available in F#

`NeedsAll` with a list of fields is refused with a message that names it, so
F# never ignores an annotation.

## Consequences

- F# joins the multi-language examples whose features it supports.
- F# and C# share one toolchain, one browser runtime and one set of checks.
  A change to the C# rules applies to F# as well.
- The C# view is a reading aid, not a translation: F# code is compiled by
  the F# compiler, so a body that the view reads approximately (for example
  a pipeline with `|>`) is still compiled exactly.
- Tachyon's error messages sometimes use a C# word for an F# construct; the
  view renames `switch` to `match` and `foreach` to `for` in the messages.

## Amendment (2026-09-30): a Tac service takes its properties

- A Tac F# service with `[<NeedsAll>]` takes its component's properties as
  primary constructor parameters, as in
  `type SnippetService(files: string list) =`. The member table builds the
  type with the properties of the start-up request, so the service exists
  once per component. Without `[<NeedsAll>]`, and for a parameter typed as
  another service, the constructor still takes only its
  `ApplicationEventPublisher` (TY1405).

## Amendment (2026-10-16): a Tac service takes its HTTP clients

- A Tac F# service may take an HTTPClient from `client/remotes` through a
  settable property with `[<Needs>]` above it, as in
  `[<Needs>] member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set`.
  On the page, the member table sets the property to `GreetingClient()`
  after the constructor runs, as for the C# private readonly field. The
  page does not see the property. A service takes at most five clients;
  any other Needs type is still refused with TY1405.
- An F# Tac HTTPClient has two constructors: `GreetingClient()` builds its
  context from its Setup, and `GreetingClient(context)` uses a given
  BehaviorContext.
- In an F# Tac test unit, `this.construct<CounterService>()` builds a service
  and sets each of its `Needs` properties from the `[<Fake>]` property of
  the test with the same name and type. While a Fake client property is
  empty, the service gets a client bound to the test's BehaviorContext, so
  the TestSetup adapter answers. Any other empty Fake, or a Needs property
  without a matching Fake, fails the case with a message that names it.
- A Helper constructed only by a selected F# TestSetup is checked against
  that TestSetup and left out of the production build, as in C#.

## Amendment (2026-10-18): caching, policies, stores and page state have runtime proof

- An F# Tac test unit, run on host .NET by
  `crates/tachyon-core/tests/it/tac_fsharp_behaviors.rs`, composes
  `client/setups`, `client/entities`, `client/stores` and `client/services`
  and runs `Cached` with `UpdateCache` and `ClearCache`, `CacheKey`,
  `Retry` (success after failed attempts, and the error after
  `maxRetries`), `MaxAtOnce` with `reject` (`OPERATION_LIMIT`),
  `AllOrNothing` (commit, and rollback when the body raises, with the
  Store's query joining the open transaction), a derived query and a
  `[<Query>]` Store member, and `PageDefaults` (normalized page,
  `PAGE_INVALID`).
- The F# Journal service in `scripts/storage-browser-test.mjs` has a
  `[<SessionState>]` member: it survives a reload in the same tab and is gone
  in a new tab.
- These markers already worked in F# through the C# lowering; this only adds
  the proof. The rule table does not change.

## Amendment (2026-10-18): Tac lifecycle, listener and topic proofs

- An F# Tac service is tested on the page event bus
  (`tac_web/fsharp_lifecycle_tests.rs`): `[<NeedsAll>]` props reach the
  primary constructor, `[<Order>]` runs listeners of one event in order, an
  `[<InBackground>]` listener runs after the others whatever its order, a
  `[<Topic>]` model in `client/models` opens its topic stream and the
  message reaches `[<OnEvent>]`, `[<OnStop>]` runs when the page disposes
  the owner, and `[<Description>]` builds.
- A `member val` without a written type whose value is a primary-constructor
  parameter takes that parameter's type in the C# shadow, so the page can
  read and set it.
- A Tac `[<Needs>]` property of a browser API such as `Clock`, or of a
  Store, stays TY1405. A Tac service creates a browser API with
  `let clock = Clock()`.

## Amendment (2026-10-18): an F# test unit takes its settings and models

- An F# test unit runs on the C# driver, so it passes its
  `[<TestSettings>]` to the behavior runtime in the same way as a C# unit.
  Each path becomes its environment name, by the rule that
  `Behaviors.properties` uses for application JSON: `tac.catalog.mode`
  becomes `TAC_CATALOG_MODE`. Each value becomes its text: `"fast"` becomes
  `fast`, `5` becomes `5` and `true` becomes `true`. So the
  `[<WhenSetting>]` and `[<Settings>]` of the TestSetup see them. Before,
  the driver passed the dotted path and the JSON text (`"\"fast\""`),
  `Behaviors.create` refused them with `configuration`, and every C# or F#
  unit with TestSettings failed before its first case.
- A Tac F# test unit now includes the `client/models` metadata, as the F#
  page build does. So `Behaviors.values(typeof<SomeModel>, ...)` binds a
  `client/models` type with `[<Setting>]` fields in a test unit too. Before,
  it failed with `configuration-properties`.
- `crates/tachyon-core/tests/it/tac_fsharp_context.rs` proves the whole
  context of a Tac F# unit: `[<UseProfiles>]`, `[<TestSettings>]`,
  `[<FreshStart>]` and a `[<ValueSource>]` case against a TestSetup with
  `[<WhenProfile>]` and `[<WhenSetting>]`; a client Setup with
  `[<Settings>]`, `[<Min>]` and `[<Max>]` that is picked only for the right
  profile and setting; a model with `[<Setting>]` fields; and a `[<Fake>]`
  HTTPClient that answers through the TestSetup adapter until the test
  assigns a fake. A unit without the profile, or with the wrong setting
  value, is refused by its TestSetup with `inactive-configuration`, and
  the run fails before any case.

## Amendment (2026-10-18): every outbound HTTP marker is proven

- A Tac F# HTTPClient may use every request marker: `GetRequest`,
  `PostRequest`, `PutRequest`, `PatchRequest` and `DeleteRequest` select the
  verb and path, and `PathVariable`, `RequestParam`, `RequestHeader` and
  `RequestBody` select argument values. The F# client goes through the same
  C# lowering as before.
- A parameter marker may stand on any parameter of an abstract member,
  tupled (`id: int * [<RequestBody>] change: BehaviorJSON -> ...`) or curried
  (`[<PathVariable("id")>] id: int -> [<RequestBody>] product: BehaviorJSON -> ...`).
  Tachyon writes the concrete member with the same parameter groups the
  abstract member declares, so a curried member is called curried. Before
  this amendment every abstract member became a tupled member, and a caller
  that used the declared curried form failed to compile with an F# type
  error. This applies to every F# role with abstract members (clients,
  stores, converters).
- The test `an_fsharp_http_client_sends_every_verb_and_request_part_through_its_test_adapter`
  (`crates/tachyon-core/tests/it/tac_fsharp_http.rs`) proves it on host
  .NET: a service with `[<Needs>]` calls one client member per verb, and a
  TestSetup adapter answers with the method, path, query, headers and body
  it received. A GET and a DELETE send no body, and the curried PUT member
  is called curried. `an_fsharp_page_service_calls_a_curried_and_a_tupled_http_client_member`
  (`crates/tachyon-core/src/tac_web/fsharp_http_client_tests.rs`) shows the
  curried and tupled members on the .NET page build, driven in node.

## Amendment (2026-10-18): the rule table checks F# Tac code

- `"fs"` is now in the Tac reach lists of the annotation rule table
  (`TAC`, `TAC_BEHAVIOR`, `TAC_FAKE`, `TAC_VALUE_SOURCE`, `TAC_NEEDS_ALL`,
  `TAC_STREAM`, and the lists that follow them). F# Tac code still passes
  through its C# view with C#'s rules, and then every marker is checked
  against the F# reach. The `FSHARP_TAC_CHECKED` list, which limited the F#
  check to StreamRequest and SocketRequest, is gone: F# Tac code holds
  stream and socket clients now (see the amendment above).
- Named has its own reach list, `TAC_NAMED`, without F#, so F# Tac code
  refuses it by name with `TY1308`. Before, Named failed with `TY1405`,
  because an F# Tac constructor parameter or `Needs` property may only be
  the `ApplicationEventPublisher`, an HTTPClient or a NeedsAll property.
  The F# reach check reads a view of the file that keeps the `[<Needs>]`
  and `[<Named>]` lines of a property; the C# view moves those properties
  into the constructor and drops their markers.
- A `Needs` property of a browser API such as Clock still stops with
  `TY1405`, by design: an F# service builds it, as in
  `let clock = Clock()`. A `Needs` property of another service stops with
  `TY1405` too.
- `[<Lifetime({"value":"prototype"})>]` on an F# Tac service works. The
  page registry read only the `@Lifetime` spelling, so it ignored the F#
  attribute and shared one instance. It now reads the service through the
  F# view, so a prototype service gets one instance for each component.
