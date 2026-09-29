# ADR 0084: WebAssembly Services Stay on the Page Thread Until .NET Needs a Worker

- Status: Accepted (the decision is to keep every WebAssembly service on the page thread for now; the worker design below is the plan for when .NET needs a worker)
- Date: 2026-09-29
- Relates to: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md)
  (the `invoke(op, json)` bridge), [ADR 0037](0037-csharp-services-reach-the-web-as-webassembly.md)
  (C# and F#), [ADR 0033](0033-dart-browser-services.md) (Dart),
  [ADR 0045](0045-go-is-a-service-language.md) (Go),
  [ADR 0049](0049-tac-ruby-uses-a-pinned-mruby-runtime.md) (Ruby),
  [ADR 0078](0078-kotlin-services-reach-the-web-as-webassembly.md) (Kotlin),
  [ADR 0082](0082-swift-services-reach-the-web-as-webassembly.md) (Swift),
  [ADR 0081](0081-release-webassembly-uses-the-smallest-compiler-per-language.md)
  (release compilers)

## Context

Yon handlers run in their own processes. Tac WebAssembly services do not:
every language runtime (Rust, Go through TinyGo, C# and F# through .NET,
Dart, Ruby through mruby, Kotlin, Swift) starts on the page's main thread,
and every call is a synchronous `invoke(op, json)` through the bridge in
`tac_web.rs`. A view reads a field with one `get` call and calls methods
inside expressions (`{quantity.describe(...)}`, `:if="pager.next()"`), and
the answer comes back in the same task.

The question: should each language runtime run in its own Web Worker, the
way Yon handlers run in processes? A worker keeps the page responsive while a
runtime starts or computes, but it makes every call asynchronous and moves
the runtime away from `localStorage`, the Clipboard, Share and Geolocation.
This ADR measures first and decides second. It changes no runtime behavior.

## Measurement

`scripts/wasm-worker-measure.mjs` builds one small fixture route per
language with the release `ty` (each route holds one service with a `count`
field, an `increment()` method and a `spin(rounds)` loop), serves it
cross-origin isolated, and drives full Chromium in new headless mode through
Playwright. Each row is the median of 3 runs, each run in a fresh browser,
with CPU throttling off (1x) and at 4x. It measures:

1. Long tasks (over 50 ms, `PerformanceObserver` `longtask`) while the
   route loads, until one second after the service is ready.
2. One typical call: `increment()` and one read of `count`, 280 calls after
   20 warm-up calls, mean, p50 and p95 (timer resolution 5 us).
3. Input delay while the service computes for 200 ms: the time from the
   input event (`event.timeStamp`) to its handler, for a click and a key
   press 30 ms after the work starts.
4. Memory: `performance.measureUserAgentSpecificMemory()` after load (it
   counts WebAssembly memory).
5. A worker prototype, not production code: a module worker imports the
   same built service module, answers `{op, name, args}` messages with the
   value and a copy of every field, and the page times the round trip
   (structured clone of the request and the answer).

It also loads five website routes (release build of `website/`) and splits
long animation frame script time by runtime.

Run it again:

```sh
TAC_BIN=/absolute/path/to/release/ty node scripts/wasm-worker-measure.mjs \
  --out /some/folder [--runs 3] [--throttles 1,4] [--runtimes rust,go,...] \
  [--site /built/website/folder] [--routes /,/docs] [--reuse-build]
```

It needs the Playwright Chromium build (`npx playwright install chromium`)
and every language toolchain that the chosen runtimes need; results go to
`<out>/results.md` and `<out>/results.json`.

Environment: Apple M4, 10 cores, 16 GB; Chromium 151.0.7922.34; release
`ty` from `swarm/integration` f908e4cf. The run held the Mac build lock, so
no other agent was building; the desktop (a video call, the Claude app) kept
the load average between 2.8 and 7.2.

### Start-up on the page thread (fixture routes)

| runtime | CPU | ready ms | long tasks | blocking ms | longest task ms | memory MB |
| --- | --- | --- | --- | --- | --- | --- |
| JavaScript | 1x | 62 | 0 | 0 | 0 | 1.4 |
| Rust | 1x | 67 | 0 | 0 | 0 | 2.7 |
| Go (TinyGo) | 1x | 77 | 0 | 0 | 0 | 2.2 |
| C# (.NET) | 1x | 132 | 1 | 5 | 55 | 34.9 |
| F# (.NET) | 1x | 133 | 1 | 4 | 54 | 34.9 |
| Dart | 1x | 68 | 0 | 0 | 0 | 1.5 |
| Ruby (mruby) | 1x | 81 | 0 | 0 | 0 | 5.6 |
| Kotlin | 1x | 66 | 0 | 0 | 0 | 1.5 |
| Swift | 1x | 86 | 0 | 0 | 0 | 3.2 |
| JavaScript | 4x | 109 | 0 | 0 | 0 | 1.4 |
| Rust | 4x | 128 | 0 | 0 | 0 | 2.7 |
| Go (TinyGo) | 4x | 133 | 0 | 0 | 0 | 2.2 |
| C# (.NET) | 4x | 387 | 2 | 111 | 137 | 34.9 |
| F# (.NET) | 4x | 386 | 2 | 108 | 132 | 34.9 |
| Dart | 4x | 123 | 0 | 0 | 0 | 1.5 |
| Ruby (mruby) | 4x | 172 | 1 | 14 | 64 | 5.6 |
| Kotlin | 4x | 124 | 0 | 0 | 0 | 1.5 |
| Swift | 4x | 170 | 1 | 0 | 50 | 3.2 |

### One call: page thread against the worker prototype

| runtime | CPU | page mean us | page p50 us | page p95 us | worker mean us | worker p50 us | worker p95 us | worker start ms | page blocking during worker start ms | memory with worker MB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| JavaScript | 1x | 0.3 | 0 | 5 | 10.6 | 10 | 15 | 5 | 0 | 1.9 |
| Rust | 1x | 25.6 | 25 | 45 | 20.0 | 20 | 25 | 8 | 0 | 4.5 |
| Go (TinyGo) | 1x | 860.6 | 30 | 3410 | 840.4 | 40 | 3300 | 8 | 0 | 14.1 |
| C# (.NET) | 1x | 255.9 | 150 | 660 | n/a | n/a | n/a | n/a | n/a | n/a |
| F# (.NET) | 1x | 255.9 | 150 | 625 | n/a | n/a | n/a | n/a | n/a | n/a |
| Dart | 1x | 7.1 | 5 | 15 | 10.7 | 10 | 15 | 7 | 0 | 2.0 |
| Ruby (mruby) | 1x | 123.4 | 105 | 205 | 71.4 | 65 | 110 | 11 | 0 | 10.2 |
| Kotlin | 1x | 54.9 | 40 | 90 | 23.6 | 20 | 35 | 7 | 0 | 2.1 |
| Swift | 1x | 52.1 | 50 | 80 | 25.8 | 25 | 35 | 15 | 0 | 5.4 |
| JavaScript | 4x | 0.1 | 0 | 0 | 45.9 | 10 | 355 | 6 | 0 | 1.9 |
| Rust | 4x | 32.9 | 10 | 40 | 76.4 | 20 | 560 | 9 | 0 | 4.5 |
| Go (TinyGo) | 4x | 3279.4 | 40 | 13090 | 976.7 | 50 | 3775 | 8 | 0 | 14.1 |
| C# (.NET) | 4x | 839.3 | 625 | 2125 | n/a | n/a | n/a | n/a | n/a | n/a |
| F# (.NET) | 4x | 864.7 | 690 | 2030 | n/a | n/a | n/a | n/a | n/a | n/a |
| Dart | 4x | 10.3 | 0 | 5 | 47.5 | 10 | 260 | 8 | 0 | 2.0 |
| Ruby (mruby) | 4x | 251.9 | 65 | 890 | 248.5 | 75 | 840 | 12 | 0 | 10.2 |
| Kotlin | 4x | 68.8 | 15 | 485 | 97.4 | 25 | 640 | 8 | 0 | 2.1 |
| Swift | 4x | 65.3 | 15 | 500 | 105.1 | 25 | 650 | 16 | 0 | 5.4 |

"Memory with worker" is the page and the worker together after the calls:
in the prototype the page keeps its own copy of the runtime, so it counts the
runtime twice. The .NET worker never answered (see Findings).

### Input while a service computes for 200 ms

| runtime | CPU | click delay page ms | click delay worker ms | key delay page ms | key delay worker ms | work rounds |
| --- | --- | --- | --- | --- | --- | --- |
| JavaScript | 1x | 160 | 0 | 159 | 0 | 48036356 |
| Rust | 1x | 155 | 0 | 156 | 0 | 54197817 |
| Go (TinyGo) | 1x | 168 | 0 | 168 | 0 | 53718033 |
| C# (.NET) | 1x | 169 | n/a | 168 | n/a | 39107292 |
| F# (.NET) | 1x | 166 | n/a | 168 | n/a | 40834943 |
| Dart | 1x | 153 | 0 | 153 | 0 | 54526999 |
| Ruby (mruby) | 1x | 167 | 0 | 165 | 0 | 2275050 |
| Kotlin | 1x | 153 | 0 | 153 | 0 | 52249063 |
| Swift | 1x | 151 | 0 | 151 | 0 | 51441130 |
| JavaScript | 4x | 166 | 1 | 166 | 0 | 12274498 |
| Rust | 4x | 173 | 0 | 172 | 0 | 14914884 |
| Go (TinyGo) | 4x | 138 | 1 | 137 | 0 | 11673673 |
| C# (.NET) | 4x | 170 | n/a | 171 | n/a | 9834334 |
| F# (.NET) | 4x | 169 | n/a | 168 | n/a | 9955642 |
| Dart | 4x | 171 | 0 | 168 | 1 | 13929604 |
| Ruby (mruby) | 4x | 169 | 1 | 168 | 0 | 555706 |
| Kotlin | 4x | 170 | 1 | 168 | 0 | 13024883 |
| Swift | 4x | 170 | 1 | 168 | 0 | 12984625 |

"Work rounds" is how many loop rounds fill 200 ms in that runtime.

### Website routes (page thread)

| route | CPU | runtimes loaded | long tasks | blocking ms | longest task ms | memory MB | script ms by runtime |
| --- | --- | --- | --- | --- | --- | --- | --- |
| / | 1x | .NET, Go, Ruby | 1 | 15 | 65 | 56.8 | .NET 0; page JavaScript 65 |
| /docs | 1x | .NET, Dart, Go, Ruby, Rust | 3 | 136 | 135 | 87.6 | .NET 97; page JavaScript 135 |
| /docs/features | 1x | Dart, Go, Ruby, Rust, Swift | 1 | 64 | 114 | 21.0 | page JavaScript 114 |
| /docs/first-app | 1x | Dart, Go, Kotlin, Ruby, Rust | 1 | 15 | 65 | 19.2 | page JavaScript 66 |
| /docs/features/companion-publish | 1x | Dart, Go, Ruby, Rust | 0 | 0 | 0 | 13.4 | none |
| / | 4x | .NET, Go, Ruby | 5 | 310 | 260 | 56.8 | .NET 192; page JavaScript 321 |
| /docs | 4x | .NET, Dart, Go, Ruby, Rust | 4 | 984 | 546 | 87.6 | .NET 407; page JavaScript 546 |
| /docs/features | 4x | Dart, Go, Ruby, Rust, Swift | 2 | 473 | 468 | 21.3 | page JavaScript 468 |
| /docs/first-app | 4x | Dart, Go, Kotlin, Ruby, Rust | 1 | 225 | 275 | 19.2 | page JavaScript 275 |
| /docs/features/companion-publish | 4x | Dart, Go, Ruby, Rust | 1 | 127 | 177 | 13.5 | page JavaScript 177 |

"Page JavaScript" is almost all one callback in `tac-client.js` after the
service modules are imported: it creates the services (their `init` calls)
and renders the page, with 40 to 205 ms of forced style and layout. The
".NET" time is `dotnet.js` loading and starting its assemblies.

## Findings

1. **Start-up.** Only .NET blocks the page noticeably while it starts: one
   55 ms long task at 1x and 110 ms of blocking time at 4x on a fixture
   route; on the website's `/docs` route `dotnet.js` alone runs 97 ms at 1x
   and 407 ms at 4x. Ruby and Swift reach one long task of 50 to 64 ms at 4x.
   Rust, Go, Dart and Kotlin never make a long task while they start: the
   browser compiles their modules off the main thread. The website's larger
   long tasks come from creating the services and rendering the page
   together, which a worker would only partly remove.
2. **One call.** A call from the page costs 5 to 50 us for Rust, Dart,
   Kotlin and Swift, about 100 us for Ruby and 150 us (p50; 660 us p95) for
   .NET. Go through TinyGo has a fast p50 (30 us) but a p95 of 3.4 ms (13 ms
   at 4x), which looks like garbage collection pauses inside TinyGo.
3. **The worker's cost per call.** The postMessage round trip with a
   structured clone adds about 10 us at 1x (the JavaScript row: 10.6 us
   against 0.3 us) and about 45 us on average at 4x, with a p95 up to 0.36
   to 0.65 ms, because a throttled page and worker take turns. The prototype
   skipped the page's browser-API caller context and stored-field hook, so
   for Kotlin, Swift and Ruby its round trip was even faster than the page
   call; a real design adds those back.
4. **Input.** A 200 ms computation delays the next click or key press by 150
   to 170 ms on the page thread in every runtime, and by at most 1 ms in a
   worker. The benefit depends on the work, not on the language. The same
   loop is about 20 times slower in Ruby (2.3 million rounds in 200 ms
   against about 50 million), so Ruby services reach long computations
   first.
5. **Memory.** A worker costs about 0.5 MB before its runtime loads (the
   JavaScript row). The runtimes themselves: .NET 35 MB on a fixture route
   (the website's `/docs` page with .NET is 88 MB), Ruby 5.6 MB, Swift 3.2 MB,
   Rust 2.7 MB, Go 2.2 MB, Dart and Kotlin 1.5 MB.
6. **Starting in a worker.** Rust, Go, Dart, Ruby, Kotlin and Swift services
   started in a module worker without a change, in 5 to 16 ms, and never
   blocked the page. The .NET runtime did not: it downloads every assembly
   and then stops after `onRuntimeInitialized`, and `dotnet.create()` never
   resolves. Running .NET in a worker needs work on its loader first.
7. **The synchronous model.** Views call service methods inside expressions
   and read fields synchronously (for example `quantity.describe(...)` in F#,
   `guidePosition.label(...)` in Kotlin, `featureFilter.shows(...)` in Swift,
   `pager.next()` in Dart). In a worker every one of these becomes a promise.

## Decision (proposed)

Keep every Tac WebAssembly service on the page thread for now. Do not add one
worker per runtime by default. Per runtime:

| runtime | recommendation | why |
| --- | --- | --- |
| JavaScript | never | no bridge at all; a call is 0.3 us, a worker adds 10 us |
| Rust | no | 25 us calls, no long task, 2.7 MB |
| Go (TinyGo) | no | no long task; look at TinyGo's garbage collector for the 3.4 ms p95 separately |
| C# and F# (.NET) | yes, first, when the two conditions below are met | the only runtime that blocks start-up (55 to 137 ms long tasks, 407 ms of script on `/docs` at 4x), the slowest calls (150 to 690 us p50) and the most memory (35 MB) |
| Dart | no | 7 us calls, no long task, 1.5 MB |
| Ruby | no by default; a candidate for opt-in | 100 us calls, one 64 ms task at 4x, but loops are 20 times slower, so heavy Ruby work blocks input first |
| Kotlin | no | 55 us calls, no long task, 1.5 MB |
| Swift | no | 52 us calls, one 50 ms task at 4x; measure again if Swift modules grow |

.NET moves to a worker only when both of these hold:

1. The .NET loader starts inside a dedicated worker (a spike on
   `dotnet.create()` in a worker).
2. Tac views can use a service whose answers arrive later: field reads come
   from a mirror, and a method called inside a view expression shows its
   last answer until the new one arrives (see Design).

Until then, the cheaper step for .NET is to start its runtime after the first
render (at idle, or on the first call) rather than during it; that is a
separate proposal.

Any runtime may later get an opt-in worker placement for services that
compute for a long time, because the input measurement is the same for every
language. The decision is by workload, not by language.

## Design (when a runtime moves to a worker)

- **One worker per runtime behind the existing bridge.** The page keeps
  `activate()` and the `invoke(op, json)` protocol; only `transport` changes
  from a synchronous call into the module to a `postMessage` to that
  runtime's worker. The worker loads the same shared module that the page
  loads today (`tachyon-csharp.js` and the others need no change to run
  there, except .NET's loader).
- **Mirrored fields for synchronous view reads.** Every answer carries the
  service's field values (the prototype sent them all; a real design sends
  the changed ones). The page keeps a mirror per owner; a view `get` reads
  the mirror and never waits. A `set` updates the mirror at once and sends
  the change; the worker's answer is the final value.
- **Method calls in views.** A call inside a view expression returns the
  last answer for the same arguments from a small per-owner cache and asks
  the worker again; when the answer differs, the view renders again. The
  first render shows the service's declared empty value. Event handlers
  (`on:click`) may await the promise.
- **Storage proxy.** `localStorage` and `sessionStorage` are not in
  workers. The storage runtime already keeps a synchronous copy of IndexedDB
  entities for WebAssembly services (`TacEntityMirror`, loaded by
  `storage.mirror()`); the same idea extends to the web storage values and
  caches that a service uses. The worker gets that copy at start and the
  page's changes as messages. The worker's `tac_host_storage` writes go to
  the page as ordered messages, and the page writes `localStorage`,
  `sessionStorage` and IndexedDB. Reads are served from the worker's copy, so
  they stay synchronous inside the WebAssembly call.
- **Browser API proxy.** Clipboard, Share, Geolocation and Notifications are
  page-only, and some need a user gesture. On the page, `tac_host_web`
  already runs through `TacWebApis`, which checks the grant and the gesture
  and answers slow APIs later as an event to the owner (`TacWebApis.later`).
  In a worker, `tac_host_web` posts the request to the page and returns
  "answer later"; the page runs it through the same `TacWebApis` code while
  the user's activation is still active, and the answer reaches the owner as
  the same event. Quick, gesture-free answers that workers also have (clock,
  random bytes, online state) are computed in the worker itself; colour
  scheme and reduced motion come from the page with the mirror.
- **Event ordering.** Each runtime has one message channel, so calls and
  answers for that runtime keep their order. An answer carries the events
  that the call published, in order; the page delivers those events before
  it resolves the call. Across runtimes, events are delivered in the order
  the answers reach the page; today's total order across languages is not
  kept and the docs must say so.
- **Crash and restart.** On a worker `error` or `messageerror` the page
  rejects every pending call with a Tachyon error that names the service,
  starts a new worker, sends `init` again for every live owner with its
  mirrored field values, and marks the mirror fresh when the answers arrive.
  After three crashes in one minute the page stops restarting that runtime
  and shows the error overlay in development.

## Consequences

- No change now: services keep their synchronous calls and reads, and the
  browser-API and storage hosts stay as they are.
- The measurement script stays in `scripts/` so the numbers can be taken
  again after a runtime, compiler or bridge change.
- Two follow-ups come out of the numbers even without workers: .NET start-up
  (the largest page cost) and TinyGo's garbage collection pauses.
- If .NET moves to a worker, views that call .NET methods inside
  expressions (`documentation.count()`, `quantity.describe(...)` on the
  website) render in two steps; the design above keeps them correct, not
  immediate.

## Limits of this measurement

- Headless Chromium on one fast Mac; 4x throttling slows the renderer but not
  the network or the GPU. Phones, Firefox and Safari were not measured.
- Chromium reports no Event Timing entries in this setup, so input delay is
  measured from the event's time stamp to its handler, not as full INP
  (processing and the next paint are not included).
- The fixture services are tiny. Real services send bigger JSON, which costs
  more to clone for a worker and more to encode for the bridge.
- The website routes were measured on the page thread only.
