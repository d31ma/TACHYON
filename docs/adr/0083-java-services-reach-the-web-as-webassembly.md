# ADR 0083: Java Services Reach the Web as WebAssembly

- Status: Accepted
- Date: 2026-09-29
- Extends: [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md)
  (WasmGC services) and [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md)
  (the companion protocol and the page bridge)
- Amends: [ADR 0030](0030-components-use-services.md) (the service languages)
- Keeps: [ADR 0072](0072-groovy-is-a-yon-language.md) (Groovy is Yon only)

## Context

Before this decision, Java was a Yon (server) language only. A Tac service
could not be written in Java.

TeaVM compiles Java bytecode to JavaScript and to WebAssembly. Since
version 0.10 it has a backend for WebAssembly with garbage collection
(WasmGC), the same kind of WebAssembly that Kotlin and Dart services use.
TeaVM brings its own small copy of the Java class library, and keeps only
the classes and methods that the program reaches.

We tried TeaVM 0.15.0 (Maven Central, `org.teavm:cli`) before we chose it:

- The WasmGC backend exports a static method marked `@JSExport` to
  JavaScript, and Java strings cross as JavaScript strings. So the page can
  call one `tacInvoke(String)` export, as it does for Kotlin.
- TeaVM ships the loader for the module (`wasm-gc-module-runtime.js`)
  inside its jar. The page loads the module with it.
- The command-line jar (`cli-0.15.0-all.jar`) registers only one of TeaVM's
  four compiler plugins in `META-INF/services`. The four plugin jars
  (`teavm-classlib`, `teavm-jso-impl`, `teavm-platform`,
  `teavm-metaprogramming-impl`) must come before it on the class path of
  the `java` process. TeaVM itself must read classes from the `-all` jar, one
  `-p` flag for each class path entry.
- Binaryen `wasm-opt -Oz` makes the module about half as large. With
  `--closed-world` the module fails at run time, because TeaVM's loader calls
  into the module, so the build does not use it.

So the WasmGC backend can do everything the bridge needs, and Tachyon does
not use TeaVM's JavaScript backend.

## Decision

### Which services use Java

- A service with a `.java` file in `client/services` and no browser script
  compiles to WebAssembly. No native host links Java, so the module runs on
  the web and in the web view of every target, like Go.
- When a service has files in several WebAssembly languages, Tachyon uses the
  first one in this order: Rust, Go, C#, Swift, Kotlin, Java, Ruby.
- WasmGC needs Chrome or Edge 119, Firefox 120 or Safari 18.2 and newer. An
  older browser gets a clear error, as for Kotlin and Dart.

### The Java prelude

- The prelude is plain Java, one class per file, under
  `native/java_web_services/prelude/`: `TacServices` (the main class and the
  `tacInvoke` export), `TacBridge` (the protocol), `TacActivation`,
  `TacField`, `TacMethod`, `TacHook`, `TacCall`, `TacValues`,
  `TacJsonReader` and `TacJsonWriter`. It uses no library outside the Java
  class library, so TeaVM compiles all of it.
- `TacBridge` speaks the same protocol as the other WebAssembly languages:
  `init`, `get`, `set`, `call`, `dispose` and `host-dispose`, the owner id,
  256 activations at most, the 1 MiB start-up request and the 64 KiB member
  request (ADR 0037 note), required properties from `@NeedsAll`, and
  `@OnStart` and `@OnStop` hooks. An exception in a method reaches the page
  with its message, for example `Web service counter: counter broke`. A
  checked exception is passed on the same way.

### Reading the service

- Tachyon reads the service class from the source. Every field and method
  that is not `private` or `static` reaches the page. A `final` field is read
  only. Constructors, nested types and initializer blocks stay inside.
- Fields, arguments and results may be `int`, `long`, `double`, `float`,
  `boolean`, their boxed types, `String`, `Object`, and `List` and `Map` (with
  `String` keys) of these. The generated table converts each page value to
  the declared type, also inside lists and maps.
- Tachyon generates `TacServiceTable.java`, which builds the member table of
  each service with lambdas. The service files keep their lines, so a
  compiler error names the right line of `client/services/<Name>Service.java`.
- `@Service`, `@Lifetime` and `@NeedsAll` are removed as for every service
  language. `@OnStart`, `@OnStop`, `@LocalState` and `@SessionState` are
  replaced by spaces before `javac` runs, so no annotation type is needed.
- The build stops with `TY1405` for a member it cannot pass to the page (for
  example `char` or `int[]`), several fields in one statement, a private or
  static member marked `@OnStart`, `@OnStop`, `@LocalState` or
  `@SessionState`, a lifecycle method with parameters, or any other
  annotation (such as `@Cached` or `@OnEvent`). The message says what works.
  It stops with `TY1406` when the file has no class of its name, or the class
  has nothing the page can use.

### Stored fields

- A `@LocalState` or `@SessionState` field is saved by the page after every
  method call, as for every WebAssembly language, and keeps its value after
  a reload.

### Browser interop

- The browser-interop guard (`TY1317`) now covers Java. It refuses
  `org.teavm`, `JSBody`, `JSObject`, `JSExport`, `JSFunctor`, `JSProperty`,
  `JSMethod`, `JSClass`, `JSModule`, `JSTopLevel`, `JSByRef` and `native`
  methods. The Tachyon prelude uses `@JSExport`; a service may not.

### Build

- `javac --release 21` compiles the services, the prelude and the table
  against the TeaVM jar. TeaVM (`-t wasm-gc -O 2`) makes `services.wasm`
  from the class `TacServices`. `jar` takes the loader out of the TeaVM jar.
- The build publishes `java/services.wasm`, `java/runtime.js` and the shared
  `tachyon-java.js`. The output is cached under `java-wasm/<key>`, where the
  key is a SHA-256 digest of the `javac` version, the TeaVM jars, the
  optimizer and every source file.
- A release build runs `wasm-opt -Oz --strip-debug --strip-producers` with the
  WasmGC features through `WasmOptimizer` and prints
  `Java: TeaVM WebAssembly release build — optimized by Binaryen`.
- `TAC_TEAVM_HOME` names the folder of the TeaVM jars: the `-all` jar and the
  plugin jars. `ty doctor` checks it, `javac` and Binaryen when the project
  has a Java web service.

## Groovy

Groovy cannot go the same way. Groovy compiles to JVM bytecode, but every
Groovy class is a `GroovyObject` and needs the Groovy runtime: the meta-class
registry, call sites and its concurrent caches. That is true even with
`@CompileStatic`. We compiled a one-method `@CompileStatic` Groovy class and
passed it with `groovy-5.1.3.jar` to TeaVM 0.15. TeaVM stopped with 137
errors, all from the runtime: `ReentrantLock`, `ReentrantReadWriteLock`,
`ConcurrentLinkedQueue`, `Set.parallelStream` and
`ClassLoader.getResources` are not in TeaVM's class library. So Groovy stays
a Yon language, and a Groovy developer writes Tac services in Java or
Kotlin.

## Consequences

- JavaScript, TypeScript, Dart, Rust, Go, C#, F#, Swift, Kotlin, Java and
  Ruby services reach every target.
- Size (one counter service with a stored field and a method with
  arguments; bytes):

  | File | Release | Without Binaryen | Release, Brotli |
  | --- | --- | --- | --- |
  | `java/services.wasm` | 69,420 | 138,300 | 29,163 |
  | `java/runtime.js` (TeaVM's loader) | 16,425 | 16,425 | 5,250 |
  | `tachyon-java.js` | 4,519 | 4,519 | 1,501 |

  The website's tally example (one field, one read-only field, one method)
  is 128,639 bytes from TeaVM and 68,595 bytes after `wasm-opt` (44,555 and
  29,028 bytes with Brotli). Most of the module is the prelude: the JSON
  reader and writer, the collections and number formatting from TeaVM's
  class library. All Java services on a site share this one module. As sent, that
  is about a thirtieth of the Swift module; as a raw file, about two thirds
  of the Kotlin module (101 KB, ADR 0078).

- It was checked in Chromium 151, Firefox 153 and WebKit 26.5, in a release
  build. The service counts, takes arguments, keeps its `@LocalState` field
  after a reload, and logs no errors.
- A Java web service has no DOM. Like the other WebAssembly services, it
  holds state and logic.
- The next step adds events and `@Topic`, models across the page boundary
  (records and classes in `client/models`), the built-in storage host
  (`Cached`, `Store`, `AllOrNothing`), the curated browser APIs and the
  behavior runtime. Until then the build refuses them with a clear message.
- Continuous integration skips the Java web tests unless `TAC_TEAVM_HOME`
  and `javac` are present.
