# ADR 0062: Yon Uses Native HTTP Types

- Status: Accepted
- Date: 2026-09-27
- Supersedes: the `YonRequest` and `YonResponse` classes of every Yon
  language (the optional response helper of ADR 0026)
- Preserves: [ADR 0048](0048-yon-routes-come-from-annotations.md) (a
  controller returns a model; the parameter markers bind values) and Handler
  Protocol v1 (the frame between the server and a handler process)

## Context

Controllers no longer need a request object: `FromPath`, `FromQuery`,
`FromBody`, `FromHeader` and `FromCookie` bind values, and a returned model
becomes JSON. A raw request and a custom answer are still needed in a few
places: root middleware, `relay(command, request)`, a validation answer with
status 400, an error handler that reads the request, and a plain `Handler`.
Each language had its own `YonRequest` and `YonResponse` for these places.
They were Tachyon inventions, so a developer had to learn them, and code that
used them could not move to or from an ordinary web framework.

## Decision

`YonRequest` and `YonResponse` are removed. Each language uses the request and
response types that its own HTTP libraries use.

| Language | Request | Response |
|---|---|---|
| JavaScript, TypeScript | Fetch `Request` | Fetch `Response` |
| Python | `urllib.request.Request` | `(HTTPStatus, headers, body)` tuple |
| Ruby | `Net::HTTPGenericRequest` (`Net::HTTP::Get` and the others) | `[status, headers, body]` array |
| PHP | `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `php://input` | `http_response_code()`, `header()`, `echo` |
| Java, Kotlin | `com.sun.net.httpserver.HttpExchange` | writes on the same `HttpExchange` |
| C# | `System.Net.Http.HttpRequestMessage` | `System.Net.Http.HttpResponseMessage` |
| Rust | `http::Request<Vec<u8>>` (`http` crate) | `http::Response<Vec<u8>>` |
| Dart | `package:shelf` `Request` | `package:shelf` `Response` |
| Go | `*http.Request` (`net/http`) | `*http.Response` |

- The server still sends the Handler Protocol v1 frame. The runtime of each
  language builds the native request from it and turns the native response
  back into a frame. The frame gains an optional `target`: the path and query
  that the client sent, so the native request has a real URL. Every frame also
  carries the header `x-tachyon-request-id`, because a native request has no
  field for the request id.
- One parameter without a marker receives the native request. PHP has no
  request object, so a PHP method takes no request parameter; the build
  refuses one and names the superglobals.
- Middleware `before` and `after` receive the native request. Answering
  nothing, or a 204, lets the request pass (a 204 adds its headers in the after
  phase). Any other answer replaces the response.
- `relay(command, request)` takes the native request and answers with the
  native response. The program still reads JSON on standard input
  (`route`, `method`, `headers`, `body`) and writes JSON on standard output.
  Path parameters are no longer forwarded; the program reads the route. PHP
  calls `Yon::relay(command)`, and Java and Kotlin call
  `Yon.relay(command, exchange)`, which write the answer on the request.
- Generated code keeps using the frame through private names (`__YonFrame`,
  `__YonAnswer`, the `__yon_frames__` / `YON_FRAMES` marker on generated
  handler classes). User code cannot name them: a `__Yon` name is refused.
- In the languages that had them (PHP, Java, Kotlin, C#, Rust, Dart and Go),
  the build refuses `YonRequest`, `YonResponse`, `YonJSON` and `YonEmpty` with
  TY2015 and names the native type of the language.
  `ty migrate fix --write` removes an unused `YonRequest` parameter and lists
  every other use as a manual step with the native request and response.
- A handler answer never sets `content-length`, `transfer-encoding` or
  `connection`; the server computes them.

## Consequences

- Rust handlers build with Cargo (`cargo build --release`, offline first) and
  the `http` crate; the Rust readiness check probes `cargo`.
- Dart handler builds write a `pubspec.yaml` with `shelf` and run
  `dart pub get` (offline first).
- PHP runs with `php-cgi`, because the CLI SAPI throws `header()` away.
  `header()`, `header_remove()`, `headers_list()`, `http_response_code()` and
  `echo` are all native: after the method, the runtime reads the status, the
  header list and the buffered body. `php-cgi` starts with `-C` (stay in the
  working directory) and with the CLI defaults Tachyon needs: no time limit,
  no output buffering, no `X-Powered-By` header, and errors on standard error.
  The runtime defines `STDIN`, `STDOUT` and `STDERR`, which only the CLI
  defines. The status is 200 until the program sets one, so an explicit 200
  with no header and no body counts as no answer. As in every PHP server,
  `header('Content-Type: text/html')` gains `;charset=UTF-8` from
  `default_charset`. A `php://` stream wrapper
  serves `php://input` and passes every other `php://` stream through.
  Readiness and `ty doctor` look for `php-cgi`: Homebrew's `php` includes it,
  Debian and Ubuntu have the `php-cgi` package, and the Windows PHP zip file
  has `php-cgi.exe`.
- Plain JavaScript `{ status, headers, body }` objects and Python and Ruby
  dictionaries with those keys are models now; a middleware that returns one
  fails with a message that names the native response.
- A GET or HEAD request has no body in the native request of JavaScript, Python and
  Ruby, as in the Fetch and Net::HTTP standards.
