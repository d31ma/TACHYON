# ADR 0099: C++ Is a Tac WebAssembly Service Language

- Status: Accepted (Tac slice; Yon route controllers, 2026-10-16 amendment;
  the rest of the Yon slice is pending)
- Date: 2026-10-04
- Builds on: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md)
  (the linear-memory page bridge),
  [ADR 0046](0046-tachyon-annotations-are-global.md) (markers need no import),
  [ADR 0056](0056-every-annotation-checks-its-target.md) (the shared rule
  table), [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (WebAssembly services reach the browser through Tachyon),
  [ADR 0067](0067-a-class-file-is-named-after-its-class.md) (one class per file, named after it)
  and [ADR 0081](0081-release-webassembly-uses-the-smallest-compiler-per-language.md)
  (release WebAssembly)
- Exception to: [ADR 0062](0062-yon-uses-native-http-types.md)
  (C++ has no standard HTTP or JSON types, so Tachyon ships C#-shaped ones)

## Context

C++ has the largest audience of the languages that Tachyon did not support.
Emscripten compiles it to WebAssembly with linear memory, so it runs in every
browser, like Rust, Go and Swift. Tachyon already pins Emscripten 6.0.10 for
the mruby and Lua runtimes (ADR 0049, ADR 0085).

C++ has no runtime reflection, no standard JSON library and no standard HTTP
library. The owner decided (2026-09-27) that Tachyon uses no third-party
library and no foreign function interface for them: Tachyon ships its own
header-only types, shaped like .NET's `System.Text.Json.Nodes` and
`System.Net.Http`, and generates the model codecs at build time.

## Decision

### Files

- A C++ Tac file is a header-only `.hpp` file with one class (or one
  `enum class` for a model), named after the file (ADR 0067):
  `client/services/<Name>Service.hpp`, `client/models/<Name>.hpp` and
  `client/tests/<Name>Test.hpp`. There is no `.cpp` file: a header-only file
  is the simplest layout that an editor understands on its own.
- An authored file may start with `#pragma once`, standard `#include <...>`
  lines and `#include "..."` lines that name other authored files, so that an
  editor finds the types. Tachyon composes one translation unit per page
  program, so it removes `#pragma once`, the quoted includes and
  `#include <tachyon/...>`, and keeps the standard includes.
- A C++ file holds no namespace, no template class, no free function and no
  file-level variable. A service declares no nested type: models go in
  `client/models`. These forms fail with `TY1371`.

### Markers

- Markers are bare PascalCase standard attributes, like every other Tachyon
  language (ADR 0046): `class [[Service]] OrderService { ... };`,
  `[[Needs]] OrderStore& orders;`, `[[OnEvent]] void placed(const PlacedOrder& order)`.
- The C++ grammar decides the places. A class attribute goes after the class
  key (`class [[Service]] Name`); an attribute before `class` belongs to no
  class and fails with `TY1371`. Member and parameter attributes lead the
  declaration. Several markers may share one `[[A, B(1)]]` or follow each
  other (`[[OnEvent]] [[Order(10)]]`).
- PascalCase keeps the markers clear of the names that the C++ standard
  reserves for its own attributes (lowercase). So Tachyon refuses lowercase
  spellings such as `[[service]]` ("write [[Service]]"), a namespace such as
  `[[tachyon::Service]]`, and an unknown PascalCase name, with the closest
  marker as a suggestion (`TY1371`). Standard attributes (`[[nodiscard]]`,
  `[[maybe_unused]]`, ...) and vendor attributes (`[[gnu::...]]`) pass
  through.
- The rule table lists C++ (`hpp`) as a Tac language and is the one place
  that decides where each marker works. C++ reaches every Tac marker.
  (Until the amendment "Tac C++ streams and sockets" of 2026-10-18 below,
  `StreamRequest` and `SocketRequest` failed with the rule's own code,
  `TY1308`.) The same shared checker (`PlacedMarker`) reports
  a wrong target, missing or extra arguments, a missing partner, a clash and a
  class requirement, in the C++ spelling. A `[[Needs]]` field whose type is
  neither a C++ service nor a C++ HTTP client fails with `TY1372`.
- Tachyon removes the markers before it compiles, so the compiler never sees
  them. Editors warn about unknown attributes; `ty init` writes a `.clangd`
  file with `-std=c++20 -Wno-unknown-attributes` for C++ projects.
- `TY1317` refuses direct browser interop in C++ as in the other languages:
  `emscripten` headers and `emscripten::val`, `EM_JS`, `EM_ASM`,
  `EMSCRIPTEN_KEEPALIVE` and the `import_module`, `import_name` and
  `export_name` attributes.

### Services

- Public non-static fields are page fields; public non-static methods are
  page methods. A `const` field is read-only. Page names are the C++ names in
  lowerCamel case. Overloaded page methods fail with `TY1371`, because a page
  call names a method only by its name.
- `[[Needs]] T& name;` injects another C++ service of the same page program.
  Tachyon lowers it to `T& name = ::tachyon::TacNeeds::of<T>();`, so the
  class keeps its default constructor. Every C++ service is a program-wide
  singleton (Spring's default scope): the page and every `[[Needs]]` field see
  the same object. Tachyon compiles the needed services before the services
  that need them, and refuses a circle and a `[[Needs]]` type that is not a
  C++ service in `client/services`.
- `class [[Service, Lifetime({"value":"prototype"})]] TallyService` makes a
  new object for each component that uses the service and for each
  `[[Needs]]` field that names it; `"singleton"` is the default.
  `"application"` fails with `TY1372`, because a C++ page program has no
  native host to keep an application object. `[[Named("FastGreeterService")]]`
  beside `[[Needs]]` chooses the class that fills a base-class field.
- A `[[NeedsAll]]` class declares no constructor. Each reference field
  without an initial value is a `[[Needs]]` field, and each other field
  without an initial value is a component property: the page passes it once
  per component, so a `[[NeedsAll]]` service is a prototype unless
  `[[Lifetime]]` says otherwise. A property is never empty, so a
  `std::optional` or pointer property fails with `TY1331`.
- `[[LocalState]] int count = 0;` and `[[SessionState]]` keep a public page
  field in `localStorage` or `sessionStorage`, as in the other languages.
- `[[Description("Keeps the score")]]` gives a class a one-line description
  for tools; on a model it needs another Tachyon marker beside it.
- `ApplicationEventPublisher applicationEventPublisher;` publishes a model
  with `publishEvent(model, retain)`. `[[OnEvent]]` takes the event model as
  its one parameter; `{"replay", "condition", "id"}` options, `[[Order(n)]]`
  and `[[InBackground]]` work as in the other languages, and a returned model
  is published again. `[[OnStart]]` and `[[OnStop]]` run on activation and
  disposal. Listener and hook methods may be private: Tachyon adds a `friend`
  declaration for the generated binding class.

### JSON and models

- The prelude is a few header-only files that Tachyon writes into every
  composed program: `tachyon/json.hpp`, `tachyon/http.hpp`,
  `tachyon/task.hpp` and the page bridge.
- `tachyon::json` follows `System.Text.Json.Nodes`: `JsonNode`,
  `JsonObject`, `JsonArray`, `JsonValue`, `JsonValueKind`, `JsonException`
  and `JsonSerializer`. The API uses C++ naming: `JsonNode::parse`,
  `toJsonString`, `getValue<T>`, `getValueKind`, indexers, `add`, `remove`,
  `containsKey`, `count`, `deepClone`, `deepEquals`;
  `JsonSerializer::serialize`, `serializeToNode` and `deserialize<T>`.
- The parser is strict RFC 8259 with the limits of the other preludes:
  1 MiB of text and 64 levels of nesting. Numbers are doubles; an integer
  read checks that the number is whole and fits the type.
- C++ has no reflection, so Tachyon generates a `JsonCodec<Model>` for every
  model in `client/models` at build time (the `JsonSerializer` equivalent).
  A missing key keeps the field's C++ default, like `System.Text.Json`; a value
  of the wrong kind throws `JsonException`. An `enum class` model crosses the
  bridge as its enumerator name. `bool`, numbers, `std::string`,
  `std::vector`, `std::optional`, `std::map<std::string, T>`, `JsonNode` and
  models nest freely.

### Build and bridge

- Tachyon composes one `services.cpp` per page program: its prelude, the
  models and their codecs, the services in dependency order, one generated
  binding class per service and the service table. `#line` directives point
  compiler errors at the authored file and line.
- `em++` compiles it with `-std=c++20 -fwasm-exceptions -sSTANDALONE_WASM
  --no-entry -sMALLOC=emmalloc -sALLOW_MEMORY_GROWTH=1`, `-O1` in development
  and `-Oz` (which runs Binaryen) in release. The build is cached by compiler
  version, flags and program.
- The module exports `tac_wasm_alloc`, `tac_wasm_dealloc`,
  `tac_native_invoke`, `tac_native_free` and `tac_http_settle`, and imports
  `tac_host_emit`, `tac_host_storage`, `tac_host_web`, `tac_host_epoch_ms`,
  `tac_host_http_start`, `tac_host_http_cancel` and `tac_host_task_done`
  from `tachyon`:
  the same linear-memory protocol as Rust and Swift (`init`, `get`, `set`,
  `call`, `event`, `dispose`). A small WASI shim prints standard output and
  standard error to the console.
- Exceptions are on. Every request runs inside `try`, so a C++ exception
  (`JsonException`, `std::runtime_error`, ...) becomes the request's error
  answer and never reaches the page as a trap.
- C++ links into no native host. Like Go and Java, a C++ service runs as
  WebAssembly in the browser and in the web view of every native target.
  A native capability that needs a linked companion does not accept C++.

### Sync and async

- A page method is synchronous unless it returns `Task<T>` (`tachyon::Task`,
  a C++20 coroutine type that Tachyon ships). A `Task` method is listed in
  `asyncMethods`; the page awaits it like a Rust `async` method. The method
  starts at once and returns a task number; when its coroutine finishes,
  Tachyon reports the answer with `tac_host_task_done`. The page side reuses
  the Rust task bridge unchanged (`tac_host_http_start`, `tac_host_http_cancel`,
  `tac_http_settle`, `tac_host_task_done`).
- An exception that leaves a task becomes the task's error answer; an
  `HttpRequestException` also carries its Tachyon code (`HTTP_STATUS`,
  `HTTP_NETWORK`, `CANCELLED`, `DEADLINE`, ...) as the error's `code`.

### HTTP

- `tachyon::http` follows `System.Net.Http`: `HttpClient` (`sendAsync`,
  `getAsync`, `getFromJsonAsync<T>`, `postAsJsonAsync`), `HttpRequestMessage`
  (`method`, `requestUri`, `headers`, `content`), `HttpResponseMessage`
  (`statusCode`, `reasonPhrase`, `headers`, `content`,
  `isSuccessStatusCode`, `ensureSuccessStatusCode`), `HttpMethod`
  (`HttpMethod::Get`, ...), `HttpStatusCode`, `HttpContent`
  (`readAsString`, `readFromJson<T>`), `StringContent`, `JsonContent` and
  `HttpRequestException`. Methods return `Task<...>` (C# returns `Task<...>`
  too); `readAsString` is synchronous because the body has already arrived.
- Every request goes through the page's same-origin browser transport (the
  one Rust and C# use): JSON bodies, `credentials: omit`, no redirects, a
  30-second deadline and a 1 MiB answer limit. A status outside 2xx fails
  with `HttpRequestException` code `HTTP_STATUS`.
- A declarative client is a class in `client/remotes`:
  `class [[HTTPClient("/api")]] OrderClient { public: [[GetRequest("/orders/{id}")]] Task<Order> find(int id); };`.
  Its methods have no body; Tachyon writes them. `{name}` in the path takes
  the parameter of that name, `[[RequestBody]]` sends a parameter as the JSON
  body, `[[RequestParam]]` adds it to the query and
  `[[RequestHeader("X-Name")]]` sends it as a header. A service asks for the
  client with `[[Needs]] OrderClient& orders;`.
- Each method of a declarative client sends its request with the client's
  adapter name: the class name without `Client`, in lowerCamel case
  (`GreetingClient` sends with `greeting`), as `@HTTPClient` names its
  adapter in the other languages. A plain `HttpClient` sends with `http`.
  The browser transport does not read the name; a test unit uses it to find
  the answer (see Test units).

### Validation and converters

- `struct [[Checked]] SignUp` with `[[Required]]` (on a `std::optional` or
  pointer field), `[[NotEmpty]]` (text), `[[Length({"min":1,"max":80})]]`
  (text, `std::vector`, `std::map`, `std::set`), `[[Min(0)]]` and `[[Max(9)]]`
  (numbers) gets a generated `TacChecks<SignUp>`. `[[Check]]` on a service
  or HTTP client parameter of a `[[Checked]]` model runs it first, so a bad
  argument fails with the `validation` code and the field path
  (`name.notBlank`) before the method runs or the request leaves.
- A `[[Converter]]` class in `client/converters` declares methods without a
  body, such as `Summary toSummary(const Product& product);`. Tachyon writes
  each one: it copies same-named fields (nested models, lists and maps of
  models too) and follows the `[[Convert({"fields":[...]})]]` plan for the
  rest, with the shared limits (32 entries, source paths of 8 names, an
  absent path is null). A `JsonNode` to `JsonNode` method needs a plan. A
  target field with no source, a source that may be empty for a field that
  is not `std::optional`, and a value of another kind fail the build and
  name the field. A service gets the converter with `[[Needs]]`.

### Helpers, topics and behaviors

- A `[[Helper]]` class in `client/helpers` is a plain C++ class that a
  service gets with `[[Needs]]`, like another service.
- A `[[Topic]]` model in `client/models` gives each page listener of that
  model its topic stream, as in the other WebAssembly Tac languages.
- `[[Cached]]`, `[[UpdateCache]]`, `[[ClearCache]]` (with `[[CacheKey]]`
  parameters and `[[CacheSettings]]` on the class), `[[Retry]]`,
  `[[MaxAtOnce]]`, `[[AllOrNothing]]` and a `[[PageDefaults]]` parameter go on
  a synchronous instance method with a body that returns a value or `void`.
  Tachyon wraps the body: the shared planner checks the options, and a small
  C++ runtime runs the policy and keeps cached values in the page's built-in
  cache through the storage host. A `Task` method and an `[[HTTPClient]]`
  method cannot carry these markers yet, because the policies run around a
  synchronous body.

### Stores

- `struct [[Entity({"adapter":"indexeddb"}), Table("notes")]] NoteEntity` in
  `client/entities` (or `client/models`) has exactly one public `[[Id]]`
  field. `class [[Store(NoteEntity)]] NoteStore : public
  tachyon::Repository<NoteEntity, std::string>` in `client/stores` declares
  query methods without a body, such as
  `std::vector<NoteEntity> findByText(const std::string& text);`, and
  `[[Query(...)]]` for the explicit form. The shared query planner reads the
  method names, so a C++ Store builds the same plans as the other languages.
  Tachyon writes each method, and the `Repository` base gives `findById`,
  `findAll`, `deleteById` and `save`. The return type
  must fit the query: `std::vector<T>` for find, `std::optional<T>` for
  findOne, `bool` for exists and a whole number for count and delete.
- The rows live in the browser's IndexedDB through the page's entity mirror.
  `[[AllOrNothing]]` on a service method collects the writes and commits them
  in one transaction, or rolls them back when the method throws.
- Only the `"indexeddb"` adapter builds. Other adapters stay out of the
  plan for now, so they fail with `TY1372`. A project
  has one Store for each Entity, and a Store method has one name.

### Settings

- `struct [[Settings({"prefix":"TAC_SHOP_"})]] ShopSettings` binds its public
  fields from `TAC_` values; `[[Min]]` and `[[Max]]` bound a number field and
  an initializer is its default. `[[Setting("${tac.app.title:Shop}")]]` binds
  one field of a model from the application configuration.
- A service binds an explicit snapshot:
  `Behaviors::properties<ShopSettings>(values)` from a
  `std::map<std::string, std::string>` or from the application `JsonNode`,
  and `Behaviors::values<Model>(application)`. The shared rules check the
  plan at build time and the same limits at run time; a bad value fails with
  the `configuration-properties` code. A behavior context gives the same
  binding from its own properties (see the 2026-10-18 amendment).

### Test units

- A test unit is `client/tests/<Name>Test.hpp` with one class marked
  `[[Test(<Name>TestSetup)]]`; the TestSetup lives beside it. `ty test`
  composes one program with the test prelude, the models, every service and
  client, the TestSetup, the unit and a generated `main`, compiles it with
  clang++ (or `TAC_CPP_TEST_COMPILER`) and runs the binary. It needs no
  Emscripten. Each public method whose name starts with `test` and that has
  no parameter is a case; a method with `[[ValueSource]]` takes one.
- A case may return `Task<void>`. The test transport answers at once, so the
  task has finished when the case returns; a task that still waits fails its
  case.
- A static `configure()` or `configure(const BehaviorExecution& execution)`
  method of the TestSetup returns `BehaviorAdapters`.
  `adapters.http["greeting"]` is a `BehaviorHTTPAdapter`, a function that
  takes the `tachyon::http::HttpRequestMessage` and returns the answer body
  as a `JsonNode`, for example
  `tachyon::json::JsonSerializer::serializeToNode(Greeting{"Hi"})`. Each case
  opens its own behavior context from the TestSetup and disposes it when the
  case ends, so `configure` runs again for each case and a case never sees
  another case's answers (see the 2026-10-18 amendment). A request whose adapter has no answer fails with an
  `HttpRequestException` whose code is `HTTP_ADAPTER`; an answer function may
  throw `HttpRequestException("HTTP_STATUS")` to test a failure. No request
  leaves a test unit.
- `[[Fake]] GreetingClient greetingClient;` is a field of the test class that
  holds the stand-in object itself: no reference, no pointer, no `static`.
  Its type is a C++ service or HTTP client of the project (`TY1372`
  otherwise). Tachyon writes a member `construct<T>()` into the test class:
  while it makes a `T`, every `[[Needs]]` field of a Fake's type gets the
  Fake field's object instead of the program-wide one. A shared service that
  is first made during `construct` keeps the shared objects, so a Fake never
  leaks into a later case. `CounterService counter;` still makes the service
  with the shared objects. A `[[Fake]]` field in the TestSetup class is
  refused, because no `construct` reads it there.
- `[[FreshStart]]` on the test class or its TestSetup makes every shared
  service again before each case, so no case sees another case's state.
- `[[ValueSource([1, 2, 3])]]` on a case with one parameter runs it once for
  each input, named `testName[1]`, `testName[2]` and so on (ADR 0080). The
  list is JSON with 1 to 256 numbers, strings or booleans; Tachyon writes
  each input as a C++ literal into the call.
- Executed-method coverage reads C++ services: the public non-static methods
  with a body are the methods that a case must call. In a test unit, Tachyon
  adds `BehaviorCalls::record("CounterService.increment");` at the start of
  each of them, and each case reports the methods it ran. A call on a Fake
  client does not count, because a client has no written methods. The call
  is recorded only at run time, so a `constexpr` method still compiles; a
  call that the compiler evaluates does not count.

### Size

- Measured with em++ 6.0.10 in a release build (`-Oz`, emmalloc):
  a page with one small service (the `ty init` greeting) loads 74.9 KB of
  WebAssembly (29.2 KB with Brotli) and a 14.6 KB engine module (3.7 KB with
  Brotli). The browser-gate fixture, with four services, an HTTP client and
  two models, is 122.7 KB. The budget for a one-service page is 100 KB of
  WebAssembly (40 KB with Brotli).
- `std::to_chars` for doubles would add about 100 KB, so the JSON writer
  prints numbers with the shortest of `%.15g`, `%.16g` and `%.17g` that
  reads back exactly.

## Amendment (2026-10-16): Yon route controllers

C++ is now a Yon language for route controllers. Services, middleware and the
other Yon roles stay pending (see Pending). The amendment of 2026-10-18 adds
services, helpers, injection, lifecycle hooks and workers.

### Files and markers

- A C++ controller is a header-only `.hpp` file in `server/controllers` with
  one class named after the file, as on the Tac side:
  `class [[RequestRoute("/orders")]] [[Public]] OrderController { ... };`.
  The same reader, marker check and `#include` rules apply: Tachyon removes
  `#pragma once` and quoted includes and keeps the standard includes.
- A model is a struct or an `enum class` in `server/models/<Name>.hpp`.
  Tachyon generates its `JsonCodec` as for a Tac model, and the route types
  read its public fields. A field with an initializer, or a
  `std::optional` field, is optional in a request body; any other field is
  required, like a C# model.
- The rule table lists `hpp` as a Yon language. `RequestRoute`, `GetRoute`,
  `PostRoute`, `PutRoute`, `PatchRoute`, `DeleteRoute`, `Public`, `SignedIn`,
  `Allow`, `FromPath`, `FromQuery`, `FromBody`, `FromHeader` and `FromCookie`
  reach C++. The amendments of 2026-10-18 add the role, injection and
  lifecycle markers, the Setup and settings markers, `ErrorHandler`,
  `HandlesError`, `ResponseStatus` and `IdentityProvider`, and the test
  markers. Every other Yon marker in a C++ file (`SocketRoute`,
  `StreamRoute`, the behavior markers, ...) fails with its rule's code, and the message says that
  C++ gets it in a later step. A C++ file in `server/controllers` without
  `[[RequestRoute]]` fails with `TY1363` or `TY2015`.

### Routes

- A route method is an instance method whose name starts with its verb
  (`getOrder` for `[[GetRoute]]`). It may be private: Tachyon adds a
  `friend` declaration for the generated dispatcher. A static route method
  fails with `TY2015`.
- Every parameter names its source with `[[FromPath]]`, `[[FromQuery]]`,
  `[[FromBody]]`, `[[FromHeader]]` or `[[FromCookie]]`; a quoted argument
  renames it, as in the other languages. One parameter may have no source
  marker when its type is `const tachyon::http::HttpRequestMessage&`: it
  receives the whole request (amendment of 2026-10-18). Any other parameter
  without a source marker fails with `TY2015`. One parameter at most reads the
  body, and a GET route reads none. A parameter with a default value fails
  with `TY2015`, because the server never sees the default: write
  `std::optional<T>` and choose the value in the method.
- Parameter and return types map to route shapes: `bool`; `int`, `long`,
  `std::int64_t` and the other integer types; `double` and `float`;
  `std::string`; `std::vector<T>`; `std::optional<T>`;
  `tachyon::json::JsonNode` and `std::map<std::string, T>` (any JSON); and
  models. The server checks and converts the arguments before the process
  starts, as for every Yon language.

### Build and run

- Tachyon composes one translation unit per controller: the JSON prelude
  (`tachyon/json.hpp`), a small Yon runtime (`YonFrame`, `YonAnswer`,
  `YonMain`), the server models and their codecs, the controller and a
  generated `TachyonYonRoutes` dispatcher with `main`. `#line` directives
  point compiler errors at the authored files.
- `clang++ -std=c++20 -O2` compiles it into a native program. The handler
  cache keys it by the program, the compiler version, the architecture and
  the operating system, and shares it through `runtime/handlers-v1` like the
  other compiled languages. A missing `clang++` fails with `TY2112`, and
  `ty doctor` lists "Yon C++ compiler 'clang++'" when a controller is C++.
- The program reads one request frame (at most 16 MiB) from standard input
  and writes one direct answer, like a Rust or Go handler. A returned value is
  written as JSON with status 200; `void`, or a value that is JSON `null`
  such as an empty `std::optional`, answers 204. An argument that does not
  fit its C++ type answers 400. An exception that leaves a route method
  answers 500 with `{"error":"Handler failed."}`; its `what()` text goes to
  standard error as a `handler.cpp_exception` line and never into the
  answer.

## Amendment (2026-10-18): Tac C++ Setup and behavior context

C++ Tac code now has `[[Setup]]`, `[[WhenProfile]]`, `[[WhenSetting]]`,
`[[UseProfiles]]` and `[[TestSettings]]`, with the same rules as the other
Tac languages (ADR 0022).

### Setup files

- A Setup is a header-only `.hpp` file in `client/setups` with one class
  named after the file and marked `[[Setup]]`. It is not a service: it takes
  only `[[WhenProfile]]`, `[[WhenSetting]]` and `[[Description]]` next to
  `[[Setup]]`, no member markers, and it declares
  `static BehaviorAdapters configure(const BehaviorExecution& execution)`.
  The execution names the environment (`"tac"`), the partition and the
  sorted profiles. `[[Setup]]` on a class anywhere else fails with `TY1371`.
- `[[WhenProfile("public")]]` and
  `[[WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]]`
  use the shared parsers, so a setting name must start with `TAC_`. Tachyon
  writes them into a plan for the Setup type.

### Behavior context

- `BehaviorContext context = Behaviors::create<CatalogSetup>(options);` opens
  a context. `BehaviorOptions` holds the partition, the profiles and the
  properties; `options.withApplication(application)` reads the properties
  from the application `JsonNode`. A blank or long partition, too many or
  repeated profiles, a bad profile name or bad properties fail with the
  `configuration` code. A Setup whose WhenProfile or WhenSetting does not
  match fails with `inactive-configuration`.
- One configuration serves every context with the same Setup, sorted
  profiles and properties. `configure` runs when the first of them opens,
  and runs again only after the last one is disposed. `context.dispose()`
  releases one context; a context that is not open fails with `cancelled`.
  `context.adapters()` gives the adapters, and
  `context.properties<CatalogSettings>()` binds settings from the context's
  own properties.
- A service keeps its context in a `BehaviorContext` field. Such a field is
  neither a prop nor saved page state.
- A C++ `[[HTTPClient]]` still sends its requests through the page to the same
  origin. The `http` adapters of a context answer requests only in a test
  unit.

### Test units

- Each case opens a context from the test's TestSetup and disposes it when
  the case ends. The partition is `"test"` unless the `[[Test]]` marker names
  one, as in `[[Test(CatalogTestSetup, {"partition":"tests"})]]`. `[[UseProfiles("test")]]` gives the profiles and
  `[[TestSettings({"tac.catalog.page.size":5})]]` gives the properties; the
  Test class's markers win over the TestSetup's. A test class field
  `BehaviorContext behaviors;` receives the case's context.

## Amendment (2026-10-18): Yon roles, injection and lifecycle

C++ Yon code now has services, helpers, injection, lifecycle hooks, local
events and scheduled workers. The rule table lists `hpp` for `Service`,
`Helper`, `Needs`, `NeedsAll`, `Named`, `Lifetime`, `Description`,
`OnStart`, `OnStop`, `OnEvent`, `Order`, `InBackground` and `Schedule`.

### Composition

- C++ keeps its own composer instead of joining the shared `yon/plan.rs`
  pipeline. That pipeline reads text-parsed sources and writes a container
  per language; C++ already has a typed reader, a service graph and a
  lowering in `tac_cpp`, and the Yon side reuses them. C++ still produces
  `plan::Route` values for the route table and the server's checks.
- One translation unit per controller or worker, in this order: the C++
  prelude (`includes`, `json`, `task`), the Yon runtime pieces
  (`yon_frame.hpp`, `yon_answer.hpp`, `yon_events.hpp`, `yon_needs.hpp`,
  `yon_main.hpp`) and `services.hpp`, the server models and their codecs,
  forward declarations of every provider, the providers in dependency order
  with the controller last, the generated container, and the generated
  `TachyonYonRoutes` dispatcher with `main`. `CppYonRuntime::PIECES` lists
  the runtime pieces, and `CppYonProgram::source` lists the parts, so a later
  step adds a header or a generated part as one new line.
- `yon_cpp` is split by job: `CppYonGraph` (providers and their checks),
  `CppYonProvider`, `CppYonScope`, `CppYonHooks`, `CppYonContainer`,
  `CppYonDispatcher`, `CppYonWorker`. The graph's `FOLDERS` list names the
  role folders; Y2 to Y6 add `server/setups`, `server/converters`,
  `server/stores` and `server/remotes` there.

### Providers

- A `[[Service]]` is one class in `server/services/<Name>.hpp` and a
  `[[Helper]]` one class in `server/helpers/<Name>.hpp`, named after the
  file. `[[Needs]] T& name;` names its provider by type, or by
  `[[Named("Class")]]`; a `[[NeedsAll]]` class declares no constructor and
  gets every reference field without an initial value. A needed class that is
  not in those folders fails with `TY2015`, and a circle fails with `TY1371`.
- Tachyon lowers each `[[Needs]]` field to
  `T& name = ::tachyon::yon::YonNeeds::of<T>();`. `YonNeeds::of<T>` is a
  template that the program specializes for each provider, so a controller,
  service or helper keeps its default constructor, and its fields are filled
  before its constructor body runs. Each class gets a `friend` declaration
  for `YonNeeds` and the dispatcher, so hooks, listeners and routes may be
  private.
- Scopes are the Yon scopes, written `[[Lifetime("singleton")]]`,
  `"request"` or `"transient"` (the Tac object form `{"value": ...}` is read
  too). A controller is request-scoped and every other class a singleton,
  and both are made once per invocation. A transient class is made for each
  field that needs it. A singleton that keeps a request-scoped object,
  directly or through another class, fails with `TY2015`. One invocation
  makes at most 4096 objects.

### Lifecycle and events

- The container records each object in the order it was made. Before the
  route runs, the invocation registers every `[[OnEvent]]` listener, runs
  `[[OnStart]]` methods in that order and drains the events; after the route
  it drains them again before it writes the answer; when the request ends,
  even through an exception, it closes the queue, runs `[[OnStop]]` in
  reverse order and destroys the objects in reverse order. A cleanup failure
  writes `Yon local lifecycle cleanup failed.` to standard error.
- `ApplicationEventPublisher applicationEventPublisher;` is a plain field,
  as on the Tac side; `publishEvent(model, retain)` queues the model under
  its `JsonCodec` name. Listener options, `[[Order]]`, conditions, replay and
  a returned model that is published again follow the other Yon languages,
  with the same bounds (128 queued events, 4096 deliveries, 1024 listeners,
  64 KiB payloads, 32 republished items). `[[InBackground]]` listeners run
  after the other listeners of the event, and their return value is not
  published again.
- C++ Yon code is synchronous: a hook or listener that returns `Task` fails
  with `TY2015`, as does an `[[OnStart]]` or `[[OnStop]]` method with
  parameters. A transient class in a graph with hooks or listeners fails
  with `TY2015`, as in the other languages. `[[Topic]]` models stay pending.
- `[[Description("...")]]` on a role class is checked as on the Tac side.

### Workers

- A worker is `server/workers/<name>.hpp` with one `class Handler` and a
  `POST()` method without parameters; the runtime calls it on its schedule,
  and its return value is the answer, as for a Go or Rust worker. The worker
  is the root of its own provider graph, so it may use `[[Needs]]`.
  `[[Schedule({"fixedDelaySeconds":300,"initialDelaySeconds":30})]]` goes
  after the class key and has the shared limits; declaring it here and in
  `.tachyonrc` fails with `TY1502`.

## Amendment (2026-10-18): Tac C++ streams and sockets

C++ Tac code now has `[[StreamRequest]]` and `[[SocketRequest]]`, held by the
page as for the ADR 0090 languages (ADR 0090, "C++ streams and sockets").

- The method goes in a `[[HTTPClient]]` class in `client/remotes`, without a
  body: `[[StreamRequest]] void receive(const Tick& tick);` or
  `[[SocketRequest]] void receive(const ChatMessage& message);`. It returns
  `void` and takes the message model as its one parameter; anything else
  fails with `TY1371`. An optional quoted path that starts with `/` is added
  to the client's path, and a class has at most one `[[SocketRequest]]`. A
  socket client may not declare its own method named `send`.
- A C++ service that keeps the client in a `[[Needs]]` field lists the
  client's streams in its `init` answer, and the page opens them for that
  service. The service receives each message with `[[OnEvent]]` on a method
  whose parameter is the message model.
- A socket client gets
  `template <typename Message> tachyon::Task<void> send(Message message) const`,
  which waits for the page and throws `tachyon::http::HttpRequestException`
  with `HTTP_NETWORK`, `HTTP_MISSING`, `HTTP_REQUEST`, `HTTP_BODY` or
  `CANCELLED`. The page program imports `tac_host_socket_start` for it. In a
  test unit there is no page, so `send` throws `HTTP_MISSING`.

## Amendment (2026-10-18): Yon C++ Setup and settings

C++ Yon code now has `[[Setup]]`, `[[Settings]]`, `[[Setting]]`,
`[[WhenProfile]]` and `[[WhenSetting]]`, with the rules of the other Yon
languages (ADR 0022, ADR 0023). The rule table lists `hpp` for them
(`YON_CONTEXT`).

- A Setup is a header-only `.hpp` file in `server/setups` with one class
  named after the file and marked `[[Setup]]`, with the same shape as a C++
  Tac Setup: only `[[WhenProfile]]`, `[[WhenSetting]]` and `[[Description]]`
  next to it, no member markers, and
  `static BehaviorAdapters configure(const BehaviorExecution& execution)`.
  `CppSetup` serves both realms: the realm of the path picks the folder and
  the environment, so a Yon WhenSetting name must start with `YON_`.
  `[[Setup]]` on a controller, worker, service, helper or model fails with
  `TY1371`. Every program of the project includes every Setup, after the
  models.
- `[[Settings({"prefix":"YON_CATALOG_"})]]` and
  `[[Setting("${yon.active.profile:development}")]]` go on models in
  `server/models`, as in C++ Tac code. A Settings prefix that does not start
  with `YON_`, or a Setting path outside `yon.`, fails at build time; the
  same check now runs for C++ Tac models with `TAC_` and `tac.`. Settings
  fields take `[[Min]]` and `[[Max]]` since the validation step (amendment
  "Yon validation and converters" below).
- The Yon runtime adds a native `TacHost` (`yon_host.hpp`) with the realm
  `"yon"`, a clock and a sleep. Behavior storage came with the caching step
  (amendment "Yon caching, policies and stores" below). The program
  includes the shared `behaviors.hpp`, `settings.hpp`, `http.hpp` and
  `setup.hpp` after it, so `Behaviors::properties<T>`,
  `Behaviors::values<T>`, `Behaviors::create<Setup>`, `BehaviorOptions` and
  `BehaviorContext` are the Tac ones. `settings.hpp` and `setup.hpp` read the
  realm from `TacHost::REALM`, so the binders want `YON_` names and `yon.`
  paths, and `execution.environment` is `"yon"`.
- `tachyon::yon::YonApplication::snapshot()` parses `YON_APPLICATION_JSON`
  and fails with the `configuration` code when the supervisor did not set it.
  A failed binding (`configuration-properties`) or an inactive Setup
  (`inactive-configuration`) that leaves a route method answers 500, like any
  exception.
- One configuration serves every context with the same Setup, profiles and
  properties within one invocation, which is one process. The caching,
  store and HTTP-client steps take their adapters from `context.adapters()`.

## Amendment (2026-10-18): Yon validation and converters

C++ Yon code now checks arguments and has converters. The rule table lists
`hpp` for `Checked`, `Check`, `Required`, `NotEmpty`, `Length`, `Min`, `Max`,
`Converter` and `Convert`.

- A `[[Checked]]` model is a struct in `server/models/<Name>.hpp` with the
  field rules of the Tac side: `[[Required]]` on a `std::optional` or pointer
  field, `[[NotEmpty]]` on text, `[[Length({"min":1,"max":80})]]` on text, a
  `std::vector`, a `std::map` or a `std::set`, and `[[Min(0)]]`/`[[Max(9)]]`
  on a number. `[[Check]]` takes no arguments and goes on a parameter whose
  type is a Checked model: on a method of a service, helper or converter, or
  on a controller route parameter next to `[[FromBody]]`. The type names the
  model; a C++ parameter is never a plain JSON value. A `[[Check]]` on any
  other type, or `[[Checked]]` on a role class, fails at build time.
- The check runs before the method body. It reports every broken rule, in
  field order and, per field, in the order Required, NotEmpty, Length, Min,
  Max (`TacCheckReport` collects them; the Tac side still throws the first one
  as `TacValidationError`). A Yon C++ program then writes
  `{"status":422,"violations":[{"field":"name","rule":"NotEmpty"}]}` instead
  of an answer, and the server builds the problem details answer of ADR 0101.
  A body whose JSON does not fit the model types (`"level":"3"` for a number)
  is refused with 400 before the check runs, and fields the model does not
  declare are dropped when the body is read.
- A `[[Converter]]` is a class in `server/converters/<Name>.hpp` that declares
  only bodiless methods, each with one named parameter. Model-to-model methods
  copy same-named fields and may add a `[[Convert({"fields":[...]})]]` plan;
  a `tachyon::json::JsonNode` method needs the plan. The checks and the
  generated definitions are the Tac ones (`CppConverter`), over the models in
  `server/models`. A controller, service or helper reaches a converter with
  `[[Needs]] ProductConverter& converter;`; `server/converters` is the third
  folder in `CppYonGraph::FOLDERS`.
- The Yon prelude now carries `checks.hpp` and `convert.hpp`, and
  `CppProgram::check_role` and `CppProgram::conversions_of` give the Yon
  composer the same checks and definitions the Tac program uses.

## Amendment (2026-10-18): Yon error handlers, ResponseStatus and native types

C++ Yon code now has error handlers, an identity provider,
`[[ResponseStatus]]` and the native request and response types. The rule
table lists `hpp` for `ErrorHandler`, `HandlesError`, `ResponseStatus` and
`IdentityProvider`.

### Error types and handlers

- An error type is one class in `server/errors/<Name>.hpp`, named after the
  file, with no Tachyon marker. It derives from `std::exception`, from a
  standard error such as `std::runtime_error`, or from another error type in
  `server/errors`. Tachyon places the error types in the program so that each
  base comes before the types that derive from it.
- An `[[ErrorHandler]]` class is one class in
  `server/controllers/<Name>ErrorHandler.hpp`. Every C++ controller uses
  every C++ error handler. A handler may have `[[Needs]]` fields, like a
  controller.
- A `[[HandlesError]]` method is an instance method. The type of its first
  parameter, such as `const OrderNotFound& error`, is the error that it
  handles. An optional second parameter,
  `const tachyon::http::HttpRequestMessage& request`, receives the request.
  The method returns a model or a `tachyon::http::HttpResponseMessage`.
- The dispatcher calls the route inside `try` and `catch`. It throws a
  caught error again inside a generated `handled` function, whose `catch`
  clauses are sorted with the most derived type first, so the nearest type
  wins. The depth counts from `std::exception`: `std::exception` is 0,
  `std::runtime_error` and `std::logic_error` are 1, the other standard
  errors are 2, and a project error is 1 more than its deepest base.
- `YonArgumentException` (a bad argument, 400) is thrown again first, so a
  handler never swallows it. An error that no method handles, or an error
  thrown inside a handler, keeps the fixed 500 answer.

### ResponseStatus

- On a route method, `[[ResponseStatus]]` takes a status from 200 to 299.
  The server applies it when the answer is 200.
- On a `[[HandlesError]]` method, it takes a status from 400 to 599, and the
  default is 500. A handler that returns an `HttpResponseMessage` with status
  200 takes the handler's status.
- The error answers of each route feed `/api.json` and the answer checks, as
  in the other languages.

### Identity provider

- One `[[IdentityProvider]]` class in
  `server/controllers/<Name>IdentityProvider.hpp` has exactly one `identify`
  method. It takes the request, or one `[[FromHeader]]`, `[[FromCookie]]` or
  `[[FromQuery]]` value, and returns the user and the names of their roles,
  such as `std::optional<Identity>`.
- Tachyon compiles it like a controller with the fixed route
  `GET /.tachyon/identity`. It takes no `[[Public]]`, `[[SignedIn]]` or
  `[[Allow]]`.

### Native request and response

- Tachyon builds `tachyon::http::HttpRequestMessage` from the request frame:
  the method, the target, the headers (repeated values joined with `", "`)
  and the body, decoded from base64 when needed. Only one route parameter
  without a source marker may have this type, and it receives the whole
  request.
- A route or handler that returns `tachyon::http::HttpResponseMessage`
  answers with its status, headers and content. Status 204 sends no body.
- The message types are the ones in `tac_cpp/tachyon/http.hpp`, shared with
  Tac C++, which the Yon program already includes for its Setup context.

## Amendment (2026-10-18): Yon caching, policies and stores

C++ Yon code now has the behavior policies and Stores. The rule table lists
`hpp` for `CacheSettings`, `Cached`, `CacheKey`, `UpdateCache`, `ClearCache`,
`Retry`, `MaxAtOnce`, `AllOrNothing`, `PageDefaults`, `Entity`, `Table`, `Id`,
`Store` and `Query` (`YON_POLICIES`).

- The markers follow the C++ Tac rules and checks (`CppBehaviors`): they go
  on synchronous instance methods with a body, and `[[CacheSettings]]` goes
  on the class. A service, helper or controller whose methods carry
  `[[Cached]]`, `[[UpdateCache]]`, `[[ClearCache]]` or `[[AllOrNothing]]`
  declares a `BehaviorContext behaviors;` field and opens it from a Setup,
  for example in `[[OnStart]]` with
  `behaviors = Behaviors::create<CatalogSetup>(options);`, and disposes it in
  `[[OnStop]]`. Without the field the build fails with `TY1371`. `[[Retry]]`,
  `[[MaxAtOnce]]` and `[[PageDefaults]]` need no context. A cache key holds
  the partition of the context; Tac C++ keeps `"default"`.
- `BehaviorAdapters` gains `caches`
  (`std::map<std::string, BehaviorCacheAdapter>`), `entities`
  (`std::map<std::string, BehaviorEntityAdapter>`), `authorize` and
  `authorizeEntity`. A `BehaviorCacheAdapter` has `get`, `set` and `remove`
  functions over `BehaviorCacheEntry { value, expiresAt }`. `authorize`
  receives a `BehaviorCacheCall` (operation `cache:read`, `cache:put` or
  `cache:evict`, cache, owner, method and key arguments) and is required when
  a cache is used. A `BehaviorEntityAdapter` is a function from the shared
  entity command to a JSON answer; it answers the transaction `begin`
  command with the transaction identifier as a JSON string.
  `authorizeEntity` is required when a Store runs a query; `save` is not
  gated. `BehaviorMemory::cache()` and `BehaviorMemory::entities()` are the
  built-in in-memory adapters. They live as long as the process, which is one
  request, and the memory entity adapters of one process share one database.
- The native `TacHost` of `yon_host.hpp` now answers behavior storage: a
  `YonStorage` scope routes cache and transaction requests to the adapters
  of the calling object's context, and Store commands go to the context of
  the Store that keeps the entity (`yon_storage.hpp`).
- An `[[Entity]]` is a struct in `server/entities` or `server/models`, such
  as `struct [[Entity({"adapter":"data"}), Table("products")]] ProductEntity`
  with one `[[Id]]` field. Yon accepts any adapter name; Tac C++ keeps only
  `"indexeddb"`. A route that returns an Entity, or a model that holds one,
  needs the Entity in `server/models`, because Yon route types read only
  that folder.
- A `[[Store]]` is a class in `server/stores/<Name>Store.hpp` with only
  bodiless query methods, as in Tac C++, and it may extend
  `tachyon::Repository<ProductEntity, std::string>`. Tachyon opens the
  Store's own behavior context from the Setup that `"setup"` names in the
  options, or by default the Store name with `Setup` instead of `Store`
  (`ProductStore` opens `ProductSetup`); the partition is the Store name in
  lowerCamel case without `Store` (`"product"`). A missing Setup fails with
  `TY2015` and names `server/setups/<Setup>.hpp`. A controller, service or
  helper reaches the Store with `[[Needs]] ProductStore& products;`;
  `server/stores` is the fourth folder in `CppYonGraph::FOLDERS`. Each Entity
  has at most one Store.
- `[[AllOrNothing({"adapter":"data"})]]` on a service method begins, commits
  and rolls back through `entities["data"]` of the service's own context,
  and Store calls inside the method join the open transaction, as in the
  other languages.

## Amendment (2026-10-18): Yon C++ HTTP clients, gateways and topics

C++ Yon code now has `[[HTTPClient]]` with `[[GetRequest]]`,
`[[PostRequest]]`, `[[PutRequest]]`, `[[PatchRequest]]`, `[[DeleteRequest]]`,
`[[PathVariable]]`, `[[RequestParam]]`, `[[RequestBody]]` and
`[[RequestHeader]]`, and `[[Gateway]]` and `[[Topic]]`, with the rules of the
other Yon languages (ADR 0042, ADR 0048, BEHAVIOR_ANNOTATIONS.md "Outbound
HTTP" and "Gateways", APPLICATION_EVENTS.md "Topics"). The rule table lists
`hpp` for them (`YON_OUTBOUND`).

### HTTP clients

- A client is a header-only `.hpp` file in `server/remotes` with one class
  named after the file, in the bodiless form of a C++ Tac client:
  `class [[HTTPClient("/api")]] CatalogClient { public: [[GetRequest("/products/{id}")]] Product find([[PathVariable("id")]] std::string sku) const; };`.
  `server/remotes` is a role folder of the Yon C++ graph, so a controller,
  service or helper injects the client with `[[Needs]] CatalogClient& catalog;`.
- Yon C++ code is synchronous, so a request method returns its answer
  (`Product`, `std::vector<Product>`, `tachyon::json::JsonNode`, `void`, ...)
  instead of a `Task`; a method that returns a `Task`, has a body or has no
  single request marker fails with `TY1371`. The Tac form keeps `Task<T>`.
- Tachyon writes each method as on the Tac side: the path is the client path
  plus the method path, `{name}` takes the parameter of that name or the one
  whose `[[PathVariable("name")]]` names it (this now works on the Tac side
  too), `[[RequestParam]]` adds a query entry, `[[RequestHeader]]` a header
  and `[[RequestBody]]` the JSON body.
- The client sends through its Setup, as in the other Yon languages: the
  Setup is `server/setups/<Stem>Setup.hpp` (`CatalogClient` uses
  `CatalogSetup`), the context partition and adapter name is the lower-camel
  stem (`catalog`), and the method calls `adapters.http["catalog"]` with the
  `tachyon::http::HttpRequestMessage`, then reads the answer type from the
  JSON the adapter returns. The client keeps its context in a private
  `tachyonBehaviors` field, opened with
  `tachyon::yon::YonRemote::open<CatalogSetup>("catalog")` when the client is
  made. A client with a request method and no such Setup file fails with
  `TY2015`; a missing adapter throws `HTTP_ADAPTER`. A client whose methods
  are all gateways needs no Setup.

### Gateways

- A gateway is a bodiless method of an `[[HTTPClient]]` class whose
  parameters and result are `tachyon::json::JsonNode` (a `const` reference
  is accepted):
  `[[Gateway({"class":"server/services/ReportService.java"})]] tachyon::json::JsonNode summarize(const tachyon::json::JsonNode& period) const;`.
  The configuration and its checks are the shared ones (`TY1308`, `TY1312`);
  the key is `<Client>.<method>`. `[[Gateway]]` on any other class, or next
  to another method marker, fails at build time.
- The build reads the declarations from the C++ graph of the caller, so the
  supervisor's gateway table sees them as for any other caller. A C++ caller
  with gateways speaks the piped resource frames that Lua uses
  (`YON_RESOURCE_FRAMES=1`): the request, each
  `{"kind":"resource","command":{"operation":"gateway",...}}` question and
  the answer are length-prefixed frames on standard input and output, so the
  C++ runtime opens no socket.
- `tachyon::yon::YonGateway::call` returns the result, or throws
  `tachyon::TacCodedError` with the message `Gateway call failed.` and the
  code `invalid`, `timeout`, `capacity` or `adapter`.

### Topics

- `struct [[Topic]] OrderPlaced { ... };` in `server/models` makes the model
  a topic. Publishing it with `applicationEventPublisher.publishEvent(...)`
  still delivers it to the local listeners first, and also puts
  `{"topic":"OrderPlaced","payload_json":"..."}` on the `topics` list of the
  direct answer, which the supervisor appends to the topic log, as for Lua.
  A listener that returns a topic model publishes it again.
- The program lists the topic names of `server/models` in
  `tachyon::yon::YonTopics::names()`. The source reports its topics and its
  `[[OnEvent]]` listeners of topic models (`Owner.method`, from the
  controller and every class in its graph) to the topic bus, which delivers
  topics from other files and languages with the `event.deliver` operation.
  For that operation the program registers only the named listeners, starts
  the objects, queues the delivered event after the `[[OnStart]]` methods,
  drains, and answers 204.

## Amendment (2026-10-18): Yon C++ test units

C++ Yon code now has test units in `server/tests`. The rule table lists
`hpp` for `Test`, `TestSetup`, `Fake`, `FreshStart`, `ValueSource`,
`UseProfiles` and `TestSettings` (`YON_TEST`).

- A test unit is `server/tests/<Name>Test.hpp` with one class marked
  `[[Test(<Name>TestSetup)]]` or
  `[[Test(<Name>TestSetup, {"partition":"tests"})]]`. The TestSetup is
  `server/tests/<Name>TestSetup.hpp` with
  `class [[TestSetup]] <Name>TestSetup`, and it may declare `static BehaviorAdapters configure()` or
  `configure(const BehaviorExecution& execution)`.
- `ty test` finds the unit like the other server units. It builds the unit
  with the C++ handler build (`clang++ -std=c++20 -O2`) and posts to `/`
  once. The program answers the usual JSON report:
  `{"unit":...,"cases":[{"name":...,"passed":...,"error":...,"called":[...]}]}`.
  It needs no Emscripten.
- The program holds the Yon runtime with a test host (`TacHost` with the
  realm `"yon"`, clock 0 and a sleep that does nothing). The host answers
  behavior storage as the native host does, through the adapters of the
  calling object's context (`YonHostStorage`), and the memory entity
  database is reset before each case. The program also holds all of
  `server/models` and `server/entities`, the error types of `server/errors`,
  every server Setup, every class of `server/services`, `server/helpers`,
  `server/converters`, `server/stores`, `server/remotes` and
  `server/controllers` with the error handlers, the TestSetup, the unit and a
  generated dispatcher. A gateway that a class of the program declares is
  read for the unit as for a controller.
- An `[[HTTPClient]]` in a test unit sends its requests to `adapters.http`
  of the TestSetup instead of its own Setup, as Tac C++ test units do
  (`YonRemote::answering()`); a missing adapter names the TestSetup.
- Cases, `[[ValueSource]]`, `Assert`, `Task<void>` cases and a
  `BehaviorContext behaviors;` field work as in Tac C++ test units (see "Test
  units" above and the Tac C++ Setup amendment).
- `[[Fake]] RateHelper rates;` holds the stand-in object itself: no
  reference, no pointer, no `static` (`TY1371`). Its type is a C++ class of
  the project in `server/services`, `server/helpers`, `server/converters`,
  `server/stores`, `server/remotes` or `server/controllers` (`TY1372`
  otherwise). Tachyon writes a
  member `construct<T>()` into the test class. While it makes a `T`, every
  `[[Needs]]` field whose type is a Fake's type gets the Fake object. A
  shared (singleton or request) object that is first made during `construct`
  keeps the real objects, so a Fake never leaks into it. A `[[Fake]]` field
  in the TestSetup class fails with `TY1371`. Yon Rust and Go have no
  construct helper; C++ has one, as in Tac C++.
- Each case is one invocation. Objects are new for each case, singletons
  too. `[[OnStart]]` runs when an object is made, also for an object that a
  case makes after it started. Listeners hear the events that the case
  publishes, and the events are delivered when the case ends. `[[OnStop]]`
  runs in reverse order at the end of the case. `[[FreshStart]]` is accepted
  but changes nothing, as in Rust and Go, because each case already starts
  with new objects. A transient or capture check that a controller build
  makes is not made again for the test program.
- Each case opens a context from the TestSetup. The partition is `"test"`
  unless the marker names one. `[[UseProfiles("test")]]` gives the profiles
  and `[[TestSettings({"yon.catalog.size":5})]]` gives the properties; a key
  must start with `yon.` and becomes `YON_CATALOG_SIZE`. The Test class's
  markers win over the TestSetup's.
- Executed-method coverage reads server C++ code too. In a test unit,
  Tachyon adds `BehaviorCalls::record("PriceService.price");` at the start of
  each public non-static method with a body of a service, helper or
  controller, and the coverage table lists `("server", "hpp")`, so the
  report uses the recorded calls.

## Pending

Each bullet is one planned step. A step deletes its own bullet, and only that
bullet, when it lands.


- Yon C++ streams and sockets: `[[StreamRoute]]`, `[[SocketRoute]]`,
  `[[OnOpen]]`, `[[OnMessage]]`, `[[OnClose]]` and `[[OnError]]`.

The rule table refuses each of these markers for C++ until its step lands.
These stay out of the plan for now:

- Yon C++ middleware and relays.
- A C++ class as the target of another language's `[[Gateway]]`: Tachyon
  writes no `server/gateways/<id>/yon.hpp` unit yet, so such a target fails
  with "No Yon gateway unit exists for .hpp yet". C++ code calls gateways.
- Tac C++ Store adapters other than `"indexeddb"`, and the behavior markers
  on `Task` and `[[HTTPClient]]` methods.
- The curated web APIs (Clock, Preferences, ...) for C++ `[[Needs]]` fields.
- Lifetime `"application"` stays refused: a C++ service runs in the page and
  has no native host.
- `ty migrate` rewriting the legacy C++ Yon handlers.

## Consequences

- C++ teams write Tac services in their own language with Tachyon's usual
  markers, and editors understand the files.
- Tachyon owns and maintains a JSON implementation and a model-codec
  generator for C++.
- Pages that use a C++ service download one more WebAssembly module, about
  the size of a Swift or Go one without their runtimes.
