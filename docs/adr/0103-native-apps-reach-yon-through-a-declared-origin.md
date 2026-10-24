# ADR 0103: Native Apps Reach Yon Through a Declared Origin

- Status: Accepted
- Date: 2026-10-18
- Amends: [ADR 0058](0058-streams-are-a-route-and-a-request.md) (a stream
  may open on the declared Yon origin over CORS)
- Amends: [ADR 0060](0060-sockets-are-a-route-and-a-request.md) (the socket
  `Origin` rule also accepts `server.origins`)
- Amends: [ADR 0094](0094-local-host-names-and-host-validation.md) (the
  `Host` check still runs first; local `http` Yon origins)
- Amends: [ADR 0096](0096-forwarding-headers-are-untrusted-at-the-edge.md)
  (CORS reads only the `Origin` header)
- Works with: [ADR 0068](0068-safety-rules-are-build-errors.md) (declared
  outgoing hosts and the page's `connect-src`),
  [ADR 0088](0088-remotes-are-the-canonical-outbound-role-folder.md)
  (`client/remotes`)

## Context

A native bundle does not load its pages from Yon. The native host serves the
pages from its own scheme:

| Target | Page origin |
| --- | --- |
| macOS, iOS | `tachyon-app://bundle` |
| Android | `https://appassets.tachyon.local` |
| Windows | `https://tachyon.local` |
| Linux | `tachyon://app` |

The page transports accepted only the page's own origin. So a route path
such as `/api/notes` went to the host scheme, where no Yon runs. Every Yon
call from a native app failed at run time, and nothing told the author why.

Samuel's decision D5 chose option (a) as the default: the app calls a Yon
server on a declared remote origin. It is the only model that iOS and
Android can have, because they cannot run a Yon server beside the app.

## Decision

### The manifest declares the Yon origin

`client/manifest.json` gets the member `tac.yon.origin`:

```json
{ "tac": { "yon": { "origin": "https://api.example.com" } } }
```

- The value is exactly an origin: `https://`, a lowercase host, and an
  optional port that is not the default. It has no path, no trailing slash
  and no user info.
- `http://` is allowed only for a local Yon: `localhost`, a name under
  `.localhost`, or a loopback IP address, such as `http://localhost:8080` or
  `http://127.0.0.1:8080`.
- Any other value stops the build with `TY1601`: "Manifest tac.yon.origin
  must be the https origin of the Yon server ...".
- `yon` is a framework member of `tac`. Services do not receive it as
  configuration.

### The page sends Yon calls to that origin

When the member is set, the build writes
`<meta name="tachyon-yon-origin" content="https://api.example.com">` into
the head of every page. Three page transports read it:

- Tac Client exchanges: JavaScript and TypeScript `@HTTPClient` methods.
- The `TacBrowserHttp` endpoint, which the HTTP clients of every compiled
  web language use (Go, Kotlin, Swift, C#, Dart, Java, Rust and the others).
- The event bus, which opens `@StreamRequest` EventSources, topic streams
  and `@SocketRequest` WebSockets.

With the origin declared, a route path such as `/api/notes` resolves against
the Yon origin instead of the page. A socket uses `wss://`, or `ws://` for a
local `http` origin.

The transports accept exactly two origins: the page's own origin and the
declared Yon origin. Any other origin is refused as before, with
`HTTP_ORIGIN` and "Tac Client requires a same-origin HTTP endpoint or the
tac.yon.origin endpoint."

Requests to the Yon origin use CORS mode. They keep `credentials: 'omit'`,
and a `Cookie` header is still refused. Identity travels in the
`Authorization` header, which works across origins.

Without the member, nothing changes: the transports reach only the page's
own origin.

### The web page allows the Yon origin

`ty serve`, `ty start` and `ty preview` serve the page with a content
security policy. Its `connect-src` now also lists the declared Yon origin
and its WebSocket form, such as `https://api.example.com
wss://api.example.com`, next to the hosts of `tac.outgoing.hosts`. So a web
build can also call a Yon on another origin. Every native host's
`connect-src`, which lists `tac.outgoing.hosts` since SEC1, lists the
declared origin the same way. The Android host also lets requests to an
`https` Yon origin's host through its request interception and asks for
the `INTERNET` permission when an origin is declared.

### Yon lists the app origins it answers

`server/application.json` gets the member `server.origins`
(`yon.server.origins`). The plan named it `appOrigins`, but keys in this file
are lowercase letters and digits, so it is `origins`.

```json
{
  "server": {
    "origins": [
      "tachyon-app://bundle",
      "https://appassets.tachyon.local",
      "https://tachyon.local",
      "tachyon://app",
      "https://app.example.com"
    ]
  }
}
```

- Each entry is written exactly as `scheme://host[:port]`, with no path.
- `*`, `null`, a path, a trailing slash, user info and an uppercase host are
  refused with `TY2010`: "yon.server.origins must be a list of app origins
  written as a scheme and host with no path ...".
- `yon.server` stays a closed set. It now holds `host`, `port`, `hosts`,
  `origins`, `proxies`, `environment` and `maxbody`.

For a request whose `Origin` header is in the list, Yon answers CORS:

- A preflight (an `OPTIONS` request with `Access-Control-Request-Method`)
  gets `204` before any route runs, with:
  - `Access-Control-Allow-Origin: <that origin>`;
  - `Access-Control-Allow-Methods: GET, HEAD, POST, PUT, PATCH, DELETE`;
  - `Access-Control-Allow-Headers` with the requested header names, when
    every name is a valid header name and there are at most 64 of them;
  - `Access-Control-Max-Age: 600`.
- Every other response to a listed origin gets
  `Access-Control-Allow-Origin: <that origin>`.
- No response has `Access-Control-Allow-Credentials`, because Tac never
  sends cookies.
- When the list is set, every response carries `Vary: Origin`.

An origin that is not listed gets no CORS headers, and its preflight goes to
the routes as before. The `Host` check (`TY1366`, ADR 0094) still runs
first.

Streams and topic streams work over CORS, because an EventSource opens
without credentials.

The WebSocket upgrade check of ADR 0060 (the `Origin` must equal the
`Host`, and a request with no `Origin` is allowed) now also accepts an
`Origin` listed in `server.origins`. Any other origin gets `403` with "This
socket only accepts pages from its own origin or an origin listed in
server.origins in server/application.json."

`ty init` writes `server.origins` with the fixed app origins of the native
targets it sets up (the table in Context). `ty add target <native>` adds that
target's origin to an existing `server/application.json` and prints a note.
The note also tells the author to set `tac.yon.origin`.

### Native builds say when Yon is out of reach

`ty bundle --target macos|ios|android|windows|linux` checks the project when
`tac.yon.origin` is not set:

1. When the project has Tac remotes (any file in `client/remotes/`, or in the
   legacy `client/clients/`), the build stops with `TY1601`: "The <target>
   app calls Yon through client/remotes, but a native app has no Yon on its
   own origin. Add tac.yon.origin ... and list the app origin in
   server.origins ... (ADR 0103)."
2. Otherwise, when the project has Yon routes, the build succeeds and
   prints: "Note: the <target> app is offline-only. The project has Yon
   routes, but client/manifest.json declares no tac.yon.origin, so the app
   cannot reach them (ADR 0103)."
3. Otherwise the build says nothing.

With the origin declared, the native page sends its Yon calls there.

## Consequences

- A native app can call a remote Yon, and a build that cannot reach Yon
  now fails or prints a note instead of failing quietly at run time.
- Each project that ships a native app must run Yon on a public `https`
  origin, set `tac.yon.origin`, and list the app origins in
  `server.origins`.
- A web build may also call a Yon on another origin. Without
  `tac.yon.origin`, web and native pages behave as before.
- Cookies do not cross to the Yon origin. An app that signs in sends its
  token in the `Authorization` header.
- Linked native services (Swift, Kotlin, C# or Rust `HTTPClient` code that
  runs natively, not as WebAssembly) still have no native transport and fail
  with `HTTP_MISSING` until threads NET1 to NET4 add one. The 2026-10-19
  amendments add one for Swift on macOS and iOS, for Kotlin on Android, and
  for C# and Rust on Windows.
- A desktop Yon sidecar (D5 option (b)) is not part of this decision.
  Nothing here stops it from being added later.

## Not yet proven

This decision makes the page send its Yon calls to the declared origin. It
does not yet prove that each host's web view lets those requests through.
Custom-scheme `Origin` values, App Transport Security, Android request
interception and WebView2 each behave in their own way. Later threads test
each host:

- NET1: macOS and iOS;
- NET2: Android;
- NET3: Windows;
- NET4: Linux.

WebKit may send an opaque `null` `Origin` from `tachyon-app://bundle`. Then
`server.origins` cannot match it, and NET1 may need a forwarder in the host.
That decision will be added to this ADR.

## Amendment (2026-10-19): Apple hosts and linked Swift clients (NET1)

### The page keeps its own transports

The macOS and iOS pages live on `tachyon-app://bundle`, a scheme that a
`WKURLSchemeHandler` serves. WebKit treats a page on such a scheme as an
ordinary origin and sends `Origin: tachyon-app://bundle` on its CORS
requests, EventSources and WebSockets, the same way other WebKit apps with a
custom scheme send theirs. So `server.origins` matches it, and the hosts get
no bridge forwarder: the page keeps the transports of this ADR, with the
same stream and socket behavior as the web.

A local Yon on `http://127.0.0.1:<port>` or `http://localhost:<port>` needs
no App Transport Security exception, because App Transport Security does
not cover IP addresses and unqualified host names. Any other Yon origin is
`https`, as this ADR already requires.

`scripts/native/yon-reach-test.mjs` checks this on the host. It builds one
app whose page calls Yon on another origin over HTTP, a CORS preflight, a
stream, a socket, a topic, a `@Check` 422 problem and a `SignedIn` route,
and it records the `Origin` that Yon received. It runs the macOS app, the
iOS simulator over `http`, and the iOS simulator over `https` through a
local TLS proxy whose certificate the simulator trusts. If that run shows an
opaque `null` origin, the hosts need the forwarder after all, and this
section changes.

On 2026-10-19 the script passed on macOS 26.6.2 and the iOS 26.5 simulator,
over `http` and `https`. Yon received `Origin: tachyon-app://bundle` in every
run, so the hosts need no forwarder.

### Linked Swift HTTP clients reach the declared origin

A Swift service linked into the macOS or iOS app had no transport, so every
`@HTTPClient` request failed with `HTTP_MISSING`. When the manifest declares
`tac.yon.origin`, the build now links a `URLSession` transport into the
Swift companion, and a client without a Setup adapter sends its requests
there:

- the path is resolved against the declared origin, with the same path and
  query rules as the page transport;
- the request keeps the descriptor's method, headers and JSON body, and the
  transport refuses the same headers as the page (`Cookie`, `Origin`,
  `Referer`, `Sec-*`, `Proxy-*`), with `HTTP_HEADER`;
- cookies are never sent or stored, and the session has no cache;
- a redirect fails with `HTTP_NETWORK`, a status outside 200 to 299 with
  `HTTP_STATUS`, an answer over the response limit with `HTTP_LIMIT`, an
  answer that is not JSON with `HTTP_RESPONSE`, and the method deadline with
  `DEADLINE`.

Without `tac.yon.origin`, and in Swift test units, nothing changes: the
request still fails with `HTTP_MISSING` unless a Setup adapter answers it.

A native request carries no `Origin` header, so Yon treats it like any other
non-browser client. Identity still travels in the `Authorization` header.

### Streams and sockets in linked Swift services

A Swift service in a native app still cannot hold a `@StreamRequest` or
`@SocketRequest` client (`TY1405`). The message now tells the author to hold
the client in a JavaScript or TypeScript page service, which reaches Yon on
the declared origin, and to receive its messages in the Swift service with
`@OnEvent`. Lifting the refusal is a later choice.

## Amendment (2026-10-19): the Android app reaches Yon (NET2)

The Android host now lets the page and linked Kotlin services reach the
declared Yon origin.

### The page

- The host's request interception passes the declared origin to the
  network. An `https` origin passes by its host, as before. A local `http`
  origin (`localhost`, a name under `.localhost`, or a loopback address)
  passes only when the scheme, host and port all match.
- Android refuses cleartext traffic by default. For a local `http` origin,
  the build writes `res/xml/tachyon_network_security.xml` and names it in
  the manifest. It allows cleartext to that one host and keeps every other
  host on `https`. An `https` origin writes no such file.
- On the emulator, `adb reverse tcp:<port> tcp:<port>` makes
  `http://localhost:<port>` inside the app reach a Yon on the build machine.

### Linked Kotlin services

- A Kotlin `HTTPClient` used by a natively linked Kotlin service now has a
  native transport. Before this, it failed with `HTTP_MISSING`.
- The transport sends the request from the app process with
  `HttpURLConnection` to `tac.yon.origin`. It follows the same rules as the
  page's `TacBrowserHttp`:
  - the path, query and header checks are the same, and `Cookie`, `Origin`
    and the other refused headers stay refused;
  - a JSON body is sent with `content-type: application/json`;
  - redirects fail with `HTTP_NETWORK`, and a status outside 200-299 fails
    with `HTTP_STATUS`;
  - the client's deadline gives `DEADLINE`, and `maxResponseBytes` gives
    `HTTP_LIMIT`;
  - the answer must be UTF-8 JSON, or the call fails with `HTTP_RESPONSE`.
- The transport sends no cookies and keeps no HTTP cache, so a `@Cached`
  read is read again from Yon (NET6 covers offline reads).
- The request comes from the app, not from a web page, so it carries no
  `Origin` header. Yon answers it like any other client, without CORS.
- Without `tac.yon.origin`, the call fails with `HTTP_MISSING`, and the
  message says to set the member or give the client an adapter in its
  Setup.
- A `suspend fun` of a linked Kotlin service now runs until it finishes,
  like a linked C# method that returns a `Task`. It runs on the service's
  owner thread, so the app's other native service calls wait until it is
  done. The page still gives up on a native call after 10 seconds, so keep
  the client deadline below that.
- A linked Kotlin service that holds `@StreamRequest` or `@SocketRequest`
  clients is still refused with `TY1405`. The message now says that the
  page's JavaScript or TypeScript client reaches Yon through
  `tac.yon.origin`.

### Evidence

- `android_yon_http_tests` compiles a linked Kotlin service with a Kotlin
  `HTTPClient` for the JVM. It runs it against a real HTTP server: a query
  and an `Authorization` header, a JSON body, a 404, a redirect and an
  answer over the size limit. It also checks that no `Cookie` or `Origin`
  header is sent.
- Unit tests cover the request interception, the network security file and
  the manifest attribute.
- `scripts/android/yon-origin-test.mjs` runs the packaged app on the
  Android emulator against a Yon on the build machine. It covers an HTTP
  route, a stream, a socket, a topic, a `@Check` 422, a `SignedIn` route and
  a linked Kotlin client. It passed on 2026-10-19 on the Android 15 (API 35,
  arm64) emulator, with Yon on the Mac reached through `adb reverse`. The
  linked Kotlin client's request came from the app's own process.

## Amendment (2026-10-19): Windows and linked native services (NET3)

### Linked services answer later through a task

Before this amendment no native host could wait for a linked service. A Rust
`async fn` that did not finish in its first poll answered `HTTP_MISSING`, and
native C# ran `Task` methods to the end on the UI thread. A linked service
that calls Yon must wait for the network, so the Rust and C# companions now
use one task protocol. A Swift companion on macOS and iOS still waits inside
the call: it runs the main run loop until its task ends (NET1). A Kotlin
companion on Android runs a `suspend fun` to its end inside the call (NET2).

1. A `call` to an async member answers `{"task":N}` at once.
2. When the task settles, the companion sends one signal through the same
   emit callback that carries `publish`:
   `{"route","service","owner","task":N,"answer":"<envelope>"}`. The
   envelope is the usual `{"value":...}` or `{"error":...,"code":...}`
   answer, written as a JSON string.
3. A signal larger than 64 KiB is replaced with
   `{"error":"The task result is larger than 64 KiB.","code":"HTTP_LIMIT"}`.
4. The page keeps each waiting call by owner and task number. A signal for
   another owner, or for an owner the page already disposed, changes
   nothing. Disposing an owner rejects its waiting calls, and the companion
   finishes that owner's running tasks with `CANCELLED`.
5. The ten-second limit on a native call covers only the first answer. A
   task then waits until the service code ends or the page disposes the
   owner. Each Yon request inside it still ends by its own deadline.

The Rust companion runs its tasks on the host's UI thread. It exports
`tac_native_set_dispatch(fn)`. A waker on any thread queues the task and calls
the hook. The host then sends `{"op":"host-resume"}` on its UI thread, and
the companion polls the ready tasks with the owner's activation set. A host
that does not install the hook keeps the old `HTTP_MISSING` answer. The
Windows host posts one `WM_APP + 2` message for each burst of wakes.

The C# companion runs each task continuation on the thread pool, one at a
time per companion, under the lock that `Invoke` already takes. Its emit
callback is thread-safe, so it needs no resume hook. Native C# `Task`,
`Task<BehaviorJSON>` and `Task<Model>` methods now become task members, as
they do in the browser. Synchronous methods answer as before.

### Linked `HTTPClient` code gets a native transport

Linked Rust and C# `HTTPClient` calls go to `tac.yon.origin`. They follow the
same rules and codes as the page transport (`tac-http.js`):

- the method list, path and query rules, the header rules and limits;
- no cookies, no redirects (a `3xx` answer is `HTTP_NETWORK`), and no
  credentials;
- a deadline of at most 60 seconds (`DEADLINE`), and at most 64 requests in
  flight (`HTTP_CAPACITY`);
- a JSON body of at most 1 MiB, nested at most 32 deep, with safe numbers;
- a non-`2xx` answer is `HTTP_STATUS`.

Rust uses WinHTTP on Windows: a thread per request, with automatic proxy and
the system certificate store, and cookies, redirects and automatic sign-in
turned off. C# uses one `HttpClient` with redirects, cookies and
decompression turned off. Without `tac.yon.origin` both answer
`HTTP_MISSING`. A Rust companion on another desktop host answers
`HTTP_MISSING` until that host adds a sender.

### Held stream and socket clients stay with the page

A natively linked C# or F# service still may not hold a `@StreamRequest` or
`@SocketRequest` client (`TY1405`). The message now says to hold the client
in a service the page runs, because the page reaches the Yon that
`tac.yon.origin` names. Lifting this rule is a later choice.

### Evidence and what is not yet proven

In the cloud on 2026-10-19:

- The page side of the task protocol passed in Node: task signals, early
  signals, failed tasks, disposed and foreign owners.
- A staged Rust companion passed with `rustc` on Linux: park, dispatch,
  resume, signal and cancel.
- The same companion was cross-compiled for `x86_64-pc-windows-gnu` and run
  under wine 9.0. Through WinHTTP it reached a local HTTP server: `200` gave
  the value and `422` gave `HTTP_STATUS`, with no `Cookie` header sent.
- The Windows resume code was compiled with MinGW and run under wine.
- A staged C# companion was built with .NET 10 and run against a local
  server. It covered task answers, status, limit, deadline, cancel and the
  refusals.
- `scripts/yon-reach-fixture-test.mjs` passed in Chromium. It covers an HTTP
  route, a stream, a socket, a topic, a `@Check` 422 and a `SignedIn` route,
  with the page on one origin and Yon on another.

Not yet run:

- the MSVC and NativeAOT build of the Windows app;
- WebView2 from `https://tachyon.local`.

`scripts/windows/yon-reach.ps1` builds the fixture for Windows, starts Yon
on `http://localhost`, and clicks every path, including the linked Rust and
C# ones, through UI Automation. It is owed on the Surface desktop, run with
N1. One open question is whether WebView2 asks for local network access
before a page on `https://tachyon.local` may call `http://localhost`. If it
does, the run shows it, and that run decides the answer.

## Amendment (2026-10-19): the Linux app reaches Yon (NET4)

The packaged Linux app now reaches Yon on `tac.yon.origin`. WebKitGTK sends
`Origin: tachyon://app` from the registered, CORS-enabled scheme, so Yon
matches it against `server.origins` and no forwarder is needed in the host.

`scripts/linux/yon-origin-native-test.mjs` proves each path. It builds one
example with an HTTP route (read and write), a `@SignedIn` route that needs an
`Authorization` header and a preflight, a topic, a `@Check` 422, a stream and
a socket. Then it opens the app under Xvfb and reads each answer through the
accessibility bus. It runs against Yon on `http://127.0.0.1:<port>`, and over
`https://localhost:<port>` when `TAC_YON_TLS_CERT` and `TAC_YON_TLS_KEY` name
a certificate that the system trusts. It runs once with JavaScript page
services and once with linked Rust services (`TAC_YON_SERVICES=rust`).
`scripts/linux/native-test.sh` runs both, so the Linux gate in CI runs them
too.

Two changes let a linked service reach Yon:

- **The page holds a linked service's streams, sockets and topics.** The page
  opens the `#[StreamRequest]` and `#[SocketRequest]` clients of a linked
  service, and the topics of its `#[OnEvent]` listeners, with the same code it
  uses for page services. When a message arrives, the page calls the service
  with the companion operation `event`. Every host now accepts that
  operation. A linked Rust service that sends on a socket hands the message
  to the page, and the page sends it on its open socket. Before this change,
  a linked Rust service with stream or socket clients built without a
  refusal but could not reach anything. Swift, Kotlin and C# services keep
  `TY1405` for now.
- **A linked Rust `HTTPClient` has a native transport on Linux.** It uses the
  native task protocol from NET3: a parked task resumes when the host calls
  the companion with `host-resume`. The GTK host resumes tasks on its main
  loop. The companion hands each request to the host, and the host sends it
  to `tac.yon.origin` with libsoup. libsoup follows no redirects, sends no
  cookies and uses the request deadline and `maxResponseBytes`. It reads the
  system trust store and proxy settings, as the web view does. The answer
  goes through the same code as the Windows transport, so both hosts give
  the same results and error codes. A linked Rust service on Linux no longer
  answers `HTTP_MISSING`, as the NET3 amendment says a Rust companion does on
  a desktop host without a sender.

Not run yet: the Linux app on WSL, and in a desktop Wayland session.

Later run (2026-10-24, LX1): `TAC_LINUX_DISPLAY=wayland` runs the reach test
in a headless mutter 46.2 Wayland session. In an x86_64 cloud container it
passed with JavaScript and with linked Rust services. GTK did not embed
WebKit's accessibility socket in the window under Wayland, so the probe reads
and clicks the page in the web process's own accessibility tree there.

## Amendment (2026-10-19): the Windows app builds with MSVC and NativeAOT (RW1)

The NET3 amendment left the MSVC and NativeAOT build of the Windows app
unrun. On Windows 11 ARM64, over SSH, `scripts/native/yon-reach-fixture.py`
wrote the reach fixture with linked Rust and C# services, and
`ty build --target windows` built it with MSVC 14.44 and .NET 10.0.302. The
app folder held an ARM64 `YonReach.exe`, a NativeAOT `TachyonCompanion.dll`
(no CLR header; it exports `tac_native_invoke`, `tac_native_free`,
`tac_native_set_emit` and `tac_native_set_capability_adapter`) and
`TachyonRustCompanion.dll` (it also exports `tac_native_set_dispatch` and
the application start and shutdown entry points).

The run found two faults, both fixed:

- The fixture's generated `RustReachService.js` and `CsharpReachService.js`
  put two decisions in the private `#label` method, so the build refused
  them with `TY1314`. `#text` now reads the answer and `#label` only
  compares it. The Linux app's linked Rust run uses the same file.
- The Windows build left `TachyonRustCompanion.dll.lib` and
  `TachyonRustCompanion.dll.exp` in `bin`. Nothing loads them, so the build
  now deletes them after it links the Rust companion.

The app was not launched: WebView2 from `https://tachyon.local` still needs
the signed-in desktop, with `scripts/windows/yon-reach.ps1`.

## Amendment (2026-10-19): WSL and linked Rust on iOS

This amendment supersedes two statements above, which stay as the history
of this decision.

- "Not run yet: the Linux app on WSL" (the NET4 amendment).
  `scripts/linux/yon-origin-native-test.mjs` passed on WSL Ubuntu 24.04.5
  (aarch64) in RL1, with JavaScript page services and with linked Rust
  services (`docs/SUPPORT_TIERS.md`, the Yon reach table). A Wayland desktop
  is still not run.
- "A Rust companion on another desktop host answers `HTTP_MISSING` until that
  host adds a sender" (the NET3 amendment). Since MR1 (2026-10-19) iOS links
  Rust too, and a linked Rust service's `HTTPClient` answers `HTTP_MISSING`
  there as on macOS, because neither Apple host runs Rust tasks yet (LR1).
  Linux and Windows have a sender; Android's linked Rust `HTTPClient`
  reaches `tac.yon.origin` (MR2).
