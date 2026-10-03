# ADR 0090: Compiled Dart HTTP Clients Await the Browser Host

- Status: Accepted
- Date: 2026-10-01
- Relates to ADR 0033 (Dart WebAssembly), ADR 0084 (page placement), ADR 0087 (injected HTTP and caching)

## Decision

A compiled browser Dart HTTPClient uses the default same-origin transport when
its Setup has no matching HTTP adapter. Its existing Future method awaits a
separate asynchronous browser host. Ordinary synchronous APIs keep their
current host ABI, and a matching custom Setup adapter takes precedence.
No worker placement or cross-origin-isolation headers are required.

The generated method converts the JSON answer with the existing typed model
reader. Invalid shape failures become BehaviorError with code HTTP_RESPONSE.
Request methods, path variables, query, headers and optional JSON bodies use
the existing HTTP interface annotations. A service receives the client through
Needs or NeedsAll; application services do not call fetch themselves.

The host accepts at most 65,536 UTF-8 request bytes, 64 concurrent requests,
a 60-second maximum deadline and 1 MiB of response data. It refuses another
origin, redirects, cookies supplied as headers and unsafe request headers.
Fetch uses credentials omit. Ordinary requests use no-store; Cached public
GETs use default cache and the first read after a successful clear/update
uses no-cache. A request with Authorization uses no-store. Failed mutations
do not invalidate a successful cached read.

AbortController implements deadlines and cancellation. Each compiled service
captures its own BehaviorSignal through the existing owner factory. A Dart
zone carries that owner across Future continuations and late initialization;
removing the owner disposes its contexts and aborts outstanding fetches.
Another service owner keeps its own signal. A failed request returns a bounded
error code without raw response text or host exception details.

Native Dart is not an available native companion target. This decision does
not install a browser transport in Yon or bypass native HTTP refusals. Existing
custom language adapters remain available in their supported environments.

## Scope and evidence

The executable website example is
`website/content/docs/calling-the-server/browser-http-dart/`.
`scripts/browser-http-browser-test.mjs` compiles it once and serves its actual
Yon controllers. It checks all three required browser engines.
`scripts/browser-http-runtime-test.mjs` directly tests the production async
host boundaries. Focused Rust tests also execute the actual Dart VM runtime.
These checks passed in Chromium, Firefox and WebKit when this ADR was accepted.

This is one qualified language slice of BROWSER-HTTP-TRANSPORT. Eight other
compiled language continuations, TestSetup/Fake replacement, self-built-client
diagnostics/migration and the deferred first-app tabs remain pending. The
next step is staged async interop that keeps each runtime on the page. C# and
F# Task interop come next; each other language needs its own decision.
The existing sync Rust/Swift ABI cannot serialize a Promise into a response,
and Binaryen 133 Asyncify rejects references live across the import. This ADR
does not choose a global worker architecture or change authored async contracts
for the other languages.


## Bounded .NET browser extension — 2026-10-03

C# Task ownership, default HTTP and captured registered scalar record results
are qualified in compiler27. Native result restrictions remain unchanged.
The cumulative C# first-app candidate now passes three actual authored SDK
cases and an ordinary post-Test .NET Wasm build with the existing Yon endpoint
in Chromium, Firefox and WebKit. Canonical tutorial files are identical to
those actual staged sources. A selected genuine TestSetup may construct its
stateless helper adapter; production intake validates that ownership, then
omits helpers reachable only from the test. This adds no managed helper
lifetime or injection contract.

F# uses the same .NET host for browser `Task` and `Task<BehaviorJSON>` results.
Its genuine ordinary Wasm/Yon proof passes all three browser engines: default
HTTP, explicit custom adapter precedence, unsafe-path refusal, disposal of a
pending request, captured stale-owner refusal, a working replacement and zero
late changes. Raw helper markers are validated before F# annotation lowering;
helpers remain explicitly constructed and caller-owned.

The actual debug compiler source is
`5d3d4a728f8cb361bc7e6e9fd486563608e38f05`; its production changes are rooted
in `2a4aeac3cb5dfe62eac3f3fc26bb3e55255ddd51`. The F# fixture constants are
saved separately in `8445c01943758b6475fbbb843f85f1d1706320b8`. External workspace
receipts are `FIRST-APP-CSHARP-QUALIFICATION.json`,
`FIRST-APP-CSHARP-CANONICAL-SOURCE.json`, `FSHARP-HTTP-QUALIFICATION.json` and
`QUEUE28-SOURCE-CONTROLS.json`. Canonical CLI coverage and coherent optimized
compiler28/final aggregate/website validation remain pending. Compiler27 is
still the latest complete qualified handoff.

This extension does not qualify F# DTO/list/ValueTask results, native async,
async lifecycle adapters, shared compiled page providers or F# cache behavior.
It adds no new browser worker placement or whole transport-matrix acceptance.
