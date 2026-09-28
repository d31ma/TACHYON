# ADR 0001: Rust Core and Runtime Boundaries

- Status: Accepted
- Date: 2026-07-26

## Context

Today one Bun-based implementation of Tachyon does compilation, server
lifecycle, runtime behavior, native generation and packaging. This work started
over from nothing (a "greenfield" rewrite). It chose a language without
assuming any existing runtime.

The core needs:

- predictable performance;
- strong types;
- memory safety;
- native artifacts;
- controlled concurrency;
- direct access to platform toolchains.

Application handlers must stay polyglot. The core must still not depend on
every supported language runtime.

## Decision

Use Rust for:

- the compiler;
- the route graph;
- the server;
- process supervision;
- packaging;
- the cache;
- diagnostics;
- observability;
- native render planning.

Use one Cargo workspace, organized as a modular monolith: one program made of
separate modules. The modules talk to each other through typed in-process
contracts. Do not add internal microservices unless a measurement shows a need
for isolation or for independent scaling.

Application language runtimes stay supervised adapters behind Handler
Protocol v1. The Rust core does not embed Bun. It does not assume that Node,
Python, Ruby, PHP or another runtime is installed, unless a project selects
that adapter.

Pin exact stable Rust security releases. Phase 0 uses Rust 1.97.1. Forbid
`unsafe` in the whole workspace. A small isolated exception is allowed only
after a reviewed safety requirement proves that it is necessary.

## Consequences

- The core build and test path needs only Rust.
- Language adapters can change without changes to compiler internals.
- Runtime compatibility is an explicit support matrix.
- Native platform bindings may later need isolated FFI (foreign function
  interface) crates and safety ADRs.
- The current JavaScript implementation stays a reference for expected
  behavior (a behavior oracle). It is not a runtime dependency of the core.

## Rejected Alternatives

- **TypeScript/Bun core**: it is the fastest way to port the source. But it
  keeps the runtime and deployment assumptions that the greenfield rewrite
  removed.
- **Go core**: it is simple to operate. But it suits compiler IR
  (intermediate representation), deterministic low-level platform integration
  and tightly controlled FFI less well.
- **C++ core**: it has mature platform access. But its default memory safety
  and dependency hygiene are much weaker.
- **Internal services per subsystem**: they add deployment and failure
  boundaries, and no measurement shows a need for them.

## Acceptance Gate

- the workspace builds on exact stable Rust;
- core crates have no application runtime dependency;
- the dependency direction is documented and has no cycles;
- tests can check every public boundary without loading legacy
  implementation code.
