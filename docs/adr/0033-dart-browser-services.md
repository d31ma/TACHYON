# ADR 0033: Dart Browser Services Compile to WebAssembly

- Status: Accepted
- Date: 2026-09-23
- Amended: 2026-09-28 (the program compiles to WasmGC; see the amendment);
  2026-10-18 (native apps raise their minimum system for WasmGC services)
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

## Amendment (2026-10-18): native apps raise their minimum system for WasmGC services

The 2026-09-28 amendment said that native hosts need a web view as new as
the browsers that run WasmGC. Nothing checked it: the iOS app declared iOS
17.0 and the macOS app macOS 13.0, so an app with a Dart, Kotlin or Java
web service installed on a system whose web view cannot run it.

### The member

`tac.application.minimumSystem` in `client/manifest.json` names the oldest
system each native app runs on:

```json
{ "tac": { "application": { "minimumSystem": {
  "ios": "18.2", "macos": "15.2", "android": 28,
  "windows": "119.0", "linux": "2.46"
} } } }
```

- `ios` and `macos` are system versions. `android` is an API level. A
  desktop web view is part of the system on macOS only, so `windows` names
  the WebView2 Runtime version and `linux` the WebKitGTK version.
- Every key is optional. A malformed value or an unknown key stops every
  build, web builds included (`TY1601`).

### The floor

Each host has its own floor, the oldest system its host code supports:

| Target | Host floor | Floor with a WasmGC service | Applied as |
| --- | --- | --- | --- |
| iOS | 17.0 | 18.2 | `MinimumOSVersion` and the `swiftc` target |
| macOS | 13.0 | 15.2 | `LSMinimumSystemVersion` and the `swiftc` target |
| Android | API level 26 | API level 26 (see below) | `minSdk` |
| Windows | WebView2 86.0 | WebView2 119.0 | a check when the app starts |
| Linux | WebKitGTK 2.40 | WebKitGTK 2.46 | a check when the app starts |

- A service needs WasmGC on a target when it runs in the page as Dart,
  Kotlin or Java there. Kotlin linked into the Android app does not count.
- Without the member, the build uses the floor in the third column when the
  app bundles such a service, and prints one note that names the languages.
- A declared value below the host floor, or below the WasmGC floor when such
  a service is bundled, stops the build with `TY1601`. The message names the
  languages and the version to raise it to. A declared value at or above both
  floors is used as written, with no note.
- WebKitGTK turned WasmGC on by default in 2.46 (`useWasmGC` defaults to
  true in JavaScriptCore's `OptionsList.h` at the `webkitgtk-2.46.0` tag; it
  was `false` in 2.44). Chromium, and so WebView2, shipped it in 119.
- Android System WebView is updated by Google Play apart from Android
  itself, so no API level guarantees it. The Android app keeps its API level,
  and the build prints a note that names Android System WebView 119. An older
  web view shows the service's own "needs a browser with WebAssembly garbage
  collection" error.

### The startup check

The Windows and Linux apps read the installed web view before they open a
page. Too old or missing, they show one readable message instead of a blank
window, record `engine.unavailable`, and start no application service:

- Windows calls `GetAvailableCoreWebView2BrowserVersionString` and
  `CompareBrowserVersions`, then shows a message box that names the runtime,
  the floor and the download page.
- Linux compares `webkit_get_major_version()` and
  `webkit_get_minor_version()` with the floor, then shows a window with the
  message and prints it to standard error.

The macOS and iOS systems refuse to open an app below its declared minimum,
and Android refuses to install one below its `minSdk`, so those hosts need no
check of their own.

## Amendment (2026-10-25): host floors after ADR 0108

This amendment supersedes two "Host floor" cells of the floor table in the
2026-10-18 amendment above, which stays as the history of this decision.
ADR 0108 raised the iOS host floor from 17.0 to 17.4 and the Windows host
floor from the WebView2 Runtime 86.0 to 118.0, the first versions that run
CSS `@scope`, which component styles compile to. The table now reads:

| Target | Host floor | Floor with a WasmGC service |
| --- | --- | --- |
| iOS | 17.4 | 18.2 |
| macOS | 13.0 (a note names Safari 17.4 for component styles) | 15.2 |
| Android | API level 26 (a note names Android System WebView 118 for component styles) | API level 26 |
| Windows | WebView2 118.0 | WebView2 119.0 |
| Linux | WebKitGTK 2.40 (a note names WebKitGTK 2.44 for component styles since GP1, ADR 0108) | WebKitGTK 2.46 |

A declared `tac.application.minimumSystem.ios` below 17.4, or
`tac.application.minimumSystem.windows` below 118.0, stops the build with
`TY1601`.

## Amendment (2026-10-25): Java needs a later floor

This amendment supersedes the Java part of the "Floor with a WasmGC service"
column above. TeaVM writes a Java `catch` as the WebAssembly `try_table`
instruction, which needs exception references (`exnref`) as well as WasmGC.
The reasons and the measurements are in the
[ADR 0083 amendment of 2026-10-25](0083-java-services-reach-the-web-as-webassembly.md#amendment-2026-10-25-java-services-also-need-webassembly-exception-references).
Dart and Kotlin keep the column above. (Dart moved later the same day; see
the next amendment.)

| Target | Floor with a Java web service |
| --- | --- |
| iOS | 18.4 |
| macOS | 15.4 |
| Android | API level 26 (a note names Android System WebView 137) |
| Windows | WebView2 137.0 |
| Linux | WebKitGTK 2.48 |

## Amendment (2026-10-25): Dart needs the JavaScript exception tag

This amendment supersedes the Dart part of the "Floor with a WasmGC service"
column and the browser floor at the top of this record ("Chrome and Edge 119,
Firefox 120 and Safari 18.2"). Kotlin keeps that column.

The loader that `dart compile wasm` writes (Dart SDK 3.13) always imports
`WebAssembly.JSTag`, the tag for exceptions thrown by JavaScript, and
`dart compile wasm` has no option to leave it out. An engine without
`JSTag` compiles the module but cannot link it. The MA5 run found it: on the
iOS 18.2 simulator (22C150) a Dart app built at 18.2 installed and showed a
blank page, with `LinkError: Tag import WebAssembly:JSTag is not an instance
of WebAssembly.Tag`. Kotlin and Go apps on the same runtime passed.

Measured in an x86_64 cloud container with a Dart module that throws and
catches (`docs/SUPPORT_TIERS.md`, "Dart web services need the JavaScript
exception tag"): Chrome 119, 120, 136 and 137 have `JSTag` and ran it;
Firefox 120, 128 and 130 have none and refused it at link, Firefox 131 ran
it; WebKitGTK JavaScriptCore 2.46.6 has none, 2.48.1 has it. Safari follows
WebKit's exception references work in 18.4, as the iOS 18.2 run shows.

The generated `tachyon-dart` module checks for `JSTag` as well as WasmGC
before it loads the program, and names the versions below in its message.

| Where | Floor with a Dart web service |
| --- | --- |
| Browsers | Chrome or Edge 119, Firefox 131, Safari 18.4 |
| iOS | 18.4 |
| macOS | 15.4 |
| Android | API level 26 (a note names Android System WebView 119) |
| Windows | WebView2 119.0 |
| Linux | WebKitGTK 2.48 |

A declared floor below these stops the build with `TY1601`, as before.
