# ADR 0045: Go Is a Yon Language and a Tac Service Language on the Web

- Status: Accepted
- Amended by: [ADR 0050](0050-go-annotations-are-comments.md): Go annotations
  now use `// @Name` comments. The bracket spelling and tooling limitations
  recorded below describe the original decision, not the current syntax.
- Date: 2026-09-25
- Amended: 2026-10-11 (a Go service needs another Go service; see the
  amendment)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (the Yon languages),
  [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md) and
  [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md) (which
  service languages reach the web as WebAssembly)
- Preserves: every annotation name and its JSON arguments, Handler Protocol
  v1, the relay and Gateway rules (ADR 0042), models (ADR 0043) and the
  one-class rule (ADR 0044)
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Context

Tachyon's server languages are JavaScript, TypeScript, Python, Java, Kotlin,
C#, PHP, Rust and Dart. Go is common for server code, and it compiles to
WebAssembly for the browser (`GOOS=js GOARCH=wasm`), so it can also answer a
Tac service on the web, as Rust and C# do.

Go has no attribute or decorator syntax. Rust's attributes (`#[Service]`) are
part of Rust, and Tachyon removes its own markers before `rustc` compiles the
file.

## Decision

- Go (`.go`) is a Yon language. A route, a worker, a service, a role file, a
  Gateway caller or target, root middleware, and a test unit may be written
  in Go, with the same rules as the other compiled Yon languages.
- Go is a Tac service language on the web. A Go service compiles to
  WebAssembly with `GOOS=js GOARCH=wasm` and Go's `wasm_exec.js` glue. Native
  hosts render the same web bundle, so a Go service reaches every target
  through it.
- Go annotations use exactly the Rust spelling: `#[Service]`,
  `#[Autowired]`, `#[Cacheable({"cacheNames":["products"]})]`, and the same
  names, arguments and placement rules as Rust. A type's markers sit on the
  line before its `type Name struct` declaration; a method's markers sit on
  the line before its `func (receiver) Name(...)` declaration. Tachyon reads
  and removes them before it calls `go build`.
- A Go service is a struct type and its methods. Views and other languages
  reach its exported members.
- Go data types may live in `client/models/` and `server/models/` under the
  rules of ADR 0043. A Go file follows the one-class rule of ADR 0044: one
  struct type (or another single type) with its methods and imports.
- Go relay and Gateway calls follow ADR 0042, including the scrubbed
  environment and the remaining deadline.
- The Go toolchain is Go 1.27 or newer. `ty doctor` checks it. The build
  machine keeps it in an external toolchain folder that the toolchain setup
  script puts on the `PATH`.

## Consequences

- Go joins every multi-language example on the website and in the guides.
- An annotated Go file is not valid Go until Tachyon removes the markers.
  `gofmt`, `go vet` and the Go language server (`gopls`) cannot read it
  directly. This is the accepted price of using the Rust spelling.
- A Go WebAssembly module starts at about 2 MB, because it carries the Go
  runtime. Pages that use Go services load more than pages that use Rust.
  TinyGo could make modules smaller later; it is not used now.

## Phase 1 Notes

Phase 1 adds plain Go routes, workers and root middleware. These points are
not written above, or differ from how other languages spell them:

- Go root middleware declares `type Middleware struct{}` with the methods
  `Before(request)` and `After(request)`, not `before` and `after`
  (ADR 0044). A Go method whose name starts with a lower-case letter is not
  exported, so Tachyon names a Go method after its operation with the first
  letter in upper case. HTTP verbs are already upper case. The members that
  other code reaches, such as a Gateway target method, use the same rule. A
  lower-case `before` or `after` in Go middleware fails the build.
- A Go handler file may start with `package main`, and no other package.
  Tachyon adds `package main` when the file has none. Unlike Java, a Go
  `import` counts as code for the empty route-marker rule, because an
  imported package runs its `init` functions.
- The one type may be a struct, an interface or another defined type. A type
  alias (`type A = B`) and a grouped `type ( ... )` declaration are refused.
- A method may return a `*YonResponse` or any value that `encoding/json` can
  encode. A nil `*YonResponse` answers 204, so middleware can return `nil` to
  let the request pass. A route method has more choices (see Final Notes).

## Models Notes

The models phase adds Go files to `client/models/` and `server/models/`.
These points are not written above, or differ from how other languages
spell them:

- A Go models file holds one type: a struct, another defined type (such as
  `type Kind string`) or a type alias, with the imports it needs. Rust models
  may declare several types in one file; Go follows the one-class rule
  instead. A struct may carry field tags such as `json:"title"`.
- Go has no enum. ADR 0043 refuses top-level constants in every language, so
  a Go model cannot declare `const` values for its defined type; code writes
  the value itself, for example `Kind("long")`.
- An interface type, a method, a function, a variable, a constant, a grouped
  `type ( ... )` declaration and a blank import (`import _ "embed"`, which
  exists only to run `init` code) fail with TY1310. A Go struct body holds
  only fields, so the check does not read it.
- A Go models file may start with `package main`, and no other package, as a
  Go handler file. A separate `models` package would need an import path
  that names the module Tachyon generates for each build, so Tachyon
  compiles every Go models file of the realm as another file of the
  program's `package main` instead. Go code uses a model by its type name
  alone, without an import, as Swift does. Two types with the same name in
  one program fail to compile, as in Swift.
- Each models file is staged as `model_<n>.go` with a `//line` directive, so
  a compile error names the authored path, for example
  `server/models/Guide.go:3`.
- A Go route, worker and root middleware receive the server models. A Go web
  service receives the client models as files of its own package (see Web
  Service Notes).

## Relay and Resource Notes

Go relay, managed resource clients and the Gateway transport follow ADR 0042
and the Rust prelude. These points are not written above, or differ from how
other languages spell them:

- The relay function is `Relay(command, request)`, with an upper-case first
  letter, like the other exported names. The resource clients are
  `DataSource{}` (`Query`, `Execute`, `Command`) and `RedisTemplate{}`
  (`Get`, `Set`, `Delete`). The Gateway transport is
  `YonGateway.Call(key, argumentsJSON)`; it returns the answer as JSON text,
  `{"result":...}` or `{"error":"invalid|timeout|capacity|adapter"}`.
- Go hides a name only from other packages. Handler code shares
  `package main` with the Tachyon prelude, so the prelude cannot keep the
  relay deadline there. Tachyon adds a second package, `yoninvocation`, to
  every Go build. It reads the request, records the start time and the
  relay environment before `main` runs, and exposes them only through
  read-only functions.
- A resource call returns `(any, error)`. Numbers keep their exact digits
  (`json.Number`). A handler method may return `(value, error)`: a
  `YonResourceError` answers HTTP 500 with the fixed resource message, as in
  Rust, and any other error fails the handler like an uncaught exception.
- On a relay timeout the prelude kills the program it started. As in Rust,
  the supervisor removes the rest of the handler's process group when the
  invocation ends.

## Web Service Notes

Go Tac services and Go client test units follow ADR 0035 and ADR 0037. These
points are not written above, or differ from how other languages spell them:

- A Go service is web-only, like a C# service on every target except
  Windows. No native host links Go, so every target runs the Go module in its
  web view. When a service has several WebAssembly languages, Rust wins, then
  Go, then C#: the Go module is smaller than the .NET runtime.
- All Go services of an application build into one module,
  `.tachyon/services/go/services.wasm`, beside a copy of Go's
  `wasm_exec.js` from the toolchain that built it. The page fetches both
  files from its own origin; the module is not embedded as base64. So, as for
  C#, a page opened from `file:` cannot load a Go service.
- Each service compiles in its own Go package, as each Rust service sits in
  its own module. A service's behavior code and its copy of the client models
  stay inside that package, so two services never clash. (Replaced: all Go
  services of a page now share one package; see the amendment of
  2026-10-11.) The protocol runtime
  is a framework package, and a service names `ApplicationEventPublisher`
  without an import. Its method is
  `PublishEvent(event string, value any, retain bool)`.
- The runtime reads and writes JSON itself. Go's `encoding/json` would add
  about 2.2 MB to every module. A value crosses the bridge the way a Rust
  `TacValue` does: text that holds a number fills a number, and a number
  fills text. A struct crosses as an object whose keys are the `json` tags or
  the field names with a lower-case first letter.
- A method reaches views when it is exported, takes no variadic, `func` or
  `chan` parameter, and returns nothing, one value, an `error`, or a value and
  an `error`. A non-nil error answers `{"error": message}`, so the view's call
  throws with the error text, as in C#.
- A Go client test unit runs natively with `go run`, against the same protocol
  runtime without `syscall/js`.

## Annotation Notes

Go server annotations follow the Rust spelling and semantics. These points are
not written above, or differ from how other languages spell them:

- Go has no constructor. Tachyon builds a managed type as `&T{}` and assigns
  its injected fields. Each `#[Inject]` dependency goes to the one field whose
  type is `*Dep` or `Dep`; two such fields fail with TY2015.
  `#[RequiredArgsConstructor]` injects every named field and
  `#[Autowired]` one field. A type that is not managed is built with a
  struct literal, so a behavior `#[RequiredArgsConstructor]` generates
  nothing.
- A behavior method has a named receiver, `BehaviorValue` parameters and the
  results `(BehaviorValue, error)`. `BehaviorValue` is `any`; Tachyon turns
  each value into JSON data (`nil`, `bool`, `float64`, `string`, `[]any`,
  `map[string]any`), so numbers are `float64`, as in Rust. The behavior
  runtime reads and writes JSON itself instead of importing `encoding/json`,
  which would add about 2.2 MB to a web service module. A behavior
  error is `*BehaviorError{Code, Message, Path}`; `BehaviorErrorCode(err)`
  reads its code. Go has no generic methods, so Rust's `Behaviors::create`,
  `validate`, `properties`, `values` and `repository` are the functions
  `BehaviorsCreate[T]`, `BehaviorsValidate[T]`, `BehaviorsProperties[T]`,
  `BehaviorsValues[T]` and `BehaviorsRepository[T]`. A configuration type
  declares `Configure(execution *BehaviorExecution)
  (*BehaviorConfiguration, error)`.
- An abstract declaration is a Go `interface`. Tachyon turns it into a struct
  with a `behaviors *BehaviorContext` field and writes its methods.
  Derived query names are written in Go style (`FindByStatus`), and a
  `Gateway` key and default target method use the Go method name.
- A test case is a method whose name starts with `Test` or `test` and returns
  `error`. The test type declares `behaviors *BehaviorContext`.
- An event listener takes one `any` payload. Go runs listeners synchronously,
  so `#[Async]` fails as in the other synchronous languages.
- Generated code and every role file become separate files of `package main`,
  so no generated code adds an import to an authored file, and a wrapped
  method keeps its line numbers. Generated names start with `__yon`, which an
  annotated Go file cannot use, or with `Behavior`/`behavior`, which a Go
  behavior type cannot use.
- A Go type can be a Gateway caller and a Gateway target (see Gateway
  Notes).

## Gateway Notes

A Go role file can be a Gateway class target, as ADR 0042 describes. These
points are not written above, or differ from how other languages spell them:

- A caller names the target method as usual, for example `"summarize"`.
  Tachyon calls the Go method with the same name and an upper-case first
  letter, `Summarize`, the same rule as for middleware phases. A missing or
  unexported method fails with `TY1312`, and the message names the exported
  method to write.
- Rust targets must use `&self`. A Go target may use a value or a pointer
  receiver. The generated unit holds the target as `*T` from the Go
  container, and Go calls both kinds of method through a pointer. Each call
  runs in its own process, so a pointer receiver cannot change state that
  another call sees.
- The method takes `BehaviorValue` parameters and returns
  `(BehaviorValue, error)` or `BehaviorValue`. Named results such as
  `(value BehaviorValue, err error)` fail with `TY1312`. Go has no overloads;
  two methods with the same name are ambiguous and fail with `TY1312`.
- A returned error or a panic reaches the caller as `adapter`, like an
  exception in the other languages.

## Website Notes

The website phase converts three of the site's own services to Go and adds Go
to every multi-language example. These points are not written above, or
differ from how other languages spell them:

- Go's `wasm_exec.js` uses a global `fs` only when the page has none. Many
  classic scripts declare a global function with that short name; the site's
  own DuVay components do. So the page keeps its own `fs`: Tachyon gives Go's
  runtime its file-system stub only while the module starts, and then puts
  the page's value back. Go reads that stub once, when it starts. After that,
  only Go's built-in `print` still looks up the global name.
- Role files of a Go web service, such as a bare `#[Repository]` or
  `#[Configuration]` in `client/repositories/` or `client/configurations/`,
  may carry no behavior annotation. Tachyon removes every remaining marker
  line before `go build`, in the web build and in a client test unit.
- The site's Go services build into one module of 2,755,077 bytes
  (781,957 bytes with gzip), under the 4 MiB limit of the service worker's
  precache.
- The snippet verifier builds, serves and tests every Go example with the
  `go` toolchain. It cannot check an annotated Go fragment on its own,
  because `gofmt` cannot read the markers; it checks a Go fragment without
  markers with `gofmt -e`.
- A Go handler build also stages the plain Go files of `server/shared/` (see
  Final Notes).

## Final Notes

The last phase closes the gaps that the website phase found. These points are
not written above, or differ from how other languages spell them:

- A route method of a plain `Handler` may take the request,
  `GET(request *YonRequest)`, or no parameter, `GET()`, as a controller
  method may. It may return nothing, one value, an `error`, a value and an
  `error`, or a `*YonResponse`. Nothing answers `null` with status 200, as a
  JavaScript method that returns `undefined` does. A non-nil `error` follows
  the rule of the Relay and Resource Notes. A method with more than one
  parameter or with other results fails with `TY2015`.
- A Go route, worker, root middleware, Gateway unit and server test unit
  compile every `.go` file under `server/shared/` as another file of their
  `package main`, staged as `shared_<n>.go` with a `//line` directive, so a
  compile error names the authored path. Go code calls a shared type or
  function by its name, without an import, as it uses a model. The other
  compiled Yon languages build only the handler, its role files and the
  models; Go can stage shared code because its package is a folder of files.
  A shared file is plain Go: it may declare several types, functions and
  variables, and the one-class rule does not apply, as for shared code in
  every language. It may start with `package main` and no other package, and
  it cannot carry a Tachyon annotation; both fail with `TY1310`. The build
  cache key includes the shared files. The snippet verifier builds a Go
  example from `server/shared/` with a probe route and calls it.
- In Tac, `Component` is a browser role. A Go web service, like a Rust or
  C# web service and a Swift or Kotlin native service, cannot use a
  `#[Component]` delegate; the build fails with `TY1308`. The message names
  the delegate file and the marker, not the service file. A Go service keeps
  that state itself or uses a bare `#[Repository]`, `#[Configuration]` or
  `#[Entity]` role file.

These Go refusals are deliberate design rules:

- A Go route does not stream. A streaming annotation on a Go method fails, as
  in Rust and Java; a Go route returns one response.
- Go is web-only in Tac. No native host links Go, so a Go service is never a
  native companion; every target runs its WebAssembly module in the web
  view. A page opened from `file:` cannot load it, as for C#.
- A Tac `#[Component]` delegate is refused beside a Go service (above).
- An event listener runs synchronously, so `#[Async]` fails.
- A Go model declares no constant, method or function (Models Notes).
- `server/shared/` holds no annotated Go code (above).
- `gofmt`, `go vet` and `gopls` cannot read an annotated Go file
  (Consequences).

## Amendment (2026-10-11): a Go service needs another Go service

A Go Tac service can mark a field with `// @Needs` and give it another Go
service's type, as JavaScript, Rust (its typed page graph) and C++
(ADR 0099, `TacNeeds::of<T>`) already allow. Before this amendment the build
left such a field alone, and Go stopped with `undefined:` because each
service had its own package.

- All Go services of a page compile into one Go package, `page`, with the
  Setup, client, Store and model files of the page once. Each service keeps
  its own factory, `TacCreate<Class>`. So `counter *CounterService` names
  the other service without an import, and the needing service calls its
  methods directly, without a trip through the page.
- The field holds the instance that the page uses. The build resolves the
  field by its type to an instance name of the page. When the page has the
  type under two instance names, the field name must be one of them. The
  factory assigns `tac.Need("counter")` after the Store and client
  injection. The page package registers each needed instance with
  `tac.Shared`, and the runtime keeps one node for it: the page's own
  activation of that instance and every field that needs it share the node.
  An instance that no service needs keeps one Go value for each activation,
  as before.
- The needed service is created and started first. Its OnStart runs once,
  before the OnStart of the service that needs it, and a later page
  activation of the same instance does not run it again. Events that it
  publishes while a page call runs go out through the owner of that call,
  so they reach every listener on the page.
- Disposal counts holders: each page activation holds its own node, and each
  node holds the nodes it needs. When the last holder lets go, the node runs
  OnStop and then lets go of its needs, newest first. So a service stops
  before the services it needs, whatever order the page disposes of them in,
  and `host-dispose` keeps the same order.
- A needed instance lives as long as the page, as a JavaScript singleton and
  a Rust singleton node do. When its last holder lets go while other Go
  services of the page are still active, the node stays, started and with
  its state, and the next holder gets it back. When the last Go activation
  of the page is disposed, the runtime stops the nodes that stayed, newest
  first. A node whose OnStart failed does not stay.
- The field never reaches the page as a data field. A `Needs` field is
  lowerCamel in Go (TY1332), so it is never exported.
- A needed Go service is a page singleton. The build refuses, with TY1406 at
  the field: a field that holds the service by value, because a copy is not
  shared; a needed service that has one instance for each component
  (`Lifetime` prototype or `NeedsAll`), because Go cannot tell which
  component's instance a field means, which Rust refuses too; services that
  need each other in a circle, as C++ does; and a type that the page has
  under two instance names when the field name is neither.
- The listeners of a needed service hear events when the page also
  activates that service. A service that only another service needs has no
  page owner, so its own `OnEvent` methods are not registered.
- Standard Go and TinyGo build the same program; the tests build both and
  run them under Node.
