# ADR 0078: Kotlin Services Reach the Web as WebAssembly

- Status: Accepted
- Date: 2026-09-28
- Amends: [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md)
  ("Swift and Kotlin stay platform languages": Kotlin no longer does),
  [ADR 0030](0030-components-use-services.md) (the service target matrix)
- Extends: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  whose protocol, per-service modules and page bridge Kotlin shares
- Preserves: the Android host's native Kotlin companion

## Context

Before this decision, a Kotlin service ran only inside the Android app. On
every other target it needed a JavaScript, TypeScript or Dart copy. Rust, Go
and C# services already reach every target as WebAssembly. Kotlin can do the
same: `kotlinc-js -Xwasm -Xwasm-target=wasm-js` compiles Kotlin to
WebAssembly with garbage collection (WasmGC).

The Android companion prelude could not be reused as it was. It used the JVM:
`org.json`, reflection over model fields, `Thread` and `@Synchronized`.
Kotlin/Wasm has none of these.

## Decision

### Which services use WebAssembly

- A service with a Kotlin file and no browser script compiles to WebAssembly
  on every target where no host links it. That is every target except
  Android. Android still links the native companion.
- When a service has files in several WebAssembly languages, Tachyon uses the
  first one in this order: Rust, Go, C#, Kotlin, Ruby. Rust, Go and C# use
  linear memory, so they run in every browser that has WebAssembly. Kotlin
  needs WasmGC.

### One protocol core, two platforms

- `native/prelude.kt` is now the protocol core, written in plain Kotlin. It
  handles fields, methods, events, `@OnEvent` options, required properties
  and disposal.
- `prelude_android.kt` adds the Android parts. These are `TacStore`,
  `TacBridge`, `org.json`, reflection and the synchronized entry point. The
  Android host calls the same symbols as before.
- `prelude_web.kt` adds the web parts. These are a small JSON reader, the
  host imports (`@JsFun`) for events and browser calls, and the exported
  `tacInvoke`. Like a Rust WebAssembly service, a Kotlin web service sends
  its events to the page. It lists its listeners in `init`. The page then
  calls it back with `op: "event"`.
- Kotlin/Wasm has no reflection. So the build generates a reader and a
  writer for every model in `client/models/*.kt` and for every browser event
  model. Fields, method arguments and listener values of any type are read
  with `tacRead<T>`. `tacRead<T>` uses `typeOf<T>()`, which works on both
  platforms.
- Methods may take arguments. A string, number or boolean argument crosses
  as JSON; any other type is read with `tacRead<T>`. This also fixes Android,
  where a method that took arguments did not compile before.
- An error in a web service reaches the page with its message, for example
  `Web service counter: counter broke`. Android keeps its fixed error text.

### Browser APIs

- The curated browser APIs are `Clock`, `SecureRandom`, `Connectivity`,
  `PageVisibility`, `Preferences`, `Clipboard`, `Share`, `Notifications` and
  `Geolocation`. In Kotlin they have the same method names as in Dart. A
  service creates the API it needs, for example
  `private val clock = Clock()`. If the page refuses a call, the call throws
  `IllegalStateException` with the page's message.
- The browser-interop guard (TY1317) now also covers Kotlin. It refuses
  `external`, `@JsFun`, `@JsExport`, `@JsModule`, `@JsName`, `js(…)`,
  `kotlinx.browser`, `org.w3c` and `kotlin.js`.
- A Kotlin service that Android links has no browser APIs. On Android, a name
  such as `Clock` stays whatever the service imports, for example
  `kotlinx.datetime.Clock`.
- The built-in cache and Store still need their own adapters in Kotlin. The
  build refuses them for Kotlin, as it does for Android.

### Build

- Every Kotlin web service, the generated readers, the core, the web prelude
  and the models compile into one program. The compiler runs in two passes:
  first a library (`-Xir-produce-klib-file`), then JavaScript and
  WebAssembly with dead code removed (`-Xir-produce-js -Xir-dce`).
- The build publishes the result as `kotlin/services.js`,
  `kotlin/services-loader.js` and `kotlin/services.wasm`. The compiler names
  the first two `.mjs`. Tachyon renames them to `.js`, because some static
  hosts do not serve `.mjs` as JavaScript.
- The output is cached under `kotlin-wasm/<key>`. The key is made from the
  compiler version, the standard library, the optimizer and every source.
- A release build runs Kotlin's own production step. This step is Binaryen's
  `wasm-opt` with Kotlin's settings (`-O3 --gufa --type-merging`, with
  garbage collection and exception handling on). Tachyon looks for
  `wasm-opt` on `PATH`, and then next to Emscripten's `emcc`. The build
  prints `Kotlin: Kotlin/Wasm production build — optimized by Binaryen`. If
  there is no `wasm-opt`, the build still ships the program, prints that it
  is about 3 times larger, and `ty doctor` reports it. A development build
  skips this step.
- `crate::wasm_optimizer::WasmOptimizer` finds and runs `wasm-opt`. Any other
  language can use it later.
- The compiler is `kotlinc` 2.1.10, the version the repository pins. The
  WebAssembly standard library is `kotlin-stdlib-wasm-js` 2.1.10. It is not
  shipped with `kotlinc`, so `TAC_KOTLIN_WASM_STDLIB` points at it.
  `ty doctor` checks both, and `wasm-opt`.

### Browser minimum

- WasmGC needs Chrome or Edge 119, Firefox 120, or Safari 18.2 or newer.
  These are the same browsers that Dart needs (ADR 0033).
- Before it loads the program, the shared module checks for WasmGC with
  `WebAssembly.validate`. An older browser gets a clear error instead of a
  failed compile.

## Consequences

- JavaScript, TypeScript, Dart, Rust, Go, C#, Ruby and now Kotlin reach every
  target. Swift is the only platform-only service language.
- Size follows the Kotlin runtime, not the service. We measured one counter
  service that also reads `Clock`. Each size is in bytes; the Brotli size is
  what the browser downloads.

  | File | Release (Binaryen) | Development | Release, Brotli |
  | --- | --- | --- | --- |
  | `kotlin/services.wasm` | 81,444 | 274,738 | 29,889 |
  | `kotlin/services-loader.js` | 7,315 | 7,315 | 2,018 |
  | `tachyon-kotlin.js` + `kotlin/services.js` | 4,567 | 4,567 | about 1,650 |

  This is the size budget: about 80 KB of WebAssembly (30 KB with Brotli)
  for a small Kotlin program. All Kotlin services on a site share this one
  program. A release build without `wasm-opt` is about 3.4 times larger.
- It was checked in Chromium 151, Firefox 153 and WebKit 26.5, in both a
  development and a release build. The service counts, reads the time zone
  through `Clock`, and logs no errors.
- A Kotlin web service has no DOM. Like the other WebAssembly services, it
  holds state and logic, and it reaches the browser only through the curated
  APIs.
- Continuous integration skips the Kotlin web tests unless `kotlinc-js` and
  `TAC_KOTLIN_WASM_STDLIB` are present.
