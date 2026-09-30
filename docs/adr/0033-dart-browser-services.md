# ADR 0033: Dart Browser Services Compile to WebAssembly

- Status: Accepted
- Date: 2026-09-23
- Amended: 2026-09-28 (the program compiles to WasmGC; see the amendment)
- Amends: [ADR 0030](0030-components-use-services.md) (the browser service
  languages), [ADR 0019](0019-companions-compile-for-their-target.md) ("Dart is
  not a companion language")
- Preserves: client-only Tac, one browser module per service, the service
  lifecycle and event contracts of ADR 0028 and ADR 0031

## Context

Dart became a Yon language with the full set of annotations. Most Dart code,
however, is written for the browser. ADR 0030 left the browser to JavaScript
and TypeScript.

Dart once used a WebAssembly companion ABI (application binary interface).
ADR 0019 retired that ABI, because it asked every language to target a
sandbox. Dart is different from the native languages: it has its own official
compiler to JavaScript, `dart compile js`, which ships with the SDK.

## Decision

A browser service may be written in Dart.

### Source and build

- `client/services/<Name>Service.dart` declares `class <Name>Service`. A
  service has one browser language: js, ts or dart.
- The build combines the service with its role files (ADR 0024). It lowers the
  service's lifecycle, event and behavior markers as it does for any Dart
  source. It then compiles the program with `dart compile js -O2`. It caches
  the result under the Tachyon cache directory, keyed by the SDK version and
  the program digest.
- Every Dart service on a page compiles into one program, with one Dart
  library per service, so the Dart runtime ships once. Each service still has
  its own module, `.tachyon/services/<Name>Service.js`, which imports the
  shared `tachyon-dart.js`.

### The bridge

- Compiled Dart has no reflection. So the build reads the service's public
  members from the source and generates a bridge.
- The published `.tachyon/services/<Name>Service.js` wraps the compiled
  program. Its default export is a JavaScript class with the service's name.
- Each JavaScript instance creates one Dart instance. Its fields read and
  write through the bridge synchronously. Its methods call through the bridge
  and return a Promise for a `Future`. The runtime needs no Dart-specific
  path.
- Only JSON values cross the bridge:
  - `int`, `double`, `num`, `String`, `bool`, `Object?`/`dynamic`;
  - `List` and `Map<String, …>` of those;
  - a `Future` of one of those.

  A public field, getter or method of any other type stays inside Dart. A view
  can only read `final` fields and getters.
- Bridged methods may take optional positional and named parameters. An
  omitted argument takes the default written in the Dart source. Named
  arguments arrive as one object after the positional ones; a JavaScript
  caller passes this object. In every language, a view's event binding passes
  positional literals only.

### Collaborators, other services and events

- `@Autowired` collaborators in the service's unit are constructed once per
  service instance.
- An injected `ApplicationEventPublisher` publishes through the runtime. So a
  Dart service's events reach JavaScript listeners, and JavaScript events
  reach Dart listeners. `@PostConstruct`, `@PreDestroy`, `@EventListener`,
  `@Order` and `@Async` work through the event plan that the module exports.
- A service reaches another service through a `late` field named after that
  service's instance. JavaScript services follow the same rule.
  - A field typed with another Dart service receives the exact Dart instance
    that the page uses.
  - A field typed `TacService` receives a proxy for any service. This is how a
    Dart service reaches a JavaScript or TypeScript service. The proxy offers
    `get`, `set` and `invoke`; `invoke` returns a Future.

### Persisted fields

- A persisted field is `$name` in session storage or `$$name` in local
  storage. The bridge restores it before `@PostConstruct` runs. The bridge
  stores it when a Dart method changes it.
- A field that no call changes is never written. So a new default value still
  reaches a returning visitor.
- A persisted field must be assignable.

### Tests

- A Tac test unit may be written in Dart. It gets the same composition and
  lowering, and it runs on the Dart VM. A service is plain Dart except for its
  bridge, and the unit does not need the bridge.
- A unit uses another Dart service directly. It replaces a JavaScript service
  with `TacService(fields, methods)`.

## Consequences

- A project with a Dart service needs the Dart SDK to build. `ty doctor`
  reports it.
- The shared program is about 77 KB before compression (WebAssembly, see the
  amendment; it was about 90 KB as JavaScript). Each service module adds
  about 3.5 KB.
- A change to any Dart service recompiles the one program. The result is
  cached by SDK version and program digest.
- Every field read copies its value across the bridge. So when a view renders
  a large list from a Dart service, it copies the list on each render.
- Native hosts load the same web bundle, so a Dart service runs in their web
  views. This was checked in the macOS host. Dart stays a browser language: it
  has no native target of its own (ADR 0019).

## Amendment (2026-09-28): the program compiles to WebAssembly

The build now compiles the shared Dart program with `dart compile wasm -O2`
instead of `dart compile js -O2`. The output is WasmGC, WebAssembly with
garbage collection. The bridge, the per-service modules and every contract
above stay the same.

### What the build publishes

- `.tachyon/services/tachyon-dart.wasm`: the compiled program.
- `.tachyon/services/tachyon-dart-loader.js`: the loader that the Dart SDK
  writes beside it.
- `.tachyon/services/tachyon-dart.js`: the shared module that every
  `<Name>Service.js` still imports. It checks the browser, mirrors storage,
  fetches the program, compiles it and runs `main`.
- The cache moves to `dart-wasm/<key>/` under the Tachyon cache directory. The
  key is still the SDK version and the program digest.
- The Dart SDK must be 3.13 or newer (`ty doctor` says so). The build refuses
  with `TY1009` when `dart compile wasm` fails.

### Two bridge differences, closed in the bridge

- **Numbers.** In JavaScript, `dartify()` gives an `int` for a whole number.
  In WebAssembly it always gives a `double`. So the bridge turns every whole
  number that arrives from JavaScript (up to 2^53 - 1) back into an `int`,
  also inside lists and maps. A Dart service sees the same values as before:
  `'$value'` of `3` is `3`, not `3.0`.
- **Errors.** A Dart exception that leaves WebAssembly reaches JavaScript as
  an opaque `WebAssembly.Exception` with no message. So the bridge catches it
  and throws a JavaScript `Error` whose message is the Dart error's text, for
  example `Bad state: no refunds`. This is what the JavaScript build showed.
  A failed `Future` still rejects the way `dart:js_interop` rejects it.

### Browsers: no JavaScript fallback

Tachyon ships no `dart compile js` fallback. WasmGC is in every current
engine: Chrome and Edge 119, Firefox 120 and Safari 18.2 and newer, and the
web views that use those engines. A fallback would double the build and
keep a second code path alive for a shrinking set of old browsers.

The shared module runs the SDK's own feature check (`program.support.js`,
WasmGC and SIMD) first. In an older browser it throws one clear error:
"Dart services need a browser with WebAssembly garbage collection: Chrome or
Edge 119, Firefox 120 or Safari 18.2 and newer." Native hosts need a web view
of the same age, for example macOS 15.2 or iOS 18.2 for WKWebView.

### Measured on the website (SidebarService and PagerService)

| | JavaScript build | WebAssembly build |
| --- | --- | --- |
| Bytes | 101,054 | 76,831 (62,242 wasm + 13,617 loader + 972 module) |
| gzip | 31,007 | 28,842 |
| Brotli | 26,891 | 25,061 |
| Start in Chromium 151 (import of `SidebarService.js`, median of 7) | 6.3 ms | 7.7 ms |
| Start in Firefox 153 | 9 ms | 15 ms |
| Start in WebKit 26.5 | 7 ms | 8 ms |
| `/docs/annotations` ready in Chromium (median of 5) | 321 ms | 322 ms |

Start time is the time to fetch, compile and instantiate the program and run
`main`, served from localhost. The page renders the same in Chromium, Firefox
and WebKit, and following the pager works in all three. The WebAssembly
program is smaller on the wire and starts a few milliseconds later.

### Unchanged

- Test units still run on the Dart VM, without the bridge.
- `@LocalState` and `@SessionState`, the storage host, events and
  `@OnEvent`, `TacService` proxies and the curated Web APIs (ADR 0065) use
  the same bridge calls as before.
- Native hosts already serve `.wasm` and allow `'wasm-unsafe-eval'`. The
  shared module reads the program with `fetch`, so it does not depend on the
  `application/wasm` type.

## Amendment (2026-09-30): models reach the view

- A field, a getter, a method argument or a method result may be a model
  from `client/models`, a `List` of models, or a model that holds other
  models. The bridge sends a field through the same `__yon_json` converter
  as a method result, so the view reads plain objects and lists, never an
  opaque Dart object. A model value from the view is read back with the
  model's reader (`__yon_read_<Model>`).
