# ADR 0037: C# Services Reach the Web as WebAssembly

- Status: Accepted
- Date: 2026-09-23
- Amends: [ADR 0019](0019-companions-compile-for-their-target.md) (C# is no
  longer Windows-only), [ADR 0030](0030-components-use-services.md) (the
  service target matrix)
- Extends: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  whose protocol, markers and per-service modules it shares
- Preserves: the Windows host's NativeAOT companion, the companion protocol

## Context

ADR 0035 took Rust to the web. It compiled the desktop companion, dispatch
table and prelude without changes, with the page acting as their host.

C# has the same shape. Its Windows companion is a generated dispatch table and
a prelude that answers one JSON request at a time. .NET can publish for the
browser (`browser-wasm`). Its JavaScript interop can export one method to the
page and import one method back.

## Decision

### Which services use WebAssembly

- A service with a C# file and no browser script reaches the web as
  WebAssembly wherever no host links it. That is every target except Windows.
- A service written in both Rust and C# uses Rust, because the Rust module is
  a small file, not a runtime.

### Build

- Every such service compiles into one assembly. The assembly contains the
  Windows companion's table and prelude, without its C entry points.
- `dotnet publish` publishes the assembly on the
  `Microsoft.NET.Sdk.WebAssembly` SDK. The output is trimmed, uses invariant
  globalization, and has no pre-compressed copies.
- The .NET runtime and assemblies are published under
  `.tachyon/services/dotnet/_framework`. They are about 3 MB. .NET's own loader
  starts them.
- The publish output is cached, keyed by its source and the SDK version.

### Page bridge

- The page reaches the prelude through `[JSExport] TacWeb.Invoke`.
  Publications leave through an imported `emit`.
- Everything else that the Rust modules do is shared: the per-service class,
  events delivered after the call returns, disposal, and
  `RequiredArgsConstructor` properties.

### Markers and members

- C# declares Tac's markers as attributes, each on its own line:
  `[Service()]`, `[Scope({"value":"prototype"})]`,
  `[RequiredArgsConstructor("field", …)]`. The build reads them and removes
  them.
- A C# method's parameters of type `string`, `int`, `long`, `double`, `float`,
  `bool` or `object` cross as JSON. A method that takes any other type stays
  inside C#.
- An `object` field is assigned the value as it is received. A list arrives as
  `object[]`, and an object arrives as `Dictionary<string, object>`.

### Limits and tools

- `init` may carry up to 1 MiB, because its properties may be a data file that
  the view passed (ADR 0036). Every other request keeps the 64 KiB limit. This
  holds in the C# prelude and in the page's native bridge.
- A request over its limit fails with an error that names the size and the
  limit in bytes, for example "The request to add is 70121 bytes. The limit
  is 65536 bytes. Pass less data to the service." The page adds the service
  name in front. (Added 2026-09-28; before, C# answered only "Native
  companion operation failed.")
- Rust, Kotlin and Swift services follow the same two limits: 1 MiB for
  `init` and 64 KiB for every other request. (Added 2026-09-29; before, they
  refused any request over 64 KiB, `init` included.)
- The Linux host's CSP (Content Security Policy) allows `wasm-unsafe-eval`,
  because Linux links no C#.
- `ty doctor` checks for the `wasm-tools` workload.

## Consequences

- JavaScript, TypeScript, Dart, Rust and C# all reach every target. Swift and
  Kotlin stay platform languages. (Amended by
  [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md): Kotlin now
  reaches every target as WebAssembly; only Swift stays a platform language.
  Amended again by
  [ADR 0082](0082-swift-services-reach-the-web-as-webassembly.md): Swift also
  reaches every target as WebAssembly, so no platform language is left.)
- A page with a C# service downloads the .NET runtime once. After that, the
  page serves it from the cache that the service worker keeps. This download
  is the cost of C# on the web, and it is why Rust wins when a service has
  both.
- Like a Rust web service, a C# web service has no DOM. It is state and logic
  over its properties.
- The runtime reads a service's field only when the field is named after
  another service. Otherwise every field of a native or WebAssembly service
  would cost a round trip each time a service was created. This includes a
  field that holds a data file.

## Amendment (2026-09-30): models reach the view

- A field, a property, a method argument or a method result may be a model
  from `client/models`, a list of models (`List<T>`, `IList<T>`, `T[]` and
  the other list interfaces, and an F# `list`), or a model that holds other
  models. The member table writes it through the JSON model context
  (`TacModels.Plain`) and reads it back with `TacModels.Read` and
  `TacModels.ReadList`, so an `object` field is no longer needed to pass a
  list or an object. A `Task<T>` member still stays inside C#.

## Amendment (2026-10-03): compiled Yon handlers share the user cache

The cached web builds (C# here, and the Kotlin, Swift, Rust, Go, Dart and Java
web builds) already live in the cache that `TAC_CACHE_DIR` selects, keyed by
their inputs. Compiled Yon handlers now use the same cache.

- After Yon builds a Kotlin, Rust, Dart, Go or Groovy handler into the
  project's `.tachyon/handlers` cache, it also writes a copy to
  `runtime/handlers-v1` in the user cache. The copy has the artifact's name: a
  digest of the authored source, the model files, the generated protocol code,
  the compiler identity, the processor architecture and the operating system.
- When the project cache has no valid artifact, Yon restores a shared copy
  with that name while it holds the project's build lock. It restores only a
  regular, non-empty file and never follows a symbolic link. The file goes
  into the project cache through the same confined publication as a compiler
  output, and execution still copies it into a private runtime workspace.
- The shared folder keeps the newest 512 entries. A shared-folder failure
  never fails a build. `ty cache clean` removes the shared copies. C# and F#
  handlers keep their own multi-file .NET caches.
- Yon asks a compiler for its version once per process for the same `PATH`
  and `JAVA_HOME`.

Security note: the user cache is now an input to the handlers that Yon runs,
as it already is to the web builds that the browser runs. Anyone who can write
to that folder can change them, so it must stay private to the developer
account. A shared copy is trusted by its name; `ty cache clean` repairs a
damaged one.

Measured effect: a warm core test run on the development Mac went from 527 s
to 334 s, because each Kotlin compile (about 8.5 s) now happens once per
machine instead of once per test project.
