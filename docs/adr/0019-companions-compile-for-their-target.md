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

## Amendment (2026-10-04): Rust companion crates, platform crates and helper executables

A real desktop app needs more than the standard library in its Rust
companion: `serde_json`, `libc` on Linux, `windows-sys` on Windows. It may
also need small extra programs, for example a hook command that another
tool starts. Before this amendment, apps built the staged `companion.rs`
again with their own Cargo project. Now the client manifest declares both,
and Tachyon builds them.

### Declaration

`tac.application.rust` in `client/manifest.json`:

```json
"rust": {
  "dependencies": {
    "serde_json": "=1.0.151",
    "serde": { "version": "=1.0.229", "features": ["derive"] }
  },
  "platformDependencies": {
    "windows": { "windows-sys": { "version": "=0.61.2", "features": ["Win32_Foundation"] } },
    "linux": { "libc": "=0.2.189" }
  },
  "executables": { "observatory-event": "/shared/native/EventEmitterMain.rs" }
}
```

- A dependency is an exact version (`=MAJOR.MINOR.PATCH`), or an object
  with `version`, optional `features` and optional `defaultFeatures`.
  Exact versions keep the build equal to the vendored copy. Git, path and
  registry fields are refused; the manifest check (`TY1601`) names the
  field.
- `platformDependencies` uses the Tachyon target names `linux`, `macos` and
  `windows`. They become Cargo `cfg(target_os = "linux")`,
  `cfg(target_os = "macos")` and `cfg(windows)` tables, so one declaration
  serves every desktop.
- An executable name uses lowercase letters, digits and `-`. Its value is a
  client Rust class file (at most eight executables). The class has
  `pub fn run() -> i32`; Tachyon writes a `main` that calls it and exits
  with its result. The file is an ordinary client Rust file and follows the
  usual file rules. It reaches companion items through `crate::models::...`
  and `crate::<Type>` (every `pub` item of the companion is re-exported for
  the executables).
- The declaration applies to the native desktop hosts. It needs at least
  one client Rust service, because the executables link the companion
  library. iOS and Android keep their WebAssembly route, and a Rust service
  that a browser build compiles to WebAssembly must not use a declared
  crate.

### Offline build from vendored crates

Like Go in ADR 0095 (amendment 2026-10-03), Tachyon never downloads a
crate.

- Vendored crates live in `vendor/rust/` at the project root (beside
  `client/`), as written by `cargo vendor`. They are outside `client/`, so
  the web build, the snapshot and the file rules never read them.
- With a declaration, the host build stages `project/rust-companion/Cargo.toml`
  (library `tachyon_companion` from `project/companion.rs`, one `[[bin]]`
  per executable, the declared tables, an empty `[workspace]`) and runs
  `cargo build --release --offline` with `net.offline=true` and
  `source.crates-io.replace-with` pointing at `vendor/rust/`. The Cargo
  target folder is in the Tachyon cache, not in `dist/`.
- When crates are declared and `vendor/rust/` is missing, the build stops
  before Cargo and prints the command to run once:
  `cargo vendor --manifest-path <staged Cargo.toml> vendor/rust`. A crate
  that the folder does not provide fails in Cargo's offline resolver.
- Without a declaration nothing changes: the companion is one file built by
  `rustc`.

### Placement

Executables go beside the host executable, so the companion finds them at
`std::env::current_exe()?.parent()`:

| Host | Main executable | Helper |
| --- | --- | --- |
| Linux | `<App>/bin/<App>` | `<App>/bin/<name>` |
| Windows | `<App>\bin\<App>.exe` | `<App>\bin\<name>.exe` |
| macOS | `<App>.app/Contents/MacOS/<App>` | `<App>.app/Contents/MacOS/<name>` |

On macOS each helper is signed ad hoc before the bundle is signed. Helpers
are ordinary programs: Tachyon does not register, install or start them,
and creates no service, login item or elevated helper.

## Amendment (2026-10-19): Rust links on iOS (decision D1)

Decision D1 of the platform-gaps plan chose to link Rust on iOS and Android,
so a Rust service gets the same native reach on mobile as on the desktops.
This amendment covers iOS; Android follows in its own amendment.

- `tac.rs` now reaches macOS, Windows, Linux and iOS. The selection rule does
  not change: a service with both a Swift and a Rust variant still links
  Swift on iOS, and a Rust-only service now links Rust there instead of
  running as WebAssembly in the web view.
- The iOS host links the companion as a static library, with the same C ABI
  and the same Swift shim as the macOS host (`tac_native_invoke`,
  `tac_native_free`, `tac_native_set_emit`,
  `tac_native_set_capability_adapter`). Swift and Rust services can share
  one app, as on macOS.
- The library is built for `aarch64-apple-ios` in a `--device` or
  `--release` build, and for `aarch64-apple-ios-sim` (`x86_64-apple-ios` on
  an Intel Mac) in a simulator build. When that Rust standard library is
  missing, the build stops with `TY1605` and names the
  `rustup target add` command.
- `tac.application.rust.dependencies` now apply to the iOS app too, and
  `platformDependencies` gains an `ios` table (`cfg(target_os = "ios")`).
  `executables` stay refused on iOS (`TY1601`), because an iOS app may not
  start other programs.
- An application-lifetime Rust service is still desktop-only (ADR 0097);
  the iOS app does not start one yet.
