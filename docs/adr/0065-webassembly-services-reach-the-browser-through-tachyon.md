# ADR 0065: WebAssembly Services Reach the Browser Through Tachyon

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md) and
  [ADR 0045](0045-go-is-a-service-language.md) (the page speaks the companion
  protocol), [ADR 0054](0054-ruby-is-a-yon-language-and-a-tac-service-language.md)
  (Ruby runs on mruby), [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md)
  (the synchronous storage host) and [ADR 0063](0063-services-and-controllers-are-cohesive.md)
  (cohesive services)

## Context

A Tac service in Rust, Go, C#, Dart or Ruby runs in the page: Rust, Go and
C# as WebAssembly, Dart as compiled JavaScript and Ruby on mruby
WebAssembly. The guides never said which browser features these services can
use. Each language also has its own way to call the browser directly: Go
`syscall/js`, Dart `dart:js_interop` and `package:web`, C# `[JSImport]`,
Rust `wasm-bindgen`, and the mruby host import. Such code breaks the native
test units of the language (a Dart `dart:js_interop` import breaks every Dart
unit), and a native target has no browser at all.

## Decision

1. A WebAssembly, Dart or Ruby Tac service reaches the browser only through
   Tachyon: the view binds its fields and methods, events carry values in and
   out, and the storage annotations reach the storage host of ADR 0059.
   Browser work lives in a JavaScript or TypeScript service, which publishes a
   typed event that the other service receives with `@OnEvent`.
2. The guide "What each service language can use in the browser" (topic
   "Rust, Go and C# in the browser") lists what each language can use, and
   only what a test checks:
   - Fields and methods, `publishEvent` and `@OnEvent`, `@Store` and caches,
     `@LocalState` and `@SessionState`: all five languages.
   - `@StreamRequest` and `@SocketRequest`: JavaScript and TypeScript only.
     Their messages reach the other languages as typed events.
   - `@HTTPClient` (amended 2026-10-04): JavaScript, TypeScript, Dart, Rust,
     Go, C#, F# and C++ (ADR 0099) reach the browser HTTP host directly
     (ADR 0090). Swift,
     Kotlin, Java and Ruby send requests through a Setup's synchronous HTTP
     adapter (ADR 0087). Python and Lua are refused (TY1308).
   - The standard library in the page (`tac_web::reach_tests`, run in node
     with the real toolchains): Go, C#, Dart and Ruby have the current time,
     random numbers and console output. Go `time.Sleep` and C#
     `Thread.Sleep` work but hold the page; Dart waits in an `async` method;
     Ruby refuses `sleep` with a message. Rust on `wasm32-unknown-unknown` has
     none of these: `SystemTime::now()` and `thread::sleep` trap, and
     `println!` prints nothing.
3. Tachyon refuses direct browser interop in Tac code at build time with
   `TY1317`: "<Language> services cannot call browser APIs. Put this code in a
   JS or TS service and send the result to this service with an event."
   - The check covers every Rust, Go, C#, Dart and Ruby file under `client/`
     that project discovery reads, so `ty build`, `ty preview`, `ty serve`
     and `ty bundle` stop before any toolchain runs.
   - Refused: Go imports of `syscall/js`; Dart imports of `dart:js_interop`,
     `dart:js_interop_unsafe`, `dart:js`, `dart:js_util`, `dart:html`,
     `package:web/` and `package:js/`; C# `InteropServices.JavaScript`,
     `JSImport`, `JSExport` and `JSHost`; Rust `wasm_bindgen`, `web_sys`,
     `js_sys` and `wasm_import_module`; Ruby `TachyonWeb` and `JS` (as `JS.`
     or `JS::`).
   - Comments and strings do not count, and a longer name that only contains
     a refused word does not count. A plain Rust `extern "C"` block stays
     allowed, because a natively linked Rust service uses it for its platform.
   - Tachyon's own preludes, the generated Dart `main` and the page runtime
     are not user files, so the check never sees them.
   - There is no opt-out: the refused code cannot work in a test unit or on a
     native target.

4. Tachyon gives Tac services a small curated set of browser APIs, written
   once in the page runtime (`template/tac-web.js`; see item 5 for its size) and
   reached through the existing bridge by a thin typed client in each
   language. The first group is `Clock` (`now`, `monotonic`, `timeZone`) and
   `SecureRandom` (`bytes` of 0 to 65,536, and a version 4 `uuid` built from
   `crypto.getRandomValues`, so it also works on an insecure origin).
   - JavaScript and TypeScript get them as page resources: `@Needs clock`
     and `@Needs secureRandom` (typed in `tachyon-behaviors.d.ts`).
   - Rust: `Clock` and `SecureRandom` prelude types (plain fields that
     `Default` fills, like `ApplicationEventPublisher`; a `#[Needs]` marker
     would make the struct list every field). The time comes from the `tac_host_epoch_ms` and
     `tac_host_monotonic_ms` imports; other calls go through a new
     `tac_host_web` import. Fallible calls return `Result`.
   - Go (`(value, error)`), C# (exceptions), Dart (`@Needs()` fields built as
     `const Clock()`; `StateError`) and Ruby (`Clock.new`; `RuntimeError`)
     reach the same page host: Go through a third `__tachyonGo` argument, C#
     through a `web` JSImport, Dart through `tachyon.dart`, and Ruby through
     its synchronous storage buffer protocol, which the page routes by the
     `api` member of the request.
   - The bridges do not show these fields to the view.
   - A natively linked service or a test unit has no page (see item 5 for
     the build refusal and the fakes).
   - Both need no permission, so the manifest grants nothing for them.
   - Every page call is synchronous, so a call made in a click handler stays
     inside the user gesture.

5. The other curated APIs use the same host and clients. The bridge is
   synchronous, so a result that the browser gives later, or a value that
   changes, reaches the service as a typed event, not as a future or a task.
   - Built-in event models: `ConnectivityChanged { online }`,
     `VisibilityChanged { visible }`, `PreferencesChanged { scheme, motion,
     language }`, `ClipboardTextRead { text }`, `ShareFinished {}`,
     `NotificationPermissionChanged { permission }`, `PositionFound {
     latitude, longitude, accuracy, timestamp }` and `BrowserCallFailed {
     api, call, message }`. One table (`route_types::WebEventModels`) is the
     only source: the listener model check adds it to the `client` models
     (never to the server), and `WebEventSources` writes the types for Rust,
     Go, C#, Dart and Ruby. A same-named user model is replaced by the
     built-in one.
   - `Connectivity`, `PageVisibility` and `Preferences` answer at once. The
     page publishes their current values when it starts, keeps them (so a
     `replay` listener hears them first) and publishes every change
     (`online`, `offline`, `visibilitychange`, `languagechange` and the
     color scheme and reduced motion media queries).
   - `Clipboard` (`writeText`, `readText`), `Share` (`open`),
     `Notifications` (`permission`, `requestPermission`, `show`) and
     `Geolocation` (`currentPosition`, 10 second timeout) need a grant:
     `tac.browserApis` in `client/manifest.json` (`BrowserApiGrants`, known
     names only, each once, `TY1601` otherwise) becomes
     `<meta name="tachyon-browser-apis">` in the page head, and the page
     refuses an API that is not listed (fail closed).
   - Clipboard calls, `Share.open` and `Notifications.requestPermission`
     need a user action: when `navigator.userActivation.isActive` is false
     the call fails at once. Chromium shows that a click through the Rust
     and Go bridges keeps the user action, so the page never has to run the
     call on its own.
   - An answer goes to the service that called. The page runtime records the
     caller while a call runs (`TacWebApis.calling`): the event runtime
     wraps every resource call of a JavaScript owner, the WebAssembly module
     wraps every call of a Rust, Go, C# or Ruby owner, and the Dart module
     wraps every Dart method. `TacApplicationEvents.send` then delivers only
     to that owner's listeners. A call without a known caller (after an
     `await` in Dart) sends its answer to every listener of the model.
   - Synchronous failures (no grant, no user action, notifications not
     allowed) are errors of the call. Failures after the call started are
     `BrowserCallFailed` events.
   - `template/tac-web.js` holds all nine APIs within 8,000 bytes (the group 1
     budget of 3,000 bytes covered two APIs); a test keeps it there.
   - A service that is linked into a native app has no page. The build
     refuses the nine API types (Rust and C#, whose preludes hold them) and
     the eight built-in event models (Rust, C#, Swift and Kotlin) in such a
     service with `TY1317` (`NativeWebApis`, run on the natively linked
     companions only). Swift and Kotlin keep their own `Clock`. Rust's
     native `Clock.now_ms` no longer applies, because the build refuses
     `Clock` there.
   - Test units have no page either. A JavaScript or TypeScript unit's
     `@Fake` field fills the service field of the same name through
     `construct` when that field is a page resource (declared, not a
     constructor argument). Go and Ruby units answer calls with
     `BrowserFakes` (`api`, `call`, value); a call without an answer fails
     with a message that names `BrowserFakes`. Rust, C# and Dart unit
     programs do not include the page clients yet, so services that use
     these APIs are tested in the browser for now.
   - JavaScript and TypeScript resources: `connectivity`, `pageVisibility`,
     `preferences`, `clipboard`, `share`, `notifications`, `geolocation`
     (`TacClipboard`, `TacShare`, `TacNotifications` and `TacGeolocation`
     in the TypeScript types, so the DOM's `Clipboard` and `Geolocation`
     keep their meaning).

## Consequences

- A service in any Tac language can use browser values, through a small
  JavaScript or TypeScript service and one typed event.
- A Rust service that needs the time or a random number gets it from an event
  for now.
- Evidence: `annotation_layout::browser_interop::tests` (every language,
  comments and strings, longer names, server code, and project discovery
  that failed before the check and passes after it) and
  `tac_web::reach_tests` (one test per language); `tac_web::web_api_tests`
  (the page runtime, its size budget and one service per language),
  `compiler::web_api_tests` (JavaScript resources) and
  `scripts/web-apis-browser-test.mjs` (JavaScript, Rust and Go in Chromium,
  with Playwright permissions, media emulation and offline mode).
- Evidence for item 5: `route_types::web_event_model_tests` (the listener
  check accepts the models in all seven client languages, failed before the
  change; the server does not; every generated language type has the same
  fields), `tac_web::web_api_tests` (the page runtime watches changes,
  checks grants and user actions and answers its caller; Rust, Go, C#, Dart
  and Ruby services read the values, hear the changes and hear the answers
  to their own calls while another owner does not), `compiler::web_api_tests`,
  `native::site_tests`, `native::native_web_api_tests` (the native refusal)
  and `client_tests::web_fake_tests` (JavaScript `@Fake`, failed before the
  change; Go and Ruby `BrowserFakes`).
- Note (2026-09-29, ADR 0082 amendment): Swift web services get the nine
  clients too, as Swift classes created in `private let` fields, with the
  built-in event models as `Codable` structs. On the web, Tachyon's `Clock`
  class hides the standard library's `Clock` protocol; write `Swift.Clock`
  for the protocol. Evidence: the three `a_swift_web_service_*` tests in
  `tac_web::web_api_tests` and the Swift row of
  `scripts/web-apis-browser-test.mjs`.

## Amendment (2026-10-18): `preferenceStrings` is a granted browser API

`PreferenceStrings` (`read`, `write`) came after this ADR. It also needs a
grant. The known names in `tac.browserApis` are now `clipboard`, `share`,
`notifications`, `geolocation` and `preferenceStrings`
(`BrowserApiGrants::KNOWN`). Without the grant, `read` answers `null` and
`write` answers `false`. It needs no user action.

## Amendment (2026-10-18): browser APIs go through a host bridge in native apps

A native app's page runs in the operating system's web view, not in a
browser. There, `navigator.clipboard`, `navigator.share`, `Notification` and
`navigator.geolocation` may be missing, may need a secure context that the
app's own scheme is not, or may need a permission prompt that only the host
can show. So the page runtime (`tac-web.js`) now sends these calls to the
host when the host says it can answer them.

1. **Which host answers what.** Every native host's shim sets
   `globalThis.__tachyonBrowserApiBridge`, a frozen list of the browser APIs
   that the host answers. The build fills it from one table,
   `BrowserApiBridge` (`native/browser_api_bridge.rs`). Today every list is
   empty; BA1 to BA4 of the platform plan add their APIs to it as each host
   implements them. A policy test refuses a listed API whose host source does
   not answer every one of its operations.
2. **Operations.** For a listed API, the page calls
   `__tachyonNativeHostCall("browserApi.<api>.<call>", payload)` with a JSON
   payload:

   | Operation | Payload | `value` in the answer |
   | --- | --- | --- |
   | `browserApi.clipboard.writeText` | `{ "text" }` | `null` |
   | `browserApi.clipboard.readText` | `{}` | the text |
   | `browserApi.share.open` | `{ "title", "text", "url" }` | `null` |
   | `browserApi.notifications.requestPermission` | `{}` | `"granted"`, `"denied"` or `"default"` |
   | `browserApi.notifications.show` | `{ "title", "body" }` | `null` |
   | `browserApi.geolocation.currentPosition` | `{ "timeout" }` in milliseconds | `{ "coords": { "latitude", "longitude", "accuracy" }, "timestamp" }` |

   The host answers `{"value": …}` or `{"error": "message"}`, as a JSON
   string or an object. An error becomes the same `BrowserCallFailed` event
   that a refused browser call gives on the web.
3. **The same results as the web.** Services see no difference: the same
   events (`ClipboardTextRead`, `ShareFinished`,
   `NotificationPermissionChanged`, `PositionFound`, `BrowserCallFailed`), the
   same permission words and the same refusals. The grant check
   (`tac.browserApis`) and the user-action check still run in the page
   before anything reaches the host. `notifications.permission()` stays
   synchronous: on a bridged host it answers the last word the host gave,
   starting from `globalThis.__tachyonNotificationPermission` when the host
   sets it at document start, and `"default"` otherwise.
4. **Fallback.** An API that the host does not list goes to the web view's
   own browser API, exactly as on the web, and fails the same way when the
   web view lacks it. The per-target support table says which hosts bridge
   which API.
5. **Reasons for asking.** `tac.application.permissions` maps an API to the
   text that the operating system shows when the app asks the user, such as
   `{"geolocation": "Shows stores near you", "notifications": "Order
   updates"}`. Only `geolocation` and `notifications` take a reason; each
   must be granted in `tac.browserApis` and be 1 to 200 characters of text
   without control characters. A native build for iOS, macOS or Android that
   grants `geolocation` without a reason, or an Android build that grants
   `notifications` without one (Android 13 and later ask), stops with
   `TY1601` and names the member. The iOS host writes the reason as
   `NSLocationWhenInUseUsageDescription`, the macOS host as
   `NSLocationUsageDescription` and `NSLocationWhenInUseUsageDescription`,
   and the Android host as the string resources
   `tachyon_permission_geolocation` and `tachyon_permission_notifications`
   for its permission rationale. Windows and Linux read no reason: each of
   those hosts decides how it asks when it bridges the API (BA3, BA4).

Evidence: `tac_web::web_api_tests` (the page runtime sends bridged APIs to
the host with their payloads, keeps the web events, turns a host error into
`BrowserCallFailed`, keeps the grant check, and falls back to the web view
for an API the host does not list), `native::browser_api_bridge` (the table
and the host-answers-every-operation policy), `native::browser_api_permissions`
(the refusals per target, the text rules and the escaped host values) and
`native::host` (the shim carries the list). Host runs belong to BA1 to BA4.
