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

This decision covers Dart. The other compiled languages, TestSetup and Fake
replacement, migration help for clients a service builds itself, and the
remaining first-app tabs are not covered yet. The
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

## Rust browser extension — 2026-10-03

Rust uses its own async pattern. An HTTPClient trait method is an
`async fn` that returns `BehaviorResult<T>`. `T` is `BehaviorValue`, `()`,
a model from `client/models`, a list of models, an `Option` or a plain value.
Tachyon writes the method. A Setup adapter of the same name answers first;
otherwise the request goes to the same browser host as Dart and C#, with the
same limits, omitted credentials and cache rules. A `Cached` miss sends a
public read; the first miss after a successful `ClearCache` or `UpdateCache`
revalidates. An answer of the wrong shape fails with `HTTP_RESPONSE`.
`Retry`, `ConcurrencyLimit` and `AllOrNothing` are refused on async client
methods for now.

A service gets the client with `#[Needs] name: std::sync::Arc<Client>`; its
other fields start at their `Default` value. A service method may be
`async fn` and return `()`, a crossing type or `BehaviorResult<T>`; an error
reaches the page as a rejected promise with the error code. The WebAssembly
module keeps the pending futures in a small executor. The page starts and
cancels fetches through `tac_host_http_start` and `tac_host_http_cancel`,
settles them through the `tac_http_settle` export, and learns that a task
ended through `tac_host_task_done`. A pending method holds its service
exclusively: until it ends, the page reads the fields as they were when the
method started, and other calls, field writes and events to that service run
after it, in order. Disposing the owner drops its tasks first, which aborts
their fetches and rejects their promises with `CANCELLED`, and then runs the
PreDestroy hooks.

In a Tac test unit, `self.construct()` builds a service through the
constructor that `Needs` produces. A Fake field
`std::cell::RefCell<Option<std::sync::Arc<Client>>>` supplies the client; while
it is empty, the service gets a client bound to the test's BehaviorContext, so
the TestSetup adapter answers. Test cases may be `async fn`. A test unit and a
native Rust companion have no browser transport: a request without an adapter
fails with `HTTP_MISSING`. `ty migrate check` reports a service that builds
its client itself (`service.builds_client`).

Evidence: `scripts/browser-http-browser-test.mjs --language=rust` builds
`website/content/docs/calling-the-server/browser-http-rust/` and checks it in
Chromium, Firefox and WebKit; `client_tests::rust_construct_tests` runs the
Fake and typed-answer cases natively.

## Go browser extension — 2026-10-03

Go keeps its own pattern: a call that waits blocks its goroutine. An
HTTPClient interface method returns `(Model, error)` or
`(BehaviorValue, error)`; the answer is read with `encoding/json` into the
model from `client/models`, and a model answer that is not an object fails
with `HTTP_RESPONSE`. A service gets the client with `// @Needs` above a field
such as `greetingClient *GreetingClient`.

The page calls every Go service member through a new goroutine. When the call
finishes without waiting, the answer is returned at once, as before. When the
call reaches the browser transport, the goroutine releases the service lock,
starts the fetch through the same host as the other languages and waits on a
channel; the page gets `{"task": id}` and a promise that settles when the
goroutine ends. While a Go call waits, other calls to the service run normally,
as in JavaScript. Disposing the owner aborts its pending fetches, and the
waiting call gets `CANCELLED`. A call that waits from a getter, a setter or
`OnStart` is refused with `HTTP_REQUEST`. The same browser host rules apply
(same origin, omitted credentials, `Cached` public reads, revalidation after
`ClearCache` or `UpdateCache`).

In a Go test unit, `test.construct(service)` fills the `Needs` fields from the
`// @Fake` fields of the test; an empty Fake client field gets a client bound
to the test's BehaviorContext (`NewGreetingClientWith`), so the TestSetup's
`Exchange` answers. Tests have no browser transport (`HTTP_MISSING`).
Coverage records no calls inside generated clients.

Evidence: `scripts/browser-http-browser-test.mjs --language=go` builds
`website/content/docs/calling-the-server/browser-http-go/` and checks it in
Chromium, Firefox and WebKit; `client_tests::go_construct_tests` runs the Fake
and typed-answer cases.

## Kotlin browser extension — 2026-10-04

A Kotlin HTTPClient is an interface whose methods are `suspend fun` without a
body. Tachyon writes each method: it goes through the behavior context's
suspend path (`invokeAwait` for reads, `mutateAwait` for `ClearCache` and
`UpdateCache`) and reads the answer with `Behaviors.answer` into the declared
type: a data class from `client/models`, a list, a plain value, `Any?` or
`Unit`. A model answer that is not an object, or a list answer that is not a
list, fails with `HTTP_RESPONSE`. Errors are `Behaviors.Failure` values whose
`code` names the problem. Retry, concurrency limits, transactions, `Check`
and `Pageable` are refused on suspend client methods. The Setup's adapter
answers first; without one, the request goes to the same browser host as the
other languages (same origin, omitted credentials, `Cached` public reads,
revalidation after a successful `ClearCache` or `UpdateCache`). The generated
client has two constructors: `Client()` builds its context from its Setup, and
`Client(context)` uses a given BehaviorContext.

A service gets the client with `@Needs private val name: Client`; on the page
Tachyon fills the field with `Client()`. A service method may be a
`suspend fun`. The Kotlin/Wasm module starts it with the standard library's
`startCoroutine`; when it finishes without waiting, the page gets the answer
at once. When it waits for the browser transport, the module keeps the
continuation, starts the fetch through `httpStart` on the Kotlin host, and the
page gets `{"task": id}` and a promise. The page settles the fetch through the
`tacHttpSettle` export, the module resumes the continuation with the service
as the executing owner, delivers its events and reports the end through
`taskDone`. A waiting method does not hold its service: other calls, field
reads and writes run while it waits, as in JavaScript and Go. Disposing the
owner drops its waiting continuations first, which aborts their fetches and
rejects their promises with `CANCELLED`, and then runs the PreDestroy hooks.
An Android companion has no browser transport: a suspend method answers
`HTTP_MISSING`.

In a Kotlin Tac test unit, `construct(Service::class)` calls the constructor
that `Needs` produces. A Fake field `var name: Client? = null` supplies the
client; while it is `null`, the service gets `Client(behaviors)`, bound to the
test's BehaviorContext, so the TestSetup adapter answers. Test cases may be
`suspend fun`; Tachyon runs them to the end and refuses a case that still
waits. Test units run on the JVM, so a request without an adapter fails with
`HTTP_MISSING`.

Evidence: `scripts/browser-http-browser-test.mjs --language=kotlin` builds
`website/content/docs/calling-the-server/browser-http-kotlin/` and checks it in
Chromium, Firefox and WebKit; `client_tests::kotlin_construct_tests` runs the
Fake and typed-answer cases.
