# ADR 0019: Companions Compile for Their Target, Not to WebAssembly

- Status: Accepted; companions belong to components since [ADR 0029](0029-pages-compose-components.md); superseded by [ADR 0030](0030-components-use-services.md), under which the target matrix applies per service in `client/services/`; amended by [ADR 0031](0031-realms-mirror-each-other.md), which mirrors those shared role folders in the server realm; amended by [ADR 0033](0033-dart-browser-services.md), under which Dart is a browser service language compiled to JavaScript; amended by [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md), under which a Rust-only service reaches the web as WebAssembly; amended by [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md), under which a C#-only one does too
- Date: 2026-08-30
- Reconciles: the unshipped companion decision dated 2026-08-23
- Supersedes: [ADR 0011](0011-wasm-companion-abi.md)

## Context

The WebAssembly companion ABI (application binary interface) gave compiled
languages a browser sandbox. The newer local architecture uses compiled page
companions for a different purpose: to reach the native platform. A browser
sandbox cannot give that access. Requiring five WebAssembly toolchains does
not solve the problem.

## Decision

Components use ordinary JavaScript or TypeScript modules. Compiled companions
beside a page use their language's native toolchain and run inside the host.

| Companion | Targets |
| --- | --- |
| `tac.js`, `tac.ts` | Web and all native web views |
| `tac.rs` | macOS, Windows, Linux |
| `tac.swift` | macOS, iOS |
| `tac.kt` | Android |
| `tac.cs` | Windows |

### Selection rules

- The table in `project.rs` is the source of truth for compilation and
  diagnostics.
- When more than one native companion reaches a target, the platform's own
  language wins over Rust.
- Selection happens per route. The compiler builds only the selected
  companion.
- A route may have companions, but none that reaches the requested target.
  Then the build fails with TY1010. It does not publish a page with its
  behavior silently left out.

### Protocol

The JSON operations remain `init`, `get`, `set`, and `call`. Initialization
describes fields and methods. It does not invoke application methods.

Rust exposes `tac_native_invoke` and `tac_native_free` through a C ABI. Hosts
written in a native language invoke their same-language companion directly.
The compiler generates a namespace and a dispatch table for each route. They
prevent name collisions and keep route state separate.

## Migration and consequences

- A browser component with `tac.rs`, `.swift`, `.kt`, `.cs`, or `.dart` must
  move platform work to a native page companion. Its view-facing module must
  stay in JavaScript or TypeScript. Old WebAssembly components receive a
  diagnostic that tells the author what to do. The compiler does not ignore
  them.
- Dart has no native target under this contract. Dart is not a companion
  language.
- Native companions have the application's operating-system privileges. They
  are not sandboxed plugins. Do not compile untrusted companion source.
- Browser modules keep normal module behavior, including imports, helpers, and
  constants. Native member discovery is not a reason to restrict them.
- Native compile and execute gates, plus browser-to-host protocol tests,
  replace the former WebAssembly gates. Removing the old gates alone does not
  count as qualification.
- Existing published versions and tags do not change. This behavior ships only
  in the new reconciliation release, with an explicit migration notice.
