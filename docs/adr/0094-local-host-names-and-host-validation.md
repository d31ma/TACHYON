# ADR 0094: Local Host Names and Host Validation

- Status: Accepted
- Date: 2026-10-03
- Amended by: [ADR 0113](0113-the-operating-system-confines-handler-egress.md)
  (2026-10-31: the operating system enforces the outgoing request guard);
  the amendment of 2026-11-11 below (a named host on a private network)
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

   > Superseded on 2026-10-31 by [ADR 0113](0113-the-operating-system-confines-handler-egress.md):
   > the resource endpoint is one door per server, and the operating system
   > refuses a connection that skips the guard, on macOS and Linux. The Go
   > prelude only keeps the default clients working; it is not the control.
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
- ~~A setting that lets a named private-network service through the guard.~~
  Done: see the amendment of 2026-11-11.
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

## Amendment (2026-11-11): a named host on a private network

### Context

Decision 4 drops every private address from a resolved name. That is right
for the public internet, but it blocks a Yon handler that runs in a container
next to the services it calls. In Docker, Docker Compose or Floci's Lambda
containers, `keycloak` and `router` are names on the container network, and
they resolve only to private addresses such as `172.18.0.4`. Listing them in
`outgoing.hosts` was not enough: the guard answered `403`. The two ways
around it were poor. An IP address in `outgoing.hosts` breaks when the
container network gives out a new address. A localhost name plus a port
forwarder (Mesail ran `socat` in every image) adds a process to every image.

The filter of decision 4 exists for two attacks, and the new setting must not
open either of them:

- server-side request forgery (SSRF): a caller makes the app send a request
  to an internal address of the caller's choice;
- DNS rebinding: a listed name, or a name under a listed wildcard, is pointed
  at an internal address after the list was written.

### Decision

1. `server/application.json` gets `outgoing.private` (`yon.outgoing.private`),
   a list of entries. Each entry names one host and the networks it may reach:

   ```json
   "outgoing": {
     "hosts": ["keycloak", "router", "api.example.com"],
     "private": [
       { "host": "keycloak", "networks": ["172.16.0.0/12"] },
       { "host": "router", "networks": ["172.16.0.0/12", "10.0.0.0/8"] }
     ]
   }
   ```

2. An entry has exactly two members, `host` and `networks`:
   - `host` is one exact name that is also listed in `outgoing.hosts`. A
     wildcard (`*.example.com`), an IP address and a localhost name are
     refused. So a wildcard can never reach a private network.
   - `networks` is a non-empty list of IP addresses or CIDR ranges. Each one
     must lie wholly inside one private-use range: `10.0.0.0/8`,
     `172.16.0.0/12`, `192.168.0.0/16`, `100.64.0.0/10` (shared address
     space, used by some clusters and VPNs) or `fc00::/7` (unique local).
     `0.0.0.0/0`, loopback, link-local and public ranges are refused.
   - A host is named once.

   A bad entry stops `ty serve`, `ty start` and `ty preview` with `TY2010`.
   The check runs on the configuration after a `YON_PROFILE` profile is
   merged, so a profile cannot add an unchecked entry. A bad entry is never
   ignored.

3. The guard still resolves a name once per connection and connects only to
   the addresses it kept. It now keeps an address when it is public (as
   before) or when it lies inside the networks of that host's entry. The
   check runs on the address that the guard passes to `connect`, after an
   IPv4-mapped IPv6 address is turned into its IPv4 form. There is no second
   lookup, so a name that changes its answer cannot widen the check:
   - a listed name without an entry that resolves to a private address still
     gets `403`;
   - a name with an entry that resolves outside its networks gets `403`;
   - an answer with public and private addresses keeps only the allowed ones.

4. Some addresses are never reached through a name, even inside a declared
   network: loopback, every link-local address (all of `169.254.0.0/16`, so
   the cloud metadata address `169.254.169.254` and the ECS task address
   `169.254.170.2`, and `fe80::/10`), unspecified, multicast, and the cloud
   metadata addresses that lie inside private ranges, `100.100.100.200`
   (inside `100.64.0.0/10`) and `fd00:ec2::254` (inside `fc00::/7`). They
   stay reachable only as before, by listing the IP address itself in
   `outgoing.hosts`, or a localhost name for loopback. That is a separate,
   explicit declaration.

5. The first time the guard connects a host to a private address through its
   entry, it logs one JSON line:
   `{"address":"172.18.0.4","event":"handler.egress_private","host":"keycloak"}`.
   The `403` for a name that resolves only to refused addresses names
   `yon.outgoing.private` as the way to reach a private service.

6. This is a Yon setting only. Pages and native apps send their requests
   from the browser or web view, checked by `connect-src` (ADR 0068), not
   through the guard, so `tac.outgoing.hosts` does not change.

### Options considered

- A flag per host (`"private": true`) that allows any private address. It is
  shorter, but it lets a name reach every private range, for example `10/8`
  when the app only meant a Docker network in `172.18.0.0/16`.
- One list of networks for every listed host. A wildcard such as
  `*.example.com` could then be pointed at an internal service, which is the
  attack decision 4 stops.
- Entries written inside `outgoing.hosts` as objects. Every current reader
  of that list reads only text, so an older `ty` would skip the object
  without a word.
- A command-line flag or an environment variable. It is not visible in the
  repository, and it is easy to leave on in production.
- The IP address or a forwarder, as before. Both still work, but container
  addresses change and a forwarder is one more process in every image.

### Consequences

- One change in the guard covers every Yon language. The guard runs inside
  `ty`, and on macOS and Linux the operating system lets a handler process
  connect only to the guard and its resource door (ADR 0113); on Windows the
  proxy settings send every standard client to the guard. The confinement
  rules themselves do not change.
- Firecracker isolation keeps its own deny-only egress setting, and
  `ty handler invoke` still runs no guard.
- Each opted-in name can reach only its own networks. If the name's DNS is
  taken over, it can still reach any address inside those networks, so keep
  each list as narrow as the deployment allows, such as the one Docker
  network the service is on.
- Mesail can drop its `socat` forwarders and write the container names with
  their network in the profile it uses for Docker and Floci.

Tests: `handler::egress::egress_private_tests` (an opted-in name reached
through the guard on this machine's own private address; a listed name
without an entry refused; rebinding outside the networks, to loopback, to
link-local and to the metadata addresses refused), `safety::private_hosts_tests`
and the `outgoing_private` tests in `application::tests`.

