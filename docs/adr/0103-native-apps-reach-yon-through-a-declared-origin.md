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
  with `HTTP_MISSING` until threads NET1 to NET4 add one.
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
