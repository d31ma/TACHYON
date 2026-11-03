# ADR 0055: Annotation Arguments Use Types

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0047](0047-annotation-names-are-plain-english.md) (the
  argument shapes of Test, Store, Entity, Settings, Convert, Check,
  PageDefaults, Cached, ClearCache, UpdateCache and Query) and
  [ADR 0031](0031-realms-mirror-each-other.md) (events are models, not names:
  see "Typed events")
- Amended by: [ADR 0057](0057-needs-all-requires-its-fields.md) (NeedsAll
  requires every field without an initial value)
- Preserves: [ADR 0046](0046-tachyon-annotations-are-global.md) (no imports),
  [ADR 0048](0048-yon-routes-come-from-annotations.md) (parameter markers),
  [ADR 0050](0050-go-annotations-are-comments.md) (Go comment markers) and the
  literal-JSON rule for options

## Context

Several annotations named things that already exist in the code with a string
or a position. `Test({"configuration":"CatalogTestSetup"})` named a class in a
string. `Store({"entity":"Product",...})` did the same. `Entity` repeated the
identifier field that `@Id` already marks. `Settings` repeated every field of
its class, with its type and default, in a JSON descriptor. `Check`,
`PageDefaults`, `Cached`, `ClearCache`, `UpdateCache`, `Query` and `Convert`
chose method parameters by number. A number or a string that repeats the code
goes wrong silently when the code changes.

## Decision

A string that names something in the code becomes a reference to it, or comes
from the signature. A string that names something outside the code stays a
string: Table names, cache names, adapter names, profiles, setting paths,
Gateway file paths and Lifetime choices.

### Class references

- `@Test(CatalogTestSetup)`, with an optional second argument
  `{"partition":"tests"}`.
- `@Store(Product, {"fields":[...],"defaultSize":20,"maxSize":100})`. The
  options stay required. A bare `@Store` is still the injectable role.
- Class spelling per language: Java `X.class`, Kotlin and PHP `X::class`,
  C# `typeof(X)`, and the bare name in JavaScript, TypeScript, Python, Swift,
  Dart, Rust, Go and Ruby. The markers are removed before the language
  compiler runs, so the parser also accepts the bare name everywhere; Swift
  also accepts `X.self`. Documentation shows each language's own spelling.

### Entity and Id

- `Entity` takes only `{"adapter":"..."}`. An `id` key is refused.
- The one field marked `@Id` is the identifier. An Entity without exactly one
  `@Id` field is refused.

### Settings

- `Settings` takes `{"prefix":"YON_APP_"}`, an optional
  `"names":{"field":"ENV_NAME"}` to rename one variable, and, only in Rust and
  Go, `"defaults":{"field":value}`. A `"fields"` key is refused.
- The class's instance fields are the settings. Static and constant fields are
  not settings.
- The type comes from the declared type: text, a whole number, a number or
  true/false. Nullable forms (`String?`, `Option<T>`, `Optional[T]`, `*T`) mean
  the same type. Other types are refused.
- JavaScript has no declared types: a JSDoc `/** @type {integer} */` or the
  literal initial value gives the type. TypeScript has a global
  `type integer = number` for whole numbers; a TypeScript `number` field with
  a whole literal default is also a whole number.
- The default is the literal initial value (text, number, true/false). A field
  with no initial value, or with `null`/`nil`/`None`, is required. Any other
  initial value is refused.
- The environment name is the field name in capitals with underscores:
  `pageSize` and `page_size` both read `<prefix>PAGE_SIZE`.
- `@Min` and `@Max` on a Settings field give its range. `Required`,
  `NotEmpty` and `Length` are refused there, and a class cannot be both
  `Settings` and `Checked`.

### Parameter markers

- A parameter marker sits on the parameter in every language, with the same
  placement as `FromPath`: `@Check input` (JavaScript, TypeScript, Java,
  Kotlin, Swift, Dart), `input: Annotated[ProductInput, check]` (Python, ADR 0056), `[Check] input` (C#),
  `#[Check] input` (Rust, PHP). Go writes `// @Check` on its own line above
  the parameter in a multi-line parameter list.
- `@Check` checks its parameter against a `Checked` class. In JavaScript,
  TypeScript and Python, the parameter's declared type names the class
  (JavaScript uses a JSDoc `@param` type) and the marker takes no argument.
  The other languages pass checked values as JSON values, and a model-typed
  parameter would not compile or would fail at run time, so the parameter
  keeps its JSON type and the marker names the class: `@Check(OrderInput.class)
  Object input` in Java. This is the simplest safe form; the runtimes did not
  change.
- `@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})` sits on the
  page parameter. `"argument"` is refused.
- `@Cached` takes no arguments. `@CacheKey` marks the key parameters; with no
  `@CacheKey`, every parameter is part of the key. `keyArgs` is refused.
- `@ClearCache` and `@UpdateCache` take only `{"method":"find"}`. Their
  parameters are matched by name to the key parameters (and page parameter) of
  that Cached method. A missing name is refused. `"arguments"` is refused.
- A Store query's page parameter is the one marked with a bare
  `@PageDefaults`; the Store gives the sizes. `Query.pageArgument` is refused.
  There is no separate page type: TypeScript's `BehaviorPageInput` is only a
  type.

### Convert

- The method takes exactly one parameter, the value it converts. `"argument"`
  is refused.
- The parameter and the result keep the language's JSON value type. A model
  type in the signature is refused in TypeScript, Python, PHP and Go with a
  plain message; the other languages' generators already refuse it. Mapping a
  model to a model needs generated readers and writers in every language and
  is left for later.

### How the compiler keeps the runtimes unchanged

The parsers turn the new forms into the policy JSON that the generators and
runtimes already read (`configuration`, `entity`, `id`, `fields`, `argument`,
`keyArgs`, `arguments`, `pageArgument`). No runtime changed.

### Migration

Old forms are refused with a message that shows the new form.
`ty migrate fix --write` converts: Test and Store strings to class
references, Entity `id` to an `@Id` line on that field, Convert's `argument`
0, `keyArgs` to `@CacheKey` markers, Check and PageDefaults to parameter
markers (adding a JSDoc `@param` type in JavaScript and the parameter type in
TypeScript and Python), ClearCache/UpdateCache `arguments`, Query's
`pageArgument` to a page parameter marker, and Settings descriptors to field
declarations when the class body is empty. It reports a manual step when a
mutation's parameter names differ from the Cached method's, when the Entity
field cannot be found, when Convert reads another parameter, and when a
Settings class already has members.

## Typed events

Decided and built on 2026-09-26 (the typed events phase). An event is a model.

### Listening

- `@OnEvent` listens for the model type of its one parameter. The name of
  that type is the event name on the bus: `placed(order: OrderPlaced)` hears
  `OrderPlaced`. JavaScript reads the type from the JSDoc
  `@param {OrderPlaced} order` line above the method, as routes do.
- `@OnEvent` takes only `condition`, `id` and `replay`, and may be written
  bare. `Order`, `InBackground`, replay and retain do not change. In a
  condition, `event` is the model name.
- A listener hears one model. Spring's list of classes is not kept: write one
  method per model.
- The compiler turns the listener's model, and every model it reaches, into a
  small description (fields, their types, which are required, and whether
  extra fields are allowed). It comes from the same model reader and follows
  the same rules as the check of a request body (`ModelTable::check`). Every
  runtime checks the payload against it before the condition; a payload that
  does not fit is not delivered and counts as a listener failure.

### Publishing

- `publishEvent(model)` and `publishEvent(model, retain)` publish a model
  value. Go and Rust keep their explicit `retain` argument.
- JavaScript and TypeScript models are types only (JSDoc typedefs and
  interfaces), so the compiler reads the event name from the declared type of
  the value (a parameter or `const` with a model type, a JSDoc `@type`, a
  TypeScript `satisfies`/`as`, or a JSDoc cast) and writes the name into the
  call. The browser never uses `constructor.name`, so minified code keeps the
  names. A value whose type the compiler cannot see is refused at build time.
- The typed languages use the value's type name at run time. Dart in the
  browser uses a generated `is` table because `dart2js` shortens type names.
  A value that is not a model (text, number, list, map) is not published.

### Returned events

- When a listener's declared return type is a model (or a list, optional
  value or future of one), the returned value is published as that model;
  the name comes from the declaration at compile time. The old
  `{event, value, retain}` records are no longer read, and a returned value
  is never retained.

### Model readers and writers

- Yon: the route readers and writers of ADR 0043 (reflection in Java,
  Kotlin, Python and PHP; `System.Text.Json` in C#; generated `YonRead` and
  `From` in Rust and generated readers in Dart; `encoding/json` in Go).
- Tac native companions and WebAssembly services: Swift models get a
  generated `Codable` conformance, Kotlin uses reflection, C# uses a
  generated source-generated `JsonSerializerContext` (safe for ahead-of-time
  and trimmed builds), Rust uses generated `TacModel` impls, and Go uses the
  existing reflection reader. The browser checks payloads for WebAssembly
  listeners; the native preludes check them for native delivery.
- The browser bus (`template/application-events.js`) grew from 10,551 to
  11,104 bytes; `scripts/application-events-test.mjs` checks that budget.
  `tac-client.js` did not change.

### String event names

String event names are refused, so there is one way: an `event` or `events`
key on OnEvent, and a text first argument to `publishEvent`, fail with
TY1308 and a message that names `ty migrate fix --write`. The fix converts a
listener whose event name matches a model (the same name, or the name in
PascalCase, such as `order.placed` to `OrderPlaced`) when its parameter
already has that type, or, in JavaScript and TypeScript, when the parameter
is untyped; it converts JavaScript and TypeScript calls with a JSDoc cast or
`satisfies`. Every other case is a manual step: declare a model named after
the event, give the parameter that type and publish a value of it. A
generated model was judged too much to guess.

Ruby listeners are typed with YARD (`# @param order [PlacedOrder]`) in both
realms (ADR 0054, "Behavior annotations").

### Cross-language delivery

Models with the same name in different languages interoperate because the
wire payload is the model's JSON. The browser page is the one bus shared by
several languages: a Rust or Go WebAssembly listener hears a JavaScript
publisher's model, and a Rust listener hears a Go publisher's model
(`scripts/runtime-browser-test.mjs`). A Yon invocation runs in one language,
so a Java listener and a Python publisher never share a bus; they share only
the contract (name and JSON shape), which the Yon tests check in every
language.

## Consequences

Renaming a class, a field or a parameter now breaks the build where it is
used, instead of silently disconnecting a string or an index. Examples and
fixtures are converted in every language. The TypeScript declarations type
the class references (`Store` checks that `fields` are fields of the Entity
class) and describe each annotation in plain English on hover.
