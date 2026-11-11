# ADR 0113: The Operating System Confines Handler Egress

- Status: Accepted
- Date: 2026-10-31
- Amends: [ADR 0068](0068-safety-rules-are-build-errors.md) (the outgoing
  request guard), [ADR 0094](0094-local-host-names-and-host-validation.md)
  (local host names) and
  [ADR 0014](0014-environment-selected-yon-isolation.md)
  (environment-selected isolation)

## Context

A server that runs handlers, started by `ty serve`, `ty start` or
`ty preview`, runs an outgoing request guard: a loopback HTTP proxy that lets a handler reach only the hosts
in `outgoing.hosts` (ADR 0068). Until now the guard was advisory. Tachyon set
the proxy variables, and a handler that honoured them went through the guard.
A handler that did not reached the network directly:

- a raw socket in any language;
- a Go transport other than `http.DefaultTransport`;
- PHP stream wrappers, which ignore the proxy variables;
- a Gateway command, which got no proxy variables at all;
- any non-HTTP client.

The release review (SEC-2) found that the guard could therefore be skipped by
any handler that wanted to. The release decision (D2) is that the operating
system must enforce the declared hosts, and must fail closed where it cannot.

Two facts shape the mechanism:

- A handler must still reach two loopback ports: the guard, and its resource
  endpoint (storage, topics, Gateways). Until now each invocation bound its
  own resource listener on a new port, so a warm process, started before its
  requests, could not know the ports it would need.
- Tachyon runs as the developer's or the operator's unprivileged user. It
  cannot rely on root, a system extension, a firewall service or a kernel
  setting that needs an administrator.

## Decision

### One rule

A handler process, every process it starts (relays), and every Gateway
command can open a network connection only to the guard port and the resource
port of its own server, on loopback. Everything else is refused by the
operating system: other loopback ports, private and public addresses, UDP
(including DNS), ICMP, and local daemons reached through Unix sockets. The
guard then applies `outgoing.hosts`, so the declared hosts are the only way
out.

### One resource port per server

The server binds one resource door on loopback when it starts. Every
invocation still gets its own listener and its own bearer token, but its
`YON_RESOURCE_ENDPOINT` names the door. The door reads the request head,
finds the invocation by its token, and forwards the connection to that
invocation's listener, which checks the token again. An unknown token gets
`401`. So the endpoint format the runners parse is unchanged, and a warm
process knows both ports when it starts.

### The mechanism per platform

| Platform | Mechanism | What it enforces |
| --- | --- | --- |
| macOS | Seatbelt: the child starts through `/usr/bin/sandbox-exec` with a generated profile | `(allow default)`, `(deny network*)`, then outbound connections to `localhost:<guard>` and `localhost:<resource>` only. It also denies Apple Events and the LaunchServices and background URL session services, which could open a connection on the handler's behalf. Inherited by every child process. |
| Linux | Landlock (ABI 4 or newer, Linux 6.7+) plus a seccomp filter, applied by `ty` itself just before it starts the handler (below) | Landlock allows TCP `connect` only to the two ports and refuses every TCP `bind`. seccomp refuses `socket` for every family except IPv4 and IPv6 TCP sockets and netlink, so UDP, ICMP, raw, packet, SCTP and Unix sockets cannot be opened. It also refuses `io_uring_setup`, and any send with `MSG_FASTOPEN`, because TCP Fast Open connects without the `connect` call that Landlock checks. Both are inherited across `fork` and `exec` and set `no_new_privs`. |
| Windows | None available to an unprivileged process | Fails closed (below). |

The Linux and macOS layers are built once per server, when the guard starts,
and applied to every handler spawn: cold, warm, streaming, Gateway class units
and Gateway commands.

### Fail closed in production

When the guard is active and the server finds at startup that the platform
layer cannot be applied (no working `sandbox-exec`, a Linux kernel without
Landlock ABI 4, a seccomp filter that cannot be built, no `ty` program to
launch handlers with, or Windows):

- `ty start` does not start. It reports `TY2113` with the reason.
- `ty serve` and `ty preview` start, and log a `handler.egress_unenforced`
  warning with the reason. They are development servers that run the
  developer's own code, and refusing them would stop development on Windows
  and on older Linux kernels.

An operator who enforces egress elsewhere, such as a serverless platform's
network policy, a VPC egress rule or a container network policy, sets
`YON_EGRESS_ENFORCEMENT=proxy` in the environment of `ty`. Like
`YON_ISOLATION`, it is a variable of the parent process only: no project file
can set it, and handlers never receive it (`YON_*` is reserved). With it,
`ty start` runs the guard as before and logs `handler.egress_unenforced` once
at startup, and the risk is the operator's. The default value is `os`; any
other value fails with `TY2113`.

### Firecracker

The Firecracker transport (ADR 0014) is unchanged. The control program is a
trusted component with its own network policy, and Tachyon already refuses
any egress setting other than deny. Tachyon does not wrap the control program
in the platform layer.

### The guard itself

- It refuses the special-use ranges the address filter missed (SEC-5): NAT64
  `64:ff9b::/96` and `64:ff9b:1::/48`, 6to4 `2002::/16`, Teredo `2001::/32`,
  IPv4-compatible `::a.b.c.d`, discard `100::/64`, documentation
  (`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`),
  benchmarking `198.18.0.0/15`, IETF protocol assignments `192.0.0.0/24`,
  reserved `240.0.0.0/4`, and multicast. As before, only an IP address listed
  in `outgoing.hosts`, or a localhost name, reaches such an address.
- It holds at most 256 connections at once, answers `503` to the next one,
  and closes a tunnel that has carried nothing for 5 minutes. The resource
  door has the same limits.
- An accept error no longer ends the guard: it is logged once per kind as
  `handler.egress_accept_failed`, and the loop waits 100 ms and goes on
  (SRE-6).
- The guard needs no per-run credential: a confined handler can reach only
  its own server's guard, and an unconfined local process can already reach
  the declared hosts itself. Several runtimes, Ruby among them, also ignore
  proxy credentials in the environment.
- It still allows any port on a declared host. A port list needs a
  `host:port` form in `outgoing.hosts`, which is a configuration change of its
  own (follow-up). The release that adds it also revisits the per-run
  credential. Both are known limits of the release candidate
  ([threat model](../THREAT_MODEL.md)).

### Linux launcher

A process confines only itself with Landlock and seccomp, and the workspace
forbids `unsafe` code, so Tachyon cannot apply them between `fork` and `exec`.
Instead, a confined handler starts as `ty __yon-egress <guard> <door> --
<program> <arguments>`. That mode applies Landlock and the seccomp filter to
itself and then replaces itself with the program (`exec`), so the handler
keeps the same process, process group and environment. Neither wrapper
looks the program up itself: `sandbox-exec` and the launcher find it inside
the confined process, with the handler's own `PATH` and working directory,
so a program that is not found never runs unconfined. On Linux the launcher
is the running program when it is `ty`, and otherwise the `ty` in its folder
or the folder above it (a test binary). If Landlock or seccomp still fails
when a handler starts, that handler exits with status 126 and a
`handler.egress_refused` event, under every server kind.

### Preludes

The PHP prelude sets the default stream context to the guard, so
`file_get_contents` and `fopen` on `http://` and `https://` go through it. The
Go prelude already sends `http.DefaultTransport`, `http.DefaultClient` and
every clone of the default transport made after startup through the guard; a
clone made in a package-level variable or `init` keeps the standard proxy
rule, which skips loopback. These keep working
clients working. They are not the control: a client that ignores the guard,
such as a Go transport built from scratch, is refused by the operating system.

## Consequences

- A handler can no longer make a non-HTTP connection, such as its own database
  driver. Managed storage goes through the resource endpoint, which keeps
  working. An application that needs another protocol must use the opt-out and
  enforce egress itself.
- Name resolution inside a handler fails. A client sends the host name to the
  guard, which resolves it once (ADR 0068), so a proxied client is unaffected.
- On macOS the profile adds about 10 ms to a process start, which warm
  processes pay once.
- On Windows, `ty start` needs `YON_EGRESS_ENFORCEMENT=proxy`. A Windows layer needs a Windows Filtering Platform filter, which needs an
  administrator once; it is follow-up work.
- On Linux kernels older than 6.7, gVisor and most serverless runtimes,
  `ty start` needs the opt-out. A network-namespace layer for those kernels is follow-up work.
- `ty handler invoke` runs no guard, and so no confinement: it is a development
  tool that runs the developer's own code.
- `ty test` runs its Yon test units without a guard as well, so they are not
  confined either. Starting the guard there, as `ty start` does, is follow-up
  work; until then it is a known limit of the release candidate
  ([threat model](../THREAT_MODEL.md)).

## What it does not cover

- Landlock filters by port, not address. A handler written to attack can open
  TCP to a remote host on the guard's or the resource door's port number.
  Both are random per server; a remote listener on every port still catches
  it. The network-namespace layer closes this.
- On macOS, Mach and XPC services other than LaunchServices and background
  URL sessions are not restricted. Another service that makes network
  requests for its caller could carry data out.
- On Linux, a Unix socket inherited from the parent is not covered by the
  seccomp `socket` rule. Tachyon passes handlers only their three standard
  streams.
- On Linux, a 32-bit (i386 or arm32) program is stopped by the seccomp
  architecture check rather than confined. x32 calls share the x86_64
  architecture value, so the filter lists the x32 numbers of the calls it
  refuses.
- Filesystem, CPU and memory confinement are unchanged (process mode is still
  supervision, not a sandbox for hostile code).

## Amendment (2026-11-11): one checked request per plain HTTP connection

### Context

For a plain `http://` request, a client sends the whole request to the
guard in absolute form (`GET http://host:port/path HTTP/1.1`). The guard
checked the host of the first request, connected to it, and then only copied
bytes. Clients keep proxy connections open and reuse them for any host: Go's
`http.Transport`, Python's urllib3 and Node.js all pool plain HTTP proxy
connections by the proxy's address, not by the target. So a later request on
the same connection went to the first request's host. The Mesail rewrite saw
a mail-out `POST /jobs/deliver` answered by Keycloak's 404 page, a realtime
push answered by the JWKS server, and a DNS check answered `503`. It is also
a security fault: the guard never checked the later request's host, and that
request, with its credentials, reached a service it was not meant for.

Two designs were possible:

1. The guard reads every request on the connection, checks each target, and
   routes each one to its own upstream connection. That makes the guard a
   full HTTP/1.1 proxy: it must frame every request and every answer
   (`Content-Length`, chunked, close-delimited, `HEAD`, `204`, `304`, `1xx`)
   to know where one exchange ends and the next begins. A framing mistake is
   itself a way to send bytes to the wrong host.
2. The guard carries exactly one request on each plain HTTP connection, and
   tells both ends that the connection closes after it.

### Decision

The guard uses design 2. For a plain HTTP request it:

- reads and checks the request head as before, and refuses a host that
  `outgoing.hosts` does not list;
- sends the upstream service the head in origin form with `Host` set to the
  checked authority from the request target, and with `Connection: close`.
  It removes the client's `Host`, `Connection`, `Keep-Alive`, `Upgrade` and
  every `Proxy-*` field. A client can no longer name another virtual host on
  the same address through `Host`;
- forwards exactly one request body, framed by `Content-Length` or by
  `Transfer-Encoding: chunked`. It answers `400` to a request whose body
  length is unclear: both fields at once, a final coding other than
  `chunked`, Content-Length values that differ or are not digits, a field
  name that is not a token, or a folded field line;
- passes `1xx` answer heads (other than `101`) through unchanged, and sends
  the final answer head with `Connection: close` in place of the service's
  `Connection` and `Keep-Alive` fields, so the client does not put the
  connection back in its pool;
- closes the whole connection when the client sends any byte after the
  first request. A second request on the same connection, kept-alive or
  pipelined, never reaches any service; the client gets the connection
  closed and opens a new one, which the guard checks again;
- half-closes the upstream connection once the client has closed its side,
  so a service that ignores `Connection: close` does not hold a guard slot
  until the 5-minute idle limit.

`CONNECT` tunnels are unchanged: a tunnel is bound to its checked host, and
clients pool tunnels by target host.

### Consequences

- Every language is fixed in the guard; no prelude changes. Go's
  `http.Transport`, Dart's `HttpClient`, Python's urllib3 and the Node.js and
  Bun clients pool plain HTTP proxy connections across hosts; Java's and
  Kotlin's `HttpURLConnection` and PHP's streams do not. An HTTP/1.1 client
  does not reuse a connection whose answer says `Connection: close`
  (RFC 9112), so it opens a new proxy connection for the next plain HTTP
  request. The Go and JavaScript handler tests in `outgoing_loopback` call
  two kept-alive services in turn through the guard.
- Each plain HTTP request opens a new loopback connection to the guard and a
  new upstream connection. HTTPS keeps its pooled tunnels.
- A plain HTTP upgrade (`ws://` sent in absolute form) no longer works
  through the guard: the guard removes `Upgrade`. WebSocket clients send
  `CONNECT` through a proxy, which still works.
- HTTP pipelining through the guard ends the connection after the first
  answer. No common client pipelines.
- The regression tests `a_kept_alive_proxy_connection_cannot_carry_a_request_to_a_second_host`,
  `a_pipelined_second_request_never_reaches_the_upstream_service`,
  `a_chunked_request_reaches_its_own_host_and_the_answer_closes_the_connection`,
  `a_plain_request_body_must_have_one_clear_length` and
  `a_request_with_two_body_lengths_is_refused` keep this contract.
- `CONNECT` tunnels still trust the client to send the checked host in TLS:
  the guard does not read the TLS server name. That limit is unchanged.
