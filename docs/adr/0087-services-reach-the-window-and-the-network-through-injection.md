# ADR 0087: Services Reach the Window and the Network Through Injection

- Status: Accepted (slices 1 to 6 are done; the native window controls of
  companions are open)
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

After `ClearCache`, `UpdateCache` or an explicit behavior-cache eviction, the
next uncached GET revalidates the browser HTTP cache with `cache: 'no-cache'`.
The service worker treats that request as network-first even when its path
uses cache-first. Other keys keep their normal cache policy. A failed read
keeps the revalidation requirement for the next attempt. This applies to the
JavaScript and TypeScript browser transport; custom Setup HTTP adapters
receive the `revalidate` command flag and own their transport behavior.

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
7. Native `WindowControls` follows the activation-owned provider seam in
   ADR 0089. Compiled Swift and Rust/macOS providers ask the host directly with
   the same grants. Rust's actual owned-window title/resize check passes; the
   host uses its AppDelegate-owned window rather than a global key window.
   C# and Kotlin host registration remains in the parallel native workstream.

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

## Amendment (2026-10-03): slice 6 landed, and `cacheManager` went with it

The `cache` option of `@HTTPClient` is refused by name; the message names
`CacheSettings`, `Cached` and `tac.cache`. `FetchCache` and its IndexedDB
database (`tachyon-fetch-cache`) are gone from `tac-client.js`, and an
`HTTPClient` read without `Cached` is always sent `no-store`. The
`cacheManager` resource only cleared that database, so it is removed too: a
`@Needs cacheManager` field is refused with a message that points to
`ClearCache` and `UpdateCache`, like Spring's `@CacheEvict` and `@CachePut`.
`ty migrate fix` drops the `cache` option when it turns an old `@Client` into
`@HTTPClient`, and notes each client whose reads should be marked `Cached`.

## Amendment (2026-10-18): one `Vary` value is public

The worker, and the page's precache of service modules, now keep a response
whose only `Vary` value is `Accept-Encoding`. `ty start` and
`ty preview --static` send that value on every precompressed file
(ADR 0075). The browser unpacks the body before the worker reads it, and the
worker checks a package file against its digest, so the value says nothing
private. Any other `Vary` value, `Vary: *` included, still keeps a response
out of both caches. The same servers now let the worker keep the files that
its asset table lists (ADR 0093 amendment).

## Amendment (2026-10-18): window controls tell the truth

The page runtime no longer drops the host's answer to a window control.
`TacWebApis.windowControls` still checks the grant and the size at once and
fails the call there. A call the host refuses after it started (an answer
with an `error` member or `"ok": false`, or a rejected host call) now reaches
the calling service as `BrowserCallFailed { api: "windowControls", call,
message }`, where `call` is the method name (`setTitle` for `title`) and the
message is the host's answer, for example `denied`. This is the same rule
ADR 0065 set for the other browser APIs: synchronous failures are errors of
the call, and failures after the call started are events. The page runtime's
size budget (`tac-web.js`) moves from 9,500 to 10,000 bytes for this.

The Linux page bridge now sends every `window.*` call from the page to the
same GTK controls that the Rust C ABI adapter uses
(`linux_window_controls.c`), with the same manifest grants. Before this,
it answered only `companion.invoke`, so JavaScript, TypeScript and
WebAssembly callers on Linux got no effect and no error. The Linux Xvfb
test (`linux_window_control_tests`) now starts with a JavaScript caller that
runs the real page runtime: it sets the title and resizes the window to
700x500 through the bridge, and when it calls `close` without the host's
grant, it hears `BrowserCallFailed` with the message `denied` and the window
stays open. It passed on 2026-10-18 under Xvfb with xfwm4 on Ubuntu 24.04
(x86_64, GTK 4.14.5, WebKitGTK 2.52.6).

An iOS app has no window controls: the system owns the window. The iOS host
now gives the page only the `title` grant, answers `window.granted`, and maps
`window.title` to the scene title (`UIWindowScene.title`, shown in the iPad
window switcher). `granted` answers `false` for every other control. The per-target support
table (ADR 0102) now marks the controls adapted on iOS with that note, which
an iOS build that declares `tac.application.window.controls` prints, and
honored on Linux. A Swift service
that the iOS app links in still has no window adapter there, so its
`granted` answers `false` and its calls throw. The iOS host change is
checked on the generated source only; the simulator gate runs on a Mac.

## Amendment (2026-10-19): native pages keep Yon reads themselves

A native build unregisters the service worker, because its bundle is local,
and a native page sends its Yon calls to `tac.yon.origin` (ADR 0103). A
worker never sees a request to another origin, so the `tac.cache` rules did
nothing in a native app.

A native build that declares both `tac.yon.origin` and `tac.cache` rules now
ships `/.tachyon/tac-yon-reads.js`, loaded before the page runtime. It holds
the same compiled rules as `tachyon-sw.js` and applies them to Yon reads
with the page's own Cache Storage (`tachyon-yon-reads`). Both page
transports use it: Tac Client exchanges (`client_runtime.js`) and the
`TacBrowserHttp` endpoint of the WebAssembly languages. It decides as the
worker does:

- It looks only at requests to the declared Yon origin. Page-origin
  requests go to the local bundle as before.
- A GET is kept only when a rule matches its path, the policy is not
  `no-store`, the request is a public read (credentials omitted, not
  `no-store`, no `Authorization` or `Range` header), and the answer is a
  success that is not `private` or `no-store`.
- `cache-first` answers from the cache when it can; `network-first` and a
  revalidating read (`cache: 'no-cache'`, after `ClearCache` or
  `UpdateCache`) go to the network and fall back to the kept read when the
  network fails.
- A successful write removes the kept reads at its path and above it.
- The limits match the worker: 256 entries, 32 MiB, 256 KiB per read and 24
  hours per entry.

One difference: the answer may vary by `Origin` as well as
`Accept-Encoding`. Yon adds `Vary: Origin` to every answer when
`server.origins` is set, and the page always sends the same app origin.

Yon still answers every route `no-store` unless the handler sets its own
`Cache-Control`, as a JavaScript handler does by returning
`Response.json(value, { headers: { 'cache-control': 'public, no-cache' } })`.
So a rule keeps only the reads that Yon marks public, on the web and in a
native app alike.

A web build does not ship the file; its worker keeps doing this work. When a
web build calls a Yon on another origin, the worker still does not see those
reads.

The per-target support table (ADR 0102) marks `tac.cache` honored on macOS,
Windows, iOS and Android, and adapted on Linux, where the web view session
is not kept after the app quits (ST1). `compiler::yon_reads_tests` checks the
rules, the offline answer and the write eviction in Node, and
`scripts/yon-reads-browser-test.mjs` checks them in Chromium, Firefox and
WebKit against a real `ty serve` on another origin that is then stopped.
The airplane-mode runs on the iOS simulator and the Android emulator run on
a Mac.

## Amendment (2026-10-19): kept Yon reads on Linux, iOS and Android

The amendment above says that the per-target support table marks
`tac.cache` honored on iOS and Android and adapted on Linux. The table said
that before ST1; it is wrong now. ST1 (`499a6246`) gives the Linux web view a
persistent WebKit session that keeps web storage on disk, in
`$XDG_DATA_HOME/<application id>/webkit` (ADR 0059 amendment of
2026-10-18). So on Linux the kept Yon reads stay after a relaunch, as on
macOS. (Windows keeps its WebView2 data in a folder too, but its relaunch
check has not run.)

The per-target support table now says this about `tac.cache`:

- macOS, Windows and Linux: no relaunch limit, because the web view keeps
  its storage on disk. An offline Yon read in each packaged app is still
  owed;
- adapted on iOS: the kept reads last only while the app runs, because
  WebKit does not keep the Cache Storage entries of the app's page across a
  relaunch. This limit is not proven yet. The iOS relaunch run used a cache
  entry that Tac's start-up sweep deletes, so the limit waits on a rerun on
  the simulator (plan item RM2);
- adapted on Android: a read kept shortly before the system stops the app
  is not always there after a relaunch.

## Amendment (2026-10-19): web pages keep cross-origin Yon reads too

The earlier amendment left one gap: a web build that sets `tac.yon.origin`
sends its Yon reads to another origin, and its service worker skips every
request to another origin. So the `tac.cache` rules did nothing for those
reads on the web either.

A web build that declares `tac.yon.origin` and `tac.cache` rules now ships
`/.tachyon/tac-yon-reads.js` as a native build does, with the same rules and
the same Cache Storage (`tachyon-yon-reads`). The page script does the work,
not the worker:

- The worker does not control the page on the first visit, and it is not
  registered on a loopback host or inside a native app. The page script
  keeps a read from the first one, on every host.
- One file and one set of rules serve the web and the native targets, so
  the two cannot drift apart.
- The worker keeps its rule of leaving other origins alone. The page
  script's own network request goes past it, so a read is never kept twice.

Same-origin reads, navigations and assets stay with the worker. A web build
without `tac.yon.origin` ships no page script, as before.

The first real page run found a hole in the earlier amendment. A JavaScript
or TypeScript service's `HTTPClient` sends its request through the page
transport in `tac-client.js`, and that transport called `fetch` directly,
so those reads went around the page script in native apps too. The
transport now sends through the page script when it is there. The earlier
browser test called the exchange without that transport, so it did not see
the hole.

`compiler::yon_reads_tests` checks that a web build ships the file next to
the worker registration. `scripts/yon-reads-web-browser-test.mjs` bundles a
web app, serves its pages with `ty preview --static` on one port and Yon
with `ty serve` on another, makes a `Cached` read, takes the browser
offline while Yon still runs, and checks in Chromium, Firefox and WebKit
that the public read answers from the kept copy while a `no-store` read
fails.
