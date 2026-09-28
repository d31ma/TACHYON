# Tac Ruby WebAssembly runtime

This directory holds the source of the browser runtime for Tac Ruby services,
not Ruby annotation or service integration. See
[ADR 0049](../../docs/adr/0049-tac-ruby-uses-a-pinned-mruby-runtime.md).
The runtime embeds mruby and the Prism source compiler.

The built `tachyon-ruby.wasm` is not in git and not inside `ty`. `ty` carries
this source text and builds the runtime on the developer's machine the first
time a project with a Tac Ruby service builds. The result goes into the
per-user Tachyon cache at `ruby-web/<SHA-256 of these sources>/`, and later
builds reuse it. A project without Tac Ruby services never needs these tools.

## What the developer's machine needs

- Ruby, to run mruby's bundled build tool (`minirake`).
- Emscripten 6.0.10 or newer (`emcc`). 6.0.10 is the verified release; the
  version is a minimum, not an exact pin.
  - Official emsdk (macOS, Linux, Windows): `emsdk install latest`,
    `emsdk activate latest`, then `source ./emsdk_env.sh`, or `emsdk_env.bat`
    on Windows.
  - macOS with Homebrew: `brew install emscripten`.
- A C compiler for mruby's host tools: Xcode Command Line Tools on macOS, gcc
  on Linux, or RubyInstaller's MSYS2 DevKit (`ridk install`, `ridk enable`)
  on Windows.
- `curl` and `tar`, which ship with macOS, Linux and Windows 10 or newer.

`ty doctor` lists any missing or old tool with these install steps. The first
build takes several minutes, because mruby and Emscripten's own system
libraries compile once.

## How `ty` builds it

Every upstream archive in `sources.txt` is downloaded and checked against its
SHA-256 before it is unpacked. A mismatch deletes the download and stops the
build. The build uses mruby's bundled build tool rather than installed gems.
The small `build-config.rb.lock` is mruby's recorded version, not a package
manager resolver. The build does not fetch additional mrbgems. Keep
`THIRD_PARTY_NOTICES.txt` with every published runtime; `ty` publishes it
beside the runtime.

## Maintainer checks

`build.sh` runs the same build with bash, for maintainers who want to inspect
a build folder. `install-emscripten.sh` installs the Emscripten release, emsdk,
Node and Python archives pinned in `sources.txt`, verified by SHA-256; those
archives are for macOS on Apple silicon.

```sh
bash runtimes/ruby-web/install-emscripten.sh /path/to/empty/emsdk
source /path/to/empty/emsdk/emsdk_env.sh
TAC_RUBY_BUILD_JOBS=2 bash runtimes/ruby-web/build.sh /path/to/build-one
TAC_RUBY_BUILD_JOBS=2 bash runtimes/ruby-web/build.sh /path/to/build-two
cmp /path/to/build-one/tachyon-ruby.wasm /path/to/build-two/tachyon-ruby.wasm
node scripts/ruby-runtime-test.mjs /path/to/build-one/tachyon-ruby.wasm
```

The runtime test needs the repository's Playwright dependency, its Chromium
browser and CRuby 4.0.6 for the shared-language comparison. It checks the raw
4 MiB size budget, imports, exports, synchronous calls, JSON boundary cases,
error recovery, console and event delivery, denied host capabilities, and an
actual strict-CSP web page. The dedicated `Ruby Web Runtime` workflow runs the
two-build comparison and this test.

## Measurements

On 2026-09-26, two fresh builds with Emscripten 6.0.10 on macOS arm64 produced
the same 1,101,104-byte runtime; the local Node compression measurement was
432,524 bytes with gzip and 355,487 bytes with Brotli. Its test boot used
4 MiB of WebAssembly memory. Other Emscripten releases can produce different
bytes. Compression and memory figures are measurements, not upper bounds.

Console lines are limited to 65,536 UTF-16 code units per descriptor. A longer
line emits its prefix once with `[Ruby console line truncated]`, discards the
rest until the next newline, then resumes. UTF-8 decoding preserves split
characters across writes and keeps stdout/stderr separate. This bounds the
loader's partial-line storage, not the application's own console history.
