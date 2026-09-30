# ADR 0087: Services Reach the Window and the Network Through Injection

- Status: Accepted (slices 1 to 5 are done; slice 6, the removal of the
  `HTTPClient` cache option, and the native window controls of companions
  are open)
- Date: 2026-09-30
- Relates to: [ADR 0022](0022-shared-behavior-adapters.md) (behavior
  annotations), [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
  (the manifest is the Tac configuration),
  [ADR 0073](0073-fsharp-is-a-yon-language-and-a-tac-service-language.md),
  [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md),
  [ADR 0082](0082-swift-services-reach-the-web-as-webassembly.md),
  [ADR 0083](0083-java-services-reach-the-web-as-webassembly.md) (the
  WebAssembly service languages and their browser APIs)

## Context

Two feature pages show a service that reaches the browser by itself:

- `features/window-controls`: a JavaScript service calls
  `globalThis.tachyonWindow.minimize()`. The native host puts
  `tachyonWindow` on the page with one function for each control that
  `tac.application.window.controls` in `client/manifest.json` grants. Only
  JavaScript can read a global object, so the other service languages cannot
  control the window, and a test unit cannot replace the window.
- `features/fetch-cache`: a service calls `fetch('/api/products')` itself.
  It does this because an `HTTPClient` read is sent with
  `cache: 'no-store'`, and the service worker skips such a request. So the
  worker's `tac.cache` rules could answer only a raw `fetch`.

The user decided:

1. A service never reads `globalThis`. The window controls come by
   dependency injection (a field the framework fills), not from a global
   object and not from a base class.
2. A service never calls `fetch`. The network belongs to an `HTTPClient`.
3. There is no new annotation for a network cache. The cache annotations
   that services already use, `Cached`, `CacheKey`, `ClearCache` and
   `UpdateCache`, are also allowed on `HTTPClient` methods, as Spring's
   `@Cacheable` works on any bean, an HTTP interface client included.

Two facts shape the design:

- A WebAssembly service (Rust, Go, C#, F#, Kotlin, Java, Dart, Ruby, Swift)
  reaches the page only through Tachyon's host bridge: one synchronous JSON
  call, `{ api, call, args }`, answered by `TacWebApis` in `tac-web.js`.
  `Clipboard`, `Clock` and the other browser APIs use it. Each language has
  a small class with the same names (`native/web_apis`, `tac_dart`,
  `tac_ruby`).
- An `HTTPClient` in a WebAssembly language sends its request through the
  HTTP adapter of its Setup, and that adapter is code in the same language.
  Only a JavaScript or TypeScript `HTTPClient` uses Tachyon's browser
  transport (`fetch`). So the service worker can answer only JavaScript and
  TypeScript reads.

Before this ADR, a Tac page had three caches for data: the service worker's
Cache Storage (policy by path in `tac.cache`), the `HTTPClient` cache in
IndexedDB (`@HTTPClient({"cache": "cache-first"})`, one policy for a whole
client), and the behavior cache of `Cached` service methods (named by
`CacheSettings`).

## Decision

### WindowControls is an injected role

`WindowControls` is a browser API like `Clipboard`. A service declares a
field and Tachyon fills it:

| Language | Field |
| --- | --- |
| JavaScript, TypeScript | `@Needs windowControls` (type `TacWindowControls`) |
| Rust | `window_controls: WindowControls` (filled by `Default`) |
| Go | `// @Needs` field `windowControls WindowControls` |
| C# | `private readonly WindowControls windowControls = new();` |
| F# | `let windowControls = WindowControls()` in the type (the C# class, see ADR 0073) |
| Kotlin | `private val windowControls = WindowControls()` |
| Java | `private final WindowControls windowControls = new WindowControls();` |
| Dart | `@Needs() final WindowControls windowControls;` |
| Ruby | `WindowControls.new` |
| Swift | `private let windowControls = WindowControls()` |

Each field is written like the language's existing `Clipboard` field.

`WindowControls` goes through the page, like `Clipboard`. A service that a
native app links in instead of running it in the page (a companion: Rust and
C# on the desktop, Swift on Apple, Kotlin on Android) has no page, so the
build refuses `WindowControls` there with `TY1317`, as ADR 0065 decided for
every browser API. A native `WindowControls` in the companion preludes, which
asks the host directly with the same grants, is planned as a later slice.

The calls have the names of the controls. Each language writes them in its
own case (`set_title` in Rust and Ruby, `SetTitle` in Go and C#):

- `granted(control)` answers at once whether the native window granted the
  control.
- `minimize()`, `maximize()`, `fullscreen()` and `close()`.
- `resize(width, height)`.
- `setTitle(text)`.

The grants stay where they are: `tac.application.window.controls` in
`client/manifest.json`. None is granted by default. The native host still
checks each grant before it acts. A call to a control that is not granted
fails at once, and the message names the grant to add. Each language fails
in its own way, as its `Clipboard` does: JavaScript throws an `Error`, Rust
returns `Err(String)`, Go returns an `error`, C# and Java throw an
exception (Java an `IllegalStateException`), Kotlin, Dart and Ruby raise,
and Swift throws. In a browser tab no control is granted, because the page
has no native window, so `granted` answers `false` there. A test unit gives
fake answers the way it does for `Clipboard`: a `@Fake windowControls` field
in JavaScript and TypeScript, and `BrowserFakes` in Go, Ruby and Swift.

The native host shim no longer defines `globalThis.tachyonWindow`. It gives
the granted list to Tachyon's own page runtime, and `TacWebApis.windowControls`
sends each call to the host as `window.<control>`. This is the only way to
reach the window. This happens in the same slice as the refusal, so that
code which still reads the global gets the build error and not a silent
`undefined`.

### Cached HTTPClient methods follow Spring

`Cached`, `CacheKey`, `ClearCache` and `UpdateCache` are allowed on the
methods of an `HTTPClient` in every Tac language. They mean exactly what
they mean on a service method. The client class names its cache with
`CacheSettings`. A `Cached` method keeps its answer under the `CacheKey`
arguments. `ClearCache({"method": "find"})` removes an answer, and
`UpdateCache({"method": "find"})` replaces it, after a successful write. They
find the target's key arguments by parameter name. The same behavior runtime
and the same storage host do the work, so the WebAssembly languages need no
second implementation.

### A cached read may be answered by the service worker

When a JavaScript or TypeScript `HTTPClient` uses Tachyon's browser
transport, a `Cached` GET is also sent as a public read: credentials are
omitted (as for every `HTTPClient` request), and it is not marked
`no-store`. So the service worker may answer it. The worker keeps its
existing rules: it never keeps a request that has an `Authorization` header,
and it never keeps a response that is `private`, `no-store`, has a `Vary`
header, or is not a success. A successful write already removes the worker's
entries under its path. A read that is not `Cached` is still sent
`no-store`, so it is always fresh.

**Where cache-first and network-first live: in the manifest.**
`tac.cache` keeps the policy of each path (`cache-first`, `network-first`,
`no-store`). A path without a rule is not kept by the worker. `Cached` takes
no argument, as before, and as Spring's `@Cacheable` names no transport
policy. The reasons:

- One path has one policy, whichever service, page or navigation reads it.
  An annotation argument would let two clients give one URL two policies.
- The worker decides before the page runs, and only the manifest is known to
  the worker at build time.
- The WebAssembly languages have no browser transport, so a transport
  argument on `Cached` would mean nothing in any of them.

**Whether the HTTPClient IndexedDB cache folds in: yes, into Cached.** The
page keeps one data cache: the behavior cache that `Cached` uses. The
`cache` option of `@HTTPClient` and its IndexedDB store
(`tachyon-fetch-cache`) are removed. A client that wrote
`{"cache": "cache-first"}` writes `CacheSettings` on the class and `Cached`
on each read. A client that wanted offline reads adds a `tac.cache` rule for
the path. The service worker stays a separate layer on purpose. It keeps
network responses, and it works before a service runs: for navigations,
assets and other tabs. The two layers work like a Spring cache in front of
an HTTP cache.

### Two refusals

- `TY1360`: a Tac JavaScript or TypeScript service file reads `globalThis`.
  The message points to `@Needs windowControls` for the window and to the
  other browser APIs.
- `TY1361`: a Tac JavaScript or TypeScript service file calls `fetch`. The
  message points to an `HTTPClient` method, with `Cached` for a public read.

A Setup is not a service: its HTTP adapter is where a custom transport lives,
so a Setup may call `fetch`. A shared script (`tac.scripts`) is not a service
either. The WebAssembly languages already cannot reach either global
(`TY1317`). Both rules are build errors, like the safety rules of ADR 0068,
and were switched on only after the website, the fixtures, the corpus and the
`ty init` templates passed them. A later slice refuses the `cache` option of
`@HTTPClient`, with a message that names `CacheSettings` and `Cached`, and
removes its IndexedDB store.

## Plan

1. This ADR.
2. `Cached`, `CacheKey`, `ClearCache` and `UpdateCache` on `HTTPClient`
   methods in JavaScript and TypeScript, the public read through the
   worker, then every WebAssembly language (rule and lowering tests; the
   runtimes already combine a cache with an HTTP exchange).
3. `WindowControls` in JavaScript and TypeScript, then every WebAssembly
   language through the host bridge, with test-unit fakes.
4. The website: both feature pages with a tab for every Tac language, and
   the caching, `HTTPClient`, browser API, annotation, glossary and error
   code pages.
5. `TY1360` and `TY1361`, switched on after the website passes them. The
   shim drops `globalThis.tachyonWindow` in the same slice.
6. The `cache` option of `@HTTPClient` is refused, and its IndexedDB store
   (`FetchCache` in `tac-client.js`) and its storage-gate rows are removed.
7. A native `WindowControls` in the companion preludes (Rust, C#, Swift,
   Kotlin) that asks the host directly with the same grants.

## Consequences

- Every Tac language can control the native window and can cache a client
  read. A test unit can fake the window.
- A service has no way to reach the network or a global object except
  through a role that Tachyon injects. So a reviewer finds all network code
  in `client/clients` and `client/setups`.
- Projects that used `@HTTPClient({"cache": ...})` or
  `globalThis.tachyonWindow` must change when slice 5 lands. The build
  message says what to write instead.
- `template/tac-web.js` grows by the window controls, so its size budget
  (ADR 0065) grows from 8,000 to 9,500 bytes.
- A `Cached` read can be kept twice: in the page's behavior cache and in the
  worker. The page cache answers first. The worker answers only when the
  page cache has no live answer, for example after a reload while offline.
