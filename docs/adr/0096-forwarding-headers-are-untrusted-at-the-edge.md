# ADR 0096: Forwarding Headers Are Untrusted at the Edge

- Status: Accepted
- Date: 2026-10-03
- Works with: [ADR 0094](0094-local-host-names-and-host-validation.md) (the
  Host check runs in the same edge layer), [ADR 0062](0062-yon-uses-native-http-types.md)
  (native request types)

## Context

A Yon handler runs in its own process. `ty` accepts the TCP connection and
passes the method, target, headers and body to the handler. Two things were
wrong with the client address:

- No handler learned the address of the client. Go's `*http.Request` had an
  empty `RemoteAddr`, Java and Kotlin `HttpExchange.getRemoteAddress()`
  answered `127.0.0.1:0`, and PHP had no `REMOTE_ADDR`.
- `ty` passed `X-Forwarded-For`, `Forwarded` and `X-Real-IP` from the browser
  unchanged. Any client could claim any address, and a back end for front
  end (BFF) that relays these headers to its own services passed the lie on.
  Rate limits, audit logs and address allow lists that read them could be
  fooled.

Spring treats these headers the same way: `ForwardedHeaderFilter` and
`server.forward-headers-strategy` use them only when the application says a
proxy in front of it sets them, and Tomcat's `RemoteIpValve` uses them only
from listed internal proxies.

## Decision

`ty` is the first proxy in front of every handler, so it owns the forwarding
headers. An edge layer runs before routing on the development server, on
`ty start` and on WebSocket upgrades. It reads the address of the TCP peer.

- When the peer is not a trusted proxy, `ty` removes `Forwarded`,
  `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`,
  `X-Forwarded-Port`, `X-Forwarded-Prefix` and `X-Real-IP`. Then it sets
  `X-Forwarded-For` and `X-Real-IP` to the peer address, `Forwarded` to
  `for=<peer>;proto=http`, and `X-Forwarded-Proto` to `http`.
- When the peer is a trusted proxy, `ty` keeps the headers it received and
  appends its own hop: the peer address to `X-Forwarded-For` and
  `for=<peer>;proto=http` to `Forwarded`. `X-Real-IP` becomes the client: the
  nearest address in `X-Forwarded-For`, read from the right, that is not a
  trusted proxy. A hop that is not an IP address stops the walk at the last
  trusted proxy.
- `server.proxies` in `server/application.json` lists the trusted
  proxies as IP addresses or CIDR ranges. It is empty by default, so loopback
  is not trusted either: another local program cannot claim a client address.
  A host name is refused, because a name can be re-pointed.

Handlers in every Yon language read the same headers, so `X-Real-IP` is the
client address everywhere. Native request types that have a peer field fill
it from `X-Real-IP`: Go `Request.RemoteAddr` (`<address>:0`), Java and
Kotlin `HttpExchange.getRemoteAddress()` (port `0`) and PHP
`$_SERVER['REMOTE_ADDR']`. The other native request types have no peer
field.

## Consequences

- A handler can relay `X-Forwarded-For`, `Forwarded` and `X-Real-IP` to its
  own services as they are.
- An app behind a reverse proxy lists that proxy in `server.proxies`
  to see the real client. Without it, every request seems to come from the
  proxy.
- The port of the client is not passed on; native peer fields report port
  `0`.
- `ty` does not terminate TLS, so it reports `proto=http` for its own hop. A
  trusted proxy that terminates TLS keeps its own `X-Forwarded-Proto`.
- Tests that build a handler request directly set these headers themselves.
