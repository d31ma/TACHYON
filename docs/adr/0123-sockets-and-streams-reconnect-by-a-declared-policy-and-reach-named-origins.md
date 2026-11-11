# ADR 0123: Sockets and Streams Reconnect by a Declared Policy and Reach Named Origins

- Status: Accepted
- Date: 2026-11-11
- Amends: [ADR 0060](0060-sockets-are-a-route-and-a-request.md) (the bus
  no longer always stops after five tries; the count restarts when the socket
  opens)
- Amends: [ADR 0103](0103-native-apps-reach-yon-through-a-declared-origin.md)
  (besides `tac.yon.origin`, a page may reach named origins from
  `tac.yon.origins`)
- Amends: [ADR 0120](0120-a-socket-service-hears-open-and-close-and-a-tac-service-runs-every.md)
  (`reconnecting` follows the declared policy; `path` is the server path)
- Preserves: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (WebAssembly services reach the browser through Tachyon) and
  [ADR 0110](0110-a-native-preview-app-loads-its-pages-from-the-development-server.md)
  (a release never reaches a development server)

## Context

The page's event bus holds the `WebSocket` of a `@SocketRequest` client and
the `EventSource` of a `@StreamRequest` client. It reconnected after 1, 2, 4,
8 and 16 seconds and then stopped for good. The count restarted only when a
message arrived, not when the socket opened. An app that must catch up after
a long outage, such as a messaging app, lost its connection after about half
a minute and had to reopen it itself.

The bus, `tac-http.js` and the JavaScript client runtime could reach only the
page's own origin and the one `tac.yon.origin`. A realtime gateway on a
second server was out of reach.

## Decision

### A reconnect policy on the request

- `@SocketRequest` and `@StreamRequest` take, besides a quoted path, an
  object: `@SocketRequest({"path": "/ws", "reconnect": {"maxTries": 0,
  "delayMs": 1000, "maxDelayMs": 30000, "jitterMs": 1000}})`. Every Tac
  language that can write these markers accepts the same object (Python:
  `@socket_request({...})`, C++: `[[SocketRequest({...})]]`, F#:
  `[<SocketRequest({...})>]`).
- `maxTries` is 0 (no limit) to 100, default 5. `delayMs` is 100 to 60000,
  default 1000. `maxDelayMs` is 100 to 600000, default 16000, and not below
  `delayMs`. `jitterMs` is 0 to 60000. Its default is 0, or 1000 when
  `maxTries` is 0, and it must be above 0 when `maxTries` is 0, so many pages
  do not all reconnect at the same moment after an outage. Any other key or
  value stops the build.
- The wait before try `n` (from 0) is `min(maxDelayMs, delayMs * 2^n)` plus a
  random part from 0 to `jitterMs`. Without a declared policy the bus keeps
  the old behavior: 1, 2, 4, 8 and 16 seconds, then stop.
- The count restarts when the socket or stream opens, and still when a
  message arrives.
- While `navigator.onLine` is false, the bus does not count tries and does
  not wait on a timer. When the page fires `online`, or the page becomes
  visible again, a connection that is not open and was not ended by the
  server opens at once, with the count at 0. This also opens a connection
  whose tries had stopped.
- A close with code 1000, 1003, 1007 or 1009 (and a stream's `end` event)
  still ends the connection. Only a `send` opens it again.
- `SocketClosed.reconnecting` is true when the bus will try again by itself:
  the close is not final and the policy has tries left (always with no
  limit).

### Named origins

- `client/manifest.json` may declare `tac.yon.origins`, a map from a name to
  an origin: `{"tac": {"yon": {"origins": {"realtime":
  "https://rt.example.com"}}, "outgoing": {"hosts": ["rt.example.com"]}}}`.
  `tac.yon.origin` becomes optional when `origins` is set.
- A name starts with a lowercase letter and has at most 32 lowercase letters,
  digits and dashes. At most 8 names. Each value follows the
  `tac.yon.origin` rule: an `https` origin with no path, or a local `http`
  origin.
- A named origin is a new outgoing host, so it follows the declared-hosts
  rules: its host must be listed in `tac.outgoing.hosts` (a `*.` entry
  covers it), unless the origin is local. Otherwise the build stops with
  `TY1601`, and the message names the host to add.
- An `@HTTPClient` names the server with `origin`:
  `@HTTPClient({"path": "/chat", "origin": "realtime"})`. Every request,
  stream and socket of that client goes to that origin. The build stops with
  `TY1308` when the name is not in `tac.yon.origins`; the check runs once per
  build over `client/remotes` and `client/clients`, because only the build
  knows the project's manifest.
- The page learns the names from one tag,
  `<meta name="tachyon-yon-origins" content='{"realtime":"https://rt.example.com"}'>`.
  The content security policy adds each named origin to `connect-src` with
  its port, as `https` and `wss` (or `http` and `ws` for a local one), in web
  pages and in the native apps.
- A native release build (`ty bundle --release`) stops with `TY1601` when a
  named origin is local, as a release never reaches a development server
  (ADR 0110).
- The other server is not a Tachyon concern. When it is a Yon server, its
  `server.origins` must list the app origin (ADR 0103). WebKit sends
  `Origin: tachyon-app://bundle` on a WebSocket from the Apple apps, so the
  gateway must accept that origin too.

### One address, carried unchanged by every language

The policy and the origin are compile-time data. The build writes them into
the path that each language already carries to the page, so no language
runtime changes:

- A named origin client's base path becomes
  `/.tachyon/origin/<name><path>`, for example
  `/.tachyon/origin/realtime/chat`.
- A stream path with a policy becomes
  `/.tachyon/reconnect/<maxTries>/<delayMs>/<maxDelayMs>/<jitterMs><path>`.
- The event bus reads and removes the reconnect part. The bus,
  `tac-http.js` (used by every WebAssembly language) and the JavaScript
  client runtime map `/.tachyon/origin/<name>` to the declared origin. A name
  that the page does not know fails closed: the bus reports a failure and
  opens nothing, `tac-http.js` answers `HTTP_ORIGIN`, and the JavaScript
  client throws. A path after the name must not start with `//`. The HTTP
  layers still accept only the page origin, `tac.yon.origin` and the named
  origins.
- An authored path may not start with `/.tachyon/origin/` or
  `/.tachyon/reconnect/`.
- `SocketOpened.path` and `SocketClosed.path` are the server path, without
  either prefix.

## Consequences

- Mesail's socket keeps trying through a long outage and reconnects at once
  when the device is online again or the app is shown, and it reaches its
  realtime gateway on a second origin.
- The always-shipped bus grows by about 0.6 KB before compression.
- An app that sets no policy and no origin behaves as before.
- An unlimited policy can load a server after an outage; the required jitter
  and `maxDelayMs` spread and limit the tries.
- The address form is internal. Code that reads a held stream's path (such
  as a native shell's stream list) sees the prefixed form.

## Verification

- `events::stream_address_tests`: JavaScript, TypeScript, Go, Ruby, Rust,
  Dart, Swift, Kotlin, Java, C# and Python clients write the expected
  address; bad names, out-of-range policies and reserved paths are refused;
  HTTP requests carry the origin.
- `tac_cpp::tests::a_cpp_client_*`, `native::fsharp_service::tests::a_held_fsharp_socket_client_carries_its_origin_and_reconnect_policy`
  and `tac_lua::tests::stream_tests::a_lua_socket_client_carries_its_origin_and_reconnect_policy`
  for C++, F# and Lua.
- `compiler::remote_origins::tests`: the build check over every client
  language.
- `template::reconnect_tests`: the real bus in Node with fake timers, sockets
  and page events (no-limit policy, default policy, offline and online, final
  close, unknown origin, a stream on a named origin), and `tac-http.js` and
  the JavaScript client runtime on named origins.
- `native::yon_member::tests`: the manifest rules, the meta tag and the
  `connect-src` sources.
