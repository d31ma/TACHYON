# ADR 0060: Sockets Are a Route and a Request

- Status: Accepted
- Date: 2026-09-26
- Extends: [ADR 0013](0013-semantic-hot-updates.md) (server-sent events stay
  the push channel for hot updates; WebSockets are added for application
  messages that go both ways) and
  [ADR 0058](0058-streams-are-a-route-and-a-request.md) (the same route and
  request pair, now for a two-way connection)
- Preserves: [ADR 0055](0055-annotation-arguments-use-types.md) (typed
  events) and [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule
  table)

## Context

`@StreamRoute` and `@StreamRequest` send messages from the server to the
browser. A chat, a shared editor or a game also sends messages from the
browser to the server on the same connection. Jakarta WebSocket gives this a
small, well-known shape: an `@ServerEndpoint` class with `@OnOpen`,
`@OnMessage`, `@OnClose` and `@OnError` methods, and one endpoint object for
each connection.

## Decision

### SocketRoute (Yon)

- `@SocketRoute(path?)` marks a class in `server/controllers/`, like
  `@RequestRoute`. The class name ends with `Controller`. The path has no
  `{variables}`.
- The class has one `@OnMessage` method and may have one `@OnOpen`, one
  `@OnClose` and one `@OnError` method. Method names are free, as in Jakarta.
  - `@OnOpen` method: `(session)`, or no parameter.
  - `@OnMessage` method: `(session, message)`. The message parameter has a
    model type (JSDoc `@param {ChatMessage} message` or
    `message: ChatMessage`), and Tachyon checks each message against it.
  - `@OnClose` method: `(session)`, or no parameter.
  - `@OnError` method: `(session, error)`, `(session)`, or no parameter.
- One object for each connection, as in Jakarta: Tachyon starts one handler
  process when the browser connects and keeps it until the connection ends.
  Fields of the class keep state for that one connection. `@Needs` fields are
  built for each connection.
- The `session` has `id`, `send(message)`, `close()` and `openSessions()`.
  `openSessions()` is Jakarta's `getOpenSessions()`: it returns every open
  session of the same path (with `id` and `send`), so one message can go to
  every connected browser.
- Messages are text frames that hold JSON. `@OnClose` always runs last. When
  `@OnOpen` or `@OnMessage` fails, `@OnError` runs and the connection stays
  open, as in Jakarta: the next message goes to the same object. The error
  text goes to the server log only, never to the browser.
- Guards (fail closed):
  - A message that is not JSON, or does not fit the model, closes the
    connection with code 1007. A binary frame closes it with code 1003.
  - A message larger than 64 KiB closes it with code 1009.
  - At most 32 connections are open at once for the whole server; a new one
    gets `503`.
  - Five minutes without a message in either direction closes the connection
    with code 1001. A connection lasts at most one hour.
  - The `Origin` header must be the page's own origin; a different origin
    gets `403`. A request with no `Origin` (not a browser) is allowed.
  - A plain HTTP request to the path gets `426 Upgrade Required`.
- The rule table puts `@SocketRoute` on a class, the four `On…` markers on
  methods inside a `@SocketRoute` class, and refuses `@SocketRoute` together
  with `@RequestRoute`. Route markers such as `@GetRoute` need
  `@RequestRoute`, so they are refused in a socket class.
- A socket keeps one handler running for the whole connection and reads each
  message while it runs. The runner or prelude of every Yon language reads
  the socket frames below, so `@SocketRoute` is written in any Yon language
  (see "Socket languages").
- `/api.json` marks the path with `"socket": true` and describes the message
  model.

### Handler protocol frames

The request frame has `operation: "socket.open"`. After it, the server writes
`message` frames (one JSON text each), a `sessions` frame (the answer to
`openSessions()`) and a `close` frame on stdin. The handler writes `event`
frames (a message for this browser, or for another session with `to`),
`sessions` frames (a question) and `close` frames on stdout. The frames use
the same four-byte length prefix as every Handler Protocol v1 frame.

### SocketRequest (Tac)

- `@SocketRequest(path?)` goes on a method of an `@HTTPClient` class, like
  `@StreamRequest`. The method takes one parameter whose model type says what
  each incoming message holds. Tac checks each message against the model,
  passes it to the method and publishes it as that model's typed event, so
  `@OnEvent` listeners in any Tac language receive it.
- Sending uses Spring's `WebSocketSession.sendMessage` shape: the client
  object gets a generated `send(message)` method. It opens the socket when it
  is not open yet and sends the value as JSON.
- The browser event bus owns the `WebSocket`: it opens when the client is
  first used, closes when its owner goes away, and reconnects after 1, 2, 4, 8
  and 16 seconds and then stops. A message resets the count.
- `@SocketRequest` is written in JavaScript or TypeScript; the other Tac
  languages receive the messages with `@OnEvent`. TypeScript declares the
  generated method with `declare send: SocketSend<ChatMessage>`; a `declare`
  field only names a type, so `@NeedsAll` does not require it.

### Socket languages

Each language keeps the same frames, limits and one object for each
connection. The session methods use the language's own naming:

| Language | Marker | Session |
| --- | --- | --- |
| JavaScript, TypeScript | `@SocketRoute` | `id`, `send`, `close`, `openSessions()` (promises) |
| Python | `@socket_route` | `id`, `send`, `close`, `open_sessions()` |
| Go | `// @SocketRoute` | `ID()`, `Send`, `Close`, `OpenSessions()`, `Request()` on `*SocketSession` |
| Java | `@SocketRoute` | `getId()`, `send`, `close`, `getOpenSessions()` (Jakarta names) |
| Kotlin | `@SocketRoute` | `id`, `send`, `close`, `openSessions()` |
| C# | `[SocketRoute]` | `Id`, `Send`, `Close`, `OpenSessions()`; a method may return `Task` |
| PHP | `#[SocketRoute]` | `$session->id`, `send`, `close`, `openSessions()` |
| Ruby | `#[SocketRoute]` | `id`, `send`, `close`, `open_sessions`; the message model comes from YARD `@param` |
| Rust | `#[SocketRoute]` | `id()`, `send`, `close`, `open_sessions()` on `&SocketSession`; fields keep state with `Cell` or a lock, because the object is shared as `Arc` |
| Dart | `@SocketRoute` | `id`, `send`, `close`, `openSessions()` (futures); `@OnError` takes `Object error` |

The generated handler calls the marked methods with the parameters they
declare, and turns each message into the model type first (for example a
Python dataclass or a Go struct). An `async def` method in Python runs on one
event loop for the connection. A method that returns an error (Go) or throws
counts as a failure and runs `@OnError`.

The compiled and prelude languages normally read the whole request and answer
once. For a socket, the supervisor sets `YON_SOCKET_FRAMES=1` (a private name
that an allowlist cannot pass on); the prelude then reads the request as a
framed Handler Protocol frame and keeps reading frames from stdin.

## Consequences

Two-way messages need no hand-written `WebSocket` code. Each connection costs
one handler process, so the session limit is small and fixed. Every Yon
language reads the same `message` frames, so a socket class can be written in
the language of the rest of the server.

## Amendment (2026-09-29)

The `@SocketRequest` method belongs to an `HTTPClient` contract (ADR 0048
amendment): a class with empty method bodies in JavaScript and TypeScript.
`declare send: SocketSend<Message>` stays allowed, because it declares a type
and holds no state.

## Amendment (2026-10-11)

The Go session has `Request()`. It returns the request that opened the
socket as a `*http.Request`: its URL with the query string, its headers and
the client address. The supervisor now passes the opening request's target,
so the query string reaches every language's socket request.

## Amendment (2026-10-11): Go socket clients

`// @SocketRequest` is also written on a Go HTTPClient interface method. The
page's event bus holds the `WebSocket` for the Go service that keeps the
client in a field; the Go service receives each message with `// @OnEvent`,
and the generated `Send(message any) error` writes to the socket and returns
an error code instead of a rejected promise. See ADR 0090, "Go streams and
sockets". The other compiled Tac languages still refuse `@SocketRequest`.

## Amendment (2026-10-17): Ruby socket clients

- A Ruby Tac `@HTTPClient` method may carry `#[SocketRequest]`. The page
  holds the socket for the Ruby service that keeps the client in a
  `#[Needs]` field. The client gets `send_message(message)`, which waits for
  the page like an HTTP call (ADR 0054 amendment of 2026-10-17).

## Amendment (2026-10-17): Rust socket clients

`#[SocketRequest]` is also written on a Rust HTTPClient trait method. The
page's event bus holds the `WebSocket` for the Rust service that keeps the
client in a field; the service receives each message with `#[OnEvent]`, and
the generated `send(message).await` writes to the socket and returns a
`BehaviorError` code instead of a rejected promise. See ADR 0090, "Rust
streams and sockets". Dart, Java and
C++ still refused `@SocketRequest` then. (Dart, Python, F#, C++ and
Java hold socket clients since; see their amendments below.)

## Amendment (2026-10-17): Dart socket clients

`@SocketRequest` is also written on a Dart HTTPClient method. The page's
event bus holds the `WebSocket` for the Dart service that keeps the client in
a `@Needs` field; the Dart service receives each message with `@OnEvent`, and
the generated `Future<void> send(Object? message)` writes to the socket and
fails with a `TacSocketError` code. See ADR 0090, "Dart streams and
sockets". Go, Ruby and Rust socket clients are held the same way (see
their amendments above).

## Amendment (2026-10-17): Swift socket clients

`@SocketRequest` is also written on a Swift HTTPClient protocol method. The
page holds the `WebSocket` for the Swift web service that keeps the client in
a property, and the generated `send(_ message:) async throws` writes to it.
See ADR 0090, "Swift streams and sockets".

## Amendment (2026-10-17): Kotlin socket clients

`@SocketRequest` is also written on a Kotlin HTTPClient interface method. The
page's event bus holds the `WebSocket` for the Kotlin web service that keeps
the client in a field, and the generated `suspend fun send(message: Any)`
writes to the socket. See ADR 0090, "Kotlin streams and sockets".

## Amendment (2026-10-17): C# socket clients

`[SocketRequest]` is also written on a C# HTTPClient interface method. The
page's event bus holds the `WebSocket` for the browser C# service that keeps
the client in a `[Needs]` field; the service receives each message with
`[OnEvent]`, and the generated `Task Send(object message)` writes to the socket
and throws a `BehaviorError` with a code instead of rejecting a promise. A
natively linked C# service refuses the client. See ADR 0090, "C# streams and
sockets".

## Amendment (2026-10-18): Lua socket clients

`-- @SocketRequest` is also written on a Tac Lua HTTPClient method. The page's
event bus holds the `WebSocket` for the Lua service that keeps the client in
a typed `-- @Needs` field; the service receives each message with
`-- @OnEvent`, and the generated `client:send(message)` writes to the socket
from the call's coroutine and raises a `BehaviorError` with a code instead of
rejecting a promise. See ADR 0085, "Tac Lua streams and sockets", and ADR
0090, "Lua streams and sockets".

## Amendment (2026-10-18): Python socket clients

`@socket_request` is also written on a Python Tac `@http_client` contract
method. The page's event bus holds the `WebSocket` for the Python service
that keeps the client in a `Needs` field; the service receives each message
with `@on_event`, and the generated `async def send(self, message)` writes
to the socket and raises a `BehaviorError` with a code instead of rejecting
a promise. See ADR 0090, "Python streams and sockets".

## Amendment (2026-10-18): F# socket clients

`[<SocketRequest>]` is also written on an abstract member of an F# HTTPClient
type. The page's event bus holds the `WebSocket` for the browser F# service
that keeps the client in a `[<Needs>]` property; the service receives each
message with `[<OnEvent>]`, and the generated
`member _.Send(message: obj) : System.Threading.Tasks.Task` writes to the
socket through the same .NET socket sender as C# and raises a `BehaviorError`
with a code instead of rejecting a promise. A natively linked F# service
refuses the client. See ADR 0090, "F# streams and sockets".

## Amendment (2026-10-18): C++ socket clients

`[[SocketRequest]]` is also written on a Tac C++ `[[HTTPClient]]` method,
declared without a body as `[[SocketRequest]] void receive(const ChatMessage& message);`.
The page's event bus holds the `WebSocket` for the C++ service that keeps the
client in a `[[Needs]]` field; the service receives each message with
`[[OnEvent]]`, and the generated
`template <typename Message> tachyon::Task<void> send(Message message) const`
writes to the socket and throws `tachyon::http::HttpRequestException` with a
code instead of rejecting a promise. See ADR 0090, "C++ streams and
sockets".

## Amendment (2026-10-18): Java socket clients

`@SocketRequest` is also written on a Tac Java HTTPClient interface method.
The page's event bus holds the `WebSocket` for the Java service that keeps
the client in a `@Needs` field; the service receives each message with
`@OnEvent`, and the generated `void send(Object message)` writes to the
socket. `send` parks the service method's TeaVM fiber until the page answers,
and throws a runtime exception whose message starts with the code instead
of rejecting a promise. See ADR 0090, "Java streams and sockets". No Tac
language refuses `@SocketRequest` now.

## Amendment (2026-10-18): C++ socket routes

A C++ class marked `[[SocketRoute("/chat")]]` holds a socket, one process for
each connection. Its `[[OnOpen]]`, `[[OnMessage]]`, `[[OnClose]]` and
`[[OnError]]` methods return `void` and take a `SocketSession&`;
`session.openSessions()` returns `std::vector<SocketPeer>`, and a thrown
exception counts as a failure. See ADR 0099, "Yon stream and socket routes".

## Amendment (2026-10-18): Origins listed in server.origins

The `Origin` guard above also accepts an origin listed in `server.origins`
in `server/application.json`
([ADR 0103](0103-native-apps-reach-yon-through-a-declared-origin.md)). An
`Origin` that equals the `Host`, or a request with no `Origin`, is still
allowed. Any other origin gets `403` with "This socket only accepts pages
from its own origin or an origin listed in server.origins in
server/application.json."

On the page side, when `client/manifest.json` declares `tac.yon.origin`, the
event bus opens a `@SocketRequest` WebSocket on that origin: `wss://` for an
`https` origin, or `ws://` for a local `http` origin. Without
`tac.yon.origin`, the socket opens on the page's own origin as before.

## Amendment (2026-10-19): held clients run in three browsers

`scripts/held-clients-browser-gate.mjs` (`bun run test:held-clients-browser`)
builds the published `sockets` and `realtime` features for one Tac language,
with the JavaScript Yon controllers, serves them with `ty serve`, and opens
the page in Chromium, Firefox and WebKit. In two tabs the service's
`SocketRequest` client opens with its component and hears the welcome; a
click in one tab sends through the client and both tabs hear the message;
and the service's `StreamRequest` client hears every tick of the stream.
`TAC_HELD_CLIENT_LANGUAGES` and `TAC_HELD_CLIENT_BROWSERS` narrow a run.

On 2026-10-19 it passed for Rust, Go, Dart, Swift, Kotlin, Java, C#, F# and
C++ in Chromium 141, Firefox 153 and WebKit 26.5, in an x86_64 Ubuntu 24.04
cloud container with `ty` built from develop `d7110f74` (Rust, Dart and C#
again on `162064a0`). Before, a
compiled-language held client had only unit tests. The first run found that
the published Dart and C# services of both features no longer built
(`TY1314` and `TY1330`); they now keep their message and tick limits in a
named constant and a private method.


## Amendment (2026-10-24): A linked service sends through the page

A natively linked Swift, Kotlin or C# service may now hold a
`@SocketRequest` client (see the same-day amendment to ADR 0058). The page
holds the `WebSocket`. The client's `send` in the companion checks the
64 KiB limit and the running owner and emits
`{"route","service","owner","send":"<client>","value":<message>}` through
the host bridge, the signal a linked Rust service sends since NET4; the
page sends the message on that owner's socket. Swift uses
`TacPageSocket.swift` (`tacSocketSend`, with `TacSocketError` now shared
with the WebAssembly build), Kotlin `TacPageSocket` in
`prelude_native_tasks.kt` (`TacSocketFailure` moved to `prelude.kt`), and
C# `Tac.SendNativeSocket` behind `TacSocketTransport.Send`. A send with no
live owner fails with `HTTP_REQUEST`, a message over 64 KiB with
`HTTP_BODY`, and a C# companion with no host emitter with `HTTP_MISSING`.
The send does not wait for the socket to open: the page queues up to 16
messages, as for a web service.
