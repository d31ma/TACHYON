# ADR 0085: Lua Is a Yon Language

- Status: Accepted; amended 2026-09-29 (Tac Lua services)
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
behavior annotations (for example Cached, Retry, Transactional),
StreamRoute and SocketRoute in Yon, topics, and behavior annotations in Tac. Resources are not available in Lua: DataSource,
RedisTemplate, gateway clients (`@HTTPClient`), Tac stores (`@Store`) and
`relay`. The rule table refuses `@HTTPClient` and `@Store` in Lua, and every
Lua refusal of the rule table ends with "Lua gets it in a later step
(ADR 0085)".

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
  one object never leaks into another. The page reads and changes every
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
