# ADR 0120: A Socket Service Hears Open and Close, and a Tac Service Runs Every

- Status: Accepted
- Date: 2026-11-10
- Extends: [ADR 0060](0060-sockets-are-a-route-and-a-request.md) (the page's
  event bus holds the `WebSocket` of a `SocketRequest` client) and
  [ADR 0028](0028-owned-application-events-and-lifecycle.md) (owned
  listeners and lifecycle)
- Preserves: [ADR 0055](0055-annotation-arguments-use-types.md) (typed
  events), [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule
  table) and [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (WebAssembly services reach the browser through Tachyon)

## Context

The page's event bus opens, closes and reconnects the socket of a
`SocketRequest` client, but no Tac service heard when that happened. A
WebSocket carries no `Authorization` header, so a socket that needs a
signed-in user authenticates with its first message. After an automatic
reconnect the server sees a new, unauthenticated connection, and the client
did not know that it had reconnected. Applications worked around it with a
server prompt (`hello`, `reauth`) that the client answered.

A Tac service also had no timer. `@Schedule` runs on the server only, and a Go
goroutine cannot call an `HTTPClient` or render the view again, because both
need a page call. Applications refreshed tokens before each call and on
`VisibilityChanged` instead.

## Decision

### Socket lifecycle events

- The event bus sends two built-in event models, `SocketOpened { path,
  reconnected }` and `SocketClosed { path, code, reconnecting }`, to the
  service that holds the socket client, and to no other service. A service
  hears them with an ordinary `@OnEvent` listener, in every Tac language that
  receives socket messages with `@OnEvent`. This is the Jakarta-style
  `@OnOpen` and `@OnClose` of ADR 0060, written in the Tac way: typed events
  and no new marker. `@OnOpen` and `@OnClose` stay Yon socket-route markers.
- `reconnected` is false at the first open and true at every later open of
  the same client: after an automatic reconnect and after a `Send` that reopens
  a socket whose retries stopped. `code` is the close code (1006 when the
  browser gives none). `reconnecting` is true when the bus will open the
  socket again by itself.
- When the socket opens and the service has a `SocketOpened` listener, the bus
  runs the listeners first and sends the messages that waited in the queue
  after they finish. A message that a listener sends while the socket is open
  goes out at once, so an authentication message reaches the server before the
  waiting messages. Without a listener, the queue is sent at once, as before.
- When the owner is disposed, the socket closes and no event is delivered.

### `@Every`

- `@Every("60s")` goes on a method of a Tac service, in every Tac service
  language. The duration is a whole number followed by `ms`, `s`, `m` or `h`,
  from 1 second to 24 hours. The method takes no parameters. A type has at most
  8 `@Every` methods. The rule table refuses `@Every` in Yon (use
  `@Schedule`), and with `OnStart`, `OnStop` or `OnEvent` on the same method.
- An `@Every` method stays an ordinary method of the service: the build
  removes only the marker. The event plan lists the timers as
  `timers: [{ method, every }]`; for a WebAssembly service the plan names the
  authored method, and the shared shell matches it to the method that the
  service exposes to the page (`refresh_token`, `RefreshToken` and
  `refreshToken` are the same name). The bus starts the timers after
  `OnStart`. Each run calls the method as a page call, the same call a click
  handler makes, so it may call an `HTTPClient` or `Send`, and the view
  renders again when it ends. So the method must be one that a view could call
  (in Go, an exported method). Runs never overlap. A run that fails or exceeds
  the 10-second work limit is reported like a failed listener.
- While `document.visibilityState` is `hidden`, runs are skipped. When the
  page is shown again after a skipped run, the method runs once at once.
- Disposing the owner (the component unmounts or the page goes away) clears
  the timer and its visibility listener.

### The contract in each language

| Language | `@Every` | `SocketOpened` and `SocketClosed` |
| --- | --- | --- |
| JavaScript, TypeScript | `@Every('60s')` on a method; the bus calls it | `@OnEvent` listener (JSDoc or TypeScript parameter type) |
| Go | `// @Every("60s")` on an exported method | `// @OnEvent` method with a `SocketOpened` parameter |
| Rust | `#[Every("60s")]` on a `pub fn` | `#[OnEvent]` method |
| C# | `[Every("60s")]` on a public method | `[OnEvent]` method |
| F# | `[<Every("60s")>]` on a member | `[<OnEvent>]` member |
| Swift | `@Every("60s")` | `@OnEvent` method |
| Kotlin | `@Every("60s")` | `@OnEvent` method |
| Java | `@Every("60s")` | `@OnEvent` method |
| Dart | `@Every('60s')` on a public method | `@OnEvent` method |
| Ruby | `#[Every("60s")]` | `#[OnEvent]` method with a YARD `@param` type |
| Lua | `-- @Every("60s")` | `-- @OnEvent` function with a `---@param` type |
| Python | `@every("60s")`, also on an `async def` method | `@on_event` method with a `SocketOpened` parameter (built-in models since ADR 0121). The listener is synchronous, because async `OnEvent` methods are still pending in Python, so it cannot await `send`: it records the state, and an `async def` method, such as an `@every` method, sends the message |
| C++ | `[[Every("60s")]]` on a public method | `[[OnEvent]]` method |

The WebAssembly languages (Go, Rust, C#, F#, Swift, Kotlin, Java, C++,
Ruby, Lua and Python) and Dart need no runtime code for `@Every`: the build
reads the timers from the service source (`ServiceTimers`; F# and Lua through
their analysis views, C++ through its own parser) into the plan, and the
shared shell calls the exposed method. Each language's own build removes the
marker, as it removes the other rule-table markers.

### Targets

Both work wherever the page's event bus runs: the web and the macOS, Windows,
Linux, iOS and Android apps, which load the page in the platform web view.
The socket events need `WebSocket`, as the socket client does; where the page
has none the client already reports a failure and no event arrives. Where the
page has no `document`, the page counts as shown. A natively linked service
(ADR 0108) cannot name the page's built-in event models, as before, so it does
not hear these events. Its `@Every` methods run only when the service runs in
the page as WebAssembly.

## Consequences

- A Go Tac service authenticates its socket in a `SocketOpened` listener and
  refreshes a token with `@Every`, without server prompts.
- No language runtime changes for `@Every`: the build reads the timers from
  the service source (`ServiceTimers`) and the shared shell calls the exposed
  method.
- `SocketOpened` and `SocketClosed` are reserved model names in `client/`,
  like the other built-in event models. (Since 2026-11-11 an app model of the
  same name wins, and `TY1376` stops the build only when a service that holds
  a socket client listens to it; ADR 0065 amendment.)
- Tested by `events::event_timer_tests`, the rule-table test
  `every_runs_in_every_tac_language_and_nowhere_in_yon`,
  `tac_web::go_socket_hooks_tests` (Go, with the authentication order), and
  one test per language that builds a small service and drives the real
  event bus with a fake `WebSocket`, fake intervals and a fake page
  visibility (`tac_web::socket_hooks_tests` for JavaScript, Rust, C#, F#,
  Swift, Kotlin, Java and C++; `tac_dart::dart_socket_hooks_tests`;
  `tac_ruby`, `tac_lua` and `tac_python` `socket_hooks_tests`).
