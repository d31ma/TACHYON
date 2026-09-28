# ADR 0049: Tac Ruby Uses a Pinned mruby Runtime

- Status: Accepted runtime decision; amended 2026-09-27 (the runtime is built
  on the developer's machine, not committed or embedded)
- Date: 2026-09-26
- Preserves: client-only Tac rendering, local application events, the 4 MiB
  service-worker precache budget, and explicit host capabilities

## Context

The approved Ruby plan uses CRuby for Yon and needs a small browser runtime
for Tac. The evaluated full ruby.wasm artifact was about 17.2 MB uncompressed,
over the existing precache limit. PicoRuby's evaluated npm distribution was
smaller, but had no provenance attestation and included an untagged mruby
revision. Its scheduler was also unnecessary for synchronous Tac calls.

## Decision

Tachyon maintains its own small mruby WebAssembly runtime. It compiles Ruby
source using the embedded Prism compiler. Browsers and native hosts need no
Ruby installation to run it.

The inputs in `runtimes/ruby-web/sources.txt` pin mruby and Prism by commit
and archive SHA-256. The same file records the Emscripten release the runtime
was verified with, and the emsdk, Node and Python archives that
`install-emscripten.sh` checks for maintainers on macOS with Apple silicon.

The browser ABI is synchronous: `tac_boot`, `tac_invoke`, `tac_alloc` and
`tac_free`. Each module instance owns its mruby state. Boot accepts Ruby source;
invoke passes a JSON string to the composed Ruby dispatcher. Answers are
UTF-8, zero-terminated, allocated strings that the caller frees. A null boot
answer means success. The `tachyon.tac_host_emit` import passes JSON event text
to the host synchronously. Annotation parsing, provider construction, owner
lifecycle and the page event bus remain framework responsibilities; this
artifact does not implement those policies.

The runtime has no filesystem, sockets, process launcher or environment
access. Its nine imports provide console output, clock, empty environment,
memory growth and event delivery. The loader requires only same-origin script
and fetch access plus CSP `wasm-unsafe-eval`; it needs neither `unsafe-eval`
nor inline scripts. Ruby evaluation happens inside WebAssembly, not through
JavaScript eval.

The built runtime must remain below 4 MiB uncompressed. Notices for mruby,
Prism, Emscripten and linked C runtime components travel beside it.

## Amendment 2026-09-27: the developer's machine builds the runtime

The first version committed the 1.1 MB `tachyon-ruby.wasm` and embedded it in
the `ty` binary. That put a binary in git and in every `ty`, even for projects
without Ruby. Tachyon uses every other WebAssembly toolchain the same way: it
finds and drives the developer's own tools (the Rust `wasm32` target, Go's
`wasm_exec.js` from `GOROOT`, the .NET `wasm-tools` workload, the Dart SDK).
The Ruby runtime now follows that rule.

- Only the runtime's source lives in the repository. `ty` carries that source
  text (`sources.txt`, the build configuration, the `tachyon-web` gem and the
  JavaScript loader), not the binary. The binary is ignored by git.
- The first build of a project with a Tac Ruby service builds the runtime into
  the per-user Tachyon cache (`ty cache` prints its root):
  `ruby-web/<SHA-256 of the runtime source>/tachyon-ruby.wasm`. Later builds
  reuse it. A change to the runtime source makes a new folder. Nothing is
  written into the project's source tree.
- Rust drives the build on macOS, Linux and Windows without a shell script.
  It downloads each pinned tarball with `curl`, checks its SHA-256, and
  unpacks it with `tar`; both tools ship with these systems. A hash mismatch
  deletes the download and stops the build. Then it runs mruby's own build
  tool with the developer's `ruby`, and links with the developer's `emcc`.
- A project with a Tac Ruby service needs Ruby, Emscripten and a C compiler
  for mruby's host tools. A project without one needs none of them.
  `ty doctor` and the build report a missing or old tool in plain English,
  with install steps (emsdk, Homebrew `emscripten`, Windows emsdk, and
  RubyInstaller's DevKit for the C compiler on Windows).
- Emscripten has a minimum version, not an exact one: 6.0.10, the release
  this runtime was verified with, or newer. No known break exists on newer
  releases. Different Emscripten releases can produce different bytes, so the
  runtime no longer has a single byte-level SHA-256 pin; the cache key is the
  source hash, and the supply-chain pins stay on the downloaded sources.
- The first build takes several minutes (mruby and Emscripten's own system
  libraries compile once). CI installs the pinned Emscripten release where
  Ruby Tac tests run; local tests that need the runtime skip with a clear
  reason when it is neither cached nor buildable.

## Differences a Ruby developer will meet

- Tac uses mruby, not CRuby. It supports the Ruby language and selected
  standard classes, not the complete Ruby standard library or gems.
- `require` accepts the built-in `json`, `set` and `time` names. Other names
  and `require_relative` raise `LoadError`. There is no filesystem loader.
- `File`, `IO`, `Socket` and `ENV` are absent. `STDOUT`, `STDERR`, `puts`,
  `print` and `warn` write to the host console instead.
- The loader bounds each partial console line to 65,536 UTF-16 code units,
  reports truncation once, and discards the remaining line until a newline.
  Split UTF-8 characters survive chunked writes. Host console retention is
  the caller's responsibility.
- Time uses UTC in this browser build. Host locale/time-zone databases are
  not included.
- Error wording and backtraces can differ from CRuby. Code should compare
  error classes, not exact English text.
- Examples of missing methods are `String#unicode_normalize`, `String#sum`,
  `Hash#compare_by_identity` and `Float#rationalize`. This is not an exhaustive
  compatibility list.
- JSON parsing and generation cover ordinary JSON values, integer precision,
  symbolized keys and bounded nesting. This is not the complete CRuby JSON
  gem option API. JSON generation rejects nonfinite numbers.
- Calls run on the browser's thread. `sleep` is refused. A long computation
  can freeze the page; this runtime is not a CPU-time sandbox. Memory is
  WebAssembly-isolated but grows as needed. The recursion limit is 10,000;
  strings and arrays are limited by available memory rather than the smaller
  default mruby limits.
- Ruby code is interpreted after source compilation. It is not expected to
  equal CRuby or native compilation performance. The original local probe
  measured roughly 19 ms to its first call; this is evaluation evidence,
  not a cross-machine performance guarantee.

## Trust and qualification

Building directly from pinned upstream source removes the npm publisher from
the runtime's build path. It does not establish independent upstream review,
signed provenance or absence of vulnerabilities. Updating any input requires a
reviewed pin update, two identical clean builds with the same Emscripten, and
the runtime tests.

`scripts/ruby-runtime-test.mjs` executes the same safe Ruby expressions in
CRuby and mruby, then tests a built runtime in Chromium under strict CSP.
It checks synchronous calls, error recovery, events and the capability
boundary. Annotation, routing, model and full service integration must pass
their own gates before Ruby is called a supported Tachyon service language.

Sources: [mruby](https://github.com/mruby/mruby),
[Prism](https://github.com/ruby/prism),
[emsdk](https://github.com/emscripten-core/emsdk), and
[GitHub runner architectures](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
