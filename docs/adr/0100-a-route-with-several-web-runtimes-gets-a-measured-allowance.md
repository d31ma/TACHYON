# ADR 0100: A Route With Several Web Runtimes Gets a Measured Allowance

- Status: Accepted
- Date: 2026-10-11
- Amends: [ADR 0074](0074-front-end-and-api-rules-are-build-errors.md)
  (route size budgets: the default script budget of a route that loads more
  than one web runtime)
- Keeps: [ADR 0093](0093-python-is-a-tac-service-language.md) (the 512 KiB
  script budget of a route with a Python service)

## Context

ADR 0074 gives every route a script budget of 200 KB as sent (`TY1337`).
The budget counts every `.js` and `.mjs` file that the route loads. A
service in Rust, Go, C#, F#, Dart, Kotlin, Swift, C++, Java, Ruby or Lua
reaches the page through the web runtime of its language: a shared module
such as `tachyon-go.js`, and for some languages more script. A route that
uses services in several of these languages loads several web runtimes.

The browser gates load many runtimes on one route on purpose. On macOS all
three were over the 200 KB budget; on Windows cached-client was over and
window-controls was just under (all sizes are script as sent):

| Gate | Host | Runtimes on the route | Script as sent |
| --- | --- | --- | --- |
| cached-client | Windows 11 arm64 | .NET, Rust, Go, Dart | 236,287 bytes (computed budget 308 KB) |
| window-controls | Windows 11 arm64 | .NET, Rust, Go, Dart | 202,215 bytes (computed budget 308 KB) |
| cached-client | macOS arm64 | .NET, Rust, Ruby, Java, Go, Dart, Kotlin, Swift | 261 KB (computed budget 352 KB) |
| window-controls | macOS arm64 | .NET, Rust, Ruby, Java, Go, Dart, Kotlin, Swift | 219 KB (computed budget 352 KB) |
| storage | macOS arm64 | .NET, Rust, Ruby, Java, Go, Kotlin, Swift | 269 KB (computed budget 344 KB) |

The Windows route totals are from the runtime-size commit (7a12dd7c0), and
the Windows .NET (about 89 KB) and Rust (about 87 KB) figures below are the
owner's Windows measurement of the same build; that host has no Ruby,
Swift, Kotlin, TeaVM or Binaryen. The macOS numbers are from
release builds of develop 667ba797 with every macOS language, Binaryen and
TeaVM.

Most of that script is runtime, not application code, and part of it cannot
be made smaller inside Tachyon:

- .NET ships prebuilt runtime scripts (`dotnet.runtime.js`,
  `dotnet.native.js` and others). Tachyon publishes them as the .NET SDK
  writes them.
- A Rust service module is WebAssembly that Tachyon embeds in
  `tachyon-rust.js` as text (ADR 0035), so the whole Rust module, with the
  Rust behavior runtime in it, counts as script. Shipping it as a separate
  binary file is a separate decision and is not part of this one.
- The other runtimes add a small shared module and glue.

ADR 0074 had two answers, and both were wrong. Raising `tac.budgets.script`
for the whole project also raises the budget of every single-language
route of that project. Turning the check off (`tac.budgets.check: false`)
removes the check from every route. The owner chose (2026-10-11) a budget
that grows only with the runtimes that a route really loads.

## Decision

### What counts as a web runtime

A route loads a web runtime when the files that the route loads (ADR 0074,
section 2) include the shared module of that runtime in
`.tachyon/services/`:

| Runtime | Shared module | Allowance |
| --- | --- | --- |
| .NET (C# and F#) | `tachyon-csharp.js` | 96 KB |
| Rust | `tachyon-rust.js` | 92 KB |
| Ruby | `tachyon-ruby-services.js` | 20 KB |
| Lua | `tachyon-lua-services.js` | 12 KB |
| Java | `tachyon-java.js` | 12 KB |
| Go | `tachyon-go.js` | 8 KB |
| Dart | `tachyon-dart.js` | 8 KB |
| Kotlin | `tachyon-kotlin.js` | 8 KB |
| C++ | `tachyon-cpp.js` | 4 KB |
| Swift | `tachyon-swift.js` | 4 KB |

- JavaScript and TypeScript are not runtimes. They are the page itself, and
  the 200 KB base already pays for `tac-client.js` and the page's own
  scripts.
- C# and F# share one runtime: both build into the same .NET program and
  the same `tachyon-csharp.js`. A route with C# and F# services loads one
  runtime.
- A runtime counts only when the route loads it. A runtime that another
  route uses, or that the build wrote but no page loads, does not count.
- Python is not in the table. ADR 0093 already gives a route with a Python
  service its own script budget (512 KiB), and that rule does not change.

### How the budget is computed

When `tac.budgets.script` is not set, the script budget of a route is:

> 200 KB + the allowances of every runtime that the route loads,
> except the runtime with the largest allowance.

- A route with no runtime or one runtime gets 200 KB, exactly as before.
  That one runtime is paid for by the base, as it always was.
- Each further runtime adds its own allowance from the table. When two
  runtimes have the same allowance, the table order decides which one the
  base covers; the total is the same either way.
- On a route with a Python service, the budget is the larger of the ADR
  0093 budget and this one.
- The WebAssembly budget (`tac.budgets.webassembly`, 3584 KB) does not
  change. WebAssembly files already have their own category.

The allowances are fixed numbers in the compiler, not sizes measured during
the build. A size measured during the build would let any runtime grow
without anyone noticing. A runtime that grows past its allowance fails the
gates, and the allowance changes only with a new measurement in this ADR.

How the allowances were set (bytes as sent, Brotli, from the release
builds above):

Each allowance is the largest script that the runtime added in any
measured build, plus 5 percent, rounded up to the next multiple of 4 KB.
The script of a runtime is its shared module and the files that only it
loads; the small module of each service (about 0.5 KB) is application code
and is not counted.

| Runtime | Files counted | Largest measured | Allowance |
| --- | --- | --- | --- |
| .NET | `tachyon-csharp.js`, `dotnet.js`, `dotnet.native.*.js`, `dotnet.runtime.*.js` | 86,401 bytes on macOS; about 89 KB on Windows | 96 KB |
| Rust | `tachyon-rust.js` (the module as hexadecimal text) | 86,917 bytes on macOS (storage); about 87 KB on Windows | 92 KB |
| Ruby | `tachyon-ruby-services.js`, `ruby/tachyon-ruby.js` | 18,025 bytes (storage) | 20 KB |
| Lua | `tachyon-lua-services.js`, `lua/tachyon-lua.js` | 9,698 bytes (`tac-lua-null-arguments` fixture) | 12 KB |
| Java | `tachyon-java.js`, `java/runtime.js` | 8,949 bytes | 12 KB |
| Go | `tachyon-go.js`, `go/wasm_exec.js` | 5,962 bytes | 8 KB |
| Dart | `tachyon-dart.js`, `tachyon-dart-loader.js` | 4,296 bytes | 8 KB |
| Kotlin | `tachyon-kotlin.js`, `kotlin/services-loader.js`, `kotlin/services.js` | 4,958 bytes | 8 KB |
| C++ | `tachyon-cpp.js` | 3,250 bytes (`tac-cpp-services` fixture) | 4 KB |
| Swift | `tachyon-swift.js` | 3,695 bytes | 4 KB |

The Rust and Ruby numbers grow with the application, because the Rust module
and the Ruby program are part of their script. Their allowances cover the
gate applications; a larger Rust or Ruby application uses the room that the
base leaves for application code, as on a single-language route.

### Overriding the budget

`tac.budgets.script` in `client/manifest.json` (kilobytes) still sets the
script budget of every route of the project. When it is set, it wins: no
route gets an allowance, a Python route does not get ADR 0093's budget, and
a multi-runtime route has exactly the configured budget.
`tac.budgets.check: false` still turns the check off. `ty preview` still
does not check budgets.

### The error

The `TY1337` error of a route that got an allowance names the computed
budget and the runtimes that it counted, for example:

> Route '/' loads 303 KB of script, and its budget is 300 KB. It is 200 KB
> for the page and its largest web runtime, .NET, plus 92 KB for Rust,
> 8 KB for Go (ADR 0100).

The error of a route without an allowance does not change.

## Consequences

- This is not a loosening for single-language apps. A route that loads no
  runtime or one runtime keeps 200 KB, its message does not change, and the
  budget-refusal controls of the cached-client, window-controls and storage
  gates still fail an oversized single-language script with exactly one
  `TY1337` at 200 KB.
- The space for application script on a route stays the same: each further
  runtime adds about what that runtime itself costs, so the application
  code of a multi-language route gets no more room than on a
  single-language route.
- The browser gates keep the budget check on, and the computed budget
  covers them. The website keeps its own `tac.budgets.script` of 320.
- A runtime that grows past its allowance fails the gates. Then either the
  runtime gets smaller or this ADR records a new measurement.
- When Tachyon ships the Rust module as a binary file, or trims the .NET
  runtime scripts, the allowance of that runtime goes down by the same
  amount.
