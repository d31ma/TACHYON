# ADR 0003: Supervised Handler Process Protocol

- Status: Accepted
- Date: 2026-07-26

## Context

Developers may write Yon handlers in several languages. Tachyon needs:

- one server lifecycle;
- stable cancellation;
- bounded resources;
- errors that tell the developer what to fix;
- the same behavior across languages.

The core must get all of this without embedding every language runtime.

Newline-delimited protocols are simple. But with them, framing, oversized
messages and recovery from protocol errors break more easily. Calling a
handler through a shell adds injection risks and quoting differences between
platforms.

## Decision

Run language handlers as supervised child processes. The server starts each
process directly.

Handler Protocol v1 uses:

- a four-byte unsigned big-endian frame length. A Lua handler's frames carry
  the length as eight lowercase hex digits instead, because Lua cannot put
  its standard streams in binary mode, and Windows text mode would rewrite a
  binary length's 0x0A, 0x0D and 0x1A bytes. Amended in WG1.
- one UTF-8 JSON envelope per frame;
- a maximum frame of 16 MiB, unless a future protocol version negotiates
  another limit;
- stable request IDs;
- explicit envelope kinds for request, response and cancellation;
- stdout only for the framed protocol, and stderr for bounded diagnostics;
- a supervisor that owns deadlines, cancellation, concurrency and lifecycle.

Never call a handler through a shell. Pass the executable and each argument as
separate operating-system values. Pass on only the inherited environment
variables that an allowlist names.

A route may find several `yon.*` handlers at the same level. It lists them in
deterministic source order. Each invocation selects one handler and one HTTP
method. Tachyon does not compose a build-time route context (ADR 0016).

## Consequences

- Every adapter can run the same contract corpus against real processes.
- The server stays isolated from handler crashes and protocol violations.
- Large streaming bodies need a future streaming extension. Unbounded frames
  are not the answer.
- Each language is explicitly responsible for its own runtime discovery and
  packaging.
- Process startup cost must be solved with pooling that is measured. Hidden
  global processes are not allowed.

## Rejected Alternatives

- **Embed every runtime**: it gives attackers more code to target. It also
  ties each release to several language engines.
- **Shell scripts as the common boundary**: they are unsafe and depend on the
  platform.
- **NDJSON (newline-delimited JSON) without length framing**: recovery is
  weaker and size limits are harder to enforce.
- **Last handler wins during context merge**: the result is nondeterministic
  and hard to audit.

## Acceptance Gate

- the Handler Protocol v1 schema passes meta-validation and rejects an invalid
  fixture;
- the threat model covers framing, resource exhaustion, environment leakage,
  cancellation and process reaping;
- future adapters must pass the same corpus against real child processes.
