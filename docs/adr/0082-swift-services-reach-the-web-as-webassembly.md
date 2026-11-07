# ADR 0082: Swift Services Reach the Web as WebAssembly

- Status: Accepted; amended 2026-09-29 (models, events, topics and storage;
  browser APIs and waits; test units, Apple waits, size and CI)
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

## Amendment (2026-09-29): models, events, topics and storage

The first step left four gaps: models could not cross the page boundary,
events stayed inside the Swift program, `@Topic` did nothing, and the build
refused the built-in cache and Store. This step closes them. It still uses no
Foundation and no JavaScriptKit.

### Models

- A model in `client/models` may be a field, an argument, a result or an
  event. The build adds `Codable` to a model that does not declare it, as the
  Apple build does.
- On the web, Tachyon ships its own encoder and decoder
  (`TacValueEncoder`, `TacValueDecoder` and their containers). They turn a
  model into the plain values that the JSON reader and writer use, and back.
  `JSONEncoder` and `JSONDecoder` are Foundation types, so they are not used.
- `Int` is 32 bits in WebAssembly. The encoder and decoder go through 64-bit
  whole numbers, so an `Int64` field keeps values up to 2^53, like a
  JavaScript number.
- A value that does not fit fails with the text "A service argument or field
  value has the wrong type." (before, the text was only "value").

### Events

- A Swift web service now works like a Kotlin or Rust one. Its start answer
  lists its listeners: key, events, order, replay, asynchronous, condition
  and models. The page's event bus registers them and calls back with the
  operation `event`. The program sends every event it publishes to the page.
- So a Swift listener hears events from every service and component on the
  page, and every page listener hears the Swift service's events. When a
  listener returns a model, the program publishes it again, as before.
- The Apple hosts keep delivering events inside the program
  (`TacPlatform.pageListeners` is `false` there).

### Topic

- The event plan of every WebAssembly service module now names the Topic
  models of `client/models` in the service's language, with their addresses
  (`topics`: `ShelfChanged` → `/.tachyon/topics/shelf-changed`).
- When the page registers a listener of a Topic model, it adds that topic's
  stream, as it does for a JavaScript listener (ADR 0061). This page code is
  shared, so Rust and Kotlin web services, which also list their listeners,
  get the same streams.

### Storage

- The Swift behavior runtime named Foundation in a few places: JSON strings
  and numbers, the lock, the clock, regular expressions, text replacement
  and `pow`. It now calls `BehaviorPlatform` and `BehaviorLock` instead.
- `behavior/polyglot/swift_foundation/` holds the Foundation form, which the
  Apple hosts and Yon keep. `native/swift_web_services/` holds the web form:
  the program's JSON reader, Swift's `Regex`, the page clock
  (`tachyon.tac_host_epoch_ms`) and no lock, because the program has one
  thread. The web build swaps one form for the other, as Kotlin swaps its
  storage file.
- The web form answers `builtinCache` and `builtinEntities` through the page
  storage host (`tachyon.tac_host_storage`), with the same requests as
  Kotlin. When the Setup names no adapter and no authorizer, the runtime uses
  the built-in cache and the `indexeddb` Store (ADR 0059). The build no
  longer refuses them for Swift on the web.
- Time in the runtime is 64-bit milliseconds: `Behaviors.now`, the deadline
  and the cache adapter's `expiresAt` are `Int64`. Today's time in
  milliseconds does not fit in a 32-bit `Int`. Code that writes
  `Double(expiresAt)` keeps working.

### Async methods

- Swift behavior methods such as `@Cached` are `async throws ->
  BehaviorJSON`, but the page calls a service method synchronously. On the
  web, the generated member starts a task and runs Swift's cooperative
  executor until the task is done (`MainActor.executor.runUntil`, from
  `@_spi(ExperimentalCustomExecutors) import _Concurrency`). The storage
  host answers at once, so the task never waits for the page.
- A `BehaviorJSON` argument is read from the page's JSON value, and a
  `BehaviorJSON` result goes back as JSON.
- An Apple companion still cannot call an `async` method.

### Sizes and checks

- Release sizes in bytes, built with Binaryen. The Brotli size is what the
  browser downloads.

  | Program | `swift/services.wasm` | Brotli |
  | --- | --- | --- |
  | The first step's counter, before this step | 3,386,408 | 919,248 |
  | The same counter now | 3,677,386 | 1,000,869 |
  | A service with models, events and a Topic listener | 3,702,520 | 1,022,211 |
  | That service and a service with the cache and Store | 3,829,277 | 1,049,292 |

  `tachyon-swift.js` is 7,957 bytes (2,411 with Brotli). The encoder and
  decoder are in every Swift program, which adds about 80 KB with Brotli.
  The behavior runtime, the storage host and the task runner add about 30 KB
  more when a service uses them.
- Swift's `Regex` alone added 1.1 MB, so the web form does not use it. The
  runtime uses six fixed patterns (names, setting names, setting keys,
  whole numbers and numbers), and `BehaviorTextPattern` checks exactly those
  by hand. A unit test fails when the runtime uses a pattern that
  `BehaviorTextPattern` does not know. Swift's task runner adds 0.28 MB.
- It was checked in Chromium 151, Firefox 153 and WebKit 26.5, in a release
  build: a model goes in and comes back, an event reaches a Swift listener
  with a condition, the listener's returned model reaches another Swift
  listener, a Topic record from the server reaches a Swift listener, and
  the cache, `@ClearCache`, the Store and a refused `@AllOrNothing` work.
  After a reload, the Store row is still there in every engine, and the
  cache entry is still there in Chromium and Firefox. No errors were logged.

### What came next

- The curated browser APIs and the behavior runtime's waits came in the
  third step (see the next amendment).
- Still open: size work (dead-code flags, Embedded Swift for small
  services) and a macOS continuous-integration step with the pinned SDK.

## Amendment (2026-09-29): browser APIs and waits

### Browser APIs

- Swift web services get the same curated browser APIs as the other Tac
  languages (ADR 0065): `Clock`, `SecureRandom`, `Connectivity`,
  `PageVisibility`, `Preferences`, `Clipboard`, `Share`, `Notifications`
  and `Geolocation`. Each is a small Swift class in the web program
  (`native/web_apis/*.swift`), and the service creates it in a field:
  `private let clock = Clock()`. The names are Kotlin's; Swift names the
  arguments of `open(title:text:url:)` and `show(title:body:)`.
- Every call goes to the page through the host import
  `tachyon.tac_host_web`, with the same JSON request as Rust and Kotlin.
  `nowMs()` and `monotonicMs()` use the direct imports `tac_host_epoch_ms`
  and `tac_host_monotonic_ms`. A call that the page refuses throws a
  `TacBrowserError` whose text is the page's message, so a service calls
  these methods with `try`.
- The built-in event models (`ConnectivityChanged`, `PositionFound`,
  `BrowserCallFailed` and the others) are `Codable` structs in the program,
  so an `@OnEvent` listener takes them like any model.
- A service method without arguments may now throw (the generated member
  calls it with `try`). A field whose type is `BehaviorContext` or a browser
  API client, or that is set to a new client such as `Clock()`, is not shown
  to the view; the page cannot read such a value. Before this, the website's
  Swift `OrdersService` did not build for the web.
- A native (Apple) Swift service still cannot use these APIs: there is no
  page, and the build refuses them (`TY1317`).

### Waits

- `@Retry` delays and a blocking `@MaxAtOnce` wait with `Task.sleep`. The
  cooperative executor then asks WASI `poll_oneoff` to wait for a clock.
  The small WASI layer did not have it, so each wait spun through hundreds
  of failed host calls. The layer now answers clock subscriptions: it waits
  in JavaScript until the deadline and reports the event. A wait is one host
  call. The call is synchronous, so the page waits, as with a sleep in Go or
  C# on the web. A Web Worker could wait without blocking the page; ADR 0084
  keeps services on the page thread for now.
- verify-snippets now bundles the Swift configuration-binding example for
  the web when `TAC_SWIFT_WASM_SDK` is set, and checks it in Chromium.

### Sizes and checks

- A program holds only the browser API classes and event models that its
  services name, so a service that uses none of them pays nothing for them.
- Release sizes in bytes, built with Binaryen:

  | Program | `swift/services.wasm` | Brotli |
  | --- | --- | --- |
  | The counter, before this step | 3,681,583 | 1,002,867 |
  | The same counter now | 3,681,899 | 1,002,487 |
  | A service that uses all nine browser APIs and six event models | 3,714,732 | 1,003,345 |

  `tachyon-swift.js` grows from 7,957 to 9,031 bytes for the clock wait.
- It was checked in Chromium 151, Firefox 153 and WebKit 26.5, in a release
  build: `Clock`, `SecureRandom`, `Preferences`, the replayed and changed
  color scheme, going offline and online again, a clipboard call refused
  without a click and accepted from a click, and `PositionFound`. In
  Chromium, which can grant the permissions, `ClipboardTextRead` and
  `NotificationPermissionChanged` also reached the service. The website's
  Swift `OrdersService` answered after three tries and waited 37 to 43 ms
  for its 10 ms and 20 ms delays, and `@MaxAtOnce` answered. No errors
  were logged.
- Tests: `tac_web::web_api_tests::a_swift_web_service_*` (clock and random
  numbers, connectivity, visibility and preferences, and the answers to its
  own calls) and
  `tac_web::swift_tests::a_swift_service_retries_after_a_delay_and_binds_settings_as_webassembly`
  (one `poll_oneoff` call per delay) failed before the change and pass
  after it. The web-apis browser gate has a Swift row.

## Amendment (2026-09-29): test units, Apple waits, size and CI

### Browser APIs in Swift test units

- A Swift test unit runs as a native program without a page. Its program
  now holds the browser API classes and event models that its code names,
  and `BrowserFakes`, like Go and Ruby (ADR 0065):
  `BrowserFakes.answer("clock", "now", 1000)` makes `Clock().nowMs()`
  answer 1000. A call without an answer throws a `TacBrowserError` that
  names `BrowserFakes.answer`. `nowMs()` and `monotonicMs()` cannot throw,
  so without an answer they read the computer's clock. An answer goes
  through JSON once, so the service sees the same types as on the page (a
  whole number answers a `Double`).
- The web host is split in two. `TacWebHost` (the calls, text, flags and
  typed answers) is shared. `TacPage` is the page: the `tachyon` imports on
  the web (`native/swift_web_services/TacPage.swift`) and `BrowserFakes` in
  a test unit (`native/swift_test_units/`).
- A unit can call a listener with a built-in event model, for example
  `service.hearConnectivity(ConnectivityChanged(online: false))`.

### Async methods in Apple companions

- The generated member of an `async` method calls `tacAwait`, which only
  the web prelude had, so a Swift companion with an `async` method (such
  as a `@Cached` method) did not compile for macOS or iOS. The Apple
  prelude now has its own `tacAwait`: it starts the work in a detached task
  and waits on the owner (main) thread for it, and while it waits it keeps
  the main run loop turning (`TacOwnerThread`). So work that the method
  sends to the main actor finishes instead of blocking forever. The
  `TacAwaited` result holder is shared by the web and Apple preludes.

### Size

We measured the release build of the counter and of the service that
uses all nine browser APIs (sizes in bytes; the Brotli size is what the
browser downloads):

| Program | Before this step | After this step | Brotli before | Brotli after |
| --- | --- | --- | --- | --- |
| Counter | 3,681,899 | 3,681,955 | 1,001,786 | 1,002,208 |
| All nine browser APIs | 3,714,732 | 3,716,562 | 1,003,559 | 1,006,896 |

`tachyon-swift.js` stays 9,031 bytes. We also built the counter with other
settings (Binaryen `-Oz` after each, Brotli at quality 11):

| Setting | Release | Brotli | Adopted |
| --- | --- | --- | --- |
| Today's flags | 3,681,955 | 1,000,764 | yes |
| `-Xlinker --strip-all` | 3,681,955 | 1,000,770 | no: Binaryen already strips |
| `-Xlinker --gc-sections` | 3,681,955 | 1,000,781 | no: wasm-ld already does it |
| `-Xfrontend -disable-reflection-metadata` | 3,679,868 | 999,819 | no: changes `String(describing:)` |
| `-enforce-exclusivity=unchecked` | 3,681,508 | 1,000,117 | no: removes a safety check |
| No `Codable` coder (hand-made) | 3,636,553 | 980,830 | no: see below |

- Without the `Codable` encoder and decoder the counter is 20 KB smaller
  with Brotli (2%), not 80 KB: most of the cost is the standard library's
  own `Codable` code, which stays. The coder also turns a whole number into
  a `Double` argument, so a program without it would behave differently.
- Embedded Swift (`-enable-experimental-feature Embedded`, from the same
  SDK) does not compile the Swift web program: 207 errors for the counter
  and 247 for the browser API service. It has no `Codable`, no
  `String(describing:)`, no `Any` values, no `as?` casts and no untyped
  `throws`, and the runtime and authored services use all of them. A
  hand-written Embedded counter with the same two methods, but without the
  page bridge or saved state, is 33,260 bytes (11,774 with Brotli): about
  110 times smaller, 85 times with Brotli. Adopting it would need a
  second runtime and would refuse ordinary Swift services, so the web
  program keeps the full standard library. A service where the download
  matters most can be written in Rust or Dart.

### Continuous integration

- The macOS job installs the swift.org 6.3.3 toolchain (the package's
  signature must be Swift Open Source, V9AUD2URP3) and the Swift SDK for
  WebAssembly with `swift sdk install --checksum`
  (`cabfa08b73bb8ac783927ecd15fa386e99d0c139c5f232445067bcf58379cae7`),
  sets `TAC_SWIFT_WASM_COMPILER` and `TAC_SWIFT_WASM_SDK`, and runs
  `tac_web::swift_tests`, the `a_swift_web_service_*` tests, the Swift test
  unit with `BrowserFakes` and the Apple companion tests.

### Checks

- `client_tests::web_fake_tests::a_swift_unit_answers_the_browser_apis_with_browser_fakes`
  and
  `native::model_tests::a_swift_native_companion_awaits_an_async_method_on_the_owner_thread`
  failed before the change (the unit did not compile; `cannot find
  'tacAwait' in scope`) and pass after it.
- In a release build the web program still answers in Chromium, Firefox and
  WebKit: every browser API, the event models, the refused clipboard call,
  the `@Retry` waits (36 to 38 ms) and `@MaxAtOnce`. No errors were logged.

### Still open

- A Swift `Task.sleep` still blocks the page while it waits (ADR 0084).
