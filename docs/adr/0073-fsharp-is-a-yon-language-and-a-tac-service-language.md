# ADR 0073: F# Is a Yon Language and a Tac Service Language

- Status: Accepted
- Date: 2026-09-28 (behavior annotations, events, lifecycle hooks,
  `NeedsAll`, sockets, streams, executed coverage and the test context:
  2026-09-29)
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
  constructor does and changes nothing. `NeedsAll` with a list of fields and
  field injection with `Needs` are refused by name, because an F# field
  always has a value.
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
  with get, set`, writable with `set`) and its public methods.
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
  Hooks and listeners are synchronous, as in C#, so `InBackground` is refused
  in Yon and allowed in Tac, where it only delays delivery.
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

### Not yet available in F#

Field injection with `Needs` is not available in F# yet: a type takes its
dependencies as primary constructor parameters. A file that uses a `Needs`
field is refused with a message that names the missing feature, so F# never
ignores an annotation.

## Consequences

- F# joins the multi-language examples whose features it supports.
- F# and C# share one toolchain, one browser runtime and one set of checks.
  A change to the C# rules applies to F# as well.
- The C# view is a reading aid, not a translation: F# code is compiled by
  the F# compiler, so a body that the view reads approximately (for example
  a pipeline with `|>`) is still compiled exactly.
- Tachyon's error messages sometimes use a C# word for an F# construct; the
  view renames `switch` to `match` and `foreach` to `for` in the messages.
