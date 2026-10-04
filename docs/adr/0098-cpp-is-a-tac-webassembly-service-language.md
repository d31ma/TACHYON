# ADR 0098: C++ Is a Tac WebAssembly Service Language

- Status: Accepted (Tac slice; the Yon slice is pending)
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
- The rule table lists C++ (`hpp`) as a Tac language. Each marker is checked
  against its rule: a marker that the rule table refuses for Tac C++ fails
  with the rule's own code, and a marker that the rule table allows but C++
  does not implement yet fails with `TY1372` and names the markers that work.
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
- `ApplicationEventPublisher applicationEventPublisher;` publishes a model
  with `publishEvent(model, retain)`. `[[OnEvent]]` takes the event model as
  its one parameter; `{"replay", "condition", "id"}` options, `[[Order(n)]]`
  and `[[InBackground]]` work as in the other languages, and a returned model
  is published again. `[[OnStart]]` and `[[OnStop]]` run on activation and
  disposal. Listener and hook methods may be private: Tachyon adds a `friend`
  declaration for the generated binding class.

### JSON and models

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
  `tac_native_invoke` and `tac_native_free`, and imports `tac_host_emit`,
  `tac_host_storage`, `tac_host_web` and `tac_host_epoch_ms` from `tachyon`:
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

- C++ service methods are synchronous in this slice. The asynchronous path
  will follow Rust's host bridge (`tac_host_http_start`, `tac_http_settle`,
  `tac_host_task_done`) with a C++20 coroutine task type, so the page side
  reuses the Rust task bridge unchanged.

### Size

- A page program with one service, one model and the JSON prelude measured
  85 KB of WebAssembly (31 KB with Brotli) in the prototype with `-Oz` and
  emmalloc. `std::to_chars` for doubles would add about 100 KB, so the JSON
  writer prints numbers with the shortest of `%.15g`, `%.16g` and `%.17g`
  that reads back exactly.

## Pending

- Yon C++ handlers (controllers, routes, middleware, error handlers, sockets,
  relays, Yon test units), compiled per handler with clang++.
- `@HTTPClient` and `tachyon::http` (`HttpRequestMessage`,
  `HttpResponseMessage`, `HttpMethod`, `HttpStatusCode`, `HttpContent`),
  asynchronous methods, `@Cached` and `@Store`, browser storage markers
  (`@LocalState`, `@SessionState`), the curated web APIs, behaviors,
  `@Lifetime`, `@Named`, `@NeedsAll`, converters, helpers and topics. Each
  fails with `TY1372` or the rule table's refusal until it lands.
- `ty migrate` rewriting the legacy C++ Yon handlers.

## Consequences

- C++ teams write Tac services in their own language with Tachyon's usual
  markers, and editors understand the files.
- Tachyon owns and maintains a JSON implementation and a model-codec
  generator for C++.
- Pages that use a C++ service download one more WebAssembly module, about
  the size of a Swift or Go one without their runtimes.
