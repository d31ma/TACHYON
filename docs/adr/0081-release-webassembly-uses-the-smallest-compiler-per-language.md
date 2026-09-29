# ADR 0081: Release WebAssembly Uses the Smallest Compiler per Language

- Status: Accepted
- Date: 2026-09-28
- Extends: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md)
  (Rust), [ADR 0045](0045-go-is-a-service-language.md) (Go),
  [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md) (C# and
  F#), [ADR 0033](0033-dart-browser-services.md) (Dart)
- Reuses: the Binaryen runner that [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md)
  added for Kotlin's production build
- Works with: [ADR 0074](0074-front-end-and-api-rules-are-build-errors.md)
  (route size budgets) and
  [ADR 0075](0075-release-builds-precompress-and-the-server-negotiates.md)
  (Brotli and gzip files), which now measure the smaller modules

## Context

A web service in Rust, Go, C#, F#, Kotlin, Dart or Ruby reaches the browser
as WebAssembly. Before this decision each language used one compiler setting
for every build. Most of those settings were chosen to make the build work,
not to make the download small:

- Rust used `opt-level=s` and no post-link pass.
- Go used the standard Go compiler, whose module carries the whole Go runtime
  (about 2.7 MB before compression for a small service).
- C# and F# trimmed the framework only partly.
- Dart used `dart compile wasm -O2`.

The download is the largest cost of a WebAssembly service on a slow phone.
Release builds should use the smallest correct output, and development builds
should stay fast.

## Decision

### Every language, release builds only

`ty build` and `ty bundle` are release builds. `ty preview` builds with the
development profile and keeps the fast settings below. After the
language's own compiler, a release build runs Binaryen's `wasm-opt` on the
module when the language needs it (the `WasmOptimizer` type from ADR 0078).
When `wasm-opt` is not installed, the build still succeeds and the build
output says the module is larger and how to install Binaryen. `ty doctor`
lists `wasm-opt` as an optional tool.

Every release build prints one line per language that says which compiler
it used and why, for example
`Go: TinyGo (the smallest download), then Binaryen wasm-opt`.

| Language | Release compiler and settings | Development build |
| --- | --- | --- |
| Rust | rustc `opt-level=z`, LTO, one codegen unit, `panic=abort`, symbols stripped, then `wasm-opt -Oz` | rustc `opt-level=s`, no `wasm-opt` |
| Go | TinyGo `-target wasm -opt=z -no-debug -gc=boehm`, then `wasm-opt -Oz`; the standard Go compiler plus `wasm-opt -O2` when TinyGo is missing or cannot build the services | the standard Go compiler |
| C# and F# | the .NET interpreter with IL trimming, invariant globalization and Emscripten `-Oz` for the native runtime; AOT when `tac.dotnet.aot` is true | no native relink |
| Kotlin | Kotlin/Wasm production build: Binaryen with Kotlin's own flags (ADR 0078) | no Binaryen |
| Dart | `dart compile wasm -O4` (dart2wasm runs Binaryen itself) | `-O2` |
| Ruby | the pinned mruby runtime, unchanged (ADR 0049 / ADR 0054) | the same |

### Go: TinyGo first, the standard compiler as the fallback

- A release build compiles every Go service with TinyGo first. TinyGo leaves
  out most of the Go runtime, so the module is many times smaller.
- If TinyGo is not installed, or it cannot compile the services (for
  example a package or `//go:linkname` that TinyGo does not support), the
  build compiles them again with the standard Go compiler and prints why.
  The build never fails only because TinyGo failed.
- `tac.go.compiler` in `client/manifest.json` forces one compiler for every
  build: `"tinygo"` (no fallback, also in development) or `"go"` (never
  TinyGo). Any other value is a build error that names the two values.
- Each compiler ships its own `wasm_exec.js` glue. TinyGo's glue schedules a
  sleeping goroutine with `setTimeout`, so a synchronous call from the page
  would return before a `time.Sleep` inside it finished. Tachyon changes the
  shipped TinyGo glue so that such a call finishes its sleeps before it
  returns, which is what the standard Go runtime does. The change looks for
  exact lines of TinyGo 0.42's glue; if a different TinyGo changed them, the
  build stops with a message instead of shipping wrong glue.
- TinyGo builds with the Boehm garbage collector (`-gc=boehm`), not its
  default precise collector. TinyGo's JavaScript glue starts a new goroutine
  for every call from the page, and each goroutine gets its own 64 KB stack
  on the heap, so every call leaves about 64 KB of garbage. The precise
  collector then runs every few calls and sweeps the whole heap (it grew to
  6 MB) each time: most calls took 25 us, but one call in four or five took
  3 to 5 ms (p95 3.4 ms, 13 ms at 4x CPU throttling). Boehm sweeps lazily,
  so a collection costs about as much as the live data. Measured with
  `scripts/wasm-worker-measure.mjs --runtimes go` (ADR 0084), 3 runs,
  medians: p95 3,395 us to 765 us at 1x and 12,805 us to 2,045 us at 4x,
  mean 837 us to 101 us; the module grows by 47 KiB raw and 14 KiB with
  Brotli (about 15%), and a page that makes thousands of calls holds a few
  MB more memory. The alternatives were worse: the conservative collector
  pauses the same, and a smaller goroutine stack (`-stack-size`) removes the
  pauses only by lowering the recursion a service may use; at 16 KB a
  62-level JSON request already overflowed the stack and hung the page.
- Development builds use the standard Go compiler because it compiles in
  about a second. A service that works in development can still meet a
  TinyGo limit in release; the fallback covers compile errors, and
  `tac.go.compiler = "tinygo"` lets a developer test with TinyGo every time.

### C# and F#: the trimmed interpreter, AOT as an opt-in

- The default is the trimmed .NET interpreter. It is the smallest
  download and fast enough for state and logic services.
- `tac.dotnet.aot = true` in `client/manifest.json` compiles the services
  ahead of time (`RunAOTCompilation`, `WasmStripILAfterAOT`). It runs heavy
  computing faster but makes the download larger and the build much slower.
- Tachyon does not guess which services are CPU-heavy. The release note and
  the docs tell the developer when to turn AOT on.
- The .NET runtime files keep their integrity hashes, so Tachyon does not run
  its own `wasm-opt` on them; Emscripten's `-Oz` link step does that work.

### Dart: -O4

`-O4` lets dart2wasm drop runtime type checks and assume that no type error
happens. Development builds keep `-O2` with the checks, so a type error shows
up while developing.

### Toolchains

Tachyon is tested with Binaryen version_133 and TinyGo 0.42.0. Both are
optional: without them a release build is larger, never broken. `ty doctor`
lists them when a project has a Rust or Go web service, and `ty init` offers
to install them into the home folder like the other toolchains: Binaryen
from its GitHub release, checked against the release's `.sha256` file, and
TinyGo from its GitHub release, checked against a SHA-256 that Tachyon pins
(TinyGo publishes no checksum file). TinyGo has no Windows on Arm download,
so there `ty init` shows the install page instead.

## Measurements

Measured on the same small service (a tally with one field and two methods,
the Web Assembly services example), release build before and after this
decision. Sizes are the WebAssembly the page downloads (for Rust, the module
that the page embeds), raw and after Brotli level 11.

| Language | Before, raw | After, raw | Before, Brotli | After, Brotli | Smaller (Brotli) |
| --- | --- | --- | --- | --- | --- |
| Rust | 142.1 KiB | 109.2 KiB | 56.7 KiB | 48.6 KiB | 14% |
| Go | 3.09 MiB | 308.0 KiB | 678.5 KiB | 98.0 KiB | 86% |
| C# | 2.87 MiB | 2.83 MiB | 943.0 KiB | 938.7 KiB | under 1% |
| Dart | 47.0 KiB | 45.6 KiB | 16.8 KiB | 16.5 KiB | 2% |
| Kotlin | 293 KB | 101 KB | | | (ADR 0078) |

For the whole Tachyon website, a release build publishes 8.7 MB instead of
11.7 MB, and sends 2.25 MB instead of 2.86 MB with Brotli. Its Go module goes
from 3.2 MB (684 KiB as sent) to 318 KiB (101 KiB as sent).

Later, TinyGo moved to the Boehm garbage collector (see the Go section above).
The tally's Go module is now 356.5 KiB raw and 112.7 KiB with Brotli (309.2 KiB
and 98.3 KiB with TinyGo's default collector on the same day), still 83%
smaller with Brotli than the standard Go compiler's.

Go gains the most: TinyGo leaves out most of the Go runtime. The .NET
browser runtime was already trimmed before this decision, and Emscripten
`-Oz` takes about 37 KB off the native runtime. Other .NET switches did not
help: the event source, debugger and resource-key feature switches did not
change the files that .NET 9 publishes, and `TrimMode=full` removed members
that a website C# service reaches only through reflection, so its calls
failed. Tachyon sets none of them. Ruby and Kotlin did not change here.

## Consequences

- Release pages download less WebAssembly for every language that changed.
- Release builds take longer: TinyGo and `wasm-opt` add seconds, and AOT adds
  minutes when a project turns it on. Results are cached per compiler
  version, settings and source, so a rebuild with no change is fast.
- Go services can behave differently under TinyGo (reflection and some
  packages are limited). The fallback handles compile errors; runtime
  differences are covered by the Go web test suite running under TinyGo.
- `ty doctor` shows TinyGo and wasm-opt as optional tools.
