# ADR 0007: TypeScript Companion Emission

- Status: Accepted
- Date: 2026-07-27

## Context

`tac.ts` is a legacy source convention. To emit it, Tachyon must turn
TypeScript into JavaScript, because a browser cannot read type annotations.
Type *checking* is a separate problem. `ty` does not claim to check types.
This repository already checks types with its own typecheck step.

We measured four options instead of estimating them. The workspace has 194
crates today. Either in-process transpiler would roughly double the dependency
tree, and the code to audit, for one file type.

| Option | New Rust crates | Licensing change |
| --- | --- | --- |
| `OXC` | ~115 | add `ISC` and `Apache-2.0 WITH LLVM-exception` |
| `SWC`, which Deno uses | ~171 | similar |
| Node `stripTypeScriptTypes` | 0 | none |
| The TypeScript compiler | 0 | none |

Deno keeps the two problems apart. It transpiles with `SWC` in Rust. It checks
types by embedding the real `tsc` in a V8 snapshot. Only the transpilation part
applies here.

## Decision

Emit `tac.ts` by running the TypeScript compiler, version 6 or newer, as a
bounded external process.

`ty` looks for the compiler in the project first. So `node_modules/.bin/tsc`
wins over `PATH`, and a pinned project dependency decides the version.

Emission is hermetic: nothing outside the command changes the output.

- `--ignoreConfig` with an explicit `--target es2022 --module esnext` means a
  project's `tsconfig.json` cannot change the output.
- `ty` passes `--noCheck`, because emission must not depend on whether a
  project passes type checking.
- The subprocess runs in the project root and never through a shell.
- `ty` checks the compiler's major version before it uses the compiler.

## Consequences

Benefits:

- There is no new Rust dependency and no licensing change.
- The semantics do not differ at all. The reference implementation defines the
  semantics. So enums, parameter properties and future syntax are correct by
  design. We do not reimplement them.
- The decision fits the existing architecture. It already supervises `node`,
  `python3`, `swiftc`, `cc`, `gradle` and `mingw`.
- TypeScript 6 and 7 are the Go-native compiler, so one file emits in about
  70 ms.

Costs:

- To build a project that uses `tac.ts`, TypeScript 6 or newer must now be
  installed. Version 6 is the first version that accepts `--ignoreConfig`. We
  checked this against 5.6, 5.9, 6.0.3 and 7.0.2.
- This is a new toolchain requirement at build time. It applies only to
  projects that use the convention.
- Without the compiler, the build fails closed: it stops with `TY1009`, and
  the message names the fix.
- Emission starts one process per TypeScript companion.

## Rejected Alternatives

`SWC` or `OXC` in process would avoid the external toolchain and would run
even faster. But each one roughly doubles the dependency tree for one file
type. Each one is also a reimplementation, so it can behave differently from
the reference compiler. Look at this option again if the external requirement
becomes a real problem.

Node's `stripTypeScriptTypes` costs nothing, and `yon.js` already requires
Node. But the API is explicitly experimental. Its strip-only mode also rejects
`enum` and parameter properties completely.

We rejected a hand-written type stripper completely. TypeScript syntax is
large, so a custom stripper would be wrong in small, hard-to-see ways. This
project can least afford to be wrong in that way while it claims to be
correct.

## Acceptance Gate

- [x] `tac.ts` emits JavaScript with types erased and enums desugared.
- [x] The generated document references the emitted module.
- [x] A compiler older than version 7 reports a version requirement rather
      than an unknown-option error.
- [x] An absent compiler fails closed with `TY1009` and names the remedy.
- [x] Emission ignores a project's `tsconfig.json`, so output is deterministic
      across projects.
