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
