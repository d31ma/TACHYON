# ADR 0113: The Operating System Confines Handler Egress

- Status: Accepted
- Date: 2026-10-31
- Amends: [ADR 0068](0068-safety-rules-are-build-errors.md) (the outgoing
  request guard), [ADR 0094](0094-local-host-names-and-host-validation.md)
  (local host names) and
  [ADR 0014](0014-environment-selected-yon-isolation.md)
  (environment-selected isolation)

## Context

A server started by `ty serve`, `ty start` or `ty preview` runs an outgoing
request guard: a loopback HTTP proxy that lets a handler reach only the hosts
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
| macOS | Seatbelt: the child starts through `/usr/bin/sandbox-exec` with a generated profile | `(allow default)`, `(deny network*)`, then outbound TCP to `localhost:<guard>` and `localhost:<resource>` only. Inherited by every child process. |
| Linux | Landlock (ABI 4 or newer, Linux 6.7+) plus a seccomp filter, applied by `ty` itself just before it starts the handler (below) | Landlock allows TCP `connect` only to the two ports and refuses every TCP `bind`. seccomp refuses `socket` for every family except IPv4 and IPv6 TCP sockets and netlink, and refuses `io_uring_setup`, so UDP, ICMP, raw, packet, SCTP and Unix sockets cannot be opened. Both are inherited across `fork` and `exec` and set `no_new_privs`. |
| Windows | None available to an unprivileged process | Fails closed (below). |

The Linux and macOS layers are built once per server, when the guard starts,
and applied to every handler spawn: cold, warm, streaming, Gateway class units
and Gateway commands.

### Fail closed in production

When the guard is active and the platform layer cannot be applied (no
`sandbox-exec`, a Linux kernel without Landlock ABI 4, a Landlock or seccomp
call that fails, or Windows):

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
- It holds at most 256 connections at once and closes a tunnel that has
  carried nothing for 5 minutes.
- An accept error no longer ends the guard: it is logged once per kind as
  `handler.egress_accept_failed`, and the loop waits 100 ms and goes on
  (SRE-6).
- The guard needs no per-run credential: a confined handler can reach only
  its own server's guard, and an unconfined local process can already reach
  the declared hosts itself. Several runtimes, Ruby among them, also ignore
  proxy credentials in the environment.
- It still allows any port on a declared host. A port list needs a
  `host:port` form in `outgoing.hosts`, which is a configuration change of its
  own (follow-up).

### Linux launcher

A process confines only itself with Landlock and seccomp, and the workspace
forbids `unsafe` code, so Tachyon cannot apply them between `fork` and `exec`.
Instead, a confined handler starts as `ty __yon-egress <guard> <door> --
<program> <arguments>`. That mode applies Landlock and the seccomp filter to
itself and then replaces itself with the program (`exec`), so the handler
keeps the same process, process group and environment. A Tachyon library used
from another program finds `ty` next to that program.

### Preludes

The PHP prelude sets the default stream context to the guard, so
`file_get_contents` and `fopen` on `http://` and `https://` go through it. The
Go prelude already sends `http.DefaultTransport`, `http.DefaultClient` and
every clone of the default transport through the guard. These keep working
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

## What it does not cover

- Landlock filters by port, not address. A handler written to attack can open
  TCP to a remote host on the guard's or the resource door's port number.
  Both are random per server; a remote listener on every port still catches
  it. The network-namespace layer closes this.
- Mach and XPC services on macOS are not restricted. A service that makes
  network requests for its caller could carry data out.
- On Linux, a Unix socket inherited from the parent is not covered by the
  seccomp `socket` rule. Tachyon passes handlers only their three standard
  streams.
- On Linux, a 32-bit or x32 program is stopped by the seccomp architecture
  check rather than confined.
- Filesystem, CPU and memory confinement are unchanged (process mode is still
  supervision, not a sandbox for hostile code).
