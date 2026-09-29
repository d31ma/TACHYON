# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Released Tachyon products use calendar versioning (CalVer) in UTC. Public
machine contracts have their own, independent major versions.

## [Unreleased]

### Breaking

- Every field, method parameter and return value of a class states its type
  (`TY1356`, ADR 0079), the sixth design rule of ADR 0069, in public and
  private members of every role folder. Where the syntax has no types, a type
  comment carries the type: JSDoc `@param {T} name`, `@returns {T}` and
  `/** @type {T} */` in JavaScript, YARD `# @param name [T]` and
  `# @return [T]` (also above `attr_reader`) in Ruby. Python, PHP and
  TypeScript write their native types (TypeScript: no implicit `any`). A
  JavaScript method without a returned value, a constructor, and a
  JavaScript or Python field that starts with one literal need no type. The
  build checks that every name in a Ruby YARD type is a Ruby, project or
  Tachyon class. A class file may not switch the checker off: `@ts-nocheck`,
  `@ts-ignore`, `@ts-expect-error`, `# type: ignore`, `# mypy: ignore-errors`
  and `# pyright: ignore` are refused with the same code (test files may
  still use `@ts-expect-error`). `ty migrate fix --write` adds the known
  types (injected fields, `-> None`, `: void`) and lists the rest. The switch
  is `tac.types.check` / `types.check`; the rule is off by default until the
  website, fixtures, corpus and `ty init` templates pass it. The website's
  own services no longer start with `// @ts-nocheck`.
- The type rule (`TY1356`) also refuses a JavaScript class field that has no
  starting value, that the constructor or an `@OnStart` method does not set,
  and that Tachyon does not fill (`@Needs`, `@Setting`, `@Fake`, a class
  marked `@NeedsAll`, an `@Entity`, or `applicationEventPublisher`). The help
  shows the fixes. It replaces `tsc`'s `strictPropertyInitialization` check, which the
  `jsconfig.json` of `ty init` now turns off for realms with JavaScript code,
  because JavaScript cannot mark an injected field with `!`. A realm that uses
  only TypeScript keeps the full `strict` check (ADR 0079 amendment).

- Tac views must be accessible (`TY1336`, ADR 0074). The build refuses a
  picture without `alt` (an empty `alt` needs `role="presentation"` or
  `aria-hidden="true"`), a button or link without a name, a field without a
  label, a skipped heading level, a `tabindex` above 0, an unknown or
  misused `aria-*` attribute, and an `<iframe>` without a title. `ty migrate
  check` lists every finding; `tac.accessibility.check: false` in
  `client/manifest.json` turns the rule off while a project migrates.
- A release build refuses a route that loads more script or WebAssembly than
  its budget (`TY1337`, ADR 0074): `tac.budgets.script` (default 200 KB) and
  `tac.budgets.webassembly` (default 3584 KB), counted as the Brotli bytes
  that Tachyon sends. The error names the five biggest files.
  `tac.budgets.check: false` turns the check off.
- A build refuses an API that breaks `server/api-baseline.json` without a new
  major version of `tac.application.version` (`TY1338`, ADR 0074): a removed
  route, method, model or field, a narrowed type, a new required value or a
  changed status. The new `ty api freeze` writes the baseline after a
  release; a project without one is not checked.
- A public method or field that nothing outside its class uses is a build
  error (`TY1333`, ADR 0071), the fifth design rule of ADR 0069. It covers the
  classes in `services`, `helpers`, `controllers` and `utils` of both realms.
  A member stays public when a Tac view uses it (`cart.total()`), when another
  class of the realm calls it, or when Tachyon calls it: an annotation on the
  member or on one of its parameters, a field without a starting value, a
  class that a setup hands to Tachyon, `IdentityProvider.identify`, and names
  such as `hotState` or `toString`. A call from a test does not count. The
  help shows the private form of the language (`private`, `_name` in
  JavaScript, Dart and Python, a lowercase Go name, no `pub` in Rust,
  `private def` in Ruby). `ty migrate fix --write` makes the simple cases
  private. The rule is on by default; `tac.visibility.check` /
  `visibility.check` set to `false` turns it off for one realm while it
  migrates. The website,
  its examples and the `ty init` templates pass it: examples that had no
  caller gained a small view or controller, and website tests now check
  public members only. The `NeedsAll` field reader skips JavaScript `#name`
  fields.

- Dart browser services now compile to WebAssembly with garbage collection
  (WasmGC, `dart compile wasm -O2`) instead of JavaScript (ADR 0033
  amendment). The build publishes `tachyon-dart.wasm` and the SDK's
  `tachyon-dart-loader.js` beside `tachyon-dart.js`, which every service
  module still imports. A Dart service needs Chrome or Edge 119, Firefox 120
  or Safari 18.2 and newer; an older browser gets one clear error, and there
  is no JavaScript fallback. The Dart SDK must be 3.13 or newer. The bridge
  keeps the old behavior: whole numbers from JavaScript arrive as `int`, and
  a Dart exception reaches JavaScript as an `Error` with the Dart message.
  On the website the program is 77 KB instead of 101 KB (28.8 KB instead of
  31.0 KB gzip) and starts in 7.7 ms instead of 6.3 ms in Chromium.
- A field that holds an injected class, a `Needs` field or a `NeedsAll`
  dependency, must use the language's variable case (ADR 0057 amendment).
  The build refuses `LedgerStore LedgerStore;`, a Python or Rust
  `ledgerStore` and a private C# `store` with `TY1332` and names the
  spelling to write: `ledgerStore` (JavaScript, TypeScript, Java, Kotlin,
  Swift, Dart, Go), `$ledgerStore` (PHP), `ledger_store` (Python, Ruby,
  Rust), `_ledgerStore` (a private or internal C# field) or `LedgerStore` (a
  C# property or public field). Only the case is checked, not the words.
  `ty migrate fix --write` renames the field, its uses in the class and Tac
  view bindings. The website's C# examples now write `_service`-style
  fields, and its Python and Rust `CheckoutService` examples write
  `application_event_publisher`.
- A field that Tachyon fills, a `Needs` field or a `NeedsAll` field without
  an initial value, may no longer be declared nullable (ADR 0057
  amendment). It is never null when a method runs, so the build refuses
  `label?: string`, `title: string | null`, `final String? path;`,
  `val path: String?`, `var label: String?`, `required string? Title`,
  `?Store $store`, `store: Store | None` and `Option<T>` with `TY1331`, in
  both realms, and names the non-null form to write (`label!: string`,
  `late final String path;`, `lateinit var store: Store`, ...). A TypeScript
  `label?:` field in a `NeedsAll` class is no longer optional state: give a
  field that may stay empty a starting value. `ty migrate fix --write`
  rewrites one-line declarations. The website's Dart `PagerService` and
  `SidebarService` now declare `late final` fields.
- `@Component` is now `@Helper`, because `client/components/` holds views
  (ADR 0047 amendment). The role folder `delegates/` is now `helpers/` in
  both realms, a helper class name ends in `Helper`, and `Gateway` command
  programs live under `server/helpers/`. The build refuses `Component` with
  its new name (`TY1308`) and a file under `delegates/` with the `helpers`
  folder (`TY1310`). `ty migrate fix --write` moves the files, renames the
  classes, their `Needs` fields and types and the old annotation, and
  rewrites paths such as `server/delegates/report.rb`. A new guide, "Helpers,
  utilities and private methods", gives the one question that decides between
  a helper, a utility, a private method and a service. The website's
  `NavigationComponent` holds only static methods, so it is now the utility
  `client/utils/NavigationUtils.dart`.
- Four design rules are build errors in user projects, in every Tac and Yon
  language (ADR 0069). Dependencies point down (`TY1327`): a loop of classes
  that need each other, a class that needs a class of a higher layer
  (controllers, then services, then helpers, then stores, repositories and
  clients), or any class that needs a controller stops the build, and the message
  names the whole loop. Names are plain English (`TY1328`): every word of a
  class, method, field or parameter name is in the English word list that
  `ty` ships, a short technical word such as `id` or `json`, or a word of
  `tac.names.words` / `names.words`; clipped words such as `usr`, `cfg`,
  `msg` or `err` are refused with the whole word. Signatures are small
  (`TY1329`): at most 4 parameters per method (request values marked
  `FromPath`, `FromQuery`, `FromBody`, `FromHeader` or `FromCookie` do not
  count) and at most 5 needed classes per class. Values have names
  (`TY1330`): a text or number other than 0, 1 and -1 written twice in a
  class, or a number in a condition, becomes a class constant. While a
  project migrates, `tac.<rule>.check` in `client/manifest.json` and
  `<rule>.check` in `server/application.json` (`dependencies`, `names`,
  `signatures`, `constants`) set to `false` turn one rule off for one realm;
  `ty migrate check` lists every place either way.
- A service that is linked into a native app (Rust, C#, Swift or Kotlin)
  can no longer name Tachyon's browser APIs or their built-in event models:
  the build stops with `TY1317`, because such a service has no page. Before,
  `Clock`, `SecureRandom` and the other APIs failed only when called.
- No secrets in the source (ADR 0068). A password, token or key in the code,
  a template or public settings stops the build with `TY1324`: the usual AWS,
  Google, Azure, GitHub, Stripe and Slack shapes, private keys, JSON Web
  Tokens, and long random values given to names such as `password`, `secret`,
  `token` or `apiKey`. Put the value in `.env`. Only a file with the comment
  `tachyon: example secrets` may keep values that say they are examples.
- No text runs as code, and queries take parameters (ADR 0068). `eval`,
  `new Function`, Python `exec`, Ruby `instance_eval` with a text, PHP `eval`,
  Java/Kotlin script engines and C# scripting stop the build with `TY1325`, as
  does query text built from pieces (`+`, `${...}`, `f"..."`, `$"..."`,
  `fmt.Sprintf`, ...) in a `query`, `execute`, `exec` or `prepare` call on a
  data source. Write the query as one literal or an UPPER_CASE constant and pass
  the values as parameters.
- Outgoing requests go only to declared hosts (ADR 0068). List them in
  `tac.outgoing.hosts` (`client/manifest.json`) and `outgoing.hosts`
  (`server/application.json`). A request call whose URL names another host, or
  a host built at run time, stops the build with `TY1326`. At run time the page's
  `connect-src` allows only the listed hosts, and Yon handler processes send
  their requests through a local guard that answers 403 to other hosts.
- Every route says who may call it (ADR 0068). Write `@Public` (anyone),
  `@SignedIn` (any signed-in user) or `@Allow(Role, ...)` (only these roles)
  on the controller class or on a route method; the method wins. A route
  without a rule stops the build with `TY1322`, and nothing turns the check
  off. Roles are types: an empty class in `server/roles/<Role>.<ext>`. One
  `@IdentityProvider` class in `server/controllers/` says who is signed in:
  its method `identify` reads the native request and returns the user and
  their roles, or no value. The server calls it before every `@SignedIn` or
  `@Allow` route in any of the eleven Yon languages, answers 401 without an
  identity and 403 without a role, and never runs the method then. A guarded
  route without an identity provider, or a second identity provider, stops
  the build with `TY1323`. `ty migrate fix` writes `@Public` only on the
  controllers it makes from old file routes, which were open before; it never
  adds a rule to an existing controller.
  Every example, fixture, corpus project and `ty init` template now says
  `@Public`. The guide "Who may call a route" shows it in every Yon language.
- Every class file in `client/` or `server/` is named after its class, as in
  Java (ADR 0067). The file name without the extension is the class name with
  the same spelling and case, in every language: models, test units, test
  setups, utilities, delegates and error types, as role files already were.
  `server/models/Order.ts` that declares `Invoice` now fails with `TY1318`.
  Views and component files, `shared/` code, workers and the root
  `middleware.<ext>` keep their names. `ty migrate fix --write` renames the
  file after its class, or renames the class when other files use the file
  name. The relays example now calls `server/delegates/ReportDelegate.rb`.
  `tools/cohesion-scan` also counts Rust files that are not named after their
  type in snake_case; `tools/cohesion-scan/FILE_NAME_BASELINE` (162) keeps
  the count from growing.
- A models file in `client/models/` or `server/models/` holds one model, and
  the file is named after it (ADR 0067). `server/models/Order.ts` with
  `Order` and `Status`, or `Order.py` with two dataclasses, now fails with
  `TY1318`, and the message names each model. `ty migrate fix --write` splits
  such a file into one file per model next to it: each new file keeps the
  imports and the comments, attributes, decorators or YARD types of its
  model, gets an import of the other models that it uses in TypeScript, Rust
  and Dart, and the TypeScript, JavaScript and Dart files that import a moved
  model import it from its new file. The model readers now name every
  declaration form: TypeScript generic type aliases and `export default
  interface`, C# `record class`, Go generic structs with their fields, Rust
  `type`, Kotlin and Swift `typealias` and Dart `typedef` aliases, and JSDoc
  `@typedef Name` without a type.

- Tac code in Rust, Go, C#, Dart or Ruby may not call the browser directly
  (ADR 0065). `ty build`, `ty preview`, `ty serve` and `ty bundle` stop with
  `TY1317` on Go `syscall/js`; Dart `dart:js_interop`, `dart:js`,
  `dart:js_util`, `dart:html`, `package:web` and `package:js`; C#
  `System.Runtime.InteropServices.JavaScript`, `JSImport`, `JSExport` and
  `JSHost`; Rust `wasm_bindgen`, `web_sys`, `js_sys` and
  `wasm_import_module`; and Ruby `TachyonWeb` and `JS`. Comments and strings
  do not count, and Tachyon's own page code is not checked. Put the browser
  work in a JavaScript or TypeScript service and send the result with an
  event. The guide "What each service language can use in the browser"
  lists what each language can use in the page, with the tests that check
  it (`tac_web::reach_tests`).
- Duplicated code is refused (ADR 0064). Tachyon runs its own copy-paste
  detection with SonarQube's rules and sends nothing out: a block of at least
  100 tokens over at least 10 lines that repeats in the same language and
  realm, even with other names or values, fails `ty build`, `ty preview`,
  `ty serve`, `ty bundle` and `ty test` with `TY1315`. Imports, annotation
  lines, tests, models, entities, generated files and lists of values do not
  count. The message names both places and tells you to create
  `client/utils/<Name>Utils.<ext>` or `server/utils/<Name>Utils.<ext>` with a
  static method. `ty migrate check` lists every duplicate
  (`duplication.block`). While you migrate, set `tac.duplication.check` in
  `client/manifest.json` or `duplication.check` in `server/application.json`
  to `false`.
- New utility classes in `client/utils/` and `server/utils/`: one
  `<Topic>Utils` class per file, no annotation, only static methods and no
  state (`TY1310` for the layout, `TY1316` for a field, a stored value or an
  instance method). Every program of the realm gets them with no import,
  `composed.d.ts` declares them, test units can test them, `ty test
  --coverage` counts their public methods, and their methods follow the
  cohesion rule (`TY1314`).
- The repository test now also keeps the number of duplicated blocks in
  Tachyon's own Rust source at or below
  `tools/cohesion-scan/DUPLICATION_BASELINE`; `cargo run -p cohesion-scan`
  lists them next to the cohesion and structure findings.
- Services and controllers must be cohesive (ADR 0063). In every Tac service
  (`client/services`), Yon service (`server/services`) and Yon controller
  (`server/controllers`), a public method only coordinates: it calls injected
  services and private methods, stores or returns their results, and builds
  values. It may not hold a branch (`if`, `unless`, `guard`, `switch`,
  `match`, `when`, `case`, `select`, a ternary), a loop (including an iterator
  call such as `filter` or `each` whose lambda holds a decision, and a
  comprehension), a short-circuit operator (`&&`, `||`, `??`, `and`, `or`,
  Kotlin's and PHP's `?:`) or `try`/`catch`/`except`/`rescue`, also inside a
  lambda that it writes. A private method holds at most one decision or loop
  (an `else if` chain and the `&&`/`||` of its condition are one decision), and
  a private method that decides may not call another private method that
  decides. The build refuses a break with `TY1314`; the message names the
  method and the construct and shows the private spelling of the language.
  `ty migrate check` lists every place (`cohesion.method`). While you
  migrate, `"tac": {"cohesion": {"check": false}}` in `client/manifest.json`
  or `"cohesion": {"check": false}` in `server/application.json` turns the
  check off for one realm.

- A Tac view now refuses braces in a plain attribute, such as
  `title="{name}"`, `class="card {kind}"` or `hidden="{open ? '' : 'hidden'}"`,
  with `TY1313` and a span on the attribute. Braces work only in text; before,
  the build wrote the braces out as literal text with no warning. The error
  names the fix: `:title="name"`, `:class="'card ' + kind"`, or for a
  true-or-false attribute `:hidden="condition"` (a false value leaves the
  attribute out). Literal braces are still allowed: a regular-expression count
  such as `pattern="[0-9]{3}"`, JSON data, `\{`, and text inside `<style>`.
  `ty migrate fix --write` turns a whole-value case such as `title="{name}"`
  into `:title="name"`.

- `YonRequest` and `YonResponse` are removed (ADR 0062). Every Yon language
  now uses its own request and response types: a Fetch `Request` and
  `Response` in JavaScript and TypeScript, `urllib.request.Request` and an
  `(HTTPStatus, headers, body)` tuple in Python, a `Net::HTTP` request and a
  `[status, headers, body]` array in Ruby, the superglobals with
  `http_response_code()`, `header()` and `echo` in PHP, `HttpExchange` in Java
  and Kotlin, `HttpRequestMessage` and `HttpResponseMessage` in C#,
  `http::Request<Vec<u8>>` and `http::Response<Vec<u8>>` in Rust, shelf
  `Request` and `Response` in Dart, and `*http.Request` and `*http.Response`
  in Go. Middleware, `relay(command, request)`, plain handlers, error
  handlers and the unmarked route parameter all receive the native request;
  answering nothing from middleware lets the request pass. The build refuses
  `YonRequest`, `YonResponse`, `YonJSON` and `YonEmpty` with TY2015 and names
  the native type. `ty migrate fix --write` removes an unused `YonRequest`
  parameter and lists the other uses. JavaScript `{ status, headers, body }`
  objects and Python and Ruby dictionaries with those keys are models now.
  Rust handlers build with Cargo and the `http` crate (readiness probes
  `cargo`); Dart handlers fetch `shelf`. A PHP method takes no request
  parameter. Relay programs no longer receive path parameters, and the
  handler protocol frame carries the new optional `target` and the header
  `x-tachyon-request-id`.
- PHP Yon code now runs with `php-cgi` instead of `php`, so `header()`,
  `header_remove()` and `headers_list()` are PHP's own functions (ADR 0062).
  Routes, middleware, relays, error handlers, sockets, topic events and test
  units all start `php-cgi`. Install it before you upgrade: Homebrew's `php`
  includes it, Debian and Ubuntu have the `php-cgi` package, and the Windows
  PHP zip file has `php-cgi.exe`. Readiness and `ty doctor` look for
  `php-cgi` and explain how to install it.

- Tac service fields keep their values through `@SessionState`
  (sessionStorage, the old `$name`) and `@LocalState` (localStorage, the old
  `$$name`) instead of name prefixes (ADR 0059). A `$` or `$$` field in a
  JavaScript, TypeScript or Dart Tac service is refused with the marker to
  write; `ty migrate fix --write` converts the fields, their uses in the
  class and the view expressions. A value saved under the old key moves to
  the new key once.
- `@StreamRoute(path?)` replaces `@GetRoute` with `@Stream` on Yon controller
  methods (ADR 0058): one marker means a GET route that answers with
  server-sent events, and the method name starts with `stream`
  (`streamTicks`, `StreamTicks`, `stream_ticks`). `@Stream` is retired, and a
  generator under `@GetRoute` or another verb route is refused with a hint.
  `ty migrate fix --write` converts `@GetRoute` with `@Stream` and renames a
  `get` method when that is safe. A finished stream now ends with
  `event: end`, and `/api.json` marks the route with
  `"produces": "text/event-stream"`.

- A Tac `@NeedsAll` service now requires every field without an initial value
  (Lombok `@RequiredArgsConstructor` rules, ADR 0057); fields with an initial
  value are plain state. Collaborators are injected; every other required
  field must come from the component's props. The build refuses a component
  usage that does not pass one (`TY1407`: "DrawerService needs `open`; pass
  :open="..." to <docs-sidebar> (from <docs-layout>)"), including a forwarded
  `:open="open"` that the layout never receives, and the browser runtime
  refuses a missing or `null` value before it creates the service. In Rust a
  struct with `Default` has no required fields, in Go only a field marked
  `// @Required` is required; in Ruby
  every `attr_*` of a NeedsAll class is required. A JavaScript or TypeScript
  service that uses managed clients now reads a field that names no client from
  the props instead of refusing it.
- Yon `@NeedsAll` follows the same rule (ADR 0057). Only fields without an
  initial value are injected; a required field with no provider stops the
  build with the class, the field and the type. Ruby Yon classes can use
  `#[NeedsAll]` with `attr_reader` fields. Compiled Tac services (Dart, Rust,
  C#, Go, Swift, Kotlin, Ruby) receive only their required props, so a prop no
  longer fills an initialized field. A Rust NeedsAll struct without `Default`
  needs no constructor, a C# `required` member is a prop field, C# prop
  names are lowerCamel (`Docs` is `docs`), and a Go field is a prop field
  only when it is marked `// @Required`.
- Every Python marker on a field or a parameter now goes inside the type with
  `typing.Annotated` (ADR 0056), so the file stays valid Python:
  `repo: Annotated[OrderStore, needs]`, `count: Annotated[int, min(1), max(100)] = 1`,
  `port: Annotated[int, setting("${yon.server.port:8080}")] = 0`,
  `query: Annotated[OrderQuery, check]`, `key: Annotated[str, cache_key]`.
  The first argument is the declared type. The old `@not_empty` line above a
  field and the `@check input` parameter form are refused with a message that
  shows the new form; `ty migrate fix --write` converts them and adds
  `from typing import Annotated`. Class and method markers stay decorators.
  Website snippet verification now runs `python3 -m py_compile` over every
  Python file in `website/content`.
- `NeedsAll` takes no arguments (ADR 0056): every declared field can be
  filled, like Lombok's `@RequiredArgsConstructor`. `@NeedsAll("a", "b")` and
  every other field list is refused; `ty migrate fix --write` removes the list.
  A class with `NeedsAll` may not declare a constructor (`constructor`,
  `__init__`, `__construct`, `initialize`, `init`, `fn new`, or a Java,
  Kotlin, C# or Dart constructor): NeedsAll builds the class. Ruby services set
  their starting values in an `#[OnStart]` method instead of `initialize`.
- Python route and client parameter markers go inside the parameter's type, so
  a controller or client file is valid Python (ADR 0048, amended):
  `order: Annotated[Order, from_body]`, `id: Annotated[int, from_path]`,
  `trace: Annotated[str, from_header("X-Trace")]`, and on a client
  `id: Annotated[str, path_variable]`. Add `from typing import Annotated`
  (now an allowed import). The first `Annotated` argument is the type that
  validation uses. The old form `@from_body order: Order` is refused with a
  message that shows the new form; `ty migrate fix --write` converts it.
- Every annotation checks the class, method, field or parameter it is written
  on (ADR 0056). One rule table in `annotation_rules.rs` is used by every
  language in Tac and Yon, and each message names the annotation, where it
  belongs, what it means, and its file and line. These mistakes were accepted
  before and are now refused:
  - an annotation on the wrong kind of member, for example `@Cached` or
    `@OnEvent` on a field, `@Service` on a method, `@Required` on a method, or
    `@FromBody` on a class;
  - a marker that takes no arguments written with arguments, or a marker that
    needs arguments written without them (`@Lifetime`, `@Order`, `@Table`);
  - a route, listener or cache method that is `static` (`@GetRoute`,
    `@OnEvent`, `@Cached`, `@Retry`, and the others that need an instance);
  - the same annotation twice on one member, two HTTP methods on one method
    (`@GetRoute` with `@PostRoute`, or a route with a request), two
    parameter sources on one parameter, `@FromBody` or `@RequestBody` on two
    parameters of one method, `@OnStart` with `@OnStop`, and `@Cached` with
    `@ClearCache` or `@UpdateCache`;
  - `@Order` or `@InBackground` without `@OnEvent`, `@Named` without
    `@Needs`, `@ResponseStatus` without a route or `@HandlesError`, and
    `@Stream` without a route;
  - `@Fake`, `@UseProfiles`, `@TestSettings` or `@FreshStart` outside a
    `@Test` or `@TestSetup` class, `@Convert` outside a `@Converter`,
    `@Query` outside a `@Store`, `@Table` or `@Id` outside an `@Entity`,
    route markers outside a `@RequestRoute` controller, and request markers
    outside an `@HTTPClient`;
  - a Yon-only annotation (routes, `@Stream`, `@Gateway`, `@Schedule`, error
    handlers) in Tac code, and an annotation in a language it does not reach
    (for example `@InBackground` in Yon outside JavaScript, TypeScript and
    Python).
  In TypeScript and JavaScript, `tsc` now also refuses an annotation on the
  wrong member, `@Convert` and `@Query` on a method that does not return a
  Promise, `@WhenProfile()` without a profile, and `@NeedsAll({value})`.

- Annotation arguments use types (ADR 0055). `@Test(CatalogTestSetup)` and
  `@Store(Product, {...})` take the class itself (`X.class` in Java,
  `X::class` in Kotlin and PHP, `typeof(X)` in C#). `Entity` takes only the
  adapter; the field marked `@Id` is the identifier. `Settings` takes only the
  prefix: the class's fields are the settings, with their declared types,
  initial values as defaults, and `@Min`/`@Max` on the field (Rust and Go put
  defaults in `"defaults"`). `@Check`, `@PageDefaults` and the new `@CacheKey`
  sit on the parameter; `@Cached` takes no arguments; `@ClearCache` and
  `@UpdateCache` match parameters by name; a Store query's page parameter is
  marked with `@PageDefaults`; Convert reads its one parameter. The old
  string, index and descriptor forms are refused with the new form in the
  message, and `ty migrate fix --write` converts them where it safely can.
- Application events are models (ADR 0055, "Typed events"). `@OnEvent` listens
  for the model type of its one parameter (a JSDoc `@param` type in
  JavaScript) and takes only `condition`, `id` and `replay`; the `event` and
  `events` keys are refused. `publishEvent(model)` and
  `publishEvent(model, retain)` publish a model value, and the name of its
  type is the event name, so same-named models interoperate across
  languages. JavaScript and TypeScript get the name at compile time from the
  declared type of the value, so minified browser code is safe. Every runtime
  checks the payload against the listener's model with the request-body
  rules before delivery. A listener whose return type is a model publishes
  the value it returns; `{event, value}` records are no longer read. The
  browser event runtime grew to 11,104 bytes. `ty migrate fix --write`
  converts listeners and JavaScript/TypeScript calls whose event name matches
  a model and lists the rest as manual steps. Ruby listeners name their
  model in a YARD tag (`# @param order [OrderPlaced]`) and Ruby services
  publish `publish_event(model)`, in Yon and in Tac; the fix converts a Ruby
  listener that already has that tag.

- Go annotations use standalone comments such as `// @Service` and
  `// @GetRoute("/items")` (ADR 0050). The comments are valid Go syntax and
  keep their spelling under `gofmt`; published examples leave out the
  `package` line, so add one before running `gofmt` on a copied example. The
  former `#[Name]` spelling is refused with a migration hint.
  `ty migrate fix --write` converts Go markers, including comma-grouped and
  multi-line ones, without changing PHP or Rust attributes. A comment marker
  close to a Tachyon name (`// @GetRote`) is refused with a suggestion; other
  `// @Word` comments, such as swaggo/swag's `// @Summary`, stay ordinary
  comments. `//@Name` spacing, text after a marker and arguments that span
  lines are refused on their own line.
- `client/manifest.json` is the whole Tac configuration (ADR 0052).
  `tac.config.js`/`.mjs`/`.ts` and `tachyon.json` are removed and stop the
  build with a message that says where each value moved. The native
  application name, entry route and icons come from the manifest's `name`,
  `start_url` and `icons`; the bundle id, version and window move to
  `tac.application` (the manifest member `tachyon` is refused). Scripts,
  styles and service-worker cache rules move to `tac.scripts`, `tac.styles`
  and `tac.cache`; page metadata moves to `tac.metadata` (`defaults` and
  `routes`). The `postBundle` hook is removed: with `tac.origin` set, the
  build writes `sitemap.xml` and `robots.txt` itself, and `tac.sitemap` lists
  the values of dynamic page routes from a data file. `ty init` writes no
  `tac.config.js`. Everything in the manifest is public; never put a secret
  there.
- Topic logs are saved data and moved out of the generated `.tachyon`
  folder (ADR 0061 amendment). They live in `data/topics/<name>.jsonl`, or
  in the folder that `topics.directory` in `server/application.json` names:
  a plain path inside the project or an absolute path with no symbolic link.
  A path with `..`, a path inside `.tachyon`, or a folder that a symbolic
  link leads out of the project stops the server with `TY1303`. On start,
  the server moves old logs from `.tachyon/topics` once, when the new folder
  has none, and prints a notice; `ty migrate check` reports old logs that
  are still there. Code that wrote `.tachyon/topics` by hand writes to the
  new folder. The stream URL `/.tachyon/topics/<name>` does not change.
  `.tachyon` and `dist/.tachyon` now hold only generated files and are safe
  to delete; the guides list what is in them.

### Added

- Test units get a JUnit 5-style `Assert` class (ADR 0080) in JavaScript,
  TypeScript, Python and Rust: `Assert.equals(expected, actual)`,
  `Assert.isTrue(condition)` (Python and Rust: `is_true`) and
  `Assert.throws(action)`, each with an optional last message (Rust:
  `equals_with_message` and the other `_with_message` forms). A failed check
  fails its case with `message ==> expected: <3> but was: <2>`. `ValueSource`
  on a test case lists its inputs, like JUnit's `@ValueSource`: the driver runs
  one case per input (`testName[1]`, `testName[2]`) and passes the input as
  the one argument. A Rust case that panics (`assert!`, `assert_eq!`,
  `Assert::`) now fails as one case instead of stopping the unit. The new
  design rule `TY1339` refuses a test case with no assertion or with a branch
  or loop; the Python `assert` and the Rust `assert!`, `assert_eq!` and
  `assert_ne!` count as assertions. It is off by default (`tests.check` /
  `tac.tests.check`) until every test unit is rewritten; `ty migrate check`
  lists the cases to change.
- Release builds compile every WebAssembly service language with the
  compiler and settings that make the smallest download, and `ty build`
  prints one line per language that says which compiler it used and why
  (ADR 0081, docs "WebAssembly compilers per language"). Rust uses
  `opt-level=z`, LTO and one codegen unit, then Binaryen `wasm-opt -Oz`. Go
  uses TinyGo (`-opt=z`) first and falls back to the standard Go compiler,
  with a note, when TinyGo is missing or cannot compile the services;
  `tac.go.compiler` (`"tinygo"` or `"go"`) in `client/manifest.json` forces
  one. C# and F# use the trimmed .NET interpreter with Emscripten
  `-Oz`; `tac.dotnet.aot: true` compiles them ahead of time instead. Dart
  uses `dart compile wasm -O4`. Development builds keep the fast settings.
  `ty doctor` lists TinyGo and `wasm-opt` as optional tools, and `ty init`
  offers to install TinyGo 0.42.0 and Binaryen version_133.
  For one small service the release download (Brotli) shrinks from 678.5 KiB
  to 98.0 KiB in Go, 56.7 KiB to 48.6 KiB in Rust and 16.8 KiB to 16.5 KiB in
  Dart; C# changes by under 1%.

- Groovy is a Yon language (ADR 0072). Controllers, services and the other
  roles, models, error handlers, workers, `middleware.groovy` and test units
  are `.groovy` files with the Java annotations (`@Service`,
  `@GetRoute('/{id}')`). Tachyon reads Groovy with the Java rules; every class
  member has a declared type, and `def` on a field or method is refused
  (TY1334). The request and response are `HttpExchange`, as in Java. Tachyon
  compiles the Groovy together with its generated Java runtime (Groovy joint
  compilation) into one cached jar with the Groovy core runtime and runs it on
  the JDK. It finds Groovy 4 or newer through `GROOVY_HOME` or `groovyc` on
  `PATH`; readiness and `ty doctor` check it, and `ty init` offers Groovy and
  installs Groovy 5.1.3 with SDKMAN.

- Simpler-form suggestions (`TY1340`–`TY1355`, ADR 0076). `ty build`,
  `ty preview` and `ty test` print `suggestion[TY13xx]` with a `help:` line
  that shows the rewritten code, for sixteen curated patterns such as an `if`
  that only returns `true` or `false`, `filter(...).length > 0`,
  `indexOf(x) !== -1`, an `else` after `return`, and an index loop that only
  reads items. A suggestion never stops the build; `ty migrate fix --write`
  applies it. JavaScript, TypeScript, Python and Ruby first, then Java,
  Kotlin, C#, PHP, Go, Rust, Swift and Dart where the rewrite behaves the
  same. `tac.simplify.suggest` and `simplify.suggest` hide them per realm.
  Docs: Going further > Simpler forms.
- Kotlin Tac services run on the web as WebAssembly (Kotlin/Wasm, WasmGC,
  ADR 0078). A service with a `.kt` file and no browser sibling now reaches
  every target: Android still links it natively, and every other target runs
  it as WebAssembly, with no web copy needed. It keeps fields, methods with
  arguments, `publishEvent`, `OnEvent` with its options, `NeedsAll`
  properties, and the curated browser APIs (`Clock`, `SecureRandom`,
  `Connectivity`, `PageVisibility`, `Preferences`, `Clipboard`, `Share`,
  `Notifications`, `Geolocation`). When a service has several WebAssembly
  languages, the order is Rust, Go, C#, Kotlin, Ruby. A release build runs
  Kotlin's production step, Binaryen's `wasm-opt`, and prints
  `Kotlin: Kotlin/Wasm production build — optimized by Binaryen`. For a
  small service this is 81 KB of WebAssembly (30 KB with Brotli), against
  275 KB in a development build. If `wasm-opt` is missing, the build says so
  and `ty doctor` reports it. The build needs `kotlinc-js` 2.1.10 and
  `TAC_KOTLIN_WASM_STDLIB`. It needs Chrome or Edge 119, Firefox 120 or
  Safari 18.2; an older browser gets a clear error. `TY1317` now also
  refuses Kotlin browser interop (`external`, `@JsFun`, `@JsExport`, `js(…)`,
  `kotlinx.browser`, `org.w3c`, `kotlin.js`).
- Kotlin WebAssembly services use the built-in browser cache and Store
  (ADR 0078), as Go, Rust and C# do: a `Cached` method or an `"indexeddb"`
  Entity needs no adapter in the Setup. Android Kotlin services still name
  their own adapters, and the build still refuses them otherwise.
- F# is a Yon language and a Tac service language (ADR 0073). Annotations are
  F# attributes, `[<Service>]` or `[<GetRoute("/{id}")>]`, on their own line;
  several may share one pair of brackets. A type takes the roles it needs as
  the parameters of its primary constructor (Spring's single-constructor
  injection). Tachyon reads each F# file through a C# view of its types and
  member signatures, so every C# rule and guard (the rule table, one class
  per file, file names, cohesion, duplication, design and safety rules, route
  and model types) runs on F# and reports the F# line. Yon compiles an F#
  program as three .NET assemblies: the C# protocol runtime, the F# code and
  models, and the generated C# container, so F# code uses `relay`,
  `DataSource`, `HttpRequestMessage` and `HttpResponseMessage` as C# does.
  Controllers, services, stores, helpers, error handlers, root middleware
  (`middleware.fs`), plain handlers, records and enum unions as models, and
  test units (`member this.testName() : unit`) work in F#. A Tac service in
  `client/services/<Name>Service.fs` runs on the .NET browser runtime of
  ADR 0037 on every target, sharing one download with C#. `ty doctor` checks
  .NET for F#, and `ty init` offers F# in both realms. TY1335 refuses an F#
  file with a `module` or `namespace` line, a file-level `let` or `do`, or an
  attribute on the same line as its declaration. Behavior annotations,
  events, lifecycle hooks, sockets, streams and the executed-coverage check
  are not available in F# yet and are refused by name.
- Compressed files (ADR 0075): a release build of the web target (`ty build`,
  `ty bundle`) writes a Brotli `.br` and a gzip `.gz` copy next to each script,
  style, page, JSON, SVG, source map and WebAssembly file of 1 KiB or more,
  keeps a copy only when it is smaller, writes the same bytes every time and
  reuses the copies of unchanged files. `ty start` and `ty preview --static`
  pick Brotli, then gzip, then the plain file from `Accept-Encoding` (a `q=0`
  value turns an encoding off) and send `Content-Encoding`,
  `Vary: Accept-Encoding` and the length and ETag of the copy; range and HEAD
  requests keep working. A watching `ty preview` sends plain files and writes
  no copies. `ty build` prints the totals (`BuildResult::compression()`). The
  Tachyon website goes from 11 MB to 2.7 MB (Go module 3.2 MB to 684 KiB).
  New pure-Rust dependencies `brotli` 9.0.0 and `flate2` 1.1.10
  (`miniz_oxide`); NOTICE holds the Brotli licence.
- JavaScript models files (ADR 0067 amendment): `client/models/` and
  `server/models/` hold JavaScript models, one JSDoc `@typedef` plus
  `export {}` per file, named after the model. They are global types of their
  realm in `composed.d.ts`. JavaScript routes, event listeners and `/api.json`
  read the JavaScript and TypeScript models of the realm, so
  `/** @import { PlatformChanged } from '/models/PlatformChanged.ts' */`
  replaces a copied `@typedef` (TY1308 no longer refuses it).
  `ty migrate fix --write` moves inline `@typedef` models into the models
  folder, keeps one file for identical copies, imports an existing model the
  copy matches, and reports a copy that differs. The build refuses an inline
  `@typedef` in a `client/` or `server/` JavaScript file outside `models/`
  (TY1310); the website examples follow the rule. The JavaScript `ty init`
  feature templates now use models files.

- `ty init` is a setup wizard (ADR 0066). In a terminal it asks which parts
  the app has (client, server or both), its Tac and Yon languages, its
  targets and optional examples (storage, sockets, topics, streams, an HTTP
  client, error handlers). Every question is also an option: `--realm`,
  `--tac`, `--yon`, `--target` and `--features`. Without a terminal and
  without options it keeps the old defaults (both realms, JavaScript, the
  web). Each chosen language gets an example class, a test unit that calls
  each public method, and a test setup, so a new app passes
  `ty test --coverage` and `ty build` with every guard on.
- `ty init` then checks the tools that the app needs, with the `ty doctor`
  checks plus the target tools. For each missing tool it shows the exact
  commands and asks first; `--yes` accepts every tool and `--skip-tools`
  skips the check. It installs with the vendor's official installer into
  the home folder, at a pinned, tested version, and checks each download
  against the vendor's published SHA-256 (the emsdk hash is pinned). It
  never runs `sudo`: it prints such commands, and it shows a link and steps
  for Xcode and the Android SDK. `TY1408` refuses an unknown or conflicting
  answer; `TY1409` reports an install that failed or a download that does
  not match its checksum.
- `ty add language client|server <language>`, `ty add target <target>` and
  `ty add feature <example>` add to an existing app with the same examples,
  tests and tool check. They never overwrite a file (`TY1402`).

- Tac services get two browser APIs, written once in the page (ADR 0065):
  `Clock` (the time, a monotonic time and the time zone of the browser) and
  `SecureRandom` (random bytes and version 4 UUIDs). JavaScript and
  TypeScript services write `@Needs clock` and `@Needs secureRandom`; Rust,
  Go, C#, Dart and Ruby services use the `Clock` and `SecureRandom` types of
  their prelude. Rust services now have the time and random numbers in the
  browser. The guide "Clock and secure random numbers" shows each
  spelling.
- Seven more browser APIs for Tac services in every language (ADR 0065):
  `Connectivity` (online or offline), `PageVisibility` (shown or hidden) and
  `Preferences` (color scheme, motion setting, language) give the current
  value, and the page publishes each change as a built-in event model
  (`ConnectivityChanged`, `VisibilityChanged`, `PreferencesChanged`) that a
  listener with `replay` also hears at start. `Clipboard`, `Share`,
  `Notifications` and `Geolocation` start the browser's work, and the answer
  reaches the service that called as a built-in event model
  (`ClipboardTextRead`, `ShareFinished`, `NotificationPermissionChanged`,
  `PositionFound`, or `BrowserCallFailed`). Built-in event models need no
  file in `client/models/`. The guides "Connectivity, page visibility and
  preferences" and "Clipboard, share, notifications and geolocation" show
  them.
- `tac.browserApis` in `client/manifest.json` grants `clipboard`, `share`,
  `notifications` and `geolocation`. Without the grant, a call fails at once
  with a message that names it; an unknown or repeated name stops the build
  (`TY1601`). The clipboard, sharing and the notification question work only
  from a click or key handler, and a Rust, Go, C#, Dart or Ruby method that a
  click runs still counts as the click.
- Test units can fake Tachyon's browser APIs (ADR 0065). In JavaScript and
  TypeScript, a `@Fake clock` field of the unit reaches the service's
  `@Needs clock` field through `construct`, like any `@Fake`. Go and Ruby
  units give each call an answer with `BrowserFakes`. Rust, C# and Dart
  units cannot use these APIs yet.
- A Yon test unit can reach the controllers in `server/controllers`, in every
  Yon language, like a controller in a Spring test: `construct(GreetingController)`
  in JavaScript, TypeScript and Ruby makes the controller with the `@Fake`
  fields for its `@Needs` fields, and the other languages make it directly.
  `construct` also makes a role that has no `@Needs` fields. Controller
  methods therefore stay in the test check of `ty preview`. `ty init` now
  writes `server/tests/HomeControllerTest.js`, which calls the home
  controller's method, and the server realm's `composed.d.ts` declares the
  route controllers, so the test type-checks. A Tac test unit can `@Fake` a field that names a
  managed client. The "Your first app" tutorial gets a step 5 that tests the
  service and the controller, and no longer needs `TAC_ALLOW_UNTESTED=1`.

- `@Topic` on a model makes its events cross processes and languages, like a
  Spring Cloud Bus remote event (ADR 0061). `publishEvent(model)` of a Topic
  model still reaches the publishing request's own listeners first; the
  runtime then sends it to the server, which appends it to
  `.tachyon/topics/<kebab-name>.jsonl` and calls each `@OnEvent` listener of
  that model in the other Yon files and languages once (at most once, at most
  8 republications). All eleven Yon languages send and receive Topic
  events. A Tac `@OnEvent` listener for a Topic model opens the
  topic stream by itself, shares one stream per topic in the page, and resumes
  after its last record; a page's own `publishEvent` stays in the page. The
  server accepts only a Topic model of the publishing file's language, a JSON
  object of at most 64 KiB, and refuses `@Topic` outside a models folder.

- A Yon worker or `Handler` method may take no parameter, like a Spring
  `@Scheduled` method: `static void POST() {}` now runs in Java, Kotlin, C#,
  Python, Ruby, Dart and Rust as it already did in JavaScript, TypeScript,
  PHP and Go (before, it answered 405 or failed to compile).
- A C# controller can take a `BehaviorJSON` body with `[FromBody]` and
  return it, so a `Check` behavior gets the request body without reading
  the request by hand.
- Yon controllers take and return models, like Spring's `@RequestBody` and
  message converters (ADR 0048). `FromBody` hands the method an instance of
  the model, and a returned model, list of models or stream of models becomes
  JSON with the model's field names. PHP gives an array its item type in a
  doc comment: `/** @var list<Line> */` on a model property, and
  `@param list<Line> $lines` or `@return list<Line>` on a controller method.
  The website controller examples now return models instead of maps.
- WebSockets (ADR 0060). `@SocketRoute("/chat")` on a JavaScript or
  TypeScript class in `server/controllers/` keeps a WebSocket open, like
  Jakarta's `@ServerEndpoint`: Tachyon starts one handler process for each
  connection, and `@OnOpen`, `@OnMessage`, `@OnClose` and `@OnError` mark its
  methods. When `@OnOpen` or `@OnMessage` fails, `@OnError` runs and the
  socket stays open, as in Jakarta. Each message is JSON text that Tachyon
  checks against the model type of the `@OnMessage` parameter. The session
  has `send`, `close` and `openSessions()` (every open connection of the
  path). Guards fail closed: 64 KiB messages, 32 open sockets, five idle
  minutes, one hour per connection, the page's own origin only, and 426 for
  a plain request. In the browser,
  `@SocketRequest` on an `@HTTPClient` method receives the messages and
  publishes each one as its model's typed event, and the client gets
  `send(message)`; the event bus owns the `WebSocket` (open on first use,
  close with its owner, reconnect after 1 to 16 seconds, five tries).
  `/api.json` marks the path with `"socket": true`. TypeScript declares
  `SocketSession`, `SocketPeer` and `SocketSend<Message>`, and a `declare`
  field is no longer a required `@NeedsAll` field. The website has a
  "Socket controllers" example and a glossary entry.
- Sockets in every Yon language (ADR 0060): Python, Go, Java, Kotlin, C#,
  PHP, Ruby, Rust and Dart hold a `@SocketRoute` class like JavaScript and
  TypeScript do, with the same frames, limits and one object for each
  connection. Each message is turned into the `@OnMessage` model first, and
  the session uses each language's naming (`open_sessions()` in Python, Ruby
  and Rust; `OpenSessions()` in Go and C#; `getOpenSessions()` in Java). The
  compiled and prelude languages read a framed request when the supervisor
  sets the private `YON_SOCKET_FRAMES` value. The website "Socket controllers"
  example has a tab for each language.

- Built-in Tac storage (ADR 0059): `@Entity({"adapter": "indexeddb"})`
  keeps a JavaScript or TypeScript Store in IndexedDB with no adapter code
  (one object store per Table, indexes for the fields its queries use, adding
  schema upgrades, AllOrNothing as one IndexedDB transaction, a bounded scan
  without an index). A `@CacheSettings` cache that the Setup does not
  configure uses the browser Cache API, loaded into memory before the first
  render, with TTL, maxEntries (least recently used first) and background
  writes; on an insecure origin it lives in memory only. `@LocalState` and
  `@SessionState` work in JavaScript, TypeScript, Dart, Swift, Kotlin, C#,
  Rust, Go and Ruby Tac services. Go, Rust and C# WebAssembly services and
  Dart and Ruby services use the same built-in cache and IndexedDB Store
  through a synchronous storage host (Ruby reaches it through the existing
  mruby runtime, with no rebuild): reads come from a copy in memory that Tachyon
  loads before the services start, and writes go to IndexedDB in the
  background. Native Swift, Kotlin, Rust and C# companions still name their
  adapters.
- `@StreamRequest(path?)` on an `@HTTPClient` method in JavaScript or
  TypeScript keeps a server-sent event stream open (ADR 0058). Tac opens it
  when the client is first used, reconnects with a growing wait and a limit,
  and closes it with its component or service. Each message is checked
  against the model type of the method's one parameter, passed to the method
  and published as that model's typed event, so `@OnEvent` listeners in every
  Tac language receive it. The realtime example no longer writes an
  `EventSource` by hand.
- `ty serve` and `ty preview` restart themselves when the `ty` file on disk
  changes. Before, a long-running preview kept the old compiler and every
  rebuild failed with errors from the old version. Open pages show
  "Tachyon was updated on disk. Restarting the preview…" (`TY1320`) and reload
  when the new server is ready. A new `ty` that does not start is reported
  and the old server keeps running; it restarts at most once for each change.
  `--no-self-restart` turns this off.
- Ruby is a Yon language and a Tac service language (ADR 0054). Annotations
  are comment lines, `#[Service]` or `#[GetRoute("/{id}")]`, alone on their
  line, so annotated files stay ordinary Ruby. Route, error and model types
  come from YARD tags (`# @param id [Integer]`, `# @return [Order]`,
  `@!attribute`). Yon runs Ruby 3.3 or newer (`YON_RUBY_RUNTIME`,
  `ty handler invoke --ruby-runtime`) for controllers, error handlers,
  workers, root middleware (`middleware.rb`), relay, `DataSource`,
  `RedisTemplate`, Gateway class targets, `server/models/*.rb` and test
  units. A Tac service in
  `client/services/<Name>Service.rb` runs in the browser on the pinned mruby
  runtime (ADR 0049) with its `attr_reader` fields, public methods,
  lifecycle hooks and event listeners, and reaches native targets through
  the web view. Client test units run in CRuby. The website's Showcase and
  Drawer services are now Ruby.
- Ruby runs the shared behavior annotations (ADR 0054 "Behavior annotations"):
  `Cached`, `UpdateCache`, `ClearCache`, `Retry`, `MaxAtOnce`,
  `AllOrNothing`, `Check` and the field checks, `Settings`, `Setting`,
  `Entity`/`Store`/`Query`, `Convert`, `HTTPClient` requests and `Gateway`
  callers, in Yon (CRuby) and in Tac services (mruby), through the shared
  rule table. The runtime is one Ruby file that passes the same boundary
  contract in CRuby and in the browser build. Yon Ruby services also get
  local events and `OnStart`/`OnStop`, and Ruby test units get `Fake`
  fields and `construct(Role)`.

- Generated TypeScript globals for composed classes (ADR 0053). `ty bundle`,
  `serve`, `preview`, `test` and `init`, and the new `ty types`, write
  `.tachyon/types/client/composed.d.ts` and
  `.tachyon/types/server/composed.d.ts`. They declare each exported,
  annotated JavaScript or TypeScript class that Tachyon composes by name in
  that realm, with its real type, so tests and role files use it without an
  import in editors and `tsc`. Deleted classes drop out on the next run, and
  client and server names never mix. `ty init` now writes
  `client/jsconfig.json` and `server/jsconfig.json` (one per realm) that
  include these files, and adds `.tachyon/` to `.gitignore`. The website's
  hand-written `composed-services.ts` is removed.

- Yon error handlers (ADR 0051). An `@ErrorHandler` class in
  `server/controllers/<Name>ErrorHandler.<ext>` answers typed errors that
  controllers and their services throw, in all ten Yon languages. Each
  `@HandlesError` method takes no argument: the declared type of its first
  parameter is the error it handles. The nearest type wins. Go matches with
  `errors.As`, so wrapped errors match. Rust matches the error type of the
  controller's `Result`. `@ResponseStatus` sets a 400 to 599 status. The
  answer is checked like a controller answer, and `/api.json` lists each
  error type, status and shape. Unmatched errors and errors thrown by a
  handler keep the fixed 500 answer. Error types live in
  `server/errors/<Type>.<ext>`. Spring and NestJS names such as
  `ControllerAdvice`, `ExceptionHandler` and `UseFilters` are refused with a
  hint to use these markers.

- A pinned mruby WebAssembly runtime for Ruby in Tac (ADR 0049), in
  `runtimes/ruby-web`. It compiles Ruby with the embedded Prism compiler,
  stays below 4 MiB, and has no filesystem, socket, process or environment
  access; the page needs only CSP `wasm-unsafe-eval`. Its sources and the
  Emscripten toolchain are pinned by commit and SHA-256, and a dedicated CI
  workflow (`ruby-runtime.yml`) rebuilds it twice and compares both builds
  with the committed hash. It is mruby, not CRuby: only `json`, `set` and
  `time` can be required, and error text can differ. Framework integration
  (annotations, providers, page events) is a separate phase.

- Website routing examples now use `RequestRoute` controllers in all ten
  Yon languages. HTTP examples use `HTTPClient` and request annotations.
  A linked glossary explains incoming routes and outgoing requests in plain
  English. TypeScript editor declarations no longer advertise retired markers.

- `ty migrate fix` fixes Tachyon's own breaking changes. It deletes
  `tachyon:behaviors` imports (ADR 0046) and turns an old root middleware
  class `Handler`, whose HTTP method functions all hold the same code, into
  class `Middleware` with `before(request)` and `after(request)` (ADR 0044).
  It renames old annotations to their plain English names in every language
  and moves files from the renamed role folders, renaming the moved role
  types and test setups where they are named (ADR 0047); it only moves files
  from an old role folder to its new one inside `client/` and `server/`,
  never deletes or overwrites a file, and reports a taken name or a linked
  folder as a manual step.
  It lists everything it cannot change safely as a manual step with the exact
  instructions: an import that renames the names, a re-export, other
  middleware, and each file that breaks the one-class rule, with the split to
  do. It prints every change and writes nothing without `--write`. It never
  follows a symbolic link, keeps file permissions, and a second run changes
  nothing.
- Go routes, workers and root middleware (ADR 0045). A
  `server/routes/**/yon.go` route and a `server/workers/*.go` worker declare
  `type Handler struct{}` with exported HTTP-verb methods such as
  `func (Handler) GET(request *YonRequest) *YonResponse`; a root
  `middleware.go` declares `type Middleware struct{}` with `Before(request)`
  and `After(request)`. A route method may leave out the request parameter,
  and it may return nothing, a value that `encoding/json` can encode, an
  `error`, a value and an `error`, or a `*YonResponse` (`YonJSON`,
  `YonEmpty`, `.Status`, `.Header`). Plain Go files under `server/shared/`
  compile as more files of the route's `package main`, so a route calls their
  types and functions without an import; they carry no annotation and no
  other package clause (`TY1310`), and a compile error names their path.
  Tachyon builds each handler offline as a Go module with the standard
  library only and caches the binary by source, shared code, models, runtime
  and `go version`. The one-class rule (ADR 0044) and the empty route-marker
  rule apply. `ty doctor` and the server preflight require Go 1.27 or newer
  (`TY2112` when `go` is missing).
- Go relay and managed resource clients (ADR 0045). A Go handler calls
  `Relay([]string{"ruby", "server/delegates/report.rb"}, request)` with the
  same rules as the other compiled Yon languages: a cleared environment
  without `YON_APPLICATION_JSON` or `YON_RESOURCE_*`, the time left in the
  request as its deadline, 16 MiB of stdout, 64 KiB of stderr, and HTTP 502
  `{"error":"Component invocation failed."}` for every failure. The deadline
  lives in a separate Go package, so handler code in `package main` cannot
  change it. `DataSource{}.Query`, `Execute` and `Command`,
  `RedisTemplate{}.Get`, `Set` and `Delete` return `(any, error)`; a handler
  method may return that pair: a `YonResourceError` answers HTTP 500 and any
  other error fails the handler. `YonGateway.Call(key, argumentsJSON)` is the
  Gateway transport. All three connect straight to the loopback resource
  endpoint and never use a proxy.
- Go models (ADR 0045). `server/models/*.go` and `client/models/*.go` hold
  Go data types: each file declares one struct, other defined type or type
  alias, with struct tags and the imports it needs, under an optional
  `package main` clause (ADR 0043, ADR 0044). Methods, functions, constants,
  variables, interfaces, blank imports and another package clause fail with
  `TY1310`. Tachyon compiles every Go models file of the realm as a file of
  the program's `package main`, so a Go route, worker or root middleware uses
  a model by its type name alone, and a compile error in a model names its
  `server/models/` path. Go web services use the client Go models.
- Go annotations and test units (ADR 0045). A Go role file, route, worker or
  test unit uses the Rust spelling on the line before `type Name struct` or
  `func (receiver) Name(...)`: roles and `#[yon::...]` names, `#[Inject]`,
  `#[Scope]`, `#[RequiredArgsConstructor]`, `#[Autowired]`, every behavior
  annotation with its JSON arguments, `#[EventListener]`, `#[PostConstruct]`,
  `#[PreDestroy]`, `#[Order]`, `#[Scheduled]`, `#[TachyonTest]` with
  `server/tests/*Test.go` and its sibling `TestConfiguration`, and the caller
  side of `#[Gateway]`. Tachyon builds each managed type as `&T{}` and sets
  its injected fields; behavior methods take `BehaviorValue` parameters and
  return `(BehaviorValue, error)`. Role files and generated code compile as
  separate files of `package main`, so they never change the imports of an
  authored file.
- Go Gateway targets (ADR 0045). A `#[Gateway]` or `@Gateway` caller in
  another Yon language may name a Go role file, such as
  `{"class":"server/services/ReportService.go","method":"summarize"}`.
  Tachyon calls the exported method with an upper-case first letter
  (`Summarize`), with a value or pointer receiver. The method takes
  `BehaviorValue` parameters and returns `(BehaviorValue, error)` or
  `BehaviorValue`. The generated unit resolves the type and its dependencies
  through the Go container, and a missing or unexported method, a different
  argument count or another type fails with `TY1312`. A returned error or a
  panic reaches the caller as `adapter`.
- Go web services as WebAssembly, and Go client test units (ADR 0045). A
  `client/services/<Name>Service.go` file declares `type <Name>Service struct`
  with `#[Service]`, `#[Scope]` and `#[RequiredArgsConstructor]` as in Rust;
  every Go service of the application compiles with
  `GOOS=js GOARCH=wasm go build` into `.tachyon/services/go/services.wasm`,
  loaded through a same-origin copy of Go's `wasm_exec.js`. Views read and
  set exported fields and call exported methods with lower-first names, so
  `Results` is `{search.results}`. `ApplicationEventPublisher`,
  `#[PostConstruct]`, `#[PreDestroy]`, `#[EventListener]` and the behavior
  annotations work as in a Rust web service, and a service uses the Go client
  models and its bare `#[Repository]`, `#[Configuration]` and `#[Entity]`
  role files. As in Rust and C#, a Tac `#[Component]` delegate is a browser
  role, so it fails with `TY1308`, and the message names the delegate file.
  A service with several WebAssembly languages uses Rust, then Go, then C#.
  Go's runtime gets its file-system stub only while the module starts, so a
  page whose own scripts define a global `fs` keeps it. `ty doctor` checks
  for Go 1.27 or newer when a project has a Go service. A
  `client/tests/<Name>Test.go` unit runs natively with `go run`.
- The website uses Go services (ADR 0045). Three of the site's own services,
  `InlineCodeService`, `CatalogService` and `ShowcaseService`, are Go, with
  Go client models and `client/tests/*Test.go` units. Every multi-language
  example on the site and in the guides has a Go version, and
  `bun run verify:snippets` builds, serves and tests the Go snippets with the
  `go` toolchain; a Go fragment without markers is checked with `gofmt -e`.
- `@Gateway` declares a Yon method that another language answers (ADR 0042).
  The method has no body. Its configuration names a class in another Yon
  language, `{"class":"server/services/ReportService.java","method":"summarize"}`,
  or a program, `{"command":["ruby","server/delegates/report.rb"]}`. The
  positional arguments travel as bounded JSON, and the other side's return
  value becomes the result. A class target runs as a generated unit under
  `server/gateways/` in the owned snapshot; a command reads
  `{"method":...,"arguments":[...]}` on stdin and writes the bare JSON result
  on stdout, with exit status 0 for success. Limits: 32 arguments, 1 MiB each
  way, depth 32, 10,000 values, 64 calls and 4 concurrent calls per
  invocation, and the time left in the request. A failure is a BehaviorError
  with the code `invalid`, `timeout`, `capacity` or `adapter` and the fixed
  message `Gateway call failed.`; the supervisor logs only
  `handler.gateway_failed`. One hop only. The build fails with `TY1312` for a
  bad target and `TY1308` for a misused annotation; `server/gateways/` is
  reserved in any letter case (`TY1310`); a missing program fails the
  preflight of the server and of `ty test` with `TY2112` (on Windows only a
  program that Rust's spawn can run counts); Firecracker isolation refuses
  gateways (`TY2010`). An overloaded target is chosen by argument count and
  JSON types; ambiguous signatures, conflicting argument counts and
  unsupported Rust receivers fail with `TY1312`, and a Python target is
  checked against its last definition. A command gets only the baseline
  environment and the explicit allowlist, which cannot restore reserved
  names; an entry with a drive prefix such as `C:x` counts as a path, and
  stderr overflow is an `adapter` failure. A unit is never started after its
  deadline. Closing an invocation cancels its calls and waits until their
  process groups are reaped. All ten Yon languages can call and be called.

- A service written only in Rust now runs on the web as WebAssembly (ADR
  0035). All such services compile with
  `rustc --target wasm32-unknown-unknown` into one module. The module is built
  for small size and embedded in `.tachyon/services/tachyon-rust.js`, so a
  `file:` web view can load it too. Each `<Name>Service.js` exposes the Rust
  service's fields and methods through the companion protocol that a desktop
  host already uses. Events the service publishes arrive as the page's
  application events. A native target that links the service natively never
  loads the module. iOS and Android link no Rust, so they run the module. A
  service's plan now marks it `native` only when this target links it. Rust
  companion methods may take `&mut self`, and code may assign a `TacValue`
  field. `ty doctor` checks for the `wasm32-unknown-unknown` target. A Rust
  service declares `#[Service()]`, `#[Scope]` and `#[RequiredArgsConstructor]`
  above its struct. Its methods take JSON arguments, and its Rust test units
  can hold a `TacValue`.

- A service written only in C# now runs on the web as WebAssembly (ADR 0037).
  All such services publish into one assembly on the .NET browser runtime,
  served under `.tachyon/services/dotnet`. A C# service uses the same protocol
  and per-service module as a Rust one. C# declares `[Service()]`, `[Scope]`
  and `[RequiredArgsConstructor]`. Its methods take JSON arguments, and an
  `object` field accepts lists and objects. `init` may include up to 1 MiB of
  properties. The Linux host allows WebAssembly, and `ty doctor` checks for
  the `wasm-tools` workload.

- Route metadata may set `noindex`. The website's head now comes from its
  `metadata` export, instead of a rewrite after bundling.

- Views read `client/shared/data/<name>.json` as `data.<name>`, and read the
  page as `page.path` and `page.development`. No service is needed in between
  (ADR 0036). A missing data file fails the build with TY1311. No service may
  be named `data` or `page`. `ty preview` marks the documents it serves as
  development documents.

- The website's services are now split evenly: three each in JavaScript,
  TypeScript, Dart, Rust and C#. Each has a unit in its own language. Its
  data-only services (pillars, FAQ, footer, hero, quickstart) became shared
  data, and its runtime note became a plain view. One clipboard service
  replaced three copies of the copy-with-feedback logic. The drawer, showcase
  and platform choice were split out of the services that contained them.

- A Dart field whose initializer has a comma inside a string is now read as
  one declaration.

- A Tac service declares `@RequiredArgsConstructor('field', …)` instead of
  writing a constructor. The runtime constructs it with the properties of the
  same names that its tag passes. This works in JavaScript, TypeScript and
  Dart. The website's services expose their data as fields that a view reads
  directly (`{faq.entries}`), instead of methods that return the data.

- Browser services may be written in Dart (ADR 0033).
  `client/services/<Name>Service.dart` compiles with `dart compile js` into
  the same `.tachyon/services/<Name>Service.js` that a JavaScript service
  publishes. So views, the runtime and hot updates treat it like any other
  service. Compiled Dart has no reflection. So the build reads the class's
  public members and generates a bridge. Fields and getters are read
  synchronously, assignments write through to Dart, and methods return their
  value, or a Promise for a `Future`. Only JSON values cross the bridge. Other
  members stay inside Dart. `@Autowired` collaborators are built once per
  instance. An injected `ApplicationEventPublisher` publishes through the
  runtime. `@PostConstruct`, `@PreDestroy` and `@EventListener` run from the
  exported event plan. Dart Tac test units run on the Dart VM under `ty test`,
  and the method-coverage guard counts them. Builds are cached by SDK and
  program digest. `ty doctor` reports the SDK. The annotation guide's Tac
  sections now have Dart tabs, and the snippet verifier bundles or runs each
  tab. These sections cover roles, caching, resilience, validation, lifecycle
  and events, test units and strict layout.

- Dart is a Yon handler language, the tenth language Tachyon supports. A route
  is `server/routes/<path>/yon.dart`, which declares `class Handler` with verb
  methods, sync or `async`. `YonRequest`, `YonResponse`, `DataSource`, `Redis`
  and `relay` are in scope, as in every other language.
  `dart compile aot-snapshot` compiles the handler once and caches it, and
  `dartaotruntime` runs the snapshot. So a request starts in milliseconds,
  instead of waiting for just-in-time (JIT) compilation. The runtime is one
  executable that macOS already trusts. So macOS does not check each newly
  compiled handler on its first launch. Ahead-of-time Dart has no reflection,
  so the dispatch names exactly the verbs the handler declares. Any other verb
  returns 405. A role file may import `dart:` core libraries. The composed
  unit moves these imports before the first declaration, as Dart requires.
  `ty doctor` checks the SDK.

- Dart has the full Yon annotation vocabulary:
  controllers and roles with constructor injection (`final` fields become
  initializing formals), the 23 behavior annotations, configuration binding,
  validation, mapping, lifecycle hooks and local events, and `@TachyonTest`
  units under `ty test`. Behavior methods that do I/O are `async` and return a
  `Future`. The website's annotation guide has a Dart tab for every Yon
  section.

- Dart Yon verbs can stream. A handler or controller method written `async*`
  returns a `Stream`. The prelude writes each value as one protocol frame, as
  Kotlin's prelude does for a `Sequence`. A failure in the middle of a stream
  ends it with one error event, and that event does not show the handler's
  message.

- Dart browser services now share one compiled program. Each service is its
  own Dart library, and each module imports `tachyon-dart.js`, so the page
  loads the Dart runtime only once. A Dart service uses another service
  through a `late` field named after it, as JavaScript services do. Another
  Dart service arrives as the page's own Dart instance. Any service can arrive
  as a `TacService` proxy (`get`, `set`, `invoke`). `$`/`$$` fields persist
  through the bridge. The bridge restores them before `@PostConstruct` and
  stores them only when a call changes them. Bridged methods accept optional
  positional and named parameters, and use the Dart defaults for omitted
  ones. A Dart test unit replaces a JavaScript service with
  `TacService(fields, methods)`. This was checked in the browser and in the
  macOS host.

- Browser test units run under Node, Bun or Deno, and see what a browser
  service sees. `TAC_JAVASCRIPT_RUNTIME` picks the runtime, as it already did
  for TypeScript services. Otherwise, the first of the three that is installed
  runs the unit. The runner moves to the next runtime only when one is
  missing. A runtime that ran and failed gives the result. Before, the runner
  reported the last runtime's error instead of the first one's. TypeScript
  runs under all three (Node strips the types). `/shared/...` resolves against
  the project's `client/` tree under each runtime. A unit gets the project's
  own DOM when it installs `@happy-dom/global-registrator`.

- Role files may import by absolute specifier. Before, composition rejected
  every ordinary import. This kept any service that imported a shared helper
  out of test units completely. Composition only needs to reject imports whose
  meaning changes when the file moves. `/shared/...` means the same thing
  wherever the composed unit ends up. Imports of the same specifier merge into
  one statement, including multi-line clauses and import attributes.
  Composition rejects a name bound from two different specifiers.

- The website is tested. Sixteen units cover all 73 public methods of its
  annotated services, under Node, Bun and Deno. `ty preview` now starts
  because the tests pass. Its scripts no longer use the `TAC_ALLOW_UNTESTED`
  override.

- Test units are compiled. `server/tests/<Name>Test.<ext>` is an entry that
  composes the role files of its realm. So every service, repository, client,
  component and mapper is available to it. `@TachyonTest` names the
  Configuration that provides its behavior context. `@TestConfiguration`,
  `@ActiveProfiles`, `@TestPropertySource`, `@MockBean`/`@MockitoBean` and
  `@DirtiesContext` provide the rest. The compiler writes `startBehaviors()`,
  `stopBehaviors()` and the driver. In JavaScript and TypeScript, it also
  writes the mock-aware `construct(Type)`. A case is found by a naming
  convention, in the same way as a controller method named after a verb. A case
  is an instance method whose name begins with `test`. The compiler lists the
  cases, because Rust has no reflection to find them at run time.

- `ty test` runs test units in both realms. A Yon unit is compiled and invoked
  like any other handler. So one path reports results for JavaScript,
  TypeScript, Python, PHP, Java, Kotlin, C# and Rust. A Tac unit has its own
  runner, because a browser unit cannot use the Yon route plan.
  `client/tests/<Name>Test.<ext>` is composed from the client roles and lowered
  for the client realm. It runs in the host that its language compiles to. The
  host is Bun or Node for JavaScript and TypeScript. It is one binary, jar or
  built assembly for Swift, Kotlin, C# and Rust. A Tac driver uses no Yon
  transport type. Every behavior runtime already has a bounded JSON serializer,
  so the driver returns its own encoded report. For this reason, support for all
  six languages needed only a small change, not a change to the handler
  contract. The command exits with a non-zero code when a case fails. It now
  also checks first that the runtimes its units need are ready. So a TypeScript
  unit under a JavaScript-only runtime gets the same readiness diagnostic as
  every other command, not Node's own error message.

- Test units of both realms are published and verified. The annotations topic
  has two new sections: `Test units: Yon` (all eight languages) and
  `Test units: Tac` (all six). The snippet verifier runs each language tab
  through `ty test` instead of bundling or serving it.

- You can declare a role abstractly, in the way Spring writes a repository or
  an HTTP interface. Use an `interface` in Java, Kotlin, C# and PHP, a
  `protocol` in Swift, a `trait` in Rust and a `Protocol` class in Python. The
  compiler lowers it into the concrete type. It writes the behavior context, the
  constructor and every method body, so the source contains only signatures.
  The compiler rejects a member with a body, a constructor written by the
  author, and an `@Autowired` field. JavaScript has no method without a body,
  and TypeScript cannot decorate an abstract member. So both keep the empty
  body that they already used.

- Pages compose components (ADR 0029). A page
  directory holds only `tac.html`. That file contains only component tags,
  whitespace and comments. Markup, state, companions and stylesheets next to a
  page fail with `TY1309`. The error names the component to move them into.
  Components are the unit of view and style. Site-wide scripts and styles are
  declared once in `tac.config.js`. `ty init` creates a page that composes
  components. The migration report classifies page companions as
  `companion.page`. The website, the gates and the published examples follow
  the rule.

- Components use services (ADR 0030). A component directory holds `tac.html`
  and, optionally, `tac.css`. A `tac.js`, `tac.ts` or native `tac.*` next to a
  component view fails with `TY1310`. The error names the service file to move
  it into. Behaviour is one `@Service()` class in
  `client/services/<Name>Service.<ext>` (`js`/`ts` for the browser,
  `swift`/`kt`/`cs`/`rs` for native targets). A view uses a service by its
  instance name (`{basket.count}`, `on:click="basket.add()"`,
  `basket.query = $event.target.value`). Loop bindings and passed properties
  do not use the instance name. A service is a page singleton. It is
  initialised on first activation and disposed when the page is hidden.
  `@Scope({"value":"prototype"})` creates one service per component instance.
  Collaborators live in `client/clients`, `client/repositories`,
  `client/configurations`, `client/entities` and `client/delegates`. An
  undefined field named after another service receives that service. `Service`
  and `Scope` are global Tac markers (ADR 0046). Browser modules are
  written to `dist/.tachyon/services/<Class>.js` with the hot-update boundary
  `service:<Class>`. A mount policy on a component that names no service gives
  `TY1405`. Native requests include `service`. They return
  `TY_NATIVE_SERVICE` for an unknown service. `ty init` creates
  `client/services/ShellService.js`. `ty migrate` reports
  `companion.component`, `service.tac`, `service.native` and
  `companion.controller`.

- FYLO is a datasource (ADR 0032). `yon.datasource.driver: "fylo"` with
  `url: "fylo:///absolute/root"` serves repositories from a local document
  store. One `fylo` subprocess, owned by the server, uses the bounded NDJSON
  (newline-delimited JSON) machine protocol. `save` maps to
  `putData`/`patchDoc`, and `delete` maps to `delDoc`. Finds and derived
  queries map to `findDocs` with `$ops` predicates over at most 10,000 matching
  documents. Raw `dataSource.query` goes through FYLO's SQL subset. FYLO
  assigns identifiers. The root must already exist. FYLO accepts no credentials
  or TLS. `ty doctor` reports the `fylo` binary when the driver is configured.

- Add `RestClient`, Spring's imperative HTTP client, next to the declarative
  `@HttpExchange`: `behaviors.restClient("catalog").get("/products").query(...)
  .header(...).body(...).retrieve()` in all nine languages. Python names it
  `rest_client`. It calls the same configured `http` adapter with the same
  rules: a bounded path relative to the origin, no headers that the transport
  owns, no body on GET or HEAD, `redirect: error`, `credentials: omit`, a
  1 MiB response limit and the operation's own deadline. There is no second
  transport.

- Add `@Retryable` and `@ConcurrencyLimit`. They copy the resilience
  annotations of Spring Framework 7, in both realms and in every supported
  language. `@Retryable({"maxRetries":1-8})` takes the optional literals
  `delayMs`, `multiplier`, `maxDelayMs` and `jitterMs`. Delays are in
  milliseconds, not a `TimeUnit`. A framework rejection is a decision, so it is
  never retried. Examples are an invalid policy, denied authorization, a
  disposed context or an expired deadline. An adapter fault or an error that
  the method raised is retried. `@ConcurrencyLimit({"limit":1-64})` takes
  `policy` `block` or `reject`. A blocked caller waits only until its operation
  deadline. A retrying call keeps its slot across every attempt. PHP runs one
  synchronous call at a time, so it accepts only `reject`.

- Add `@Transactional({"adapter":"<entities adapter>"})` on an async service
  method in both realms. The framework calls `begin` before the body, `commit`
  after it returns and `rollback` when it raises. Every repository and `@Query`
  command issued between them includes the transaction. A nested Transactional
  joins the open transaction. There are no savepoints. A retrying method opens
  a new transaction for each attempt. The managed Yon datasource now answers
  `{"operation":"transaction","action":"begin"|"commit"|"rollback"}`. At most
  four transactions can be open at once. Each one holds one connection. The
  datasource rolls it back after ten seconds without activity. So a handler
  that dies during a transaction cannot keep a connection. Authored SQL still
  cannot contain `BEGIN`, `COMMIT` or `ROLLBACK`. FYLO rejects a transaction
  instead of pretending to support it. An entity adapter that ignores the
  transaction key is not transactional.

- Add `@Scheduled` on a worker under `server/workers`. It declares
  `fixedDelaySeconds` (1–86,400) and an optional `initialDelaySeconds` in the
  code, not in `.tachyonrc.workers`. The worker uses the same supervisor,
  deadlines and bounds as a request-driven handler. Declaring a schedule in both
  places is `TY1502`. The marker is removed before the worker compiles.

- Add `@Description`: one bounded literal line that a registration includes
  for tools. It goes on a class that already declares a behavior.

- Add owned application events and lifecycle to Tac and Yon: EventListener,
  PostConstruct and PreDestroy. Since ADR 0031, code publishes events through
  an injected `ApplicationEventPublisher`. Add browser Resource injection.
  Remove the public this.tac object, the TacRuntime type and the implicit mount
  hook. Native activations use new owner IDs and reject out-of-date callbacks.
  Yon events stay local to one invocation. Network topic and SSE (server-sent
  events) delivery does not change. See ADR 0028 for language limits.

- Add server-owned Yon datasource and Redis resources, with repository
  injection through required fields, in all eight Yon languages. Provide
  parameterized SQL dialects, verified TLS, bounded operations and SQLite
  cancellation. Add Docker qualification with real drivers, and private
  resource access for each invocation. SQL Server runtime qualification is
  still not done (ADR 0027).

- Add discovery and injection of browser JS/TS Client objects over
  HttpExchange. They have owned cancellation and explicit cache policies for
  anonymous JSON. Native HTTP adapters are still configured by hand.

- Normalize ordinary JSON-compatible Yon return values in all eight languages.
  Response helpers stay optional, for HTTP metadata and raw bodies. Add strict
  parse and stringify equivalents where the runtime has no standard JSON API.

- Add Application Configuration v2: tac and yon namespaces, binding from
  underscores to paths, and private literal Yon secrets from .env. Publish only
  the captured base tac projection at /.tachyon/application.json. Builds never
  open secrets. Migrate old flat sections, JSON placeholders and parent
  application overrides to the split model (ADR 0026).

- Infer managed Yon constructor dependencies with RequiredArgsConstructor.
  Direct, uninitialized, non-static fields select concrete providers in
  declaration order. An optional quoted list of fields can select or reorder
  them instead. You do not need to write an Inject list or a constructor. Rust
  uses Arc<Provider>. JavaScript requires exact lowerCamel provider names.
  Defaults and static state are kept. Ambiguity is an error (fails closed).
  Explicit setup is still available as an option. ADR 0027 adds scoped
  discovery, managed resources and browser Client creation. Update all eight
  Yon website examples (ADR 0025).

- Enforce role annotation naming and placement with `TY1310`. The rules are:
  domain-prefixed type suffixes, file names that match exactly, plural parent
  directories and one primary role per file. Captured role folders next to each
  other form bounded source units in one language. ADR 0027 replaces the
  original manifest design. Inline role declarations must migrate. Behavioral
  modifiers and plain entrypoints do not change. Add website examples for all
  eight Yon and six Tac companion languages (ADR 0024).

- Add the shared `Value` annotation in every Yon language and every Tac
  companion language. It binds one instance field from the resolved
  application configuration through `Behaviors.values`. It supports the
  placeholder form `"${path:default}"` and a typed object form, with canonical
  nested application paths. Tac may bind only `tac.*`, and Yon only `yon.*`
  (ADR 0026). The docs include a complete worked `application.json` example
  for `ConfigurationProperties`, `Profile` and `ConditionalOnProperty`.

- Extend shared annotations with CachePut/CacheEvict, ConfigurationProperties,
  Profile/ConditionalOnProperty, Validated/Valid and field constraints, and
  entity-bound Repository/Query with bounded query methods derived from AND
  conditions. Typed configuration uses explicit TAC_/YON_ values. Validation
  is pure. Query execution requires a separate entity authorization hook.
  Document optional folders organized by feature, without adding implicit
  scanning across files.

- Add shared behavior annotations in every supported Tac and Yon language. They
  work over explicit application cache, persistence and outbound HTTP adapters.
  They include bounded pagination, explicit constructor generation,
  configuration and lifecycle cancellation. The annotations and their types
  are global in every language (ADR 0046). Native languages have owned
  runtime adapters without embedded JavaScript. Required polyglot CI runs the
  compiler and runtime fixtures. See ADR 0022 and the separate Spring review.

- Add optional build-time Yon controller and provider annotations, named
  instance HTTP methods, explicit source-local constructor injection and scoped
  provider caching in all eight adapters. Before invocation, the framework
  checks for missing tokens, cycles, scope capture and duplicate registration.
  Plain Handler sources are still supported. See ADR 0021 for controllers and
  DI (dependency injection), and ADR 0022 for shared behavior.

### Changed

- `ty preview` and `ty serve` (while watching) never exit because of a
  problem in the project (ADR 0077). A failed rebuild keeps serving the last
  good build and prints the problems; open pages show them in an accessible
  error box (code, message, `file:line`, help, a Dismiss button) that the next
  good build removes, and pages opened later read them from
  `/.tachyon/diagnostics`. A project that is broken at start gets an error
  page (503) at the same address; the preview keeps watching and opens the
  site once it builds. A coverage refusal (`TY2023`) or a failed test run
  (`TY2020`) at start shows in the box instead of stopping the preview.
  Before a self-restart (`TY1320`) the old server runs the new `ty` with
  `--check-project`; when the new `ty` refuses the project, the old server
  keeps running, shows the reason and checks again after the next save. Only
  `TY1301` and `TY1302` stop the preview. `ty build`, `ty bundle`, `ty start`,
  `ty serve --no-watch` and `ty test --coverage` still fail on every error.

- Method coverage (`TY2022`, `TY2023`) now checks that a test case really
  called each public method, not that it wrote the name (ADR 0070). In a test
  build only, the compiler adds one line at the start of each public method of
  an annotated class and of each public static method of a utility class, in
  every language that has test units. The line records the call; a production
  build has no such line. Each case reports the methods that it called, and
  `ty test --coverage` lists every class with its called methods, the case that
  first called each one, and the methods that no case called. A call on a Fake
  does not count. `ty test` writes the calls to `.tachyon/coverage/calls.json`
  (`TY2024` when it cannot). `ty preview` reads that record, runs the test
  units once when the record is missing or older than a role file or a test,
  and refuses to start with `no test case called X.y`. `TAC_ALLOW_UNTESTED=1`
  and the list of exempt method names are unchanged. When the units of a
  language cannot run on the machine, for example Swift units without swiftc,
  that language falls back to the old name check, and the report labels it
  `[name check]`.

- The mruby WebAssembly runtime for Tac Ruby services (ADR 0049, amended) is
  no longer in git or inside `ty`. `ty` carries only its source and builds it
  on the developer's machine the first time a project with a Tac Ruby service
  builds, into the per-user Tachyon cache under
  `ruby-web/<SHA-256 of the runtime source>/`; later builds reuse it, and
  nothing is written into the project. Rust drives the build on macOS, Linux
  and Windows: it downloads each pinned tarball with `curl`, checks its
  SHA-256 (a mismatch deletes the download and stops), unpacks it with `tar`,
  and runs the developer's `ruby` (mruby's `minirake`) and `emcc`. Such a
  project now needs Ruby, Emscripten 6.0.10 or newer (a minimum, no longer an
  exact pin) and a C compiler; a project without Tac Ruby services needs none
  of them. `ty doctor` and the build name any missing or old tool with install
  steps (emsdk, Homebrew `emscripten`, Windows emsdk, RubyInstaller DevKit).
  The first build takes several minutes. `ty` is about 1.1 MB smaller. CI
  installs the pinned Emscripten where Ruby Tac tests run, and those tests
  skip with a reason when the runtime is neither cached nor buildable.
  `runtimes/ruby-web/check.sh` and the artifact SHA-256 pin are removed;
  `scripts/ruby-runtime-test.mjs` now takes the built runtime's path.

- `ty serve` and `ty preview` start faster. A restart serves the last build at
  once and rebuilds in the background, then reloads open pages. Dart, the
  WebAssembly languages (Rust, C#, Go, Ruby) and script services build in
  parallel; `TAC_BUILD_JOBS` limits how many toolchains run at once. The
  development server builds C# services without the native relink (release
  settings stay for `ty bundle`, with a separate cache entry), and TypeScript
  service output is cached. Each build prints its phase timings. On the
  website (Mac, debug `ty`): cold start 15.2 s to 5.1 s, warm restart 2.3 s to
  0.7 s.
- Website code examples are real files under `website/content/`, with their
  real extensions. `website/client/shared/data/docs.json`, `features.json` and
  `languages.json` are generated from them by `bun run content`, and a test
  fails when those files are stale or edited by hand. The docs "Edit this
  page" links open the topic or feature JSON file in `website/content/`.
- **Breaking:** annotations have plain English names (ADR 0047), so a
  beginner who reads English as a second language can guess what each one
  does. `Autowired` and `Inject` are `Needs` (on a field, or on a class with
  its provider types), `RequiredArgsConstructor` is `NeedsAll`, `Qualifier`
  is `Named`, `Scope` is `Lifetime`, `Repository` is `Store`, `Mapper` and
  `Mapping` are `Converter` and `Convert`, `Configuration` is `Setup`,
  `ConfigurationProperties` is `Settings`, `Value` is `Setting`, `Profile`
  and `ConditionalOnProperty` are `WhenProfile` and `WhenSetting`,
  `PostConstruct` and `PreDestroy` are `OnStart` and `OnStop`,
  `EventListener` is `OnEvent`, `Async` is `InBackground`, `Scheduled` is
  `Schedule`, `Cacheable`, `CachePut`, `CacheEvict` and `CacheConfig` are
  `Cached`, `UpdateCache`, `ClearCache` and `CacheSettings`, `Retryable` is
  `Retry`, `ConcurrencyLimit` is `MaxAtOnce`, `Transactional` is
  `AllOrNothing`, `PageableDefault` is `PageDefaults`, `HttpExchange` is
  `WebCall`, `Validated` and `Valid` are `Checked` and `Check`, `NotNull`,
  `NotBlank` and `Size` are `Required`, `NotEmpty` and `Length`,
  `TachyonTest` and `TestConfiguration` are `Test` and `TestSetup`,
  `MockBean` and `MockitoBean` are `Fake`, and `ActiveProfiles`,
  `TestPropertySource` and `DirtiesContext` are `UseProfiles`,
  `TestSettings` and `FreshStart`. Every spelling follows, such as Python's
  `@needs_all` and Rust's `#[yon::cache_settings]`. An old name fails with
  `TY1308`, and the message names the new one. The role folders follow:
  `client/` and `server/` hold `stores/`, `setups/` and `converters/`
  instead of `repositories/`, `configurations/` and `mappers/`, a role type
  ends with its new role name (`ProductStore`, `CatalogSetup`,
  `ProductConverter`), and a test setup is `<Name>TestSetup.<ext>`. A file
  in an old folder fails with `TY1310`. Runtime error messages that named an
  old annotation name the new one. JSON argument keys, error codes and
  framework types such as `BehaviorConfiguration` do not change. Rust's own
  `#[test]` is never the `Test` marker. `ty migrate fix --write` renames the
  markers in every language, moves the files out of the old role folders and
  renames the moved role types and test setups everywhere they are named.
- **Breaking:** Tachyon annotations, lifecycle names, `Behaviors` and the
  framework types are global in every language (ADR 0046). JavaScript and
  TypeScript no longer import them: any import from `tachyon:behaviors`,
  value or type-only, fails with `TY1308`, and the message names the file and
  line and asks you to delete the import. `tachyon-behaviors.d.ts` now
  declares the markers and types with `declare global`, including
  `NeedsAll`, `Schedule`, `Id`, `Controller`, `Component`, `Needs` and
  `Stream` (the ADR 0047 names), and no longer declares the `tachyon:behaviors`
  module. A module that uses only `Behaviors` is now lowered, so the binding
  exists at run time. `ty init`, the templates, the website and the guides
  drop the import. `ty migrate fix --write` deletes these imports.
- **Breaking:** annotated files and plain Yon handlers, workers and middleware
  hold one class and its imports (ADR 0044). Extra top-level declarations
  fail with `TY1310`, so a project with helper code in these files stops
  building until the code moves to its own file, `models/` or `shared/`.
  An empty or comment-only route marker is allowed only when a controller
  for that route exists; without one it fails with `TY2015`. Rust permits
  impl blocks belonging to the file's one type; generated units and external
  relay/command programs are not subject to the authored-file rule. A few
  harmless forms stay accepted, such as a Python module docstring,
  `"use strict";`, PHP `declare(strict_types=1);`, Rust `#![...]`, a Kotlin
  `package` line, a C# `namespace` (file-scoped, or one block around the
  class) and `#nullable` lines; a Java route marker may keep its `package`
  and `import` lines. Code that runs when the file loads is rejected: a
  JavaScript or TypeScript `static { ... }` block, a computed base class
  such as `extends mixin(Base)`, and a JavaScript, TypeScript or Python
  decorator argument that is not a literal or a class name. The ADR lists
  every accepted and rejected form. `ty migrate fix` lists each file that
  breaks the rule and the split to do.

- **Breaking:** root middleware is a class named `Middleware` with
  `before(request)` and `after(request)`. The runtime calls `before` before
  the route and `after` after it, for every request, whatever its HTTP
  method, in every Yon language. A missing `before` or `after` lets that
  phase pass; a 204 answer still continues and any other status still
  answers. Before this change, the runtime called middleware as
  `Handler.<HTTP method>`, so PHP middleware without `OPTIONS` or `POST`
  answered CORS preflights and form posts with 405. A `Handler` class in a
  middleware file now fails with `TY1310`; rename it to `Middleware` and move
  its code into `before` and `after`. `ty migrate fix --write` does this
  when every HTTP method function holds the same code, and otherwise lists
  the file as a manual step.

- Annotation discovery now composes every same-language file in a role folder
  that carries a framework annotation, including behavior-only classes such
  as a `Validated` input in `delegates/`, not only files with a role
  annotation. A plain helper is not composed because of an array index
  (`row[id]`), a Python matrix product (`a @ b`) or a Kotlin label
  (`this@Label`).

- TestConfiguration classes live in same-language sibling files named after
  the class, with the exact same case. TachyonTest selects one explicitly;
  production bundles do not discover test configurations.

- Breaking for relay programs: a `relay(command, request)` program now gets
  only the baseline environment, the explicit allowlist, `YON_HANDLER_PROTOCOL`
  and its handler's `NO_PROXY` values. It no longer
  receives `YON_APPLICATION_JSON` or any `YON_RESOURCE_*` value, even when
  allowlisted. A relay program that read `YON_APPLICATION_JSON` must read its
  configuration from its own files or an explicitly allowlisted variable.

- A relay's deadline is now the time left in the request, after queueing and
  process start, and every relay in a request shares it. The supervisor
  writes the time left (minus the cancellation grace) into
  `request.deadline_ms` just before it writes the request, so handlers also
  see the remaining time instead of the original value. PHP captures the relay
  timing and environment before the application initializes, C# counts from
  its process start, and handler code cannot change the budget. A
  compiled-language relay now also reports `handler.relay_failed` categories.
- Every relay failure reaches the handler as HTTP 502 with the one message
  `Component invocation failed.` in all nine languages. PHP, Java, Kotlin, C#,
  Rust and Dart used to answer some failures with `Component returned an
  invalid response.` or `Component could not be started.` Relay output that is
  JSON but not an object is now a `protocol` failure everywhere.

- Models folders also hold data types in Rust, C#, Dart and Kotlin in both
  realms, in Swift in `client/models/` only, and in Java, Python and PHP in
  `server/models/` only (ADR 0043). A models file holds data, not behavior;
  docs/BEHAVIOR_PROJECT_STRUCTURE.md lists what each language may declare.
  A method, a function, a constant, a service marker, any other statement,
  or a language in the wrong realm fails with TY1310, and the message says
  where the code belongs. A service imports a model in its language's usual
  way: `use crate::models::Name;`, `using Models;`,
  `import 'models/Name.dart';`, the Swift type name, `import models.Name`,
  `import models.Name;`, `from models import Name` or `use Models\Name;`.
  The framework adds the realm's models files of that language to native
  companions, WebAssembly and Dart browser services, Yon handlers and test
  units, and never routes or ships a models file on its own. Composed Rust
  and PHP role files now skip a repeated, identical `use` line, so several
  role files can import the same model.
  The project-root middleware file may import models, as a handler does.

- Add `client/models/` and `server/models/` for type-only TypeScript files
  (ADR 0041). A models file holds only `interface`, `type` and `import type`
  statements. Other files use it with `import type { X } from '/models/X.ts'`,
  which TypeScript removes, so the file is never bundled or run. A role
  annotation, any other statement, or a file in another language in a models
  folder fails with TY1310, and the message names the folder to move it to.
  When role files are composed, `import type` statements now stay type-only.
  The website's service interfaces moved to `website/client/models/`.

- `tac.config.js` default-exports a class, and its values are static fields
  of that class: `static application`, `static scripts`, `static styles`,
  `static metadata`, `static cache` and the `static postBundle` hook
  (ADR 0040). The build rejects the old file-level exports and a default
  object with TY1601; the message names the static field to write instead.
  The scaffold, the website and the documentation use the class.

- Every Tachyon source file is simplified for a new-graduate reader. Code has
  no explanatory comments and no file-level variables, the CLI help text lives
  in clap attributes, and the documentation uses plain English. Behavior is
  unchanged.

- A `@RequiredArgsConstructor` with no arguments on a Tac service takes every
  uninitialised field from the component's properties. This matches Lombok's
  annotation, which takes no arguments (architecture decision record, ADR
  0039). Declaring it makes one service per component. You need `@Scope` only
  to override that, or for a per-component service without properties. A
  service that used it to inject clients now autowires them.

- The website documents this release's changes. Two new feature pages cover
  shared data and the page in views, and Rust and C# on the web. The guides
  and feature pages for where services run, page metadata (`noindex`),
  `ty doctor`, loop scope and `RequiredArgsConstructor` are updated. Code
  examples show each service language where it actually runs. Rust and C#
  serve the web as WebAssembly, and Dart examples have their own tab. The
  snippet verifier builds the web example to WebAssembly in both languages.

- Tachyon autowires resources by their Spring names (ADR 0038). `@Resource` is
  removed. An `@Autowired` field named after a resource receives it. The
  browser's resources are `view` (was `element`), `lifecycle` (was `signal`),
  `taskScheduler` (was `renderScheduler`), `cacheManager`, `resourceLoader`
  and `applicationEventPublisher`. The server's resources are `dataSource`,
  `redisTemplate` (was `redis`, now the `RedisTemplate` type) and
  `applicationEventPublisher`. Tachyon rejects `@Resource` and each former
  name, and says what to write instead.

- A marker that takes no arguments is written without parentheses, as Spring
  writes it: `@Service`, `#[Service]`, `[Service]`. In JavaScript and
  TypeScript, a role marker without arguments now means the role of the realm
  that holds the file. So `@Service` under `client/` is the Tac service. The
  empty argument list still works, and the TypeScript declarations accept both
  forms. The documentation, website, scaffold and fixtures now use the form
  without parentheses.

- A `<loop>` binding is a local variable inside its loop. So a binding with
  the same name as a service no longer loads that service for the view. Only a
  name the view reads outside a loop, or a handler it names, loads a service.

- Declare an `@HttpExchange` the way Spring does, and the way a repository
  query is already declared: the signature only, with an empty body. `query`,
  `headers` and `body` map names to argument indices. The compiler writes the
  request, and nothing ever calls the empty body. A method that writes a body
  still returns its own request descriptor, and it may not also map arguments.
  The published fixtures in all nine languages include a declared exchange.

- Use Spring's name for every annotation that had a Tachyon-only name.
  `@Cache` is now `@CacheConfig`, `@Pageable` is `@PageableDefault`,
  `@HTTPExchange` is `@HttpExchange`, and the `Delegate` role is `@Component`.
  Its folder stays `delegates/`, so a Tac project's `client/components/` still
  holds only views. Tachyon rejects a removed name and names the replacement,
  instead of reporting the name as unknown.

- Inject fields the way Spring does. `@Autowired` on a direct, uninitialized
  instance field adds it to the constructor that Tachyon generates.
  `@Qualifier` names the provider when the field's own name or type does not.
  Both realms and all nine languages accept them, and every published example
  now uses them. Lombok's `@RequiredArgsConstructor` and JSR-330's
  `@Inject(A, B)` still parse while third-party sources migrate.

- Each realm's configuration now sits next to it (ADR 0034). The web app
  manifest moved from the project root to `client/manifest.json`. Yon's
  `application.json` and its profiles moved to `server/application.json` and
  `server/application-<profile>.json`. `.env` stays at the root. A file at an
  old root location stops the build, and the message says where the file
  belongs. `ty migrate` also reports it.

- `server/application.json` and its profiles are the `yon` namespace itself,
  so they have no `yon` root key. `yon.*` paths, `YON_` names and the
  `YON_APPLICATION_JSON` snapshot are unchanged. Tachyon rejects a leftover
  wrapper, and `ty migrate` reports it.

- Tac's public configuration moved from `application.json` to the `tac` member
  of `client/manifest.json`, the web app manifest (ADR 0034). The build
  validates it as before. The build publishes the manifest at
  `/manifest.webmanifest` and links it from every page. The build no longer
  writes `/.tachyon/application.json`. `TAC_` names and `tac.` paths are
  unchanged. `application.json` now belongs only to Yon (Application
  Configuration v3). A `tac` section left in it stops the build with a message
  that names the manifest, and `ty migrate` reports it. Every behavior runtime
  now binds a document that has other members next to its realm section. So a
  service binds the manifest it loads without changes. `ty init` writes
  `client/manifest.json` and `server/application.json`.

- Every public method of an annotated class needs a test, and `ty preview`
  enforces this. A class-level marker says that the framework owns some of the
  type's behavior. So the type's public methods are the application's contract.
  The development server does not start while any of those methods is not named
  by a `test*` case. It lists the missing methods (`TY2023`). The rule does not
  affect `ty start`. `TAC_ALLOW_UNTESTED=1` turns the rule off for one run.
  `ty test` prints the same report, but fails on it only with `--coverage`.
  "Covered" means that a case names the method. This is only a record, not
  proof that the method ran. Methods that the compiler writes are exempt,
  because a derived query, a declared `@HttpExchange` and a `@Mapping` contain
  no code written by the author. `ty init` now creates a Configuration and a
  test unit that covers the service it creates. So a new project starts with
  all checks passing.

- The published-example verifier now stages its work on the volume with the
  most free space, and it keeps what it compiles. Each step's project directory
  is now stable and reused between runs. Before, it was a new `mkdtemp`
  directory that was deleted at the end. So `<project>/.tachyon/
  handlers` — where `ty` caches compiled handlers — is kept. An unchanged Rust
  example ran again in 1.4s, compared with 12.4s on a cold run.
  `TAC_SNIPPETS_SCRATCH` sets the root directory. `TAC_CACHE_DIR` sets the
  standalone runtime cache. Both default to the ANNEX volume, next to
  `CARGO_TARGET_DIR`.

- The verifier runs independent steps at the same time. It stops waiting for a
  toolchain that does not get enough CPU time. Each step stages its own project
  and binds port 0, so steps can safely run at the same time. The verifier
  buffers the output of each step and writes it in the order the steps were
  submitted. So the log is the same at any level of concurrency. The limit on
  parallel steps comes from the *free* cores, not the total core count. A
  `kotlinc` without enough CPU does not just make the suite slower. It makes
  the suite fail when the 30-second server readiness deadline expires. After a
  language misses that deadline twice, the verifier skips the remaining
  examples for that language and names the cause once. It does not wait for the
  deadline again. On a loaded machine, this turned 33 repeated timeouts into 33
  skips. It cut an annotations run from about 25 minutes to 3 minutes. It cut
  the full suite from 1935s with 36 failures to 847s with 7 failures on a cold
  run, then 489s on a warm run. With `TAC_SNIPPETS_REQUIRE_TOOLCHAINS=1`, the
  verifier never skips a language this way. CI (continuous integration) is not
  the busy machine that this rule is for, and CI must check every example.
  Every run now ends with a timing report that shows where the time was spent.

- A marker that takes no arguments no longer needs an argument list.
  `@Configuration`, `@Autowired`, `@Mapper`, `@TestConfiguration` and
  `@DirtiesContext` can now be written without parentheses in JavaScript and
  TypeScript too, as in every other language. Bare `@Service`, `@Component` and
  `@Repository` are still Yon's role stereotypes. So their Tac forms still use
  parentheses. The compiler still rejects a marker that gets an argument it
  does not accept.

- The compiler now rejects Spring's slicing and mocking annotations and names
  the alternative. Before, it ignored them without a message. `@WebMvcTest`,
  `@WebFluxTest`, `@DataJpaTest`, `@JsonTest`, `@RestClientTest`,
  `@AutoConfigureMockMvc`, `@SpyBean` and `@MockitoSpyBean` each say what to
  write instead. There is no global context to slice, so `@TachyonTest`
  replaces the slicing annotations. There is no bean to wrap, so `@MockBean`
  replaces the spies. `@SpringBootTest` is removed. Use `@TachyonTest` instead.
  JUnit's generic names (`@Test`, `@BeforeEach`, `@Disabled`) still have no
  effect. They belong to your test runner, and other tools can use the same
  names.

- A role file registers one type that has behavior. A second class-level
  `@Controller`, `@Service`, `@Repository`, `@Client`, `@Component`, `@Mapper`,
  `@Configuration`, `@CacheConfig` or `@Scope` in the same file fails with
  `TY1310`. The error names the type to move. The reason is that the filename
  states the one type that the file registers. Companions that describe a type
  but do not register it stay next to it. Examples are a `@Validated` DTO (data
  transfer object), a `@Value` or `@ConfigurationProperties` holder, and a
  helper without annotations. They stay because `@Valid({"type":"OrderInput"})`
  and `Behaviors.values(PublicSettings, …)` look up their type in the same
  source file. A reserved `yon.*` or `tac.*` entry composes the role files that
  it finds. So several behavior types can correctly share one entry.

- `@Mapper` and `@Mapping` use MapStruct's names. `@Mapper` is the role, in
  `mappers/<Name>Mapper`. `@Mapping` defines the whole method as a literal plan.
  Each entry in the plan names a `target`. It takes either a dotted `source`
  path or a literal `constant`. A `source` path has at most eight names, and a
  path to a missing value maps to `null`. `argument` selects the parameter that
  the entry reads. It defaults to the first parameter. A plan has at most 32
  entries, and a target written twice fails the build. The mapper drops an
  input field that no entry names. All nine languages support it, each in its
  own runtime, in both realms. `client/mappers` and `server/mappers` are added
  to the shared role folders.

- Realms mirror each other (ADR 0031). Yon role folders are shared per realm:
  `server/services`, `server/repositories`, `server/clients`,
  `server/configurations`, `server/entities` and `server/delegates`. Every
  route uses them, in the same way as `client/<role>`. A route directory holds
  only its `yon.<ext>` entry and its `controllers/` folder. A role file next to
  a route fails with `TY1310` "@X type 'Y' sits outside its realm's shared role
  folder". `ty migrate` reports it as `role.route` with the shared destination.
  `Controller` and `Stream` are the only markers that exist only in Yon. Tac
  gained `@Component()`, bare `@Repository()` and `@Inject(A, B)`. Yon gained
  `@Resource()` for `dataSource`, `redis`, `applicationEventPublisher` and, in
  JS/TS, `signal`. Events follow Spring. `@EventPublisher` is removed. A class
  declares an uninitialised `applicationEventPublisher` field and calls
  `publishEvent(event, value[, retain])`. `@EventListener` takes `event` or
  `events`, a compiled `condition` over `value` and `event`, `id` and `replay`.
  `@Order(n)` orders listeners, lowest first. `@Async()` delays a listener
  until after the current delivery pass. Yon php/java/kt/cs/rs rejects
  `@Async()`. A listener can return an `{event, value}` record, or an array of
  at most 32 records. That result is published again.

- Replace authored Tac/Yon module manifests with annotation-driven entry
  scopes. Old manifests fail with migration guidance. Replace public
  `this.tac.fetch` with declarative Client methods. Opt-in offline cache
  policies still work.

- Check JavaScript behavior deadlines with a monotonic clock around execution.
  This prevents late cache writes when application work delays timers.

- Dispatch annotated Yon controllers by uppercase HTTP method names. The extra
  `@GET`/`@POST`/other verb markers are no longer needed. Removed markers fail
  with migration guidance. Constructor injection, response behavior and
  streaming do not change.

- Replace Tac lifecycle/signal decorators with a plain default-exported
  presentation model. It has an optional `mount()` for lifecycle work and
  explicit owner-scoped signals: `this.tac.publish(...)` /
  `this.tac.subscribe(...)`.

- Keep plain `Handler` as the minimal convention. Ordinary helpers without
  annotations need no role naming. Optional role-annotated types follow the
  strict suffix, file and folder rules added in ADR 0024.

- Infer streaming from native generator/yield syntax. Make relay an explicit
  `relay(command, request)` call. Keep the existing shell-free bounds,
  deadlines, sanitization, cancellation and process-group cleanup (reaping).

- Use Node as the default Yon JavaScript runtime. The framework still probes
  configured JavaScript and TypeScript runtimes for capabilities before
  startup.

- docs/THREAT_MODEL.md now states what environment scrubbing does and does
  not do. Removing `YON_APPLICATION_JSON` and `YON_RESOURCE_*` from a relay
  program or a Gateway command prevents accidental inheritance. It is not a
  sandbox: the program runs as the same operating-system user, so it can still
  read its parent's environment and the project's files. The document also
  lists the Windows baseline, which differs from the Unix one.

### Removed

- Remove runtime decorator shims, generated layer annotations, Rust
  procedural-macro compilation, layer dependency graphs that the framework
  enforced, and the old `@Stream`/`@Relay` rewriting path for mandatory layers.
  The compile-time annotations of ADR 0021 replace that architecture. They do
  not bring back its runtime shims.

### Fixed

- The website's Documentation page shows its guides again. Its C#
  WebAssembly service received all of `docs.json`, with the text of every
  guide. That file grew past the 1 MiB that `init` may carry (ADR 0037), so
  the service refused the page's properties with "Native companion operation
  failed.". The docs pages now pass `guides.json`: the reading order, the
  groups and each guide's title and summary. A website test keeps every data
  file that a template names at 1 MiB or less.
- The test rule (`TY1339`) now reads only test units. Before, it scanned
  every JavaScript, TypeScript, Python and Rust file, and a large minified
  library made `ty build` of the website take more than ten minutes.

- A page with both Kotlin and Go WebAssembly services now starts. Go's
  `wasm_exec.js` puts a small `process` object on the page, and the Kotlin
  loader read `process.release.name` from it and stopped; the published
  Kotlin loader now reads `process.release?.name`.

- The visibility rule (`TY1333`) now reads Groovy files when it looks for
  uses of a member. Before, a Groovy controller that called a gateway did not
  count, so the Python service method behind the gateway was refused as unused.
- The visibility rule (`TY1333`) keeps the methods of a class public when a
  `Gateway` names its file, such as
  `{"class":"server/services/ReportService.java","method":"summarize"}`.
  Before, a gateway that named another method, or a gateway in a language
  whose client calls look different, left the method refused as unused.
- The Outgoing HTTP: Tac example now calls `catalog.load()` from its view.
- The website tests now build the site with the newest `ty` in the Cargo
  target folder when `TAC_BIN` is not set. Before, an old `release/ty` won
  over a newer `debug/ty`, so a change such as utility classes in Dart
  services looked broken. New tests check that Dart, Rust, Go, C# and Ruby
  web services call a `client/utils` class without an import.
- A `@LocalState` or `@SessionState` field of a WebAssembly Tac service
  (Rust, Go, C#, Kotlin, Ruby) that one of the service's own methods or
  `OnEvent` listeners changed was not saved, so a reload brought back the
  old value. The page now reads the saved fields after each call into the
  service and stores the ones that changed.
- `TY1341` (comparison with true or false) is no longer suggested in Go. A
  Go value of type `any` can be compared with `true`, but it cannot be a
  condition by itself, so `ty migrate fix` wrote Go that did not compile.
- A Python `async def` method was read only up to its first line, so the
  cohesion check (`TY1314`) never saw its branches and loops, and the
  visibility check (`TY1333`) took its local values for public class
  fields. The whole body is now read, as for a plain `def`.
- A Kotlin Android service method that takes arguments now compiles; before,
  the generated table always called it with none. A model-typed or `Float`
  Kotlin field can be set from the page.
- `TY1330` quoted a repeated text together with the method called on it,
  as in `"received".to_owned`, and suggested a Rust constant that does not
  compile. It now shows only the text (`"received"`) and suggests
  `const RECEIVED: &str = "received";`. Only a number can have a fraction
  part after its point.
- A Tac view that passed a property, such as `:documentation`, to a
  component whose `NeedsAll` service has a field named like another
  service's instance (`documentation` next to `DocumentationService`) lost
  the value without a word: the field got the service. The build now stops
  with `TY1407` and says to rename the field or to stop passing the
  property (ADR 0057).
- A Ruby test unit that used `Clock` or `SecureRandom` stopped with an
  undefined `host_emit` method, because the page check always found the
  `TachyonWeb` module. The unit now gets the fake answers (ADR 0065).
- A Yon test unit in `server/tests` now sees `SocketRoute` controllers,
  error handlers and the classes in `server/errors`, as the test-units guide
  says, so a socket controller or an error handler can be tested.
- A Tac view (a `tac.html` under `client/pages` or `client/components`) that
  contains a doctype, `<html>`, `<head>` or `<body>` now fails the build with
  `TY1307`, as the docs say. The error points at each of these tags. Before,
  the build accepted the view. Tachyon writes the document around every view,
  so remove these tags and put page metadata in `client/manifest.json`, under
  `tac.metadata`. The same tags written as text, such as `&lt;html&gt;` in
  `<pre>` or `<code>`, still work. The Tac browser runtime no longer holds
  code for a view that writes its own document.
- A handler or middleware answer no longer sets `content-length`,
  `transfer-encoding` or `connection` on the HTTP response. A Dart shelf 204
  from `after` carried `content-length: 0` onto the route answer and stopped
  the server connection.
- A Dart handler cannot extend its relay deadline again: the deadline is set
  once, and a socket call renews it through a separate start time.
- Two TypeScript or JavaScript role files that import the same module as
  `'/models/X'` and `'/models/X.ts'` (or `.js`) no longer stop the build with
  TY2101. Tac and Yon now treat an import with and without the extension as
  one module and keep the first spelling.

- Yon models keep their parent fields: a Java model class that extends
  another model now reads and writes the parent's fields, and a Dart model
  that extends another model and uses `super.name` in its constructor now
  compiles and keeps the parent's fields. A Dart controller no longer fails
  with "Type not found" when the project has models that it does not import.
  C# streams now send model fields in camelCase like other answers, and a
  Dart stream of models no longer ends with TY2107 (ADR 0048).
- A C# Tac service in the browser can now use its annotations (ADR 0059). An
  `async` method that returns `Task` or `Task<BehaviorJSON>`, or a method
  that returns `BehaviorJSON`, is a page method, and a `BehaviorJSON`
  parameter takes any page value. Before, the page could not call these
  methods, and `public async System.Threading.Tasks.Task<...>` became a field
  named `System` that did not compile (the C# caching example had this
  error). A `BehaviorContext` field is no longer a page field. Every C#
  service with the behavior runtime now gets the page storage host (before,
  the build failed with "The name 'BehaviorStorage' does not exist"). The
  .NET runtime no longer keeps its own copy of its files in the Cache API
  (`BlazorCacheBootResources` is off), so a page with denied storage still
  renders when it has a C# service.
- A Kotlin, Swift, native Rust or native C# Tac service that uses a cache
  (`@Cached`, `@UpdateCache`, `@ClearCache`) or a Store whose adapter no Setup
  names now stops the native build with a plain message, for example "Kotlin
  services need a cache adapter in their Setup". Before, the service built and
  failed at run time with `cache-adapter` or `entity-adapter`: the built-in
  browser cache and Store reach only JavaScript, TypeScript, Dart and Go, Rust
  and C# WebAssembly services (ADR 0059).
- A Swift or Kotlin native Tac service with `@NeedsAll` can declare `let x: T`
  or `val x: T` without a value (ADR 0057). Tachyon generates the constructor
  (Swift `init?(tacProps:)`, a Kotlin secondary constructor) that fills these
  fields from the component's props, because a NeedsAll class may not write
  one. A missing value or a value of the wrong type stops the service with
  `TY_NATIVE_REQUIRED` instead of creating it.
- A Rust Tac service with behavior annotations builds again for the web and
  native hosts: its `behaviors: BehaviorContext` field is no longer offered to
  the page as a field.
- A rebuilt `ty` no longer reuses routes from an earlier `ty` build of the same
  version: the route reuse key includes the `ty` executable identity.
- A Kotlin or C# native Tac service that throws now answers the page with
  the fixed error "Native companion operation failed.", like Swift and Rust.
  Before, the Android and Windows runtimes sent the exception message to the
  page.
- A Swift or Kotlin Tac service now accepts `@Service`, `@Lifetime` and
  `@NeedsAll`, like the other service languages. Before, the iOS, macOS and
  Android builds refused them with `TY1308` "This behavior annotation belongs
  on a method". A `@Lifetime` prototype now also reaches the page.
- A Tac service in Rust, C#, Go, Dart, Swift or Kotlin now refuses a bad
  `Lifetime` argument, as JavaScript and TypeScript already did. The value must
  be `{"value":"singleton"}` or `{"value":"prototype"}`. Before, any text was
  accepted, and only the word `prototype` mattered. The error points at the
  marker's line. Every service language now reads these markers with one
  shared helper.
- The home page Tac snippets now mark every service class, in each language's
  own spelling.
- A Rust or C# service now builds for native targets that do not link it,
  as ADR 0035 and ADR 0037 say. The native web view runs its WebAssembly
  module, as it already did for Go. Before, the native build refused with
  `TY1010`: Rust on iOS and Android, and C# on macOS, iOS, Android and
  Linux. A linked language still wins where the target links one.
- The Windows host's Content Security Policy now allows
  `'wasm-unsafe-eval'`, like the Linux host and the web server. Without it,
  the WebView2 page could not compile a Go service's WebAssembly module.
- A native build now refuses a web bundle file over 16 MiB with `TY1605`.
  Native hosts serve files only up to that size, so such a file used to
  fail silently at run time.
- The `TY1010` help no longer suggests the retired `tac.js` and `tac.ts`
  files. It names the service files that reach the target, such as
  `client/services/BasketService.js`.
- Rust, C# and Go WebAssembly services now receive other services' local
  events in the browser. Their listeners join the page event bus with
  condition filtering, Order, retained replay and owner-bound cleanup.
  The page also orders the publisher's own listeners and defers Async
  listeners to a microtask, regardless of the publishing language.
  Returned events are forwarded once, and a service does not receive its
  own publication twice through the page bridge.
- The "Configuration binding: Tac" example ran in no language: its manifest
  holds `tac.catalog.title`, which falls under the `TAC_CATALOG_` prefix, and
  no configuration declared a `title` field. Every language now declares it,
  and the snippet verifier runs the example in a real browser (or as a
  native test unit) instead of only bundling it.
- The C# behavior runtime wrote JSON through `JsonSerializer.Serialize`, which
  fails in a trimmed WebAssembly build where reflection is off. It now writes
  JSON with `Utf8JsonWriter` and `JsonEncodedText`.

- A JavaScript or TypeScript `@TestConfiguration` was never registered as a
  Configuration. So a unit that named one could not create its context. The
  other seven languages already registered it. It is now registered, so
  test-only configuration can stay in the unit. It no longer has to be included
  in every service bundle. `ty init` creates it that way.

- When a browser unit's driver threw an error before the first case, the unit
  reported "0 passed, 0 failed" and exited without an error. It now fails and
  gives the reason. A unit that declares no case fails and says that it has no
  method with the `test` prefix.

- The lifecycle markers of a test unit (`@PostConstruct`, `@PreDestroy`,
  `@EventListener`) reached the runtime without being lowered by the compiler.
  The runner now runs the same event pass that a browser service goes through.

- `ty test .` failed with "Captured project root is invalid." The command now
  resolves a relative project path first. The library entry point now also
  resolves symlinks, because Bun does not load a module through a symlinked
  prefix such as macOS's `/var`.

- A browser unit could resolve `/shared/...` into a directory that no longer
  existed. Bun caches transpiled modules by their content, and it stores a
  plugin's resolved path in the cached output. A staged unit has the same bytes
  on every run, but the project that it resolves into moves between runs. Unit
  runs now skip that cache.

- Method coverage matched method names across the whole project. So one case
  that named `list` covered every `list` method. A unit now covers only the
  types that it names.

- Keep C# handler compilation independent of persistent .NET build servers.
  This stops an old compiler server from blocking valid builds after the
  toolchain volume reconnects. Compiler deadlines and the framework's
  content-addressed artifact cache stay.

- Make the JavaScript shared-deadline regression test deterministic under host
  load. It covers expiration during authorization, cache lookup and result
  storage. It does not depend on closely spaced wall-clock timers.

- Keep HTTP-only verification independent of file-watcher registration.
  Dedicated rebuild and watcher-lifecycle tests still turn on watching
  explicitly.

- Reject malformed `Value` descriptors without compiler panics or silent type
  defaults. Support TypeScript definite-assignment and optional fields. Align
  native field-name and selected-string bounds across behavior runtimes.
  Restore the template bindings of the standalone Java/Kotlin configuration
  gate.

- A PHP relay program no longer inherits the descriptors its handler opened.
  Every descriptor above 2 maps to `/dev/null`, so the program cannot write
  into the handler's response stream, for example through `php://stdout`.
  PHP warnings now go to stderr instead of corrupting the protocol output.

- The Bun and Dart managed-resource clients no longer send the invocation's
  bearer token through `HTTP_PROXY`. Every runner connects directly to the
  loopback endpoint, and the supervisor adds `127.0.0.1` to `NO_PROXY` and
  `no_proxy` for a handler that has an endpoint.

- A JavaScript relay to a missing program fails at once with the `start`
  category. Under Bun it used to wait for the deadline, and under Node it
  reported `exit`.

- A handler whose deadline passes while it waits for a process slot fails with
  `TY2110` and never starts. A passed deadline now wins over a free slot, and
  the supervisor checks the time left again just before it spawns.

- A `#[Component]` (or C# `[Component]`) delegate beside a Rust, C#, Swift
  or Kotlin web service failed with `TY1308` at an offset in the service file.
  The error now names the delegate file and the line of its marker, as it
  already did for Go.

- When `rustc`, `kotlinc`, `dart` or `dotnet` was not on the PATH, a compiled
  handler failed with `TY2004` and help about the 1 MiB source limit. It now
  fails with `TY2112`, names the missing program and says what to install,
  like Go.

- `ty doctor` exited with 0 even when the project could not build. It now
  exits with 1 when a toolchain that the project needs to build or serve is
  missing or too old. A toolchain that is needed only to bundle a native
  target is shown as `info` and does not change the exit status.

## [26.36.01] - 2026-08-31

### Added

- Reconcile the newer local feature list with the released Rust engine. Kept
  features: publish/subscribe, persistence decorators, bounded browser helpers,
  counted loops, manifest and configuration capture, and target-native
  companions.
- Validate captured request body, header and parameter contracts with bounded
  CHEX execution. Publish deterministic API schemas and an interactive
  reference.
- Restore development `preview`, production `start` with an immutable bundle,
  and single-target `bundle --native`. Existing command aliases still work.
- Before publication, run behavioral schema, companion ABI (application binary
  interface), compiler, browser and website gates against the actual
  downloaded release executables.

### Changed

- Native targets render the bundled Tac application through WKWebView,
  WebView2, WebKitGTK or Android WebView. Native Host contract v3 replaces the
  old widget-tree metadata. Page companions compile for their native OS.
  Browser WebAssembly companions are replaced (ADRs 0018 and 0019).
- Fetch persistence is explicit, has no credentials and is bounded.
  Service-worker caching skips authenticated or private responses and verifies
  packaged static bytes. Native hosts remove old service workers.
- Android pins AndroidX WebKit 1.14.0 for a frame-aware asynchronous bridge
  with no legacy fallback. Pending calls and worker admission are bounded.
- Keep normal JavaScript module helpers. Keep all released guarantees for
  process supervision, source snapshots, routing, environment namespaces and
  atomic builds.

### Fixed

- Keep input focus, caret, selection and the connected editing node during
  reactive updates. This includes inputs without IDs and custom-element slots.
  Website search no longer moves the caret to the end after results update.
- Keep later native field edits when earlier asynchronous replies arrive.
  `Range`/`If-Range` requests skip persistent response caching. Partial
  `206`/`Content-Range` responses cannot become cache hits for complete
  responses.
- Share bounded Foundation JSON parsing and canonical request serialization
  across Apple hosts and Swift companions. This replaces the handwritten value
  scanner and rejects duplicate root keys before dispatch.
- Decode HTML entities once, remove a leading UTF-8 BOM (byte order mark) and
  add mobile viewport metadata. Bound nested empty loops and cancel old
  component mounts.
- Keep native companion state per route, reject bridge requests with a
  mismatched route or origin, and package Android `_id` dynamic route
  directories.
- Serve Linux native stylesheets with their HTTP content type. Route Windows
  WebView2 resources, including dynamic pages and their native companions,
  through the canonical local responder.
- Limit compiled-controller endpoint and stream discovery to the controller,
  so colocated helper types are excluded. Run the published layered examples
  against all eight server runtimes. Report snippets that did not run
  separately.

### Migration notes

- Native companions are trusted application code with OS privileges. They are
  not sandboxed WebAssembly plugins. Automatic native member discovery supports
  fields and methods with zero arguments. Methods with parameters need explicit
  member tables.
- Native-call timeouts stop the wait. They do not stop native execution, and
  they do not undo completed mutations. Keep companions responsive. A hung
  companion may need a relaunch. Android runtimes without the required WebView
  messaging feature report the bridge as unavailable. They do not fall back to
  a legacy interface.
- Existing version tags and assets do not change. See
  [the reconciliation inventory](docs/RECONCILIATION.md) and the
  [client migration guide](docs/CLIENT_RUNTIME_MIGRATION.md) for acceptance
  evidence and compatibility decisions. Before publication, the release
  workflow verifies the restored features in downloaded executables.
## [26.35.07] - 2026-08-30

### Added

- Added required layer contracts: `@Controller`, `@Service`, `@Repository`,
  `@Client`, and `@Component`. Also added supervised `@Relay` migration for
  non-Yon programs, and direct multi-frame `@Stream` responses for JavaScript,
  TypeScript, Python, PHP, Kotlin, and C#.
- Added a structured signal lifecycle for `serve`, `preview`, and
  `bundle --watch`:
  - Signal handlers are registered before the process reports that it is
    ready.
  - Proof of registration is written to stderr as JSON lines that contain no
    private data. It is separate from the application's readiness message on
    stdout.
  - The first signal requests a graceful shutdown. The development server puts
    a time limit on the internal tasks it owns. Preview may wait for open
    connections to close on their own. Bundle watch may finish its synchronous
    fingerprint pass or its current bounded build.
  - Supervisors apply their own grace period.
  - A second signal forces the process to exit. The exit code is `130` for
    Unix `SIGINT` or Windows CTRL-C, `143` for Unix `SIGTERM`, and `131` for
    Windows CTRL-BREAK.

### Changed

- Development topic subscriptions now share one bounded incremental reader
  (tailer) per topic. It has admission limits, replay cursors, removal of slow
  consumers, and clear ownership at shutdown. Compiler, native, WASM,
  post-bundle, and doctor tool processes use one supervisor for the whole
  process tree. It applies a deadline and an output limit.

- Yon handlers, middleware, and workers now use the runtime that the framework
  provides for JavaScript, TypeScript, Python, Java, C#, Kotlin, PHP, or Rust.
  The old `Handler` fallback, `.tachyonrc.interpreters`, shebang handlers, and
  executable handler discovery are removed. `.tachyonrc.workers` is still
  supported.
- The JavaScript and Python runtime overrides are now
  `YON_JAVASCRIPT_RUNTIME` and `YON_PYTHON_RUNTIME`.

### Fixed

- Keep the lexical loop scope through nested iterables, text, dynamic
  attributes, conditions, component props, and event arguments. When a loop
  name repeats, the nearest loop hides the parent name only inside that loop.
  Siblings get the outer binding back. A component owns the evaluated props it
  receives.
- Check every Yon runtime that the project needs before the development server
  prepares output or binds a socket. A missing executable now reports its own
  `TY2112` diagnostic. This happens at startup and also when the executable
  disappears between the check and the invocation. `ty doctor` uses the same
  time-limited checks. It counts the JavaScript / TypeScript and Java / Kotlin
  runtime needs only once, and it never prints configured executable paths.
  The C# readiness check now builds a small project that the framework
  provides against the installed SDK. A runtime-only `dotnet` no longer passes.
- Limited the output of relays for compiled languages, and stopped delegate
  stderr from appearing in responses. Relays, streaming timeouts, and
  subscriber disconnects now apply deadlines and clean up child processes.
  Windows PHP relays do not use non-blocking `proc_open` pipe multiplexing,
  because the platform does not support it.
- End a failed stream with one sanitized SSE error event that includes the
  request ID. This covers failures before the first value and after partial
  delivery.
- Keep every admitted event on finite streams that succeed. Apply JavaScript
  stdout backpressure before asking the generator for the next value.
- Reject symlinked or unreadable non-route layer sources in a predictable way.
  Ignore stereotype names that only appear inside comments and
  language-specific strings.
- Tie one discovery pass to a retained project capability that does not follow
  symlinks. Route graphs, layer checks, Tac pages, components, and shared
  assets, build and native configuration, and Yon execution now read an owned,
  bounded snapshot. They no longer reopen authored paths from the environment.
  Standalone handlers take a snapshot of the complete permitted `server/**`
  dependency boundary before invocation.
- Bind the development server from that same discovery object. Its initial web
  build, route dispatch, selected root middleware, `.tachyonrc` schedules, and
  scheduled-worker sources can no longer mix files from before and after the
  project root is replaced.
- The development server now owns its scheduled-worker and source-watcher
  tasks. Graceful shutdown cancels them and waits a limited time for them to
  finish. Dropping a server that is only bound starts no background work and
  cannot leave a task running on its own.
- Every streaming-handler bridge, hot-update stream, and topic stream is now
  part of the same owned lifetime. Shutdown cancels response producers before
  Axum's graceful wait. It keeps the last part of one bounded deadline to abort
  and join the remaining tasks. So an SSE client that never disconnects cannot
  keep the server or a handler child alive. Finished producer tasks are
  cleaned up continuously instead of piling up until shutdown.
- Reject Firecracker execution of TypeScript and prepared direct-language
  artifacts before the driver starts. This stays in place until a bounded
  artifact-transfer contract exists. That transport still supports JavaScript
  and Python.
- Restricted the project-local `.tachyon/handlers` cache:
  - It uses directory capabilities that do not follow symlinks.
  - Cleanup checks file identity first.
  - Publication is atomic, and runtime copies are private.

  The cache now recovers abandoned locks. It enforces a limit of 256 entries /
  512 MiB. This limit is separate from `TAC_CACHE_DIR`.

## [26.33.02] - 2026-08-11

### Added

- Added a Yon isolation boundary that an environment setting selects.
  Operators can keep the supervised process backend. Or they can send handler
  entry points to a Firecracker control program through framed Handler
  Protocol v1, with limits on CPU, memory, and pool size, and deny-only egress
  settings. A partial or conflicting Firecracker configuration fails closed.
- Added semantic development updates driven by events, through Hot Update
  Protocol v1. They include:
  - bounded merging of filesystem events;
  - the last good diagnostics;
  - CSS and component updates that keep state;
  - a safe full reload when the structure changes.
- Added a weekly fuzz campaign with a persistent corpus, and an acceptance gate
  for semantic hot updates in a real browser.

### Changed

- Made Tac render only on the client. Each route's bootstrap document now
  includes a bounded render plan and `/.tachyon/tac-client.js`. It contains no
  authored subtree rendered on the server. Component companions mount on
  explicit schedules. The old `hydrate` name remains only as a schedule alias.
- Made Yon REST-only. Builds never run Yon, the compiler rejects `yon.html`, and
  Route Manifest v1 no longer lists a build-time view context.
- Put all environment variables that the project owns in one of two
  namespaces. `TAC_` is for client, compiler, packaging, and tooling variables.
  `YON_` is for server and handler variables. The removed `TACHYON_` names do
  not work as aliases.
- Removed old JavaScript implementation paths from the repository. The
  unchanging v26.30.04 behavioral oracle and a full-stack migration fixture
  that cannot run are kept.

### Fixed

- Kept process-group ownership, termination of child processes, pipe
  settlement within the deadline, and bounded reaping. This applies to both
  process and Firecracker-driver handler invocations.
- Kept these guarantees: stable-release recovery, rollback verification after
  a corrupt checksum, exact toolchain setup, website acceptance, native
  execution hardening, and the standalone installer's atomic handling of a
  failed upgrade.
- Generated static assets now win over dynamic route patterns. Mounted website
  components now request rerenders through the current `tachyonComponent`
  boundary.

## [26.33.01] - 2026-08-10

### Fixed

- The Windows installer now decodes byte-array checksum responses as UTF-8. So
  `SHA256SUMS` responses without a file extension still fail closed and can be
  used.
- Atomic Windows upgrades now use a real temporary backup path. So `.NET`
  `File.Replace` keeps the installed executable and does not reject a null
  backup path.
- Made staged-release recovery stronger. It now verifies a deliberately
  corrupted copy of the Windows executable and keeps the canonical checksum
  manifest unchanged.

## [26.32.07] - 2026-08-09 (unpublished)

This release candidate cannot be changed, and it was never made public. Staged
verification of the Windows installer found a defect in atomic replacement.

## [26.31.06] - 2026-08-01

### Removed

- **FYLO is no longer part of Tachyon.** The binary, its vendored shim, and
  every integration point are removed:
  - the browser client that the compiler injected;
  - the `fylo` facade in the polyglot companion preludes and template
    capabilities;
  - the `/_fylo` browser routes;
  - the `db/` scaffold;
  - the installer step that downloaded it.

  The installer now installs only the standalone `ty` binary. Request
  identifiers are generated natively. Legacy `OPTIONS.schema.json`
  request/response validation is not implemented. `ty migrate check` reports
  it with an explicit migration action.
  Consequences a project should plan for:
  - The Rust implementation has no built-in OpenTelemetry. Projects that need
    emitted spans must add instrumentation outside Tachyon or stay on the
    archived release.
  - Yon realtime keeps its append-only topic logs and its SSE cursor based on
    an integer position. It no longer registers clients into a collection,
    because nothing read that collection. Its default root is `.yon-realtime`
    instead of `.fylo-data`.
  - Companion scripts that use a bare `fylo` no longer get an injected
    import. `ty migrate check` no longer reports FYLO constructs.
- Removed from the repository: the JavaScript compiler, server, browser
  runtime, native generators, vendored shims, tests tied to that
  implementation, benchmarks, and old JavaScript TypeScript configurations. The
  ambient type contract for applications now lives under `api/types/`. The
  full-stack migration input that is kept is now separate, non-executable Rust
  test data.

### Added

- Finished the new Rust rewrite (written from scratch) through Phase 7. The
  Rust implementation is now the only stable release candidate. The unchanging
  compatibility oracle stays outside the shipped runtime.
- Added native host generators for iOS, Linux, Windows, and Android, next to
  macOS. They share one staging path that does not depend on the platform and
  one planner. Each target has its own output, and each platform records its
  toolchain.
- Added reproducible gates for each platform:
  - an iOS simulator run;
  - a pinned Linux container that reads accessible names and drives the UI over
    AT-SPI (the Linux accessibility service);
  - an Android emulator run that checks native widget classes through
    `uiautomator`;
  - a Windows UI Automation gate for CI.
- Checked the cutover evidence again by running it on 2026-07-31. This covered
  the compatibility differential (3/3 corpus projects), the release lifecycle
  drill, and the iOS, Android, and Linux native gates. `docs/CUTOVER.md` now
  records the basis for each condition. The basis is either an execution
  result or a named maintainer decision.
- Added build-provenance attestation to the release lifecycle job. Anyone can
  trace a published artifact to the workflow and commit that built it, with
  `gh attestation verify` and without a maintainer key.
- Added ARIA-role adapter mapping for native targets. When the author of an
  unknown tag declares a role, the compiler still knows what the tag is. So a
  design system gets native widgets by naming the roles it must give its users
  anyway. The compiler does not need to know the design system.
- Added browser companions in Dart, Kotlin, Swift, and C#, next to the existing
  Rust one:
  - Each language's own compiler builds its companion.
  - `scripts/wasm/companion-browser-test.mjs` proves all five in a real
    browser, on one page.
  - An authoring prelude includes the ABI. So a companion is plain code in its
    language, not a subset that a transpiler understands.
  - Dart, Kotlin, and C# cannot produce a module that instantiates on its own.
    So ADR 0011 adds a second shape for them.
  - Swift meets the original shape, when it gets an empty WASI (WebAssembly
    System Interface) environment. The island runtime now supplies it.
- Added capability probes to `ty doctor`. Each wasm toolchain is asked to build
  for its wasm target, instead of reporting its version. All of these languages
  ship wasm support separately from the compiler. Before this change, four
  toolchains reported ready on a machine where none of them could build.
- Added `ty migrate check`. It classifies a legacy project against the Rust
  features without running it. It attaches a required action to every finding
  that is unsupported or changed.
- Added a shared compatibility corpus and a browser-based differential. It
  compares route graphs, semantic DOM, and HTTP status across both
  implementations. Declared differences stay visible.
- Added `docs/PARITY_LEDGER.md`. It records every feature as identical,
  equivalent, changed, unsupported, or Rust-only.
- Added four fuzz targets that cover every trust boundary and run under
  `AddressSanitizer`. Also added sanitizer, soak, recovery-drill,
  performance-budget, SBOM (software bill of materials), and auditable-build
  gates.
- Added a weekly long fuzz campaign. It restores the previous corpus and runs
  all four boundaries for two hours in total by default. It publishes the
  changed inputs for audit and uses them in the next campaign.
- Added reproducible release artifacts with checksums for each file. Also added
  a lifecycle drill that covers verification, tamper detection, install,
  upgrade, rollback, and uninstall.

### Changed

- Made automated security qualification of the exact head commit the stable
  release gate. Independent human security review is still welcome, but it no
  longer blocks merge, tagging, or publication.

### Fixed

- Kept the website's mobile navigation, brand link, language controls, and
  footer links at the 44-pixel minimum touch-target size. Removed a nested
  interactive chip from the home link, and respected the bottom safe area.
- Made release archives deterministic under both GNU tar and BSD tar. The
  staged file timestamps are now normalized before archiving.
- Windows upgrades now download and verify into a temporary file in the same
  directory before they replace `ty.exe`. So a network or checksum failure
  keeps the installed binary.
- Unix upgrades now put the verified bytes next to the installed binary before
  the atomic replacement. Release CI tests on Unix and Windows that a failed
  upgrade keeps the installed binary.
- The handler supervisor now owns a POSIX process group or a Windows Job Object
  for each invocation. It ends child processes after every outcome, and it
  limits pipe settlement by the request deadline. It proves that no child PID
  remains after success, timeout, and cancellation.
- Replaced Linux native `file://` WebSurfaces with a private resource scheme for
  each view. The scheme percent-decodes and canonicalizes paths, and it rejects
  symlinks. It limits navigation and loads to the generated surface and
  WebBundle roots. Remote WebSurfaces now enforce the declared effective port
  on every platform.
- Handler and middleware process diagnostics no longer cross the HTTP boundary.
  Public failures include only a request reference.
- Removed the installer's execution of CHEX and TTID. These were transitive
  dependencies that could change and were not verified. The Rust runtime uses
  neither external binary.
- Removed the last FYLO environment keys, ambient APIs, generated README text,
  and `db/` files from `ty init`.
- Added the migrated website acceptance suite to required CI. Removed its old
  Amplify recipe that built from source. Deployment uses a qualified prebuilt
  `dist/web` archive, because the showcase uses five language toolchains.
- Made the enterprise qualification matrix work on all hosted runners:
  - restored the four-target `cargo-fuzz` package with reviewed seed inputs;
  - normalized extended Windows paths before calling MinGW;
  - compared macOS and web screenshots at the host's real usable viewport;
  - handled Android SDK license-pipe termination explicitly;
  - rebuilt the standard library for Linux sanitizer runs;
  - kept a release artifact for provenance attestation.

- A generated Windows application published its side-by-side manifest to the
  bundle root as `application.manifest`. Windows never reads that file. It
  reads only `<executable>.manifest` next to the executable. So the process ran
  without the Common-Controls v6 activation context that the manifest declares.
  The manifest is now published as `bin/<executable>.exe.manifest`. This alone
  does not fix the open Windows accessibility finding in
  `docs/PHASE_7_EVIDENCE.md`. Every control still appears to UI Automation as a
  generic pane, not as a button, output, or heading.
- The development and preview servers could exit as soon as they reported
  that they were ready. If registering the Ctrl-C handler failed, the shutdown
  future finished at once. So the server completed its graceful shutdown
  before it served anything. It then refused every later connection with no
  diagnostic. The `ty build --watch` loop ended the same way. Now a process
  that cannot detect an interrupt keeps serving, and its supervisor stops it.
- A project whose `client` or `server` path was a regular file reported the
  wrong diagnostic on Windows. When a parent path component is a file, Unix
  reports `NotADirectory`, but Windows reports `NotFound`. `NotFound` is the
  same code that means the source root does not exist. So a broken project
  layout was reported as having no views (`TY1002`), not as an unusable source
  root (`TY1001`). The compiler now finds the blocking parent by checking the
  shape of the path. It no longer trusts a platform-specific error kind.
- The template parser panicked on malformed markup. `<slot>` has no children.
  But recovery from a token error could leave one open on the parser stack.
  Adding the next child then hit an `unreachable!()`. So a `tac.html` that was
  edited by hand or generated could stop the compiler instead of reporting a
  diagnostic. A leaf now rejects the child with `TY1301`. The impossible state
  no longer exists at all, instead of being only unreachable. The
  `template_frontend` fuzz target found this in CI. The crashing input is now a
  regression test and a permanent corpus seed.
- A native build rejected the compiler's own output. Only scripts under
  `/.tachyon/` were removed before native planning. So a route's own
  `/client.js` stayed, and the view contract rejected it with `TY1306`. This
  meant that no page with a companion or an event binding could be built for
  any native target. Every script in generated HTML is generated, because the
  compiler rejects an authored one before rendering.
- Two routes wrote one `WebSurface` over another. Node ids restart at
  `n_000001` for each route, and the payload directory was named after the id.
  So the second route that was built replaced the first route's page. The
  website's home content showed another route's empty surface. The payload
  path now includes the route. The node id stays as the Native UI contract
  specifies.
- A `WebSurface` had a fixed height on every platform: 180 points on iOS,
  macOS, and GTK, and 480 pixels on Android. So a taller subtree was cut off,
  with no scroll bar of its own to show the rest. Each host now uses the
  document's own height.
- A fallback subtree rendered without styles, because the route's stylesheet
  link is removed as a generated asset. The stylesheets that a route links are
  now inlined into the fallback document instead. This also restores the media
  queries that decide which variant of a responsive page is visible.
- The compiler could not plan an island whose content is entirely deferred. An
  `aria-label` written on a component becomes a property of its companion, not
  an attribute of the island. So nothing could name the island, and every
  native build failed with `TY1603`. The name is now read from the props.

- Every gate script in `scripts/` was committed without its executable bit. So
  the release lifecycle drill failed at `build-artifact.sh: Permission denied`
  from a clean checkout. CI's `./scripts/linux/native-test.sh` would have
  failed the same way. This was found by running the drill, not by reading it.

- Accessible names set on generic GTK4 containers never reached AT-SPI. A GTK4
  button with its own label ignored its declared accessible name. Both problems
  were found by running the real accessibility bus, not by inspection.

### Changed

- Switched the repository entry points and CI to the Rust rewrite. Root package
  commands now run the compiled `ty` CLI. A dedicated CI policy job rejects any
  attempt to add back the removed JavaScript runtime or test entry points.
- The compatibility differential now downloads the unchanging v26.30.04
  release executable. It verifies the executable's pinned SHA-256 digest and
  exposes it as `RELEASED_TY_BIN`.
- `NativeBuildOptions` now includes a `NativeTarget`. Native artifacts are
  published under a separate directory for each target.

### Added

- Finished the Rust Phase 4 macOS native vertical slice:
  - Native UI v1;
  - semantic SwiftUI adapters;
  - deterministic accessibility identities;
  - bounded declarative state and actions;
  - route and controller lifecycle logging;
  - atomic `.app` packaging.
- Added a local WebSurface fallback for the smallest subtree, isolated HTTPS
  WebSurfaces with no bridge, Capability Manifest v1 that denies by default,
  and Artifact Manifest v1 with complete digests.
- Added a real macOS application gate. It drives AppKit controls through
  Accessibility and keyboard events. It compares the required semantics with a
  mobile Chromium reference, and it enforces a rough screenshot-layout budget.
- Finished the Rust Phase 3 view pipeline:
  - bounded expressions and escaped bindings;
  - both Tac control syntaxes;
  - recursive components and slots;
  - composed JavaScript/Python Yon context;
  - server-rendered islands;
  - View IR (intermediate representation) and source maps;
  - recovery that reports several diagnostics;
  - verified incremental reuse of routes.
- Added all five island activation policies with deterministic wrappers,
  same-origin generated modules, and failure behavior that keeps the SSR
  (server-side rendering) output. Also added interaction replay and a real
  Chromium acceptance gate.
- Added View Source Map v1, the safe-view/island ADR, the Phase 3 normative
  specification and evidence, and updates to the threat model and support
  documents.
- Finished the Rust Phase 2 Yon handler boundary:
  - typed Handler Protocol v1 request, response, error, and cancellation
    envelopes;
  - deterministic JavaScript and Python handler discovery;
  - a compiled `ty handler invoke` workflow.
- Added supervision that starts one process directly for each request. It has
  strict framed stdout, bounded stderr, deadlines that include queue time,
  cooperative cancellation, forced termination and reaping, concurrency limits,
  crash recovery, and a child environment that denies by default.
- Added shared contract suites that use real runtimes and real binaries. They
  cover:
  - sync and async handlers;
  - Unicode paths and data;
  - all protocol HTTP methods;
  - environment allowlisting;
  - adapter authoring failures;
  - process crashes;
  - malformed and oversized output;
  - stdout smuggling and stderr flooding;
  - concurrency admission.
- Finished the Rust Phase 1 vertical slice:
  - deterministic Tac/Yon static route discovery;
  - bounded HTML validation;
  - Route Manifest v1 output;
  - stable diagnostics;
  - atomic publication;
  - project scaffolding;
  - a development server that is safe on loopback.
- Added acceptance tests on the real binary for initialization, repeatable
  builds, failure recovery, HTTP behavior, security headers, and execution on
  each platform. Also added a Rust coverage ratchet: 80% for the whole
  workspace.
- Set up the base of the new Rust rewrite, written from scratch. It has an
  exact toolchain, safety and lint policies for the whole workspace, and
  executable schema contracts.
- Added product context, six accepted architecture decisions, a first threat
  model, support vocabulary, governance, and release engineering.
- Added separate Rust CI for Linux, macOS, and Windows, with gates for
  dependencies, licenses, schemas, documentation, and immutable actions.

## [26.30.04] - 2026-07-23

### Added

- Generated Linux desktop hosts now provide global shortcuts, window
  click-through that can be turned off again, and declared window capture.
  Each feature is enabled only when the host detects support for it. Wayland
  uses the XDG GlobalShortcuts and Screenshot portals. X11 uses `XGrabKey` as
  a fallback for shortcuts. When the compositor does not support a feature,
  the feature stays blocked. Linux still does not advertise capture
  protection.

### Fixed

- Native controllers without a DOM now run companion `@onMount` handlers only
  after the host bridge is installed. So startup `host.on(...)` subscriptions
  and managed `contentSurface.open(...)` calls work without constructor
  delays.
- When the compiler lowers JavaScript and TypeScript native-provider code, it
  now leaves strings, template text, regular expressions, and comments
  unchanged, byte for byte.
- Controllers produced by compiled `ty` release binaries no longer reference an
  undeclared native-parser constant during render or dispatch. The bundler had
  renamed that constant.
- Linux portal calls now subscribe to the asynchronous response before they
  call the portal method. So an immediate response from the backend cannot be
  lost.

## [26.30.03] - 2026-07-22

### Changed

- On macOS, Windows, and Linux, managed remote content now appears inside the
  primary native window, in its declared 25/75 split. It no longer opens in a
  separate top-level window. So the primary `appWindow` controls apply to the
  trusted controls and the active remote surface together, as one
  presentation.
- Managed-content state and host manifests now report the composed
  presentation contract. Bundles without managed origins leave out the layout
  metadata that they do not support.

### Fixed

- When the active managed surface is closed or replaced, the host now removes
  it from the composed pane and shows another open surface. The native bridge
  stays hidden from remote content.
- The documentation now explains composed presentation compared with detached
  presentation, the platform limitations, and how to use a global shortcut to
  recover a click-through window.

## [26.30.02] - 2026-07-21

### Added

- Native-first macOS, Windows, and Linux hosts now implement:
  - desktop window controls;
  - isolated managed-content surfaces;
  - declared camera and microphone permissions;
  - accurate reporting of capability state.

  macOS and Windows also implement global shortcuts, and window listing and
  capture that require permission.
- Release CI now compiles generated native hosts on macOS, Linux, and Windows.
  This includes the platform-specific managed-content and screen-capture
  adapters.

### Fixed

- Compiled standalone Tachyon binaries now embed the native UI runtime that has
  no DOM. So a newly initialized project can bundle native targets without
  resolving runtime paths that exist only in the source tree.
- Native host manifests now advertise only generated operations. They keep the
  declared permissions and the exact-origin managed-content policies. Remote
  content stays isolated from the native bridge for the whole application.

## [26.29.03] - 2026-07-15

### Added

- Raw native filesystem capabilities now include the opt-in `fs.stat`,
  `fs.mkdir`, and recursive `fs.remove` operations on Android, iOS, macOS,
  Windows, and Linux. Matching companion APIs exist for Dart, Rust, Kotlin,
  Swift, C#, JavaScript, and TypeScript.
- Android native hosts now provide encrypted secret storage through the
  Android Keystore. They also provide local user verification through the
  framework `BiometricPrompt`. So `secrets.*` and `auth.verifyUser` now work
  on Android as well as on Apple hosts.
- `ty bundle --csp-check` (or `TAC_CSP_CHECK=1`) checks the staged JavaScript
  output for constructs that generate code at runtime. A strict CSP (Content
  Security Policy) blocks these constructs. If the check fails, the whole
  bundle fails before it replaces the previous bundle.

### Fixed

- Detection of companion decorators and FYLO auto-imports no longer treats an
  unrelated import, followed later by a `from` expression, as a real binding.
- Element IDs that the author sets no longer break Tac event dispatch or
  value-binding dispatch. Delegated handlers receive the bound element as
  `event.currentTarget`.
- A handler can clear state that its own conditional render pass uses later.
  This no longer leaves the DOM in a broken state before the clean render of
  the state alone can complete.
- An element with several `on:*` handlers now calls only the handler that
  matches the type of the dispatched event.

## [26.28.06] - 2026-07-11

### Added

- Tac components and pages can use portable Dart, Rust, Kotlin, Swift, and C#
  companions, as well as JavaScript and TypeScript. Every language lowers to
  the same reactive controller ABI and the same implicit browser/FYLO/device
  prelude.
- The native capabilities now include:
  - discovery of sandbox paths;
  - secure storage and local user verification on Apple hosts;
  - browser-standard file save;
  - geolocation, notifications, and media capture;
  - inspection of permission state;
  - the buffered `app.ready` host event.
- You can configure the native privacy declarations for camera, microphone,
  location, and notifications with `tachyon.devicePermissions` in
  `package.json`.
- You can enable raw native operations one at a time with
  `tachyon.nativeCapabilities`: `fs.readText`, `fs.writeText`, `fs.readDir`,
  and `shell.exec` (desktop only).
- The website now has a capability atlas split into sections. It has separate
  routes for overview, compose, react, connect, store, observe, and extend. It
  also has a live portable-bridge panel that covers the permission and
  capability model.

### Changed

- Tac companions are now UI controllers that do not depend on a target. They
  are no longer client worker endpoints. The client worker/WASM compiler and
  runtime are removed. Portable companions use normal bundled modules on web,
  desktop, iOS, and Android.
- Native filesystem and shell access is disabled by default. Tachyon validates
  package declarations strictly, filters them by target support, and embeds
  them into generated metadata. The JavaScript facade and the native handler
  each enforce them independently.
- Native external URL operations accept HTTP(S) only. Other rules:
  - Android delivers bridge messages only to the trusted appassets origin and
    the main frame.
  - Apple and Linux bridge scripts run only in the main frame.
  - Android media requests grant only the camera/audio resources that the host
    explicitly recognizes.
- Android native hosts disable application backup and cleartext network
  traffic by default.
- The website's `/atlas` route is now a redirect shell. Each Atlas section
  that search engines can index has its own canonical route and a responsive
  one-column layout on mobile.

### Fixed

- Dart toolchain setup and portable companion diagnostics now report
  unsupported source constructs with deterministic failures that say what to
  fix.
- Native configuration no longer silently drops misspelled permission names or
  capability names. Invalid declarations fail the build before it replaces an
  existing bundle.
- When a page asks for camera or microphone access, Android no longer grants
  unrelated WebView permission resources.

## [26.28.01] - 2026-07-06

### Fixed

- `tac.bundle` no longer clears the whole `dist/` directory. A run replaces
  only the `dist/<target>` directories that it built. It replaces each target
  atomically: stage, remove the old one, rename. Before this fix, building one
  target deleted the output of every other target. This deleted `dist/web`
  while a dev server was using it. It also emptied a generated macOS app that
  was running from `dist/macos`, so its page modules, which load on demand,
  silently returned 404.
- The SPA (single-page application) runtime now requests dynamic-route page
  modules by their `_slug` directory on disk (`pages/docs/_topic/tac.js`).
  Before, it used the `:slug` form from the route manifest. So dynamic routes
  now work on static hosts and native asset loaders, not only on the dev
  server.
- The service worker now registers with `type: "module"`. The built worker is
  an ES module, so before this fix it failed to parse on every production
  origin.
- The generated Android host overrides the default asset-ignore pattern of
  AAPT (Android Asset Packaging Tool). That pattern silently removed `_slug`
  page-module directories from the APK.
- The service worker cache version now includes a content hash of the client
  source tree, as well as the route-manifest hash. So any rebuild with changed
  inputs removes old caches. Before, redeploys with the same version (and
  reinstalls of the native app) served old JS/CSS from the Cache API until the
  user cleared the site data.

### Changed

- The generated macOS host installs a standard menu bar: an App menu with
  About/Hide/Quit, an Edit menu with clipboard actions, and a Window menu.
  Without a main menu, Cmd+C/V/Q do not work in a WKWebView app. App authors
  add custom menus in `buildMainMenu()` in the generated `TachyonApp.swift`.
- The generated macOS host serves the bundle through a `WKURLSchemeHandler` at
  `tachyon://localhost/`. It no longer uses `file://` with workarounds in the
  file-access preferences. The secure-context origin enables Web Workers (Tac
  Workers running wasm) and FYLO persistence backed by OPFS (Origin Private
  File System), as on the iOS and Android hosts. It handles directory indexes
  and deep links in the same way, and external links open in the default
  browser.
- The generated iOS project now really includes its app icon. The xcodegen
  spec now includes the generated `Assets.xcassets` and sets
  `ASSETCATALOG_COMPILER_APPICON_NAME`. So the home-screen icon shows the
  Tachyon mark instead of the blank placeholder.
- On both native hosts, external links open in the system default browser.
  They no longer move the WebView away from the app. The Android host sends an
  `ACTION_VIEW` intent for any origin other than its own. The iOS host's
  `WKNavigationDelegate` passes http/https/mailto URLs outside the
  `tachyon://` scheme to `UIApplication.open`.
- The generated iOS host now includes its web bundle as `WebBundle/` instead of
  `Resources/`. With a top-level folder named "Resources", codesign/installd
  treat the flat iOS bundle as an old-style shallow bundle. Installation then
  fails with a misleading "Missing bundle ID" error. The `.ipa` archive also
  now leaves out AppleDouble sidecar files (`ditto --norsrc`).
- The generated iOS host serves the bundle through a `WKURLSchemeHandler` at
  the `tachyon://localhost/` origin instead of `file://`. The secure-context
  origin enables Web Workers (Tac Workers running wasm) and OPFS-backed FYLO
  persistence inside WKWebView. It handles directory indexes and deep links
  without extensions in the same way as the Android host. Service workers are
  still not available in WKWebView, and the runtime works without them. The
  viewport of the prerendered shell now includes `viewport-fit=cover`, so
  `env(safe-area-inset-top)` works on iOS.
- `tac.bundle --target` accepts lists separated by spaces as well as lists
  separated by commas. Both `--target ios, android` and
  `--target ios android` work. Before, it silently ignored everything after
  the first argument.
- The generated Android host draws behind a transparent status bar:
  - Pages get the status-bar inset as the `--tac-safe-top` CSS variable,
    because WebView cannot read `env(safe-area-inset-top)`.
  - A new `ui.statusBarStyle` bridge capability lets pages switch the system
    icons between `dark-content` and `light-content` when their theme
    changes.
- The generated native icon set (`NativeIconAssets`) now draws the current
  Tachyon mark (slate tile, gradient chevrons) in SVG, Android vectors, and
  raster PNGs. So launcher icons match the PWA (progressive web app) icons.
- The Android host now includes a proper adaptive launcher icon
  (`mipmap-anydpi-v26/ic_launcher` with background, foreground, and
  Android 13+ monochrome layers) instead of a plain drawable. So the icon
  fills the launcher mask from edge to edge, like first-party apps.
- The generated Android host runs without the native action bar
  (`Theme.Material.Light.NoActionBar`) and disables WebView scrollbars. This
  makes it feel like a native app.

### Added

- Yon supports every language. There is no list of supported languages. The
  only limit is the toolchain on the developer's machine. How it works:
  - A `yon.<ext>` handler runs by its extension. The extension names the
    language. An `interpreters` map turns the extension into a run command
    (`.go` → `go run`, `.rb` → `ruby`, …). The map starts with common
    defaults. Each project can add to it or override it under
    `interpreters` in `.tachyonrc`.
  - No shebang and no `chmod` are needed. Write `server/routes/ping/yon.go`,
    declare its HTTP methods in an `OPTIONS.schema.json` next to it, and it
    runs as a process. The handler reads the JSON request on stdin and writes
    the response on stdout.
  - A prebuilt executable or a script with its own shebang also works.
  - The built-in languages only make handlers easier to write: you write
    `class Handler` and do not need the stdin/stdout code. A pluggable
    provider registry (`src/server/process/language-providers.js`) provides
    them. Apps add or override providers in `server/yon.providers.js`. For
    example, an app can point python at a virtualenv interpreter.
  - `HandlerAdapter.supportedLanguages` / `isSupportedLanguage` are removed.
    `HandlerAdapter.knownLanguages` lists the convenience adapters for tools.
    It does not limit which languages can run.

### Removed

- Yon no longer has a step that compiles routes to binaries for deployment.
  `yon.build` (the `yon.build` bin, `src/cli/build.js`) and the
  `native-binary` execution backend are removed. The developer deploys the
  `server/` source as it is, and every route runs as a process. Routes in
  compiled languages compile on the first request and cache the result. So
  production needs the same toolchain that the developer used, not pre-built
  binaries.
- Yon's own WASM execution backend is removed
  (`src/server/process/backends/wasm-compiled.js`), together with the backend
  registry, which is now empty. Every Yon route runs as a subprocess. There is
  no in-process WASM path. The Tac frontend worker compiler is separate and
  does not change.
- Tac workers now support only Rust, JavaScript, and TypeScript. The C#, C++,
  Swift, and Kotlin worker compilers/providers are removed. `yon.<ext>`
  backend handlers in those languages do not change, because they are part of
  Yon, not Tac.
- The generated Android host serves the bundle through `androidx.webkit`
  `WebViewAssetLoader` at the trusted `https://appassets.androidapp.com/`
  origin instead of `file://`. It uses an explicit domain, directory indexes,
  and service-worker interception. Web Workers (Tac Workers running wasm),
  OPFS-backed FYLO persistence, and service workers now all work inside the
  native app. The deprecated file-URL access grants are removed. The iOS host
  README describes the matching `WKURLSchemeHandler` upgrade that iOS still
  needs.
- Production bundles (or `tac.bundle --package`) export native artifacts that
  you can distribute, when the system toolchains exist. `dist/<target>/` then
  contains only the artifact:
  - `dist/android/<App>-<v>.apk` through Gradle. It falls back to signing
    with the debug keystore. Use `TAC_ANDROID_KEYSTORE*` for store signing.
  - `dist/ios/<App>-<v>.ipa` through xcodegen + xcodebuild. It is unsigned by
    default. Use `TAC_IOS_TEAM_ID` to sign it.
  - An ad-hoc-signed `dist/macos/<App>.app` through swiftc.
  - `dist/linux/<App>/` (executable + Resources) through CMake.
  - `dist/windows/<App>/` (`.exe` + Resources) through CMake/MSVC.

  When a toolchain is missing, the build logs a skip and keeps the host
  project that can still be built. `--skip-package` turns packaging off.
- Prerendered shells automatically link a web app manifest when
  `client/shared/assets/manifest.webmanifest` (or `manifest.json`) exists.
  They show its `theme_color` as a `<meta name="theme-color">` tag. With the
  built-in service worker, an app only needs a manifest next to the favicon
  to become an installable PWA.
- Desktop native Tac workers, starting with Rust on macOS, Linux, and Windows.
  `tac.bundle --target <desktop>` compiles `browser/workers/<route>/tac.rs`
  files to native executables. It puts them in the host's
  `Resources/workers/` directory. The generated desktop native hosts expose a
  `tachyon.worker` bridge capability. It starts the executable for each
  request, sends a JSON envelope on stdin, and returns the response envelope
  to the frontend.

- Yon backend routes for Rust/C++/C# now compile to WebAssembly when their
  handler fits the subset that Tachyon supports. Tachyon compiles them
  **in-house** (no rustc/clang/.NET) and runs them **in-process** through a
  pluggable execution backend. Handlers outside the subset automatically use
  the existing subprocess runner instead. Subset handlers can:
  - read request fields (`request.query/path/header("k")`);
  - return structured JSON object literals (`{ "k": value, ... }`).

  Tachyon resolves and builds both in-house, with no JSON library inside the
  wasm module. So wasm routes return validated object responses. The combined
  `/language` example route shows this. Its Rust (`yon.rs`) and C++
  (`yon.cpp`) handlers fit the subset and compile to WebAssembly in
  production.

### Changed

- Tac workers now use five language families, each with explicit targets:
  - JavaScript/TypeScript for web;
  - Swift for macOS/iOS;
  - Kotlin for Android;
  - C# for Windows;
  - Rust for every target.

  `client/workers` no longer accepts C, C++, Go, Python, and Zig.
- The website in the repository (`website/`) changed from a dense capability
  atlas into a minimal marketing landing page. It now has a dark hero section
  with an install command, a picture of the file-system scaffold, a feature
  grid, polyglot code examples in tabs, and a selected interactive demo. The
  demo uses the polyglot, desktop native, and canvas panels. The old atlas
  components are removed.

- Bundled npm dependency modules now live under `/shared/modules/` instead of
  `/modules/` (for example, `/shared/modules/dayjs.js`). This is a clean
  break: the server no longer serves the old `/modules/*` URLs.

### Added

- The app shell now includes an inline `<script type="importmap">`. It maps
  bare npm specifiers (for example, `dayjs`) to their bundled
  `/shared/modules/` URLs. You can add your own mappings in
  `browser/shared/importmap.json`, with `shared/importmap.json` as a fallback.
  Your entries override the generated entries.

### Changed

- Tac worker `OPTIONS.schema.json` now has the same shape as the Yon route
  schema, but it groups response schemas into three status buckets: `ok`
  (2xx), `clientError` (4xx), and `serverError` (5xx). For each HTTP method,
  the keys are `payload` (request) plus the response buckets that apply.
  2xx responses always use the `ok` schema. The worker schema no longer has
  the old `response`/`error` keys, numeric status-code keys, or the `native`
  capability declaration block. Capabilities are now allowed through the
  `TAC_NATIVE_CAPABILITIES` environment variable, a comma-separated
  allowlist. Tachyon reads it at compile time and builds it into the worker
  runtime. Raw OS capabilities (`fs.*`, `shell.*`, `process.*`) still need an
  extra entry in `TAC_DANGEROUS_CAPABILITIES`.
- Tac frontend messaging now uses one signals system. `@publish` and
  `@subscribe` replace the old `@emit`, `@provide`, and `@inject`
  decorators. Field signals keep their value, for context-style values.
  Method signals do not keep a value, for event-style messages.
- Tac method subscribers can now pass `{ onMount: true }`. The subscriber then
  runs once after mount and also reacts to later signal publications.
- `@subscribe` and `@publish` without arguments now use the name of the
  decorated field or method as the default signal name.
- Removed the `@render` decorator from the strict v2 Tac API. Companion state
  now renders again when a field is assigned. This includes `$` and `$$`
  persistent fields.
- Upgraded `@d31ma/fylo` to `^26.23.7`. The new version adds these options to
  `fyloOptions()`, set through environment variables:
  - query caching (`FYLO_CACHE_BACKEND`, `FYLO_CACHE_METHOD`,
    `FYLO_CACHE_TTL`);
  - WORM (write once, read many) mode (`FYLO_WORM=strict`);
  - an RLS (row-level security) switch (`FYLO_RLS`);
  - a local queue switch (`FYLO_QUEUE`);
  - sync-mode selection (`FYLO_SYNC_MODE`).

  The Fylo browser now lets you inspect and restore soft-deleted items
  through `/_fylo/api/deleted` (GET) and `/_fylo/api/restore` (POST).
- Upgraded the `@d31ma/fylo` dependency of the Tachyon website from
  `^26.19.7` to `^26.23.7`.
- Native bundle targets now put their webview host directly in
  `dist/<target>/`. They no longer use a separate `dist/<target>-native/`
  directory. Native targets no longer produce the standalone web bundle. Its
  assets are embedded under the host's `Resources/`. With
  `--skip-native-host`, the build still produces the plain web bundle at
  `dist/<target>/`.

### Removed

- Removed support for Swift (`yon.swift`) and Kotlin (`yon.kt`) backend route
  handlers. These are mobile-first languages. Yon backend routes now target
  server languages only. Removed the compiled-runner Swift/Kotlin adapters,
  the `/languages/swift` and `/languages/kotlin` example routes, and their
  services/repositories.
## [26.23.01] — 2026-06-01

### Added

- Yon realtime SSE mailboxes, turned on by `YON_REALTIME_ENABLED=true`:
  - `/_yon/realtime/clients` issues durable TTID client IDs.
  - `/_yon/realtime/stream` replays queued messages over Server-Sent Events.
  - `/_yon/realtime/messages` stores messages in FYLO's local queue. It also
    wakes any client stream that is connected at that moment.
- The example app now includes a Yon realtime messaging part. It has MVC
  (model-view-controller) routes under `/realtime`, FYLO/CHEX schemas for demo
  clients and messages, and a Tac companion script. The browser UI markup and
  styles are not done yet.
- The compiler injects a `fylo` browser client into Tac. This client now:
  - caches successful collection reads in IndexedDB;
  - supports explicit cache policies;
  - uses cached reads when the browser is offline;
  - invalidates collection cache entries after FYLO mutations.
- The example users panel now shows every FYLO browser cache policy. It also
  has a temporary create/delete flow that shows invalidation after a mutation.
- `bun serve` now treats an app with `browser/` + `db/` and
  `YON_DATA_BROWSER_ENABLED=true` as having a built-in backend. App authors can
  use the Tac FYLO browser wrapper without adding a `server/` folder.

### Changed

- Upgraded `@d31ma/fylo` to `26.22.7`. Tachyon's FYLO integrations now use the
  root-first constructor API.
- When `NODE_ENV=production`, Yon compiled route adapters cache the build
  artifacts for Java, C#, Dart, C++, Swift, Kotlin, and Rust. The cache key uses
  the route file, the service files in the same language, and the adapter
  cache version.
- Yon now supports C++ backend route handlers through `yon.cpp`, generated
  `YonJson` helpers with no dependencies, and the compiled route adapter.
- Yon now supports Swift (`yon.swift`) and Kotlin (`yon.kt`) backend route
  handlers through the compiled route adapter:
  - Swift handlers put static verbs inside an `enum`/`struct`/`class Handler`
    and exchange JSON through `Foundation`.
  - Kotlin handlers expose verbs from the companion object of a
    `class Handler` (or from a top-level `object Handler`). They receive a
    generated `YonJson` helper with no dependencies.
  - Example routes are under `/languages/swift` and `/languages/kotlin`.
- Yon now supports Rust (`yon.rs`) backend route handlers through `rustc`.
  Rust handlers use `struct Handler; impl Handler { pub fn GET(...) { ... } }`
  and receive a generated `YonJson` helper with no dependencies. An example
  route and service are under `/languages/rust`.
- The navigation rail of the example capability atlas now opens as a
  responsive drawer on mobile. On desktop, the rail stays visible at all times.

### Fixed

- The example FYLO users panel now hides previews that look like API keys
  before it shows them in the browser.

## [26.22.03] — 2026-05-27

### Added

- Query filtering in the style of PostgREST on Fylo browser collection
  endpoints. `GET /_fylo/<collection>/?field=operator.value` supports `eq`,
  `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, and the `not`
  prefix. Reserved parameters:
  - `select`: chooses which fields to return.
  - `order`: sorting.
  - `limit`, `offset`: pagination.
- The browser example dashboard now has a Tac input gallery. It covers native
  input types, `select`, `datalist`, and `textarea`. It shows events as they
  happen and has reactive value bindings that persist.
- The example browser app is now a guided capability atlas. It is one
  application tour that links Tac composition, reactive browser state, Yon
  routes, FYLO storage, telemetry, and WebAssembly companions.
- A Browser studio panel shows canvas drawing driven by Tac. It also shows the
  native semantic `progress`, `meter`, `output`, `time`, and `details`
  elements with scoped responsive CSS.
- Tac templates support `:checked="field"`. Checkbox and radio state stays
  reactive when a component renders again.
- The example app in the repository now takes part in standalone strict type
  checking. Its Tac globals and browser dependencies resolve.

### Changed

- **BREAKING**: Yon route handlers use one class per route. You write
  `export class Handler` with static methods named after HTTP verbs
  (`static async GET(request)`). This replaces the old convention of one
  directory per method (`routes/<path>/GET/yon.<ext>`). All polyglot adapters
  (JS, TS, Python, Ruby, PHP, Java, C#, Dart, C++) support the new pattern.
- Upgraded `@d31ma/fylo` to `^26.22.3`. Removed the `getHistory()` calls.
  Added `findDeletedDocs()` and `restoreDoc()` to the machine-interface demo.
- The Fylo browser query panel now accepts filter input in the style of
  PostgREST. It no longer has SQL/findDocs modes. The `<fylo-browser>` Tac
  component and the plain JS shell both use the new interface.

### Removed

- Removed the `POST /_fylo/api/query` endpoint (SQL and find queries).
  Filtering that uses indexes now uses PostgREST-style query parameters on the
  collection URL. Removed the `fylo.sql()` client method.
- Removed Go and Rust language support from the example routes and handler
  adapters.
- Removed version history display from the Fylo browser. Fylo 26.22.3 removed
  the `getHistory()` API that it used.

### Fixed

- The `HandlerAdapter.hasMethod` regex now matches async generator handlers
  (`static async *GET()`) inside class-per-route definitions.
- `Router.validateSegmentPath` no longer reports false duplicate route errors
  for nested slug routes (for example `/items/_id/detail` versus
  `/items/detail`). It now maps slug segments to a `:` placeholder instead of
  removing them, so comparisons keep the path depth.
- Static prerendering now keeps literal replacement-token text, such as Tac's
  `$` and `$$` persistence sigils, when it inserts rendered pages into the
  HTML shell. It no longer expands this text.
- Browser acceptance tests accept `TACHYON_EXAMPLES_E2E_PORT`. This avoids
  false failures when another process already uses the default example port.
- The example inventory form now binds its CRUD (create, read, update, delete)
  draft state directly to its visible native input. A successful Yon save now
  always clears the field.
- Tac client-side navigation keeps query parameters and URL fragments on the
  first hydration, on intercepted link clicks, and on browser history
  navigation.
- New regression tests for component stylesheets check that template-literal
  characters in author CSS comments stay unchanged in the emitted Tac modules.
- Tac `:value` bindings now render loop-local expressions without evaluating
  their identifier outside the loop scope. This prevents a `ReferenceError`
  during builds such as `<loop :for="option of options"><input :value="option" />`.
- Synthetic value-binding updates keep `$event.target`,
  `$event.currentTarget`, and `$event.type`. DOM-style `@input` and `@change`
  handlers therefore still work together with `:value`.
- `@onMount` registration now waits until the renderer binds the companion.
  Mounted callbacks now run even when an app constructor calls `super(props)`
  without passing on the injected Tac helper argument.

## [26.21.7] — 2026-05-24

### Added

- New `POST /languages/javascript/fylo` example route that runs the full FYLO
  machine-interface suite (about 25 operations). The suite moved off the
  lightweight `/languages/javascript` dashboard endpoint so that diagnostics
  stay fast.

### Changed

- **BREAKING**: Tac page and component templates now use `tac.html`. They can
  have optional sibling files
  `tac.js`/`tac.ts`/`tac.css`/`tac.wasm`/`tac.tac.json` and companions backed
  by source files. The old `index.html` convention for Tac source is removed.
  The bundled `dist/**/index.html` output is unchanged, so static hosting and
  previews work as before. Page and component modules in dist are emitted as
  `tac.js`.
- **BREAKING**: Yon route handlers move to
  `server/routes/<path>/<METHOD>/yon.<ext>`:
  - The HTTP method is now the parent directory of the handler file.
  - Every handler file is named `yon.<ext>`, in every language.
  - The companion `OPTIONS.schema.json` still sits next to the method
    directories.
  - Polyglot class-based handlers (Java, C#, Python, PHP, Ruby) take their
    class name from the `yon` basename. The result is a Pascal-cased `Yon`
    class.
- Repository layout changes:
  - `package-contract/` moved to `tests/package-contract/`.
  - `stress/` moved to `tests/stress/`.
  - Integration test runners and workers moved to
    `tests/integration/helpers/`.
  - The `bun test`, `test:blackbox`, and `stress:tac` scripts were updated to
    match.

  Only the layout changed. Runtime behavior did not change.
- Upgraded `@d31ma/fylo` to `26.21.6`. The parts of FYLO that Tachyon uses
  (`new Fylo(fyloOptions(root))`, `Fylo.uniqueTTID`, `executeSQL`,
  `findDocs`, `inspectCollection`) are unchanged. The upgrade brings upstream
  bug fixes and the `LocalQueue` / `publish` / `consume` queue exports. Example
  apps may use these exports directly.

### Internal

- Replaced the in-place `tsc --noEmit` typecheck with `scripts/typecheck.js`.
  This wrapper runs the same compiler command. On working copies in
  cloud-synced folders, it first copies the selected project into the OS temp
  directory.
  - `tsconfig.src.json` is still the default release gate.
  - Pass `tsconfig.tests.json` or `tsconfig.examples.json` to run those
    scoped projects.
  - Settings: `TACHYON_TYPECHECK_TIMEOUT_MS` (default 2 minutes),
    `TACHYON_TYPECHECK_STAGE=1` or `0`, and `TACHYON_TYPECHECK_KEEP_STAGE=1`.
- The FYLO browser now protects against collection names that look like path
  traversal. It checks them when it resolves REST-style collection URLs and
  event-tail files. An invalid event-tail collection name returns a 200 JSON
  error, in the same style as the existing FYLO browser API.
- Removed the last FYLO browser DOM `innerHTML` rendering for dynamic
  collection, document, history, REST result, encrypted-field, and event
  values. The UI now writes dynamic values with text nodes.
- `tsconfig.json` again inherits from `tsconfig.base.json`. Bun's runtime module
  resolver then finds the `@/*` → `./examples/server/*` path alias that the
  example polyglot route handlers use.
- Updated the fallback handler path in `Pool.prewarmAllHandlers` to the new
  `<METHOD>/<routeFileName>` layout. After the Yon rename, the old `<METHOD>`
  fallback built paths that did not exist, without any error. As a result, most
  polyglot handlers were missing from the pool of warmed processes.
- Restored `static chexSchemas = new Map()` on `Validate`. The schema
  validator needs it to match responses. It had been removed from the working
  tree during a refactor.
- `FyloMachineRepository.ensureDemoSchema()` now writes to a
  `fylo-demo-items` collection instead of `items`. The demo run no longer
  overwrites the real items schema that other example routes validate against.

## [26.21.05] — 2026-05-22

### Added

- Stage 3 decorator versions of the `Tac` runtime helpers:
  `@inject(key, fallback?)`, `@provide(key)`, `@env(key, fallback?)`,
  `@onMount`, and `@emit(name)`.
  - Companion scripts can use them as bare identifiers. When the Tachyon
    compiler sees the `@<name>` syntax in a companion script, it imports them
    automatically. User code needs no `import` line.
  - Outside companion scripts (tests, library code), import them from the new
    `@d31ma/tachyon/decorators` package export.
  - The existing instance-method API (`this.inject`, `this.provide`,
    `this.onMount`, …) is unchanged.
- Automatic binding of props to fields. `Tac` copies values from `this.props`
  onto every instance field with the same name that the subclass declared. The
  constructor no longer needs `this.foo = this.props.foo ?? default`
  assignments.
  - Matching ignores a leading `$` on the field name. So `$`-prefixed
    persistent fields bind to the prop without the prefix
    (`$clicks` ↔ `props.clicks`).
  - A direct match with the prefixed key still wins.
  - Only fields that the subclass explicitly declared take part.
  - `props` and `tac` are skipped, so a malicious prop object cannot overwrite
    framework state.
- New `./decorators` package export that resolves to
  `src/runtime/decorators.js`.

### Changed

- The build-time compiler class from the `./compiler` entry was exported as
  `Tac`. It is now exported as `Compiler`. Its source moved from
  `src/compiler/template-compiler.js` to `src/compiler/index.js`. The
  `./compiler` package export still resolves. If you imported the default
  export under the name `Tac`, rename the binding to `Compiler`. This avoids a
  clash with the runtime `Tac` class.
- Internal reorganization of `src/server/`. Flat files moved into
  subdirectories by responsibility:
  - `route-handler.js`, `browser-env.js`, and `schema-validator.js` are now
    under `src/server/http/`.
  - `openapi.js` is under `src/server/openapi/`.
  - `logger.js`, `console-logger.js`, and `telemetry.js` are under
    `src/server/observability/`.
  - `process-pool.js` is under `src/server/process/`.
  - `yon.js` stays at the `src/server/` root.

  `package.json#main`, `#types`, and `exports."."` now point to the new
  `src/server/http/route-handler.js` path. `exports."./server"` is unchanged.
- Build-time manifests (`route-manifest.json`, `shell-manifest.json`) moved
  from `src/runtime/` to `src/shared/manifests/`. This separates generated
  artifacts from runtime source. The compiler path constants and `.gitignore`
  were updated.

### Removed

- Unused `Compiler` import from `src/cli/serve.js`. The compiler module has
  no top-level side effects, so the import did nothing.
- Unused `layout-manifest.json`. The compiler writes `Compiler.layoutMapping`
  to `shell-manifest.json`. No code ever read or wrote
  `layout-manifest.json`. Removed it from `src/shared/manifests/` and
  `.gitignore`.

### Fixed

- Upgraded `@d31ma/fylo` to `26.21.2`, which brings `@d31ma/chex` `26.21.02`.
  This version added a `SchemaValidator` API. It requires the `.schema.json`
  extension in schema references and `path:`-prefixed cache keys. Changed
  `validateWithChex` to fill the CHEX cache with keys in the expected format.
- Changed `Router.optionsFileName` from `OPTIONS.json` to `OPTIONS.schema.json`
  so the route handler finds the renamed schema files.
- Renamed route `OPTIONS.json` files to `OPTIONS.schema.json` and schema
  history files (`v1.json`) to `v1.schema.json`. This matches the framework's
  naming convention. Moved the test env file from the root to
  `tests/.env.test`.

## [2.0.0]

Tachyon 2.0 is a full rewrite:

- The framework splits into `Tac` (frontend) and `Yon` (backend and runtime).
- The source moves from TypeScript to JavaScript with strict JSDoc.
- The CLI commands have new names.

If you upgrade from 1.x, expect to change imports, bin names, and any code
that expected the old `dist/layouts/` output.

### Breaking Changes

- Source rewritten from TypeScript to JavaScript + JSDoc. The package `main`,
  `types`, and the `.`, `./server`, `./compiler` exports now resolve to `.js`
  files.
- The `./server` export now points to `src/server/yon.js` (before:
  `process-executor.ts`).
- CLI binaries renamed:
  - `tach.init` → `yon.init`
  - `tach.serve` → `yon.serve`
  - `tach.bundle` → `tac.bundle`
  - `tach.preview` → `tac.preview`
- Removed the `dist/layouts/` output. `shells.json` now describes page shells.
  The runtime uses a single app shell template. It injects the HMR (hot module
  replacement) client only in development.

### Added

- Polyglot backend handlers: any executable file with a shebang can serve a
  route. It receives JSON on stdin.
- Object-oriented companion classes with `export default class extends Tac`.
  Anonymous subclasses are also supported.
- Automatic `sessionStorage` persistence for `$`-prefixed instance fields. The
  key is module path + page/component identity + field name.
- Local-first browser `fetch()` inside Tac page and component scripts. It is
  backed by IndexedDB, caches `GET`/`HEAD` reads, and invalidates the cache
  after mutations.
- Explicit allowlist of browser env values through `TAC_PUBLIC_ENV` and
  `this.env(key, fallback)`.
- A generated OpenAPI 3.1 document at `/openapi.json`. A self-hosted Tachyon
  docs UI at `/api-docs` has request auth, filtering, deep links, cURL
  generation, and live "try it out" requests.
- OpenTelemetry storage in FYLO:
  - Request spans and handler spans are linked.
  - Incoming `traceparent` headers continue the trace.
  - Responses include `Traceparent` / `X-Trace-Id` headers.
  - Settings: `OTEL_ENABLED`, `OTEL_FYLO_ROOT`, `OTEL_SERVICE_NAME`, and
    `OTEL_CAPTURE_IP`.
- Scoped component CSS through `@scope ([data-tac-scope="..."])` wrappers.
- Shared frontend folders: `browser/shared/assets/*`, `browser/shared/data/*`,
  the `browser/shared/scripts/main.js` entry, and `browser/shared/styles/*`.
- Built-in health and readiness endpoints: `/health`, `/healthz`, `/ready`,
  `/readyz`.
- `yon.serve` adapts to the app's folders:
  - With only `browser/`, it bundles and serves the frontend.
  - With only `server/`, it serves the backend routes.
  - With both, it runs the full stack on one port.
- Tac template runtime with `{expr}`, `{!expr}`, `@event`, `:prop`,
  `:value`, `<loop :for>`, `<logic :if>`, and `<my-component lazy />`.
- `Tac` helpers: `env`, `fetch`, `emit`, `inject`, `provide`, `onMount`,
  `rerender`, `isBrowser`, `isServer`, `props`.
- Distributed rate limiting through a `rateLimiter` exported from
  `middleware.js`. Upstash Redis works without extra setup. The in-memory
  limiter is kept for deployments with a single instance.
- Production hardening:
  - Bun password verification for hashed Basic Auth.
  - Limits on request body and parameters.
  - Handler timeouts.
  - Rejection of expired JWTs (JSON Web Tokens) when the token can be decoded.
  - Route request and response validation through `OPTIONS`.
  - Configurable CSP.
  - Optional HSTS (HTTP Strict Transport Security).
- Request context that is aware of proxies. CORS (cross-origin resource
  sharing) checks the origin and rejects a request before the handler runs.
  Document requests are detected with `Sec-Fetch-*` headers, or with
  `Accept: text/html` when those headers are missing.
- Cache headers for runtime assets, chunks, shared assets, and shared data.
- Handler responses get an inferred content type: `application/json` for
  output that looks like JSON, `text/plain` otherwise.
- `tac.bundle --watch` and `tac.preview --watch` rebuild static output as you
  work.

### Changed

- Publishing moved to GitHub Packages under `@d31ma/tachyon`.
- Scaffolds now use JavaScript first, with strict JSDoc. The runtime still
  supports TypeScript companion scripts.

### Fixed

- The `findDocs('otel-spans')` call in the telemetry integration test now
  passes the required `StoreQuery` argument. This keeps `tsc --noEmit` clean
  against `@d31ma/fylo` 2.3.

## [1.11.1] and earlier

See the Git history and GitHub release notes for changes before 2.0.

[Unreleased]: https://github.com/d31ma/Tachyon/compare/v26.35.07...HEAD
[26.35.07]: https://github.com/d31ma/Tachyon/compare/v26.33.02...v26.35.07
[26.33.02]: https://github.com/d31ma/Tachyon/compare/v26.33.01...v26.33.02
[26.33.01]: https://github.com/d31ma/Tachyon/compare/v26.31.06...v26.33.01
[26.32.07]: https://github.com/d31ma/Tachyon/compare/v26.31.06...v26.32.07
[26.31.06]: https://github.com/d31ma/Tachyon/compare/v26.30.04...v26.31.06
[26.30.04]: https://github.com/d31ma/Tachyon/compare/v26.30.03...v26.30.04
[26.22.03]: https://github.com/d31ma/Tachyon/compare/v26.21.7...v26.22.03
[26.21.7]: https://github.com/d31ma/Tachyon/compare/v26.21.05...v26.21.7
[26.21.05]: https://github.com/d31ma/Tachyon/compare/v2.0.0...v26.21.05
[2.0.0]: https://github.com/d31ma/Tachyon/compare/v1.11.1...v2.0.0
[1.11.1]: https://github.com/d31ma/Tachyon/releases/tag/v1.11.1
