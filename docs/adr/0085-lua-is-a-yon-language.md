# ADR 0085: Lua Is a Yon Language

- Status: Accepted; amended 2026-09-29 (Tac Lua services), 2026-10-04 (Lua
  stores, Schedule and converter nulls), 2026-10-16 (a Tac Lua Needs field
  cannot name another service), 2026-10-17 (Yon Lua HTTP clients,
  Gateway and InBackground; Tac Lua behavior annotations, browser state,
  stores and schema models), 2026-10-18 (Tac Lua HTTP clients and helpers)
- Date: 2026-09-29
- Builds on: [ADR 0048](0048-yon-routes-come-from-annotations.md) (routes),
  [ADR 0050](0050-go-annotations-are-comments.md) (comment markers),
  [ADR 0051](0051-controller-error-handlers.md) (error handlers),
  [ADR 0056](0056-every-annotation-checks-its-target.md) (the shared rule
  table), [ADR 0079](0079-every-member-states-its-type.md) (type comments)
  and [ADR 0080](0080-a-test-case-asserts-and-holds-no-logic.md) (test units)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (the Yon languages)
- Adds: Tac Lua services on a Lua WebAssembly runtime, built like the
  pinned mruby runtime of [ADR 0049](0049-tac-ruby-uses-a-pinned-mruby-runtime.md)

## Context

Lua is small, fast to start, and common in games, editors (Neovim), web
servers (OpenResty) and embedded devices. Many teams already write Lua for
plug-ins and scripts. Lua has no classes and no annotations: a class is a
table, and its methods are functions stored in the table.

## Decision

### Realm

- Lua (`.lua`) is a Yon language: controllers, services and the other roles,
  error handlers, error types in `server/errors/`, root middleware
  (`middleware.lua`), models in `server/models/` and test units.
- Lua (`.lua`) is also a Tac language for services, client models and test
  units (see the amendment below).

### A class is one global table in its own file

- A Lua class file holds one class table: `OrderService = {}`. The table is
  global, because Tachyon joins the class files of one realm into one
  program, and a global name reaches every file. `local OrderService = {}`,
  `return OrderService`, `require`, a second table, a method of another
  table, a one-line method, or any other statement outside a method is a
  build error (TY1357), with advice that names the right form.
- A method is `function OrderService:list()` (an instance method) or
  `function OrderService.configure(execution)` (a function on the table). Its
  `end` stands at the start of a line.
- The fields of a class sit in the table constructor, one field on each
  line: `order_service = nil,`. Names in capitals are constants.
- A method whose name starts with `_` is private.

### Annotations are comment markers

- Lua has no annotation syntax, so an annotation is a comment on its own
  line: `-- @Service`, `-- @GetRoute("/{id}")`. The names and the arguments
  are the same as in every other language. The comment is the whole marker:
  Lua itself ignores it, so the file runs unchanged.
- A marker above the table marks the class; a marker above a
  `function` line marks the method; a marker above a field in the table
  marks the field; a marker inside the parameter list marks the parameter
  below it.

### Types are LuaCATS comments

- Every field, parameter and return value has a type (ADR 0079), written as
  a LuaCATS comment, the form that the Lua language server reads:
  `---@class Order`, `---@field name string`, `---@type OrderService`,
  `---@param id integer`, `---@return Order[]`.
- `string`, `integer`, `number`, `boolean`, `T[]`, `T?` and
  `table<K, V>` map to text, whole numbers, numbers, true or false, lists,
  optional values and maps. A model is a `---@class` with `---@field` lines
  in `server/models/`.

### How Tachyon reads Lua

- Tachyon writes a line-for-line shadow of each Lua class file in the Ruby
  form (a Ruby class, `#[Name]` markers and YARD types) and runs the Ruby
  checks on it: the rule table, the strict layout, the type rule, the design
  rules and the route and model readers. Every message points back at the
  Lua file and line, in Lua words.
- The program that runs is the Lua source itself, the models, and a
  generated part that builds the classes, fills `-- @Needs` fields, reads
  route arguments, turns answers into JSON and calls the error handlers.
- Tachyon runs Lua 5.4 or newer with the `lua` program. `ty doctor` and the
  readiness check ask for it.

### Requests and answers

- A route parameter without a marker, an error handler's second parameter
  and root middleware receive a request table: `request.method`,
  `request.path`, `request.headers` (lowercase names), `request.body`, and
  `request:header(name)` and `request:query(name)`.
- Middleware answers with a `{ status, headers, body }` list, or `nil` to
  let the request pass.
- An error is a table whose metatable is its error type:
  `error(setmetatable({ message = "order 42" }, OrderNotFound))`. The nearest
  type wins: a handler for a type whose `__index` chain reaches the base runs
  after a handler for the type itself.

### Test units

- A test unit is `server/tests/<Name>Test.lua` with `-- @Test(Setup)`. A
  case is a method whose name begins with `test`. It checks with
  `Assert.equals`, `Assert.is_true` and `Assert.throws`, and
  `-- @ValueSource([1, 2])` runs it once for each input.
  `self:construct(Role)` builds a role with its `-- @Needs` fields, and a
  `-- @Fake` field of the unit replaces the field with the same name.

### Not yet

These stop the build until a later step adds them: the rule table says that
the annotation is not available in Lua (`TY1308`), or the lowering names this
ADR (`TY2015`):
topics. Resources are not available in Lua: DataSource, RedisTemplate and
`relay`. Every Lua refusal of the
rule table ends with "Lua gets it in a later step (ADR 0085)". A Yon Lua
class that needs DataSource or RedisTemplate, or calls `relay(...)`, stops
the build with `TY2015`, and the message names the resource and this ADR.

## Consequences

- Lua teams write Yon controllers and services in plain Lua that the Lua
  language server understands.
- The Ruby readers and rules serve Lua too, so a rule change reaches both
  languages at once. A Ruby message that has no Lua form keeps its Ruby
  wording.
- Lua needs no build step and no cache: a request starts `lua` with the
  protocol runtime and the joined program.

## Amendment 2026-09-29: Tac Lua services

- A Tac service can be `client/services/<Name>Service.lua`, written like a
  Yon Lua class: one global table with `-- @Service`, fields in the table and
  methods `function Name:method()`. Lua comes last in the web language order,
  after Ruby.
- It runs on Tachyon's own Lua WebAssembly runtime. `runtimes/lua-web` holds
  its source: `sources.txt` pins the Lua 5.4.9 archive from lua.org by
  SHA-256 (`2335b6c5…fb8e6`, the sum lua.org lists), `tachyon_lua.c` is the
  small C boundary and `tachyon-lua.js` is the loader. The first build of a
  project with a Tac Lua service downloads and checks the archive, then links
  the Lua core, the base, coroutine, table, string, math and utf8 libraries
  and the boundary with the developer's `emcc` (Emscripten 6.0.10 or newer)
  into `lua-web/<SHA-256 of the runtime source>/tachyon-lua.wasm` in the
  Tachyon cache. Nothing is committed or written into the project. The build
  shares its download, checksum and cache steps with the Ruby runtime. The
  runtime is about 170 KB.
- (Amended 2026-10-17) Like the Ruby runtime, the Lua runtime links the C
  library, `malloc` and the compiler builtins that Emscripten brings. So the
  `lua/THIRD_PARTY_NOTICES.txt` that a build writes beside the runtime holds
  the Lua license and the Emscripten, musl and compiler-rt licenses, taken
  from the Emscripten 6.0.10 sources (`LICENSE`,
  `system/lib/libc/musl/COPYRIGHT` and `system/lib/compiler-rt/LICENSE.TXT`).
- The runtime has no `io`, `os`, `package`, `debug`, `dofile` or
  `loadfile`. It loads only Lua text, never bytecode. The boundary is the
  same synchronous `tac_boot`, `tac_invoke`, `tac_alloc` and `tac_free` as
  the Ruby runtime. `tachyon.tac_host_emit` delivers events, and
  `tachyon.tac_host_ask` answers storage and browser-API requests with a
  string that the runtime frees.
- The page's program is the shared JSON code of the Yon runtime, a Lua
  prelude (`TacServices`), the client models, and each service file as
  written, followed by one `TacServices.register` line. Tachyon reads the
  members, NeedsAll, Lifetime, OnStart, OnStop, OnEvent and Order from the
  same Ruby shadow that it uses for Yon.
- Each object starts with its own copy of the table's fields, so a list in
  one object never leaks into another. With `-- @NeedsAll`, a field written
  `name = nil` is a property that the page passes, and a field with a
  starting value keeps it. The page reads and changes every
  public field and calls every public method. A name that starts with `_` is
  private.
- An event is a model table: `setmetatable({ total = 25 }, PlacedOrder)`.
  `self.application_event_publisher:publish_event(order)` publishes it; the
  field `application_event_publisher = nil` is autowired by its name, as in
  Spring, and the global `ApplicationEventPublisher` works too. A listener
  whose `---@return` is a model publishes its answer again.
- A Tac test unit in Lua is `client/tests/<Name>Test.lua`. It runs with the
  `lua` program, with the same prelude and the same `Assert` and
  `-- @ValueSource` as a Yon Lua unit. `self:construct(Service)` builds a
  service with its own fields, and a `-- @Fake` field of the unit replaces
  the field with the same name.
- Tachyon's browser APIs are global tables that a Lua service calls with
  `:`: `Clock:now_ms()`, `SecureRandom:uuid()`, `Connectivity:online()`,
  `PageVisibility:visible()`, `Preferences:scheme()`,
  `Clipboard:write_text(text)`, `Share:open(title, text, url)`,
  `Notifications:show(title, body)` and `Geolocation:current_position()`.
  A refusal of the page is a Lua error. Their built-in events reach
  `-- @OnEvent` listeners like any model. In a test unit,
  `BrowserFakes.answer(api, call, value)` answers a call.
- `ty init --tac lua` scaffolds a Lua greeting service and its test unit.
  `ty doctor` lists `emcc` for "Lua services on the web", or the cached
  runtime once it is built.

## Amendment 2026-09-30: Yon Lua events and lifecycle hooks

- A Yon Lua role may hold `-- @OnStart`, `-- @OnStop`, `-- @OnEvent` and
  `-- @Order`, with the same events list, condition, id and replay as the
  other Yon languages. Tachyon reads them from the Ruby shadow and writes a
  Lua event container around each route: it resolves the controller, the
  error handlers and the event owners, registers the listeners, runs OnStart
  in the order the objects were made, drains the events before and after
  the route, and runs OnStop in reverse order when the request ends.
- An event is a model table: `setmetatable({ total = 25 }, PlacedOrder)`.
  `ApplicationEventPublisher:publish_event(order)` publishes it, and a field
  `application_event_publisher = nil` is that publisher, bound by its name.
  Lua models carry their name (`PlacedOrder.__name`) for this.
- Lua has no asynchronous methods, so `@InBackground` stays refused
  (`TY1308`). A Lua class does not publish or listen to topics yet, because
  topics travel on the resource channel, which Lua does not have (`TY2015`).

## Amendment 2026-09-30: Yon Lua behavior annotations

- The behavior annotations work in Yon Lua: CacheSettings, Cached, CacheKey,
  UpdateCache, ClearCache, Retry, MaxAtOnce (policy reject, because one Lua
  process serves one call), AllOrNothing, Setup, Settings and the other
  configuration markers, the validation markers, Converter and Convert, and
  Entity, Table, Store and Query over the entity adapters of a Setup.
- Tachyon reads the markers from the Ruby shadow with the same polyglot
  reader as every language and gives Lua a plan: the records of each type
  and the methods to wrap. It wraps each behavior method on its own lines,
  `function T:m(a) return self.behaviors:invoke(self, "T", "m", table.pack(a),
  function(a)` ... `end) end`, so line numbers stay, and appends the Lua
  behavior runtime and `Behaviors.register(records, "yon", module)`.
- The Lua behavior runtime is a port of the Ruby one without the HTTP
  exchange, the RestClient and gateways. A class opens its context with
  `self.behaviors = Behaviors.create(Setup, { partition = "orders", signal =
  BehaviorSignal.new() })`. An adapter is a table whose methods the runtime
  calls with `:`, such as `cache:get(key, execution)`, and
  `Behaviors.construct(Type, values)` makes one. It passes a Lua copy of the
  behavior boundary contract.
- A Lua test unit opens a behavior context from its Setup for each case, and
  `self:construct(Role)` gives it to the role.

## Amendment 2026-09-30: Yon Lua streams and sockets

- A Lua `-- @StreamRoute` method runs in a coroutine and sends each value with
  `coroutine.yield(value)`; a private helper that it calls may yield too. The
  Lua runner writes each value as a length-prefixed event frame and ends the
  process when the method returns, like the PHP runner.
- A Lua `-- @SocketRoute` class keeps one object for each connection. With
  `YON_SOCKET_FRAMES`, the Lua runner reads the request and the messages as
  frames and calls the `-- @OnOpen`, `-- @OnMessage` (the message is read into
  the model of its parameter), `-- @OnClose` and `-- @OnError` methods. The
  session has `send(message)`, `close()` and `open_sessions()`, and each peer
  has `send(message)`. When OnOpen or OnMessage fails, OnError runs and the
  socket stays open.
- A socket class holds no events or lifecycle hooks, as in the other
  languages.

## Amendment 2026-09-30: pure converters in both realms

- A Lua converter is a `-- @Converter` class table in `converters/`. Its
  methods have empty bodies. LuaCATS `---@param product Product` and
  `---@return Summary` select models in the same realm's `models/` folder.
- No `-- @Convert` marker is needed when the target fields can be inferred.
  An explicit `-- @Convert({"fields":[...]})` supplies named overrides. The
  compiler owns the model table and realm; application configuration cannot
  replace them.
- Mapping copies same-named fields, widens integers to numbers, projects
  nested models and model lists, drops source-only fields, and diagnoses
  missing, incompatible or optional-to-required target fields. Recursive
  projection plans retain the shared depth limit of eight.
- Yon controllers and Tac service classes request the provider with a typed
  `-- @Needs` field. Yon uses its existing provider graph. Tac composes the
  converter and registers its provider on the service class; each service
  activation constructs its own injected converter. Mappings read only their
  argument and need no behavior context or external adapter.
- Tac Lua accepts these pure mappings on its existing Lua 5.4 WebAssembly
  runtime. Other Tac behavior annotations remain unavailable. Resources,
  HTTPClient, Gateway, topics and unrelated adapters gain no additional
  support from this converter slice.

## Amendment 2026-10-03: Topic stops the build in Lua

- A Lua model marked `-- @Topic` used to build. In Yon each request that
  published it then failed with a handler crash (`TY2104`), and in Tac the
  page never opened the topic stream. Now the rule table leaves Lua out of
  Topic in both realms, so the build stops at the model file with `TY1308`
  and the message says that Lua gets it in a later step.
- Python Tac services have no event adapter yet (`TY1605`), so a Python
  model marked `@topic` under `client/` stops the build in the same way.
  Python Yon keeps Topic.
- The Topic check of a model file asks the rule table, so the build and the
  website's language coverage read the same list.

## Amendment 2026-10-03: Lua JSON values and the behavior clock

- The shared Lua JSON code reads `null` inside a list as the `YonNull`
  marker, so the other items keep their places, and writes it back as
  `null`. A `null` object field still reads as `nil`. Tac member calls and
  Lua test units turn a `YonNull` argument back into `nil` in its place, so
  `[null, 5]` calls `method(self, nil, 5)`. An empty JSON object reads as a
  table with the `YonJson.OBJECT` marker and is written back as `{}`.
- The reader refuses input that is not JSON: hexadecimal, overflowing or
  zero-led numbers, raw control characters in strings and broken surrogate
  pairs.
- A Yon Lua socket decodes each message with `pcall`. A message that is not
  JSON or is nested more than 64 levels deep goes to `-- @OnError` with a
  clear message, and the socket stays open. A `null` message reaches
  `-- @OnMessage` as `nil`.
- The behavior clock comes from the realm. Stock Lua has no clock finer
  than one second and no sleep, so Yon Lua waits between Retry attempts with
  the operating system's sleep command (`sleep` on macOS and Linux,
  PowerShell `Start-Sleep` on Windows), built only from an integer number of
  milliseconds. A wait does not use the processor. The Yon Lua deadline is
  CPU time plus time slept, raised to the whole seconds that the wall clock
  has passed, so a timeout fires at most about two seconds late and never
  early, also while an adapter blocks on I/O. Tac Lua reads the page's
  monotonic clock and refuses to wait with a clear message; Retry stays
  unavailable in Tac Lua.

## Amendment 2026-10-04: Lua topics

This amendment replaces "Topic stops the build in Lua" above.

- A Lua model marked `-- @Topic` is a topic in both realms. `--` is a Topic
  comment leader, so the model table, the topic stream map and the build
  checks read the marker like `// @Topic` in Go.
- Stock Lua has no sockets, so a Yon Lua handler cannot call the resource
  channel. When a Yon Lua route publishes a Topic model, the event runtime
  keeps the publication, and the handler adds a `topics` list of
  `{topic, payload_json}` entries to its JSON answer. The supervisor gives
  each entry to the same topic outbox that the resource channel uses, with
  the same checks (only a Topic model of the publishing file, an object, at
  most 64 KiB, at most 8 hops), after the process ends and before the
  response goes out. A refused or failed publication is logged as a
  `handler.topic_failed` event, and the response is still sent.
- A Yon Lua `-- @OnEvent` method on a Topic model is a server listener. The
  topic bus sends `event.deliver` to the Lua file, and `Handler.yon_deliver`
  runs only the listeners the delivery names, as in Ruby.
- A Tac Lua `-- @OnEvent` method on a Topic model opens the topic stream in
  the page, like every service that runs as WebAssembly.

## Amendment 2026-10-04: Lua stores, Schedule and converter nulls

- A Yon Lua Store is a provider. A store file is a class table marked
  `-- @Store(ProductEntity, {...})` whose methods have empty bodies. Each
  method is a derived query such as `find_by_status` or `count_by_status`, or
  a `-- @Query` method, and `-- @PageDefaults` marks a page parameter. Like the other languages, Tachyon
  adds the provider role (`-- @Store` on its own line) and an `-- @OnStart`
  method, `start_context`, that opens the store's behavior context from its
  Setup (`CatalogSetup` with `{"setup":"CatalogSetup"}`, or the store name with
  `Setup` instead of `Store`). A controller or service asks for the store with
  a typed `-- @Needs` field. A store call inside a service method with
  `-- @AllOrNothing` joins the open transaction of the same entity adapter.
  Lua has no base classes, so a Lua Store does not extend `Repository`: it
  declares `find_by_id` and the other queries that it needs.
- The entity adapter of a Lua Setup is a table with an `execute` method,
  which the runtime calls as `adapter:execute(command, execution)`;
  `authorizeEntity` is a function that takes `(command, execution)`.
- `-- @Schedule({"fixedDelaySeconds":300})` on a Lua worker in
  `server/workers/` sets its cadence, as in the other languages. Tachyon read
  the marker through the Lua lexer, which hides comments, so the marker was
  ignored and the worker never ran. Now it reads the marker from the Ruby
  shadow, the same way as the other Lua markers, and a bad marker fails with
  `TY1502` at the Lua line.
- A Lua Converter writes `null` for a target field whose source is missing,
  like every other language and MapStruct. Lua tables cannot hold `nil`, so
  the mapped table holds `YonNull`, and the JSON answer writes it as `null`.
  Lua code that reads the mapped table directly sees `YonNull`, not `nil`.
- An unresolved `-- @Needs` dependency (`TY2015`) points at the field that
  needs it in every Yon language, not at the class. In Lua the span covers
  the field's whole line instead of an empty span at the class table.

## Amendment 2026-10-16: a Tac Lua Needs field cannot name another service

- A Tac Lua service whose `-- @Needs` field has the type of another Tac
  service, such as `---@type CounterService` above `counter = nil,`, used to
  build. The page never filled the field, so it stayed `nil`, and the first
  call through it failed in the browser (`handler_failed`).
- Now the build stops with `TY2015` at the field's line, like a
  `WindowControls` or `FilePicker` field, and the message names the field,
  the service and this ADR. Another service is a file of that name in
  `client/services/`, in any language.
- The check reads the Ruby shadow with the same reader as Ruby Tac
  (ADR 0054), so both languages refuse the field in the same way.
- A Tac Lua `-- @Needs` field still takes a Converter. The
  `application_event_publisher` field and the global browser APIs do not
  use `-- @Needs` and are not affected. A Lua service that wants another
  service lets the view call it, or publishes an event that the other
  service handles with `-- @OnEvent`.

## Amendment 2026-10-17: Yon Lua HTTP clients, Gateway and InBackground

- `-- @HTTPClient`, `-- @GetRequest`, `-- @PostRequest`, `-- @PutRequest`,
  `-- @PatchRequest`, `-- @DeleteRequest`, `-- @PathVariable`,
  `-- @RequestParam`, `-- @RequestBody` and `-- @RequestHeader` work in Yon
  Lua. A Yon HTTPClient never needed the resource channel: like Ruby, it sends
  each request through the `http` adapter of its Setup, so the refusal was
  wider than the missing sockets.
- A Lua client is a class table in `server/remotes/` or `server/clients/`,
  marked `-- @HTTPClient`, whose methods have empty bodies. Parameter markers
  stand inside the parameter list, above the parameter they mark. Tachyon adds
  an `-- @OnStart` method, `start_context`, that opens the behavior context from
  the Setup (`{"setup":"CatalogSetup"}`, or the client name with `Setup`
  instead of `Client`), with the adapter name as the partition. A controller or
  service asks for the client with a typed `-- @Needs` field, and a test unit
  builds it with `self:construct(CatalogClient)`.
- The Lua behavior runtime builds the request like the Ruby one: it fills the
  path variables (letters, digits and `- _ . ~` only), the query, the headers
  and the body from the arguments, refuses a missing query or header value, a
  body on GET or HEAD and a transport-owned header, and calls
  `adapter:exchange(request, execution)` of the Setup's `http` adapter named
  by the client. The request carries `method`, `path`, `query`, `headers`,
  `body`, `redirect = "error"`, `credentials = "omit"`,
  `maxResponseBytes`, `signal` and `deadline`. Stock Lua has no HTTP library,
  so the adapter is the application's code, as for caches and entities.
- `-- @InBackground` works on a Yon Lua `-- @OnEvent` method. The Lua event
  runtime runs such a listener after every other listener of the same event,
  in the same drain pass, and never publishes its return value again. That is
  what JavaScript, TypeScript and Python do, and it needs no asynchronous
  methods. The other event checks are unchanged: a Lua method still cannot be
  asynchronous.
- The rule table checks a Lua file's Ruby shadow with the Lua language list,
  so a marker that Lua accepts and Ruby refuses no longer stops the build in
  Ruby words.
- `-- @Gateway` works on a method of a Yon Lua `-- @HTTPClient`. Other
  languages reach the resource channel over a local socket, and stock Lua
  cannot open one, so a Lua handler that has gateways gets a resource pipe
  instead. The supervisor sets `YON_RESOURCE_FRAMES=1` and sends the request
  as a length-prefixed frame. A gateway call writes a frame
  `{"kind":"resource","command":{"operation":"gateway",...}}` to standard
  output, and the supervisor answers it with a frame on standard input, with
  the same limits and the same `invalid`, `timeout`, `capacity` and `adapter`
  failures as the socket channel. The last frame that is not a resource call
  is the handler's answer. The pipe serves only gateway calls, and only in a
  plain request and answer: a Lua stream or socket route that calls a gateway
  gets the `adapter` failure.

## Amendment 2026-10-17: Tac Lua behavior annotations, browser state, stores and schema models

This amendment replaces "Other Tac behavior annotations remain unavailable"
in the converter amendment and "Retry stays unavailable in Tac Lua" in the
clock amendment.

- A Tac Lua service runs the behavior annotations on the same Lua behavior
  runtime as Yon Lua: CacheSettings, Cached, CacheKey, UpdateCache,
  ClearCache, Retry, MaxAtOnce (policy reject, because the page calls a Lua
  service one call at a time), AllOrNothing, Setup, Description, Settings,
  Setting, WhenProfile, WhenSetting, Checked, Check and the validation
  markers, Entity, Id, Table, Store, Query and PageDefaults. The rule table
  lists Lua for each of them in Tac.
- Tachyon wraps each behavior method on its own lines, as in Yon, and
  registers the records with `Behaviors.register(records, "tac", module)`.
  The page program holds the behavior runtime once, after the prelude, and
  only when a service uses it.
- A service opens its context itself, like a Ruby service:
  `self.behaviors = Behaviors.create(BrowserSetup, { partition = "notes",
  signal = BehaviorSignal.new() })` in an `-- @OnStart` method.
- With no adapter of that name in its Setup, a cache and the `indexeddb`
  entity adapter use the page's Tachyon storage through `TachyonHost.ask`
  (`cache.get`, `cache.set`, `cache.delete` and `entity`), and need no
  `authorize` function, as in Ruby. The behavior clock reads the page clock
  and rounds the wall time down to whole milliseconds.
- Retry runs the next attempt at once, without waiting, like Tac Ruby: the
  page gives Lua no way to wait, and blocking the page would freeze it.
- A Tac Lua Store in `client/stores/` is a provider, like a Converter. A
  typed `-- @Needs` field receives it, Tachyon adds `start_context` to the
  store, and each service activation builds its own store and opens the
  store's context from its Setup.
- `-- @LocalState` and `-- @SessionState` mark a Lua service field. The page
  keeps the field in `localStorage` or `sessionStorage` through the same
  field accessors that it uses for every WebAssembly service.
- A Lua schema model in `client/models/` (Checked, Settings, Setting and the
  validation markers) is read like the other Tac languages (ADR 0092). Its
  markers are comments, so the model runs unchanged. Yon Lua schema models
  stay refused with `TY1310`.
- A Tac Lua test unit opens a behavior context from its TestSetup for each
  case when the program uses behaviors. `-- @TestSettings` paths reach the
  context as `TAC_` keys, `-- @FreshStart` is the default for every Lua unit,
  and `-- @Fake` replaces the field with the same name. The unit uses the
  `lua` program's own clock.
- Still refused in Tac Lua: `-- @Helper` (Lua is read through the Ruby
  shadow, and Ruby Tac has no Helper yet), `@HTTPClient` and its request
  markers (Lua has no browser HTTP transport yet; see the next amendment),
  and `@StreamRequest` and
  `@SocketRequest` (the browser's JavaScript holds the stream; a Lua service
  hears each message with `-- @OnEvent`). The Tac Lua streams and sockets
  amendment below accepts both.

## Amendment 2026-10-18: Tac Lua HTTP clients

- `-- @HTTPClient`, `-- @GetRequest`, `-- @PostRequest`, `-- @PutRequest`,
  `-- @PatchRequest`, `-- @DeleteRequest`, `-- @PathVariable`,
  `-- @RequestParam`, `-- @RequestBody` and `-- @RequestHeader` work in Tac
  Lua. The client is written as in Yon Lua, in `client/remotes/` or
  `client/clients/`, and Tachyon adds the same `start_context` method.
- A Tac Lua client is a provider, like a Store or a Converter. A typed
  `-- @Needs` field receives it, and each service activation builds its own
  client and opens the client's context from its Setup.
- With an `http` adapter of the client's name in its Setup, the request goes
  to that adapter, as in Yon. Without one, it goes to the browser through the
  same fetch transport that every WebAssembly Tac language uses, with the
  same request fields, limits and `HTTP_*` failure codes as Tac Ruby.
- The browser answers a fetch later, so each page call to a Lua service runs
  in its own coroutine. A call that sends a request waits in its coroutine
  and the page carries on; the answer resumes it. Like Tac Ruby, a request
  can only be sent during a service call or an event, never from
  `-- @OnStart`, at most 64 requests wait at once, and removing a service
  cancels its waiting calls with `CANCELLED`. An `-- @AllOrNothing` block
  belongs to the coroutine that opened it.
- Still refused in Tac Lua: `@StreamRequest` and `@SocketRequest`, for the
  reason the previous amendment gives. The Tac Lua streams and sockets
  amendment below accepts both.

## Amendment 2026-10-18: Tac Lua helpers

This amendment replaces the `-- @Helper` part of "Still refused in Tac Lua"
in the Tac Lua behavior annotation amendment. Ruby Tac now has Helpers (ADR 0054), so the Ruby
shadow no longer stands in the way.

- A Tac Lua class table in `client/helpers/` (at any depth) may carry
  `-- @Helper`. The rule table lists Lua for Helper in Tac.
- A service asks for a Helper with a typed `-- @Needs` field:
  `---@type LevelHelper` above `levels = nil,`. A Helper may need another
  Helper, a Store or a Converter the same way.
- Tachyon writes `TallyService.__tachyonHelpers = { ["levels"] =
  "LevelHelper" }` after the service, and the same table for each Helper
  that needs a Helper. The page builds the Helper when it builds the service
  object, so each service object gets its own Helper with its own copy of
  the Helper's fields. The field is not a page field: the view cannot read
  or set it.
- A Helper that needs itself through its `-- @Needs` fields stops the
  service when the page builds it, and the message names the Helper.
- The page never runs a hook on a Helper. A Lua Helper with
  `-- @OnStart`, `-- @OnStop` or `-- @OnEvent` fails with `TY1605` at the
  marker, like a Ruby Helper, and the message names the Helper; the hook
  belongs on the service.
- A Tac Lua test unit does not fill Helper fields yet:
  `self:construct(Role)` leaves them `nil`.

## Amendment 2026-10-18: Tac Lua streams and sockets

This amendment replaces the `@StreamRequest` and `@SocketRequest` parts of
"Still refused in Tac Lua" in the two amendments above. The page's JavaScript
still holds each connection, as for every other Tac language, so a Lua
client only has to declare its streams and give the service a way to send.

- A Tac Lua `-- @HTTPClient` method may carry `-- @StreamRequest` or
  `-- @SocketRequest`. The method is empty and takes one parameter whose
  `---@param` model type says what each message holds:
  `function TicksClient:receive(tick)` with `---@param tick Tick`. A client
  has at most one `-- @SocketRequest` method. The rule table lists Lua for
  both markers in Tac.
- A Lua service that keeps the client in a typed `-- @Needs` field lists the
  client's streams in its `init` answer as `streams`, next to its listeners,
  and the page's event bus opens them when the service starts and closes
  them with its owner. Each message is checked against its model and
  published as that model's event, which a `-- @OnEvent` method receives. A
  service that does not need the client opens nothing. The limits are those
  of ADR 0058 and ADR 0060, and a service holds at most 16 such methods
  through its clients.
- Tachyon writes `TicksService.__tachyonClients = { ["ticks"] =
  "TicksClient" }` after the service for each held client that is not
  already a provider, and the page builds the client when it builds the
  service object. The field is not a page field. A client with only stream
  or socket methods has no behavior context and needs no Setup.
- A socket client gets a generated `client:send(message)`. It writes the
  message as JSON and waits for the page in the call's coroutine, like an
  HTTP request (the amendment above): it returns `nil` once the page has
  sent or queued the message, and otherwise raises a `BehaviorError` whose
  `behavior_code` is `HTTP_NETWORK` (for example, 16 messages already wait
  for the socket to open), `HTTP_MISSING` (no page), `HTTP_REQUEST` (no
  service call is running, for example in `-- @OnStart`), `HTTP_CAPACITY`
  (64 calls already wait), `HTTP_BODY` (more than 64 KiB, or a value that is
  not JSON) or `CANCELLED` (the owner went away). A Tac Lua test unit has no
  page, so `send` raises `HTTP_MISSING` there.

Evidence: `tac_lua::tests::stream_tests` builds a real Lua service to
WebAssembly and runs it under Node with the real event bus and fake
`EventSource` and `WebSocket` classes, with the checks of the Go test in
ADR 0090. It also covers the `init` streams, the generated `send`, a service
that holds no stream, and `HTTP_MISSING` in a test unit.

## Amendment 2026-10-18: Tac Lua cached reads, test units and the browser HTTP gate

- A `-- @Cached` read on a Tac Lua HTTP client is a public read: on a cache
  miss the browser request carries `publicRead`, so the browser may answer it
  from its HTTP cache. The first read of a key after a successful
  `-- @ClearCache` or `-- @UpdateCache` of that key carries `revalidate`, so
  the browser asks the server again (`cache: 'no-cache'`), as in Tac Ruby.
  A context remembers at most 256 such keys.
- A Tac Lua test unit now gives a service its HTTP client, Store or
  Converter: `self:construct(CounterService)` builds the service with each
  typed `-- @Needs` provider, and the provider uses the test's behavior
  context, so the TestSetup's `http` adapter answers. A `-- @Fake` field
  that is set replaces the provider. Without an adapter a request fails with
  `HTTP_REQUEST`, because a test unit has no browser.
- Executed-method coverage leaves out Lua methods with an empty body, such
  as the request methods of an HTTP client, because Tachyon writes them.

Evidence: `scripts/browser-http-browser-test.mjs --language=lua` builds
`website/content/docs/calling-the-server/browser-http-lua`, serves it with
`ty serve` and checks it in Chromium, Firefox and WebKit (on 2026-10-18 it
passed in Chromium in a Linux container, and in Firefox 153 and WebKit 26.5
on the macOS CI runner): the real Yon answer, a page that keeps running
while the server waits, cookies left out, the cached read and its
revalidation after `-- @ClearCache`, a Setup adapter that answers instead of
the network, and cancellation when one owner is disposed while another stays
live. The first-app tutorial has Lua tabs for step 4 and step 5.

## Amendment 2026-10-19: what the early Lua refusals mean today

This amendment supersedes the statements below. They stay as the history of
this decision.

- "Not yet" in "Decision". Topics work in both realms (the 2026-10-04
  amendment "Lua topics"). The rule table no longer ends a Lua refusal with
  "Lua gets it in a later step (ADR 0085)": every Lua cell of the table is
  accepted, so the text was removed from `annotation_rules/scan.rs`. What
  still stops the build is outside the rule table: a Yon Lua class that needs
  DataSource or RedisTemplate, or calls `relay(...)`, fails with `TY2015`,
  and a Lua schema marker on a `server/models` file fails with `TY1310`
  (ADR 0092).
- "`@InBackground` stays refused (`TY1308`)" in the 2026-09-30 events
  amendment. Yon Lua accepts it since the 2026-10-17 amendment "Yon Lua HTTP
  clients, Gateway and InBackground"; the test that runs it is
  `local_events.rs a_lua_in_background_listener_runs_after_the_others_and_publishes_nothing`.
  In the same paragraph, "A Lua class does not publish or listen to topics
  yet" is replaced by "Lua topics".
- "Topic stops the build in Lua" (2026-10-03). "Lua topics" (2026-10-04)
  already replaces it; the test that runs a Lua topic is
  `topic_events.rs lua_topic_events_cross_to_python_and_back`.
- "Still refused in Tac Lua" in the 2026-10-17 behavior annotation
  amendment and the 2026-10-18 HTTP client amendment (Helper, StreamRequest
  and SocketRequest) is replaced by "Tac Lua helpers" and "Tac Lua streams
  and sockets" (2026-10-18).
