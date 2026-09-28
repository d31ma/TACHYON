# ADR 0082: Swift Services Reach the Web as WebAssembly

- Status: Accepted
- Date: 2026-09-28
- Amends: [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md)
  and [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md)
  ("only Swift stays a platform language": Swift no longer does),
  [ADR 0030](0030-components-use-services.md) (the service target matrix)
- Extends: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  whose protocol, exports and page bridge Swift shares
- Preserves: the macOS and iOS hosts' linked Swift companion

## Context

Before this decision, a Swift service ran only inside the macOS and iOS apps.
On the web, and on Windows, Linux and Android, it needed a JavaScript,
TypeScript or Dart copy, and the build failed with `TY1010` without one.

swift.org ships an official Swift SDK for WebAssembly. With it, `swiftc`
compiles Swift to WebAssembly for WASI (`wasm32-unknown-wasip1`). The program
uses linear memory, so it runs in every browser that has WebAssembly, like
Rust, Go and C#.

The Apple companion prelude could not be reused as it was. It used
Foundation: `JSONSerialization`, `UserDefaults`, `Thread`, `UUID`,
`FileHandle`, `JSONEncoder` and `JSONDecoder`. Foundation exists for WASI,
but it is large. We measured one small program, built with `-Osize` and the
static standard library:

| What the program imports | WebAssembly | Brotli |
| --- | --- | --- |
| Only the Swift standard library | 3.38 MB | 0.89 MB |
| `FoundationEssentials` | 7.9 MB | 2.0 MB |
| `Foundation` | 45.9 MB | not measured |

## Decision

### Which services use WebAssembly

- A service with a Swift file and no browser script compiles to WebAssembly
  on every target where no host links it. That is the web, Windows, Linux and
  Android. macOS and iOS still link the Swift companion.
- When a service has files in several WebAssembly languages, Tachyon uses the
  first one in this order: Rust, Go, C#, Swift, Kotlin, Ruby. Swift comes
  before Kotlin because it uses linear memory and needs no WasmGC.
- Every service language now reaches every target. So no build fails any
  more because a service has no language for the target, and the `TY1010`
  check for that case is removed. `TY1010` still means that a native app has
  more than 128 native services.

### One protocol core, two platforms

- `native/prelude.swift` is now the protocol core. It uses no Foundation. It
  handles fields, methods, `@OnStart` and other hooks, required properties
  and disposal.
- `prelude_apple.swift` adds the Apple parts: `TacStore`, JSON through
  `JSONSerialization`, models through `JSONEncoder` and `JSONDecoder`, the
  `UUID` owner check, the main-thread check and the fixed error text. The
  Apple hosts call the same symbols as before.
- `prelude_web.swift` and `swift_web_services/TacJSONReader.swift` add the
  web parts. They use only the Swift standard library: a small JSON reader,
  the exports and the host import. The exports are the same as Rust's:
  `tac_wasm_alloc`, `tac_wasm_dealloc`, `tac_native_invoke` and
  `tac_native_free`. The program sends events to the page through the
  `tachyon.tac_host_emit` import (`@_extern(wasm)`, a Swift experimental
  feature).
- The page side is shared with Rust. Only the start differs: Swift loads
  `swift/services.wasm`, gives it a small WASI layer (console output, clock,
  random numbers and "not supported" for files), and calls `_initialize`.
- An error in a web service reaches the page with its text, for example
  `Web service counter: counter broke`. The Apple hosts keep their fixed
  error text.

### Fields and methods

- Methods may take arguments. The generated code keeps the argument labels,
  so `func add(_ by: Int, why: String)` is called as
  `add(tacArgument(tacArguments, 0), why: tacArgument(tacArguments, 1))`.
  This also fixes the Apple companions, where a method that took arguments
  did not compile before.
- A field setter converts the page's value to the field's type. For example,
  the page may set a `Double` field to `2`.
- On the web, fields, arguments and results may be `String`, `Bool`, the
  integer types, `Double` and `Float`, and arrays of them. `Int` is 32 bits
  in WebAssembly, so use `Int64` for larger numbers.
- A service may use its own structs inside the program. A model that crosses
  to the page (as a field, an argument or an event) needs `Codable`, which
  comes in the next step, together with events.

### Stored fields

- A `@LocalState` or `@SessionState` field is saved by the page after every
  method call, as for every WebAssembly language. The field keeps its value
  after a reload.

### Browser interop

- The browser-interop guard (TY1317) now also covers Swift. It refuses
  `JavaScriptKit`, `JavaScriptEventLoop`, `JSObject`, `JSValue`, `JSClosure`,
  `JSPromise`, `@_extern` and `@_expose`. Tachyon does not use JavaScriptKit.

### Build

- Every Swift web service, the models, the core and the web prelude compile
  into one program, `services.swift`. The flags are `-target
  wasm32-unknown-wasip1 -static-stdlib -Osize -parse-as-library`, a reactor
  (`-mexec-model=reactor`) and `-enable-experimental-feature Extern`.
- The build publishes `swift/services.wasm`. The output is cached under
  `swift-wasm/<key>`. The key is a SHA-256 digest of the compiler version,
  the SDK, the optimizer and the program. (The Kotlin key had the same
  shape, but it was not a digest, so it was too long to save. It is a digest
  now too.)
- A release build also runs Binaryen's `wasm-opt -Oz` through
  `WasmOptimizer`. The build prints
  `Swift: WebAssembly release build — optimized by Binaryen`. Without
  `wasm-opt`, it ships the program and says so, and `ty doctor` reports it.
- Kotlin and Swift share the tool run, the error text, the notes, the
  optimizer step and the cache save (`native/web_program_build.rs`).
- The Swift compiler and the SDK must have the same version. Tachyon pins
  Swift 6.3.3. `TAC_SWIFT_WASM_SDK` points at the SDK's
  `wasm32-unknown-wasip1` folder. `TAC_SWIFT_WASM_COMPILER` names the
  swift.org `swiftc` when another `swiftc` comes first on `PATH` (Xcode's
  `swiftc` cannot use the swift.org SDK). `ty doctor` checks both, and
  `wasm-opt`.

## Consequences

- JavaScript, TypeScript, Dart, Rust, Go, C#, Swift, Kotlin and Ruby reach
  every target. No service language is platform-only now.
- Size follows the Swift standard library, not the service. We measured one
  counter service with a stored field and a method with arguments. Each size
  is in bytes; the Brotli size is what the browser downloads.

  | File | Release | Development | Release, Brotli |
  | --- | --- | --- | --- |
  | `swift/services.wasm` | 3,386,408 | 3,463,770 | 919,248 |
  | `tachyon-swift.js` | 7,540 | 7,540 | 2,290 |

  This is the size budget: about 3.4 MB of WebAssembly (0.9 MB with Brotli)
  for a small Swift program. `wasm-opt` removes only about 2%, because most
  of the program is the standard library's runtime and metadata. All Swift
  services on a site share this one program. `import Foundation` would make
  it more than ten times larger, so a Swift web service should not import it.
- It was checked in Chromium 151, Firefox 153 and WebKit 26.5, in a release
  build. The service counts, takes arguments, keeps its `@LocalState` field
  after a reload, and logs no errors.
- A Swift web service has no DOM. Like the other WebAssembly services, it
  holds state and logic.
- The next step adds events and `@Topic`, `Codable` models across the page
  boundary, the built-in storage host and the curated browser APIs. Until
  then, the build refuses the built-in cache and Store for Swift, as it does
  for Kotlin.
- Continuous integration skips the Swift web tests unless
  `TAC_SWIFT_WASM_SDK` and a matching `swiftc` are present.
