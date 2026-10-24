# ADR 0093: Python Is a Tac Service Language

- Status: Accepted
- Date: 2026-10-03
- Builds on: [ADR 0030](0030-components-use-services.md) (page singletons
  and prototype services),
  [ADR 0043](0043-models-hold-data-types.md) (models),
  [ADR 0084](0084-webassembly-services-stay-on-the-page-thread-until-dotnet-needs-a-worker.md)
  (services run on the page thread) and
  [ADR 0091](0091-role-folders-allow-nesting-and-require-explicit-registration.md)
  (role markers)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (Python was a Yon
  language only)

## Context

Python was a Yon language: Yon runs it with native CPython. Tac had no Python
backend, so a team that writes its server in Python had to write its page
services in another language. Ruby and Lua reached Tac through an interpreter
compiled to WebAssembly (ADR 0049, ADR 0085). Pyodide is a maintained build
of CPython for the browser, with a module loader and conversions between
JavaScript and Python values. The full Pyodide distribution is over 200 MB,
and by default it can fetch missing packages from the network, so Tachyon has
to pin and limit what it loads.

## Decision

### Services

- A Tac service may be a Python class in `client/services/<Name>Service.py`,
  marked with `@service` like a Yon Python service. Helpers in
  `client/helpers/` and models in `client/models/` may be Python too.
- The page reads and writes public fields and calls public methods through
  the existing Tac service protocol. Fields and method results cross as JSON
  through explicit codecs; models from `client/models/` are typed on both
  sides. A read-only field cannot be written from the page.
- `Needs` and `NeedsAll` fields with explicit types build the service's
  dependencies, including Python Helpers. `Named` selects one declared
  provider. Only a `NeedsAll` field of a Service can be a page property. A
  Helper that its caller creates itself must be created in the caller's own
  code. Singleton and prototype lifetimes follow ADR 0030. A dependency graph stops at 4096 acquisitions and 32
  levels, and a cycle is refused.
- `@on_start` and `@on_stop` run synchronously when the owner starts and is
  disposed. Disposal releases the owner's Python objects and the JavaScript
  proxies that point at them.

### Runtime

- The project opts in with `"python": {"runtime": "pyodide-314.0.7",
  "profile": "offline"}` under `tac` in `client/manifest.json`. Any other
  runtime or profile is refused.
- The runtime is Pyodide 314.0.7 (CPython 3.14.2, ABI 2026_0). Its five core
  files (`pyodide.mjs`, `pyodide.asm.mjs`, `pyodide.asm.wasm`,
  `python_stdlib.zip`, `pyodide-lock.json`) are pinned in the compiler by
  byte size and SHA-256. The developer puts them in `python-web/314.0.7/`
  in the Tachyon cache. `ty` never downloads or installs them, and
  `ty doctor` reports whether they are present and match.
- A build copies the runtime into
  `.tachyon/services/python/314.0.7/` only when a route reaches a Python
  service. The files go through the normal build steps: hashes,
  pre-compression, cache keys, route budgets and the service worker, so an
  installed app works offline. A route with a Python service gets a
  512 KiB script budget unless the project sets its own; the WebAssembly
  file may be up to 10 MiB and the other runtime files up to 4 MiB each.
- (Amended 2026-10-16) The build also writes `THIRD_PARTY_NOTICES.txt`
  beside the runtime in `.tachyon/services/python/314.0.7/`, as Ruby and Lua
  do. It holds the licenses of Pyodide (MPL-2.0, with the address of its
  source code), CPython 3.14.2 (PSF License) and the libraries linked into
  `pyodide.asm.wasm`: hiwire, HACL\*, libffi, zlib, bzip2, Zstandard,
  liblzma, SQLite, Emscripten, musl, compiler-rt, libc++ and libc++abi. The
  text is kept in `runtimes/python-web/THIRD_PARTY_NOTICES.txt`, taken from
  the upstream sources of the pinned release.
- Only the standard library in the pinned archive and the project's own
  modules are available. Pyodide loads its index, lock file and standard
  library from the copied files, and Tachyon installs no packages.

### Limits

These fail at build time with `TY1605`, which names the file:

- `async def` methods and asynchronous `@on_start` or `@on_stop`. The bridge
  is synchronous; the asynchronous settlement contract of other Tac
  languages is not connected to Python yet.
- Streams and browser API resources (see the amendment below for events).
  (Replaced for streams: Python holds stream and socket clients; see the
  amendment of 2026-10-18.)
- Constructor arguments that are not typed `Needs` or `NeedsAll` fields, and
  `Needs` fields that name no Python provider.
- A missing, changed or incomplete runtime.
- (Amended 2026-10-04) A browser API in the service's code, such as
  `Clock()`, `SecureRandom` or `Geolocation`. The runtime has no browser API
  adapter, so the use failed only at run time before. Use the API in a
  JavaScript, TypeScript or compiled service and pass the value to the Python
  service as an argument.

(Amended 2026-10-04) The Python runtime has no behavior or storage adapter.
The annotation rule table (ADR 0056) therefore leaves Python out of
`LocalState`, `SessionState`, `CacheSettings`, `Cached`, `CacheKey`,
`ClearCache`, `UpdateCache`, `PageDefaults`, `Retry`, `MaxAtOnce` and
`AllOrNothing` in Tac. A Python Tac service with one of them fails once, with
`TY1308` "... is not available in Python Tac code; it works in ...". Before, a
saved field built and was never saved, and a synchronous `@retry` method
failed with "must use async def" while an `async def` failed with `TY1605`.
`OnEvent`, `Order` and `InBackground` stay available: see the events and
topics amendment below. (Replaced: Python joins the Tac lists of these
behavior annotations and of the saved fields; see the amendment of
2026-10-17.)

Python Tac code cannot call the server through a client either. The rule
table leaves Python out of `HTTPClient` and its request annotations
(`GetRequest`, `PostRequest`, `PutRequest`, `PatchRequest`, `DeleteRequest`,
`StreamRequest` and `SocketRequest`), so a Python Tac client fails with
`TY1308`, as Lua does (ADR 0085). Call the server from a JavaScript,
TypeScript or compiled service and pass the answer to the Python service as
an argument. (Replaced: Python now has `HTTPClient` and the request
annotations other than `StreamRequest` and `SocketRequest`; see the
amendment of 2026-10-17.)

At run time a request or answer over 1 MiB, a member request over 64 KiB,
more than 256 live owners, or a call on a busy owner is refused. Python runs
on the page thread, so a long computation blocks the page. Threads,
subprocesses, native sockets, the operating-system file system and native
extensions are not available. Third-party packages are not supported. This
decision covers web builds; Python services in native app hosts are not part
of it. (Replaced: see the amendment of 2026-10-18 on native hosts.)

### Tests

Python Tac test units run with `ty test`, under the same supervisor as the
other languages.

(Amended 2026-10-04) `ty init --tac py` and `ty add language client py`
scaffold a Python greeting service with its test, and write the pinned
`tac.python` setting into `client/manifest.json`. The developer still copies
the runtime files; `ty doctor` names the folder.

## Consequences

- A Python team can write page services in Python and share models with a
  Yon Python server.
- The first visit downloads about 13 MB of runtime before compression, much
  more than a JavaScript or Rust service. The route budget and `ty doctor`
  make that cost visible.
- Asynchronous methods, browser API resources, streams and third-party packages
  need their own decisions before they work in Python.

## Amendment 2026-10-04: events and topics

- A Python Tac service may have synchronous `@on_event` methods, with the
  same options as in Yon (`id`, `condition`, `replay`, `@order` and
  `@InBackground`). The page event bus calls them; a returned model is
  published again unless the listener is `@InBackground`.
- A field typed `ApplicationEventPublisher` is the page publisher. Tachyon
  sets it after it creates the service; `publish_event(model, retain)`
  sends a model from `client/models/` to the page bus.
- A listener for a model marked `@topic` opens the topic stream
  `/.tachyon/topics/<name>` by itself, like the other WebAssembly services.
- `async def` methods, streams and browser API resources still fail with
  `TY1605`. (Replaced for streams: see the amendment of 2026-10-18.)

## Amendment 2026-10-11: routes with several web runtimes

- [ADR 0100](0100-a-route-with-several-web-runtimes-gets-a-measured-allowance.md)
  gives a route that loads more than one web runtime a measured script
  allowance. Python is not one of those runtimes, and the 512 KiB budget of
  a Python route does not change. When a Python route also loads other web
  runtimes, its budget is the larger of the two.

## Amendment 2026-10-16: `async def` service methods

- A public method of a Python Tac service may be `async def`. The page gets
  a promise for it, like the asynchronous methods of other Tac languages
  (ADR 0090). Its arguments and its answer cross through the same typed
  codecs as a synchronous method. A Python Helper may have `async def`
  methods that a service awaits.
- The runtime starts the coroutine as an asyncio task on the Pyodide event
  loop and answers the call with `{"task": id}`. When the task ends, it
  reports the encoded answer to the page through the host's `done` callback;
  the page settles the promise through the shared task answers
  (`TacTaskAnswers`), as Go, Kotlin and Ruby do, and then delivers the
  events that the task published.
- A waiting method does not hold its service: other calls, field reads and
  writes run while it waits, as in JavaScript, Go, Kotlin and Swift. The
  page always gets a promise, even when the method does not wait.
- An exception in the task rejects the promise with
  `Python Tac operation failed.`; Python's own message stays in the
  runtime. An answer over 1 MiB is refused the same way.
- Disposing the owner cancels its waiting tasks before the `@on_stop`
  hooks run, and their promises reject with code `CANCELLED`. Cancellation
  reaches the coroutine as `asyncio.CancelledError` at its next `await`, so
  its `finally` blocks run after the stop hooks. Leaving the page cancels
  every task.
- At most 256 tasks wait at once; another `async def` call is refused
  before the method starts.
- Still refused with `TY1605`: asynchronous `@on_start`, `@on_stop` and
  `@on_event` methods, streams and browser API resources. Python has no
  HTTPClient; that is a separate product decision. (Replaced: Python has an
  `HTTPClient`, and an asynchronous `@on_start` builds; see the amendment of
  2026-10-17. Streams are replaced too: Python holds stream and socket
  clients; see the amendment of 2026-10-18.) Native hosts stay out of scope.
  (Replaced: see the amendment of 2026-10-18 on native hosts.)
- Evidence: `backend_tests::python_async_methods_settle_through_page_tasks_and_cancel_on_dispose`
  runs the generated program in CPython. The ignored browser test
  `pinned_python_tac_services_async_models_and_offline_reload_use_the_real_compiler`
  builds `crates/tachyon-core/tests/fixtures/tac-python` and checks a view
  click, the promise, a field read while the method waits, a failure and a
  cancellation on dispose, online and after an offline reload, in Chromium.
  Firefox and WebKit runs are still owed. (Replaced: the test runs in
  Chromium, Firefox and WebKit; see the amendment of 2026-10-18 on browser
  engines.)

## Amendment 2026-10-16: saved fields

- A Python Tac service field may be marked `LocalState` or `SessionState`
  inside its type: `theme: Annotated[str, local_state] = 'light'`
  (`LocalState` and `SessionState` are accepted spellings too). The rule
  table (ADR 0056) now lets Python use these two markers in Tac; the other
  behavior annotations of the 2026-10-04 amendment still fail with
  `TY1308`.
- The page saves and restores these fields as it does for the other web
  service languages: through the field's getter and setter, under a key
  named after the page, the service and the field's page name (`last_title`
  is saved as `lastTitle`). The Python runtime needs no storage adapter of
  its own. A restored value crosses the field's typed codec, like any other
  write from the page.
- The stored-field reader now notices the snake_case spellings; before, a
  source that spelled only `local_state` or `session_state` was skipped.
- Evidence: `backend_tests::python_local_and_session_state_fields_reach_the_page_storage`
  and the browser test above, whose `LibraryService.loaded` field is
  `LocalState` and comes back after the offline reload in Chromium.
  `SessionState` shares the page code but has no browser check of its own
  for Python yet. Firefox and WebKit runs are still owed. (Replaced: the
  browser test checks a `SessionState` field in all three engines; see the
  amendment of 2026-10-18 on browser engines.)

## Note 2026-10-16: offline through `ty preview` is not qualified

The browser test serves the compiler's `dist` from its own HTTPS test origin
with `public, max-age=60` and no `Vary`. Served through `ty preview` (with
or without `--static`), every response carries `Cache-Control: no-store`
from the server's defensive headers and `Vary: accept-encoding`, and the
service worker caches only public responses without `Vary`. The page then
caches none of the Pyodide files, so it cannot reload offline. This holds
for every web service language, not only Python. Whether the preview server
should send cacheable headers for build outputs is a separate server
decision; until then, offline Python Tac is qualified only from an origin
that serves the build as public static files.

## Amendment 2026-10-18: `ty preview --static` and `ty start` qualify offline

The server decision is made. `ty start` and `ty preview --static` send
`Cache-Control: no-cache` for a file that the built worker's asset table
lists, when the request has no `Authorization`, no cookie and no query. The
server reads the table from the `tachyon-sw.js` it serves, and reads it again
when that file changes, so it never guesses from a file name. Every other
answer keeps `no-store`, and so does every answer of the watching
development server. The precompressed files still carry
`Vary: Accept-Encoding`, which the worker now accepts as its only Vary value
(ADR 0087 amendment). `no-cache` and not a year-long `immutable` lifetime:
the build keeps the same URL for a changed file, such as
`/.tachyon/tac-client.js`, so a long lifetime would keep old bytes in any
browser that does not run the worker, including every loopback preview.
`scripts/preview-offline-browser-gate.mjs` reloads a page offline under
`ty preview --static` in Chromium, Firefox and WebKit. The Pyodide files are
in the same table, so offline Python Tac is qualified there too; that
gate's fixture has no Python service.

## Amendment 2026-10-17: async methods, behaviors, saved fields, stores and clients

- An `async def` method builds. The page gets a promise: the bridge answers
  `{"task": n}`, and the shared task settlement of the other Tac languages
  sends the answer when the coroutine finishes. Disposal cancels the
  owner's unfinished tasks, and their callers get `CANCELLED`. The
  2026-10-16 `async def` amendment above gives the details.
- `@on_start` may be `async def`. A call that arrives before it finishes
  waits for it. `@on_stop` and `@on_event` stay synchronous; an
  `async def` one still fails with `TY1605`.
- Python reaches the page's storage and HTTP hosts. With no adapter in its
  Setup, a Python behavior context uses the page cache for `CacheSettings`,
  the page entity store (adapter `indexeddb`) for `Entity` and
  `AllOrNothing`, and the browser HTTP transport for a generated
  `HTTPClient`. A Setup adapter still replaces each of them. Python
  therefore joins the Tac lists of `CacheSettings`, `Cached`, `CacheKey`,
  `ClearCache`, `UpdateCache`, `PageDefaults`, `Retry`, `MaxAtOnce`,
  `AllOrNothing`, `Store`, `Query`, `HTTPClient`, `GetRequest`,
  `PostRequest`, `PutRequest`, `PatchRequest`, `DeleteRequest`,
  `PathVariable`, `RequestParam`, `RequestBody` and `RequestHeader`.
- A `Store` in `client/stores/` or an `HTTPClient` in `client/remotes/` is a
  `Protocol` contract, as in Yon. The build writes its class, and a `Needs`
  field reaches it like any other provider.
- `LocalState` and `SessionState` fields are written in the type, as
  `Annotated[str, local_state]`. The page saves and restores them like the
  saved fields of the other languages; the field's page name is its
  lowerCamelCase name, as the 2026-10-16 saved fields amendment above says.
- `ValueSource` works in Python Tac test units, which run under native
  CPython as in Yon.
- `StreamRequest` and `SocketRequest` stay refused with `TY1308`. The
  page's JavaScript holds a browser stream or socket, and the Python
  runtime has no adapter that hands its messages to a Python client yet.
  (Replaced: Python accepts both; see the amendment of 2026-10-18.)
- This amendment replaces the 2026-10-04 limits on behaviors, saved fields
  and `HTTPClient`, and the `async def` limit in "Limits" above. It also
  replaces two limits of the 2026-10-16 amendments above: an asynchronous
  `@on_start` no longer fails with `TY1605`, and Python now has an
  `HTTPClient` and the other behavior annotations listed here.

## Amendment 2026-10-18: stream and socket clients

- A Python Tac `@http_client` contract method may carry `@stream_request` or
  `@socket_request` (`StreamRequest` and `SocketRequest` are accepted
  spellings too). The method takes one parameter whose model type says what
  each message holds, for example
  `def receive(self, tick: Tick) -> None: ...`. The rule table (ADR 0056)
  adds Python to the Tac list of both markers, so they no longer fail with
  `TY1308`, and a Python service that needs such a client no longer fails
  with `TY1605`.
- The page's JavaScript still holds the connection. A Python service that
  keeps the client in a `Needs` field lists the client's streams in its
  `init` answer, and the page's event bus opens, checks, publishes and
  closes them as for the other Tac languages (ADR 0090, "Python streams and
  sockets"). Each message reaches the service's `@on_event` method whose
  parameter is the message model.
- A client with only stream or socket methods needs no Setup: it has no
  behavior context.
- A socket client gets `async def send(self, message) -> None`. It raises
  `BehaviorError` with code `HTTP_NETWORK`, `HTTP_MISSING`,
  `HTTP_REQUEST`, `HTTP_BODY` or `CANCELLED`. A Python test unit runs in
  CPython without a page, so `send` raises `HTTP_MISSING` there.
- This amendment replaces the stream and socket limits above: in "Limits",
  in the events and topics amendment, and in the 2026-10-16 and
  2026-10-17 amendments.

## Amendment 2026-10-18: typed HTTP answers and cached reads

- A Python Tac `@http_client` method reads its answer into its return
  annotation. `-> Greeting`, where `Greeting` is a dataclass of
  `client/models`, gives a `Greeting`; `-> list[Greeting]` and
  `-> Greeting | None` work too, nested dataclass fields are read the same
  way, and `str`, `int`, `float` and `bool` check a plain value. `dict`,
  `object` and a `TypedDict` model keep the JSON value. An answer that does
  not fit, such as one with a field the dataclass does not declare or
  without a field it needs, raises `BehaviorError` with code
  `HTTP_RESPONSE`. The same happens in a Python test unit, where the
  TestSetup's `http` adapter answers. Before this amendment such a method
  returned the plain dict whatever its annotation said.
- A `@cached` read is a public read: on a cache miss the browser request
  carries `publicRead`. The first read of a key after a successful
  `@clear_cache` or `@update_cache` of that key carries `revalidate`, so the
  browser asks the server again, as in Tac Ruby and Tac Lua.

Evidence: `scripts/browser-http-browser-test.mjs --language=python` builds
`website/content/docs/calling-the-server/browser-http-python`, serves it with
`ty serve` and checks it in Chromium, Firefox and WebKit (on 2026-10-18 it
passed in Chromium in a Linux container, and in Firefox 153 and WebKit 26.5
on the macOS CI runner) with the checks of the Lua gate (ADR 0085). The
first-app tutorial has Python tabs for step 4 and step 5.

## Amendment 2026-10-18: browser engines and `SessionState`

- `scripts/python-tac-browser-test.mjs` runs the Python Tac browser test in
  Chromium, Firefox and WebKit. `TAC_PYTHON_BROWSERS` picks engines, as
  `TAC_*_BROWSERS` does for the other browser gates; the default is all
  three. Each engine writes its own receipt,
  `python-tac-browser-<engine>.json`, in the retained fixture folder.
- The test hostname `python-tac.test` reaches the local test origin through
  a small HTTP `CONNECT` proxy that Playwright sets for each browser. The
  Chromium-only `--host-resolver-rules` flag is gone. Firefox needs
  `dom.serviceWorkers.testing.enabled` to run the service worker behind the
  local test certificate.
- Offline means the test origin refuses every request. Chromium and Firefox
  also emulate offline; WebKit does not, because its emulation fails
  worker-served navigations. The test checks that no pinned Python runtime
  file and no service module reached the network while offline.
- `LibraryService.shelved` in the fixture is a `SessionState` field. The
  test checks that the page saves it in `sessionStorage` and the
  `LocalState` field `loaded` in `localStorage`, that both come back after
  the offline reload, and that a new tab restores `loaded` but starts
  `shelved` at its default.
- This amendment replaces the "Firefox and WebKit runs are still owed"
  notes and the missing `SessionState` browser check in the 2026-10-16
  amendments above.

## Amendment (2026-10-18): Python services in native hosts

This ADR said that Python services in native app hosts were out of scope.
The code already builds them, so this amendment records what it does.

- A native build (`ty bundle --target macos`, `windows`, `linux`, `ios` or
  `android`) runs the same web build as the web target. Python is never
  linked into a host: the linked languages are only Rust, Swift, Kotlin
  and C# (`NativeCompanion`). So a Python service runs on Pyodide inside
  the host's web view, on the page thread, as it does in a browser.
- No target refuses a Python service. The same rules apply as on the web:
  `tac.python` must name `pyodide-314.0.7` with the `offline` profile, and
  the runtime files must be in the Tachyon cache.
- The build copies the five Pyodide files into
  `.tachyon/services/python/314.0.7/`, and the host stages the web bundle
  inside the app. The largest file, `pyodide.asm.wasm` (about 9.2 MiB), is
  under the 16 MiB limit per bundle file. Every host serves `.wasm` as
  `application/wasm` and `.mjs` as JavaScript.
- A service that also has a Rust, Swift, Kotlin or C# source that the target
  links uses that linked source instead. Python is the last choice among the
  WebAssembly languages (`WebLanguage::PREFERENCE`).
- The service worker is off in native builds, so the offline amendments
  above do not apply there. The files are already inside the app.

This is verified at the code level only. No host has run a Python service
yet: no native gate loads Pyodide in WKWebView, Android WebView, WebView2 or
WebKitGTK. Load time, memory and the custom bundle origins of each host are
not checked.

## Amendment 2026-10-19: the "Still refused" list of 2026-10-16

This amendment supersedes the "Still refused with `TY1605`" list in the
2026-10-16 `async def` amendment above, which already carries inline
replacement notes. The list stays as the history of this decision. Today:

- An asynchronous `@on_start` builds (the 2026-10-17 amendment), and Python
  holds stream and socket clients and has an `HTTPClient` (the 2026-10-18
  and 2026-10-17 amendments).
- Still refused with `TY1605`: an asynchronous `@on_stop` or `@on_event`
  method, and browser API resources such as `Clock()`. The test
  `backend_tests.rs python_async_methods_and_async_start_build_but_async_stop_is_pending`
  checks both sides of the first line.

## Amendment (2026-10-19): converters

- A `@converter` `Protocol` class in `client/converters/` is a Python Tac
  provider, like a `Store` or an `HTTPClient`. A service or Helper reaches
  it with a typed `Needs` field, such as
  `product_converter: Annotated[ProductConverter, needs]`. Before this
  amendment Python Tac found providers only in `client/services`,
  `client/helpers`, `client/stores` and `client/remotes`, so that field
  failed with `TY1605` ("Python Tac Needs must name a declared
  same-language provider").
- `Convert` works as in Yon: a JSON-value method maps with its literal field
  plan, and a method between two `client/models` dataclasses copies the
  same-named fields, maps nested models and lists, and returns a dataclass
  of the declared type. `def` and `async def` methods both work. The
  mapping reads and writes models with the behavior runtime's own reader
  when the page has no Yon model reader, so the result is the same under
  Pyodide and under CPython.
- A converter method with a body still fails with `TY1308` ("Convert writes
  the whole method; declare its signature with an empty body").

Evidence: `tac_python::converter_tests` runs one service with a typed
converter (with and without `Convert`) and a JSON-value converter on
Pyodide 314.0.7 through Node and under CPython, and checks the refusal.
On 2026-10-19 the website "Mapping: Tac" and "Typed model mapping: Tac"
Python tabs were bundled with `ty serve` and clicked in Chromium in a Linux
container: they showed `product 7: Anvil` and
`product 7: Anvil, detail, 1`.

## Amendment (2026-10-19): a Python service ran in the Linux app

This amendment supersedes "No host has run a Python service yet" in the
2026-10-18 amendment above, which stays as the history of this decision.

RL1 ran a Python service on Pyodide inside WebKitGTK, in the packaged Linux
app under Xvfb in the cloud (`scripts/linux/page-services-native-test.mjs`,
`docs/SUPPORT_TIERS.md`, Linux runs owed, RL1): the Tally page answered a
click from the Python service, whose first page took 4.0 seconds. The
support table marks `tac.python` `honored` on Linux. The macOS, Windows, iOS
and Android hosts have not run a Python service yet, and the table marks
them `runOwed`.

## Amendment (2026-10-24): a Python service ran in the Windows app

SD1 ran the same Tally page with its Python service inside WebView2, in the
built Windows app on the signed-in desktop of a Windows 11 ARM64 machine
(`scripts/windows/page-services-native-test.mjs`, `docs/SUPPORT_TIERS.md`,
Windows on the signed-in desktop). The build used the pinned Pyodide 314.0.7
cache, copied from another machine with matching SHA-256 digests. The
support table marks `tac.python` `honored` on Windows; macOS, iOS and Android
stay `runOwed`.
