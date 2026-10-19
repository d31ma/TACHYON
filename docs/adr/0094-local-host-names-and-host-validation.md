# ADR 0094: Local Host Names and Host Validation

- Status: Accepted
- Date: 2026-10-03
- Works with: ADR 0068 (outgoing requests go only to declared hosts: this
  ADR closes two ways around its run-time guard), ADR 0077 (the error page
  server gets the same Host check)

## Context

`ty serve`, `ty start` and `ty preview` bind `127.0.0.1` by default, and they
answered every request whatever its `Host` header said. A page on an
attacker's name can re-point that name to `127.0.0.1` (DNS rebinding). The
browser then treats the local server as the attacker's own origin, so the
page can read Tac pages and call Yon routes. The WebSocket check (`Origin`
equals `Host`) passes too, because both carry the attacker's name. Chrome
asks before a public page reaches a local address; Firefox and Safari do not.

The outgoing request guard of ADR 0068 had two gaps. A handler with a
resource endpoint got `NO_PROXY=127.0.0.1`, so a client that honours the
proxy settings reached any port on `127.0.0.1` without the guard, and Go's
standard client never sends loopback addresses to a proxy at all. And the
guard checked the name, then let the operating system resolve it: an allowed
name (for example under a `*.example.com` wildcard) that points to
`127.0.0.1`, a private address or `169.254.169.254` reached that address.

Developers also want readable local names. Browsers resolve `localhost` and
every name under `.localhost` to loopback themselves (RFC 6761), treat them
as secure contexts, and keep cookies apart per name, while cookies ignore the
port. Bun 1.4.1 answers these names with `::1` and `127.0.0.1` without asking
the operating system, because Windows, musl (Alpine) and glibc without
systemd-resolved do not resolve names under `.localhost`.

## Decision

1. A *localhost name* is `localhost` or a name that ends in `.localhost`,
   compared without regard to ASCII case, with one trailing dot allowed (the
   Chromium rule). A name used to bind also needs valid labels: 1 to 63
   letters, digits or hyphens, so `.localhost` and `a..localhost` are
   refused.
2. Every server bound to a loopback address checks `Host` before routing. It
   answers a localhost name, an IP address, the host of `tac.origin`, and a
   name listed in the new `server.hosts` (`yon.server.hosts`, same syntax as
   `outgoing.hosts`). Any other name gets `403` with `TY1366`, and a `Host`
   that cannot be read gets `400`. The port is not compared, because proxies
   and tunnels change it. A request without `Host` (HTTP/1.0) passes:
   browsers always send one. A server opened with `--allow-non-loopback`
   checks only when `server.hosts` is set, so a reverse proxy in front of
   `ty start` keeps working. This matches Vite's `server.allowedHosts`,
   webpack-dev-server's `allowedHosts: 'auto'`, Rails' `config.hosts` and
   Django's `ALLOWED_HOSTS`. Like Spring Boot with
   `server.forward-headers-strategy=none`, the check reads `Host` itself and
   ignores `X-Forwarded-Host`.
3. When the outgoing guard is active, handler processes get no `NO_PROXY` or
   `no_proxy`. The runners reach their resource endpoint over a direct socket,
   so they do not need it. The Go prelude points `http.DefaultTransport` at
   the guard (`YON_OUTGOING_PROXY`).
4. The guard resolves an allowed name once and connects to the addresses it
   resolved. It drops loopback, private, unique local, link-local, shared
   (`100.64.0.0/10`) and unspecified addresses, and refuses a name that has
   no other address with `403`. An IP address listed in `outgoing.hosts`
   connects as listed. A localhost name connects only to loopback (see 6).
5. `--host` (alias `--hostname`) and `server.host` accept a localhost name.
   The server binds `127.0.0.1` without asking the operating system, counts
   as loopback (no `--allow-non-loopback`), and prints the name in its ready
   line, such as `http://shop.localhost:8080/`. Other names stay refused with
   `TY1302`.
6. The guard answers a localhost name in `outgoing.hosts` with `[::1]` and
   then `127.0.0.1`, without asking the operating system and ignoring an
   `/etc/hosts` entry for the name. Loopback stays deny-by-default: the name
   must be listed. `*.localhost` is not added to `NO_PROXY`, because that
   would skip `outgoing.hosts`.

## Consequences

- A local tunnel or a reverse proxy that sends its own name to a loopback
  server now gets `403` until the name is in `server.hosts`.
- A Yon handler that reached a private-network service by name through the
  guard must list the service's IP address instead, or reach it through a
  localhost name.
- Clients that honour the proxy settings (JavaScript and TypeScript, Python,
  Ruby, PHP with cURL, Go, C#, Rust with reqwest) reach `api.localhost` on
  every operating system. Java, Kotlin and Dart default clients and raw
  sockets use their own resolver, so names under `.localhost` work there only
  where the operating system resolves them (macOS and systemd-resolved
  Linux).
- Cookies set on `shop.localhost` are not sent to `admin.localhost`. A cookie
  with `Domain=localhost` is not a reliable way to share cookies between
  them.

## Later

- Several projects behind one port, chosen by `Host`.
- A second listener on `::1` (dual stack).
- TLS for names under `.localhost`.
- A setting that lets a named private-network service through the guard.
- A repeatable `--allowed-host` flag.

## Amendment (2026-10-18): App origins and local Yon origins

[ADR 0103](0103-native-apps-reach-yon-through-a-declared-origin.md) adds
`server.origins` and `tac.yon.origin`. The `Host` check of this ADR does not
change, and it still runs before anything else, including a CORS preflight.
`server.origins` is about the `Origin` header, not the `Host` header: it
says which app pages may call Yon from another origin. A name in
`server.origins` is not answered as a `Host`; list it in `server.hosts` for
that.

`tac.yon.origin` allows `http://` only for a local Yon. Local means a
localhost name as defined in decision 1 of this ADR (`localhost` or a name
under `.localhost`), or a loopback IP address.

## Amendment (2026-10-19): Java and Kotlin handlers use the guard

The consequence above about Java, Kotlin and Dart was wrong on two counts.
Dart's `HttpClient` reads `HTTP_PROXY` and `HTTPS_PROXY` by default, so a
Dart handler already sent its requests through the guard and reached names
under `.localhost` on every operating system. Java and Kotlin did not: the
JDK reads its proxy from system properties, not from the environment, so a
Java or Kotlin handler skipped the guard altogether. It reached an unlisted
loopback port, and a name under `.localhost` failed wherever the operating
system does not resolve it (Windows, musl, glibc without systemd-resolved).

The Java and Kotlin preludes now read `YON_OUTGOING_PROXY` before the
handler runs and set `http.proxyHost`, `http.proxyPort`, `https.proxyHost`
and `https.proxyPort` to the guard, and `http.nonProxyHosts` to the empty
string. An empty list keeps the JDK from adding its own default exceptions
(`localhost`, `127.*`, `[::1]`), so loopback stays deny-by-default as decision
6 says. `java.net.http.HttpClient` and `HttpURLConnection` both use these
settings; Groovy handlers share the Java prelude. The resource endpoint is
reached over a plain `java.net.Socket`, which these properties do not touch.

So JavaScript and TypeScript, Python, Ruby, PHP with cURL, Go, C#, Rust with
reqwest, Dart, Java, Kotlin and Groovy default clients all reach
`api.localhost` on every operating system. Raw sockets still use the
operating system resolver. `tests/it/outgoing_loopback.rs` checks Java,
Kotlin and Dart both ways: an unlisted loopback port gets `403`, and a listed
`api.localhost` is reached.
It passed on Linux in the cloud and on Windows 11 ARM64 (2026-10-19, see
`docs/SUPPORT_TIERS.md`, "Yon handlers on Windows").
