# ADR 0090: Compiled Dart HTTP clients await the browser host

- Status: Accepted Dart contract; other compiled language continuations pending
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
See `docs/evidence/BROWSER-HTTP-DART.md` for results and limitations.

This is one qualified language slice of BROWSER-HTTP-TRANSPORT. Eight other
compiled language continuations, TestSetup/Fake replacement, self-built-client
diagnostics/migration and the deferred first-app tabs remain pending. The
bounded design is `docs/rfc/browser-http-async-continuations.md`; the remaining
contract decision is recorded in `docs/queue/NEXT/BROWSER-HTTP-ASYNC-CONTRACT.md`.
The existing sync Rust/Swift ABI cannot serialize a Promise into a response,
and Binaryen 133 Asyncify rejects references live across the import. This ADR
does not choose a global worker architecture or change authored async contracts
for the other languages.
