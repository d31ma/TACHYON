# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Released Tachyon products use calendar versioning (CalVer) in UTC. Public
machine contracts have their own, independent major versions.

## [Unreleased]

### Added

- **Yon handlers write logs on every answer** (ADR 0127). Every Yon language
  gets `YonLog` with the levels `debug`, `info`, `warn` and `error`, a
  message and optional fields. `ty start`, `ty serve`, `ty handler invoke` and
  `ty test` write each line to standard error as a `handler.log` JSON line
  with the `request_id`, `route` and `handler`, whether the call answers,
  fails or times out. `TAC_HANDLER_LOG_LEVEL` sets the lowest level (default
  `info`). A call keeps at most 100 lines; `handler.log_dropped` counts the
  rest. `handler.stderr` (`TAC_HANDLER_STDERR=1`) now also names the route and
  the handler, covers streams and sockets, and leaves out log lines.
- **Tac services create and use passkeys** (ADR 0126). Every Tac service
  language gets a `Passkeys` client for the browser's WebAuthn, with the new
  grant `passkeys` in `tac.browserApis`. Options and credentials are WebAuthn
  JSON text: `create(optionsJson)` answers with `PasskeyCreated {
  credentialJson }`, `get(optionsJson)` and the autofill `offer(optionsJson)`
  answer with `PasskeyAsserted { credentialJson, conditional }`, `check()`
  answers with `PasskeySupport { platform, conditional }`, and every failure
  sends `PasskeyFailed { call, reason, message }` with a reason word
  (`cancelled`, `exists`, `unsupported`, `refused`, `busy`, `invalid`,
  `failed`). The code ships in `.tachyon/tac-passkeys.js` only to pages that
  grant `passkeys`. Native apps answer `unsupported` until the platform
  passkey APIs come.
- **Open pages hear a Web Push message** (ADR 0118 amendment). When a push
  arrives while the app is open, every open page of the app gets the built-in
  event `PushReceived { tag, path, data }` in every Tac service language, so
  it can sync at once; the notification is still shown. `data` is the
  message's optional `data` member as text. The push worker posts only to
  windows of its own origin, and a page accepts the message only from its own
  push worker.
- **Services ask for persistent storage and read the space used** (ADR 0123).
  Every Tac service language gets a `StorageSpace` client. `estimate` needs no
  grant and answers with `StorageEstimated { usage, quota, persisted }`;
  `persist` needs the new grant `persistentStorage` in `tac.browserApis` and
  answers with `StoragePersisted { persisted }`. In a browser the page uses
  `navigator.storage`; an insecure page answers quota 0. In the five native
  apps the host answers: `persisted` is true and `quota` is the free disk
  space. `docs/PAGE_CLIENT_CHECKLIST.md` lists every file a new page client
  touches.
- **Sockets and streams reconnect by a declared policy and reach named
  origins** (ADR 0124). `@SocketRequest` and `@StreamRequest` take
  `{"path": "/ws", "reconnect": {"maxTries": 0, "delayMs": 1000,
  "maxDelayMs": 30000, "jitterMs": 1000}}` in every Tac language; `maxTries`
  0 means no limit. The count restarts when the connection opens, an offline
  page does not count tries, and the page reconnects at once when it is online
  or shown again. Without a policy nothing changes (1, 2, 4, 8 and 16 seconds).
  `tac.yon.origins` in `client/manifest.json` names other servers, such as
  `{"realtime": "https://rt.example.com"}`, and
  `@HTTPClient({"path": "/chat", "origin": "realtime"})` sends that client's
  requests, streams and sockets there. The host must be in
  `tac.outgoing.hosts`, `connect-src` allows the origin, an undeclared name
  stops the build, and a native release refuses a local named origin.
- **Store tables load by partition** (ADR 0125). An `"indexeddb"` Entity can
  name a partition field, such as
  `@Entity({"adapter":"indexeddb","partition":"conversationId"})`. The table
  can then hold any number of rows; 10,000 rows is now the limit of one
  partition. Every Store query must compare the partition field with Equals
  (`TY1308` otherwise). WebAssembly and Dart services call `load`, `loaded`
  and `unload` on `context.repository(Entity)` and hear
  `StorePartitionLoaded` or `StorePartitionFailed`; a read of a partition
  that is not loaded fails with `storage_not_loaded`. The page no longer
  copies partitioned tables into memory at start, and a failed background
  write sends `StoreWriteFailed` instead of only a console message.
- **A Yon handler can reach a named service on a private network** (ADR 0094
  amendment). List the name in `outgoing.hosts` and add an entry such as
  `{"host": "keycloak", "networks": ["172.16.0.0/12"]}` to the new
  `yon.outgoing.private`. The outgoing request guard then connects that name
  to the addresses it resolves inside those networks, so a handler in Docker
  or Floci can call `keycloak:8080` without a port forwarder. Every other name
  still gets `403` for a private address; wildcards, IP addresses and
  localhost names cannot have an entry; loopback, link-local and cloud metadata
  addresses stay refused; and a bad entry stops the server with `TY2010`.
- **Every Tac service language gets `Files`** (ADR 0116): JavaScript,
  TypeScript, Rust, Go, C#, F#, Swift, Kotlin, Java, Dart, Ruby, Lua, C++ and
  Python get the same calls and file events in their own style.
- **Go services pick, hold and upload files** (ADR 0116). Grant `files` in
  `tac.browserApis` and add a `// @Needs` field `files Files`. `Pick` opens the
  file chooser, and `data-tac-drop-zone` elements take dropped files; each file
  arrives with its name, size, media type and SHA-256. The page holds the bytes by
  file id; `Read` returns chunks of up to 32 KiB, and `Upload` sends a file
  with `fetch` (no cookies, no redirects) only to a path on the page's or Yon's
  origin or to an `https` host in `tac.outgoing.hosts`. The files runtime
  (`/.tachyon/tac-files.js`) loads only on pages that grant `files`. The macOS
  app now answers file inputs with `NSOpenPanel`, and the Android app with the
  system chooser. `TY1407` for Go now points at `Files`.
- **Tac services can move and trap focus.** A new `Focus` browser API
  (ADR 0115) moves focus to an element named with `data-focus`, traps Tab
  and Shift+Tab inside a container such as a dialog, and returns focus to
  the opener on release. Every Tac service language has it except Python,
  which gets browser APIs with ADR 0121: Go uses `// @Needs` with
  `focus Focus`, JavaScript and TypeScript use `@Needs focus`, and Rust,
  C#, F#, Swift, Kotlin, Java, Dart, Ruby, Lua and C++ get a `Focus` client
  like their `Clock`. Each call runs after the next render, and a failure
  arrives as `BrowserCallFailed`. An element that passes focus to a child (a
  shadow host with `delegatesFocus`, or a custom element that focuses an
  inner field) counts as focused, so the move does not report a failure.
- Tac HTTP clients read JSON error bodies in every Tac service language. When
  the server answers an error status with `application/json` or
  `application/problem+json`, the call fails with `HTTP_STATUS`, its status
  number and the body, and each language reads the body into a model (for
  example `BehaviorErrorBody(err, &model)` in Go, `error.body_as::<Model>()`
  in Rust, `Behaviors.errorBody(error)` in JavaScript). A Setup or TestSetup
  adapter can answer with a status and a body, so a test can fake an error
  answer. Native Rust and C# services keep the status only (ADR 0117).
- **Web Push for Tac pages.** Grant `push` in `tac.browserApis`, and every
  Tac service language gets a `Push` client: `subscribe` with the server's
  VAPID key answers with a `PushSubscribed` event that holds the
  endpoint and keys the server needs. A separate push service worker shows
  each message and opens its same-origin path when clicked. Native apps
  answer that Web Push needs a web browser (ADR 0118).

- **Event bindings take key filters, modifiers and document listeners.**
  `on:keydown.escape`, the arrow keys, `home`, `end`, `enter`, `space` and
  `tab` run a handler for one key. `shift`, `ctrl`, `alt` and `meta` name the
  held keys, which must match exactly, so `on:keydown.enter` does not run on
  Shift+Enter. `.prevent` and `.stop` act before the handler starts, so they
  work for Go services in WebAssembly too. `.outside` runs a handler for a
  click outside the element. `on:document:<event>` and `on:window:<event>`
  listen on the document or window while the element is on the page. Event
  names may contain `-` (`on:w-change`). Unknown or repeated modifiers fail
  with `TY1306`. Every target runs this in its web view, and every Tac
  service language gets it: the website shows the same menu in all fourteen
  languages, and a test builds it once per language. See ADR 0114.

- Python Tac services can use Tachyon's browser APIs: `Clock`,
  `SecureRandom`, `Connectivity`, `PageVisibility`, `Preferences`,
  `Clipboard`, `Share`, `Notifications`, `Geolocation`, `WindowControls`
  and `Focus`. A service declares a client as a `needs` field, such as
  `clock: Annotated[Clock, needs]`, and calls snake_case methods such as
  `clock.now_ms()`. The calls go through the same page host as every other
  language, with the same `tac.browserApis` grants. A Python service can also
  listen for every built-in browser event model, and a Python test unit
  answers the calls with `BrowserFakes.answer`. Before, a Python service that
  used a browser API stopped the build with `TY1605` (ADR 0121).
  the opener on release. Go services use `// @Needs` with `focus Focus`;
  JavaScript and TypeScript services use `@Needs focus`. Each call runs after
  the next render, and a failure arrives as `BrowserCallFailed`.
- A Tac service that holds a `SocketRequest` client hears `SocketOpened`
  (`path`, `reconnected`) and `SocketClosed` (`path`, `code`, `reconnecting`)
  with `@OnEvent`, so it can authenticate again after an automatic reconnect.
  Waiting messages are sent after the `SocketOpened` listeners. Every Tac
  service language, Python included (ADR 0120).
- `@Every("60s")` on a Tac service method, in every Tac service language, runs it
  as a page call at that interval, skips runs while the page is hidden, runs
  once when the page is shown again, and stops when the view unmounts
  (ADR 0120).

### Fixed

- **One store deadline per answer, and the FYLO operation in every deadline
  failure** (ADR 0128 amendment 2). All store calls of one handler run that
  answers once share one deadline from the first call (`timeout`,
  default 5 s), so during a FYLO stall one HTTP answer fails after about one
  deadline instead of one per store call (Mesail measured 11 to 16 s).
  Streams and sockets keep one deadline per store call, and a scroll's first
  page keeps 30 s. The deadline failure now names the FYLO operation also
  for calls that waited behind the stuck one ("... deadline exceeded during
  FYLO putData; ..."). A Linux-only test on a frozen ext4 disk proves that
  the next call waits for the stuck `fylo` and starts no second one.
- **A FYLO process stopped by a deadline is waited for** (ADR 0128
  amendment). When a store call passed its deadline, the FYLO datasource
  killed its `fylo exec --loop` process without waiting, and the next call
  started a second one at once. A process stuck in a slow disk write still
  held the root, so the new one answered `EROOTLOCKED` to every call until
  the old one exited (more than 10 seconds were measured in Mesail). Now the
  process stays in its slot until it has exited: the next call waits for the
  exit within its own deadline, and fails with the deadline error rather
  than `EROOTLOCKED` when the root does not come free in time. A broken pipe
  is handled the same way. The deadline failure names the FYLO operation,
  and a `storage.fylo_process_stopped` line gives the operation and the
  milliseconds from the kill to the exit. The FYLO datasource also takes
  `timeout`, the store-call deadline in milliseconds (1000 to 60000, default
  5000). The key is lowercase because `application.json` keys may hold only
  lowercase letters and digits.
- **A busy FYLO root waits, and store failures are logged** (ADR 0128). When
  another process holds the FYLO root (`EROOTLOCKED`), the FYLO datasource
  now waits and sends the request again for up to 2 seconds before it fails
  as before; other FYLO errors still fail at once. Every failed storage
  command of a Yon handler now writes one `handler.storage_failed` line with
  the `request_id` and the diagnostic, such as `TY2160` "FYLO operation failed
  (EROOTLOCKED).", to the standard error of `ty`. Before, the cause was
  dropped. The handler's answer does not change.

- **A Windows PHP relay answers its own deadline when PowerShell starts
  slowly.** The relay's watchdog (it ends the program's process tree at the
  deadline) is a PowerShell process. PHP for Windows is an x64 program, so on
  Windows ARM64 PowerShell started from it runs under x64 emulation and needed
  1.5 to 2.8 s to start; a relay with less time left answered after its
  deadline, and the supervisor reported `TY2110` instead of the relay's `502`.
  Now the relay reads the program's answer only after the watchdog says it is
  armed, or after the program has exited. If the deadline comes first, the
  relay ends the program and answers `502` on time; the watchdog still ends
  the rest of the process tree when it starts. A fast program that exits
  before the watchdog is armed still answers at once.
- **A `.env` value can set a typed Yon setting** (ADR 0026 amendment). A
  `.env` name may now replace a single value of `server/application.json` or
  the selected profile, and Tachyon converts it to that value's JSON type:
  `YON_MAILOUT_DIRECTENABLED=true` over `"directenabled": false` gives handlers
  the boolean `true`, and `YON_MAILOUT_MAXAGEMINUTES=60` over a whole number
  gives the number `60`, so typed fields in every Yon language decode them.
  Before, every `.env` value was a string and replacing a JSON value was
  refused. A value that does not fit the declared type stops `ty start` with
  `TY2010`, which names the variable and the expected type but never the
  value. Names that are not in the JSON configuration stay strings, and lists
  and objects cannot be replaced.

- **An oversized `@LocalState` or `@SessionState` value fails loudly**
  (ADR 0059 amendment). The 64 KiB limit stays. A larger value was skipped
  without a word, so a reload brought back an older value. Now an assignment
  throws a `RangeError` that names the key, the size and the limit, and keeps
  the field's last value; a service method that sets such a value throws, or
  its promise rejects, and the saved copy keeps its last value.

- **Pages in the macOS and iOS apps no longer fail where they need a random
  id.** These apps serve the page from `tachyon-app://`, which is not a secure
  context, so `crypto.randomUUID` is missing there. Picking a file, starting a
  native service and starting a C# HTTP call each called it directly. A shared
  helper now uses `crypto.randomUUID` when it exists and otherwise builds a
  version 4 UUID from `crypto.getRandomValues`.

- **An app model with the name of a built-in event builds again** (ADR 0065
  amendment). Since 26.46.02, `develop` added built-in event models such as
  `UploadFinished`, `FilePicked`, `PushReceived` and `SocketOpened` to every
  page, so an app with its own model of that name stopped building (in Go:
  `UploadFinished redeclared`). Now the app's own model wins in every Tac
  language, and the build leaves out the built-in type of that name. The build
  stops with the new `TY1376` only when an `@OnEvent` listener of that model
  can also hear the built-in event (its browser API is granted, the page
  always sends it, or the service holds a socket client); the message names
  the model and says to rename it.

- **`ty test` and `ty start` agree on Yon handler code** (ADR 0122). Workers,
  identity providers, socket controllers and gateways now compose the error
  types of `server/errors/`, as route controllers and test units did; before,
  a Go identity provider or worker that used one failed with `TY2004` at
  `ty start`. A Go `@FromBody`, socket message or `@HandlesError` type from
  another package, such as `core.Order` from `server/packages`, now compiles
  under `ty start` and `ty bundle`; before, it failed with "undefined: core".
  The docs now say which request-body fields are required in each language
  (in Go, a field without `,omitempty` that is not a pointer) and why
  `ty test`, which calls methods directly, does not make that check.

- **The outgoing request guard no longer sends a request to the wrong host**
  (ADR 0113 amendment). For plain `http://` requests the guard checked only
  the first request on a proxy connection and then copied bytes, but Go,
  Python and Node.js reuse a proxy connection for any host. A later request
  could reach the first request's service, with its credentials. The guard
  now carries one checked request per plain HTTP connection: it sets `Host`
  to the checked authority, sends `Connection: close` to the service and to
  the client, forwards exactly one framed request body, answers `400` to a
  body with an unclear length, and closes the connection if the client sends
  a second request on it. `CONNECT` tunnels are unchanged.

- A bound boolean on an `aria-*` attribute now writes the text `true` or
  `false`. Before, `:aria-expanded="menu.open"` wrote `aria-expanded=""` for
  true, which a screen reader reads as false, and left the attribute out for
  false. Strings and numbers keep their text, `null` still leaves the
  attribute out, and other attributes such as `hidden` and `disabled` keep the
  HTML rule (ADR 0015, amendment of 2026-11-10). This holds for a Tac service
  in every language: a `bool` field in Rust, Go, C#, F#, Kotlin, Java, Swift,
  Dart, Ruby, Lua, Python, C++, JavaScript or TypeScript writes the same
  `true` or `false`. The new docs section "ARIA states from true-or-false
  fields" shows each language.

### Changed

- Go services may use the plain error check `if err != nil { return ..., err }`
  (or a `%w` wrap with `fmt.Errorf`) in a public method without `TY1314`, as
  Rust's `?` is allowed (ADR 0063, ADR 0119).
- A Go browser HTTP request or socket send in `OnStart`, a getter or a setter
  now fails with a message that names the event pattern to use instead.
- Swift services may use `guard let x = <expression> else { return nil }` and
  Kotlin services `val x = <expression> ?: return null` in a public method
  without `TY1314`: one binding and a `return` of nothing or of one plain
  value (ADR 0063, ADR 0119).
- A browser HTTP request or socket send that cannot wait in Kotlin, Java,
  Swift, Ruby and Lua now fails with `HTTP_REQUEST` and the same message as
  Go, which names the event pattern to use instead. The browser HTTP docs of
  every Tac language say which languages can wait in an `async` `OnStart`
  (ADR 0119).
- The docs now say that `YON_APPLICATION_JSON` holds the settings under a
  `yon` root (ADR 0119).
- `@SessionState` and `@LocalState` fields are now saved and restored on a
  service that only another service needs, in Go, Java, F#, Python and C++,
  as JavaScript, TypeScript and Dart already did. The key is the same as for
  a service that a view uses. A WebAssembly service's `init` answer lists the
  services it needs, and the page plan carries their saved fields (ADR 0119).
- A `Schedule` worker in `server/workers/` can take services through `Needs`
  fields in every Yon language, as a controller does: Tachyon builds the
  worker from `server/services/` and the other role folders and calls its
  instance `POST` on each run. A worker with no `Needs` compiles as before
  (ADR 0119).
- A Rust service linked into a native app now also saves and restores the
  `#[SessionState]`/`#[LocalState]` fields of a Rust service that it holds
  through `Arc` and that no view uses, on macOS, Windows, Linux, iOS and
  Android, with the same key. Its `init` answer lists `needs`, and the page
  opens a field-only activation through the holding service. Such a field is
  a `std::sync::Mutex<T>` or `std::sync::RwLock<T>`; a plain one stops the
  build (`TY1406`), because `Arc` gives shared access only (ADR 0119).

## [26.46.02] - 2026-11-10

### Highlights

- **The latest release.** 26.46.02 is the latest release on GitHub; no target
  is promoted to supported yet, and the no-support promise in `SUPPORT.md` is
  unchanged. Its stable release also deploys the website: the release's site
  zip goes to staging on Floci, and after kestrel's staging check passes for
  the same zip, to production at `tachyon.del.ma`.
- **Run the website's Yon examples.** The code examples show recorded runs
  of each Yon controller, and can run an example live through a snippet host.

### Added

**Website and docs**

- The website shows the recorded requests and answers of each Yon controller
  example (712 recordings in 15 languages, in `website/runs`), and a Run live
  button that sends a run to a snippet host (`website/server`,
  `website/runner`). Examples that need FYLO storage show their recorded run
  only.
- One website build calls the API origin of its own site: production,
  staging, or the page's own origin elsewhere.

**CLI and tooling**

- Each stable release attaches its website zip (`tachyon-site-<version>.zip`,
  with its SHA-256 and a build attestation), and `site-deploy.yml` deploys
  it to production after the staging check, rolling back when the production
  probes fail.

### Changed

**CLI and tooling**

- JSON that `ty` writes has its object keys in sorted order, however `ty` is
  built: route answers and bodies, storage results, topic records, Redis
  values, behavior records, SQL column lists and build output such as the
  service worker's asset list. Some output changes key order from earlier
  builds; the content is the same.
- Release builds use the new `shipped` feature set of `tachyon-cli`. Build
  from source with `cargo build --release --locked -p tachyon-cli --features
  shipped`; `storage-all` still works.
- CI and the storage tests use FYLO 26.45.07 (was 26.33.02), installed by
  `scripts/ci/install-fylo.mjs` with its checksum.

**Platform support**

- On Windows on Arm the installer now gets the ARM64 `ty` from the latest
  release.

### Fixed

**Storage and topics**

- SQLite operations stop at one operation deadline, so a slow query answers
  with the interrupt error instead of "deadline exceeded; writes may have
  committed".
- With FYLO 26.45.07, FYLO writes on Windows are fast again in one FYLO
  process, as Tachyon runs it: the first write takes about 2 s, later writes
  a few milliseconds (each took about 0.8 s with 26.33.02).

**Website and docs**

- The website's server realm has its own type-check configuration, and the
  Perl relay examples write JSON with sorted keys.

## [26.46.01] - 2026-11-09

### Highlights

- **One annotation model in every language.** Classes take their role from an
  annotation and a folder: `Service`, `Store`, `HTTPClient`, `Converter`,
  `Setup`, `Entity` and `Delegate`, with `Needs` for injection and shared
  behavior annotations such as `Cached`, `Retry` and `AllOrNothing`. Each
  language writes them in its own syntax, and the build checks every one
  (ADRs 0021, 0022, 0047). See `docs/annotations`.
- **Yon routes come from controllers.** A `RequestRoute` class in
  `server/controllers/` answers `GetRoute`, `PostRoute` and the other verbs,
  takes and returns models, and can stream (`StreamRoute`) or keep a socket
  open (`SocketRoute`) (ADR 0048). See `docs/controllers`.
- **`client/manifest.json` is the whole Tac configuration.** It is a standard
  web app manifest with a `tac` member for scripts, styles, caching, the
  native window, menus, links and the Yon origin. The build says what each
  member does on each target (ADR 0052). See `docs/application-configuration`.
- **Many more languages.** Tac services can be written in JavaScript,
  TypeScript, Rust, Go, C#, F#, Swift, Kotlin, Java, Dart, Ruby, Lua, Python
  and C++; the compiled ones run in the page as WebAssembly. Yon runs
  controllers in JavaScript, TypeScript, Python, Java, Groovy, Kotlin, C#,
  F#, PHP, Rust, Dart, Go, Ruby, Lua and C++. See `docs/service-languages`
  and `docs/yon-languages`.
- **Native apps on macOS, iOS, Android, Linux and Windows.** The web-view
  hosts now take their window, menus, tray icon, links and browser API
  grants from the manifest, open system file pickers, reach Yon through a
  declared origin, and `ty bundle --release` builds signed store and
  installer packages. See `docs/native-hosts`.
- **Safety rules are build errors.** The build refuses secrets in the source,
  code built from text, query text built from pieces, requests to hosts that
  the configuration does not list, and routes that do not say who may call
  them (ADR 0068). `ty start` also confines each handler's network access
  at the operating-system level (ADR 0113). See `docs/route-access` and
  `docs/operations`.
- **Warm Yon handlers.** One handler process now serves many requests, so a
  request after the first one no longer pays the runtime start
  (`YON_ISOLATION=warm`, the default; ADR 0111). See
  `docs/isolation-and-security`.
- **Topics.** A model marked `Topic` carries its events across processes,
  Yon languages and open pages, and Tachyon keeps each topic's values in a
  log under `data/topics/` (ADR 0061). Each topic declares who may read its
  stream. See `docs/topics`.

### Breaking

Each item is a change against 26.36.01. Run `ty migrate check` first: it
lists every place below that it can find. `ty migrate fix` prints a plan, and
`ty migrate fix --write` applies it. Items without a fix are manual steps.

#### Configuration and project layout

- `client/manifest.json` is the whole Tac configuration (ADR 0052).
  `tac.config.js`, `tac.config.mjs`, `tac.config.ts` and `tachyon.json` stop
  the build, and the message says where each value moved. The application
  name, entry route and icons come from the manifest's `name`, `start_url`
  and `icons`. The bundle id, version and window go to `tac.application`.
  Scripts, styles and cache rules go to `tac.scripts`, `tac.styles` and
  `tac.cache`, and page metadata to `tac.metadata`. The `postBundle` hook is
  removed: with `tac.origin` set, the build writes `sitemap.xml` and
  `robots.txt`, and `tac.sitemap` lists dynamic route values. Everything in
  the manifest ships to the client, so keep secrets out of it. Affects every
  Tac project. Migrate: `ty migrate fix --write` moves the plain values, then
  follow its manual steps (hooks and computed values) and delete the old
  files.
- Topic logs moved from `.tachyon/topics` to `data/topics/<name>.jsonl`, or
  to the folder that `topics.directory` in `server/application.json` names
  (ADR 0061 amendment). On start, the server moves old logs once, when the
  new folder is empty, and prints a notice. A path with `..`, a path inside
  `.tachyon`, or a symbolic link out of the project stops the server with
  `TY1303`. The stream URL `/.tachyon/topics/<name>` does not change.
  Affects projects that publish topic events, and scripts that read or wrote
  `.tachyon/topics` by hand. Migrate: nothing to do for the move itself;
  update such scripts. `ty migrate check` reports logs left in the old
  folder.
- The project-root `.env` holds only uppercase `YON_` secret names. It can
  no longer set framework transport, profile, endpoint, runtime or
  isolation names, such as the `YON_PORT`, `YON_HOST`, `YON_HOSTNAME` and
  `YON_SKIP_BUNDLE` in the 26.36.01 scaffold's `.env.example`; other names,
  such as `DATABASE_URL`, are refused too. `ty start` stops with `TY2010`.
  Affects projects that copied `.env.example` or keep other names in
  `.env`. Migrate: `ty migrate check` reports each refused name with its
  replacement (`environment.env_name`). `ty migrate fix --write` comments
  out the reserved names in `.env.example` and renames the others to `YON_`
  names; it never edits `.env`, so rename those names by hand and keep the
  values. Set the host and port in `server/application.json`
  (`server.host`, `server.port`) or on the command line.

#### Yon routes and server code

- `server/routes/` is no longer read (ADR 0048). A route is a controller in
  `server/controllers/<Name>Controller.<ext>`, marked `@RequestRoute(path)`,
  with an instance method per verb marked `@GetRoute`, `@PostRoute`,
  `@PutRoute`, `@PatchRoute` or `@DeleteRoute`. `@Controller` is refused
  with `TY1308` and names `@RequestRoute`. The server answers `HEAD` with
  the `GET` route and `OPTIONS` with the route schema. Affects every Yon
  project. Migrate: `ty migrate fix --write` turns each route file into a
  controller (and writes `@Public` on it, see the build rules below); it
  lists files it cannot convert, such as a file with no verb method or
  static methods that use `this`. It converts a `@Controller` class with
  static verb methods, such as the route that the 26.36.01 `ty init` wrote.
- `OPTIONS.schema.json` is no longer read. Request validation and the
  published API schema (`/api.json`, `/api-docs`) come from the controller's
  parameter markers (`@FromBody`, `@FromPath`, `@FromQuery`, ...) and model
  types. A broken `@Checked` rule answers `422` with an
  `application/problem+json` body (ADR 0101). Affects routes with a schema
  file. Migrate: manual. `ty migrate fix` lists each schema file; write the
  same rules on the model types, then delete the file.
- `YonRequest` and `YonResponse` are removed (ADR 0062). Each language uses
  its own types: Fetch `Request`/`Response` in JavaScript and TypeScript,
  `urllib.request.Request` and an `(HTTPStatus, headers, body)` tuple in
  Python, the superglobals with `http_response_code()`, `header()` and
  `echo` in PHP, `HttpExchange` in Java and Kotlin, `HttpRequestMessage` and
  `HttpResponseMessage` in C#, and `http::Request<Vec<u8>>` and
  `http::Response<Vec<u8>>` in Rust. Middleware, `relay(command, request)`,
  handlers and error handlers all receive the native request. A PHP method
  takes no request parameter, and relay programs no longer receive path
  parameters. The build refuses the old types with `TY2015`. Affects
  handlers that named these types. Migrate: `ty migrate fix --write` removes
  an unused `YonRequest` parameter and lists every other use.
- PHP Yon code runs with `php-cgi` instead of `php` (ADR 0062), so
  `header()`, `header_remove()` and `headers_list()` are PHP's own
  functions. Affects every project with PHP routes, middleware or workers.
  Migrate: install `php-cgi` before you upgrade (Homebrew `php` includes it;
  Debian and Ubuntu: the `php-cgi` package; Windows: `php-cgi.exe` in the PHP
  zip). `ty doctor` checks for it.
- Yon keeps handler processes warm by default (ADR 0111). `YON_ISOLATION`
  takes `warm` (new default), `process` (the 26.36.01 behavior: one process
  per request) or `firecracker`. A warm process serves many requests of one
  handler, one at a time, so module-level state (a static counter, a
  module-level map) now survives between requests. `YON_WARM_MAX_REQUESTS`
  and `YON_WARM_IDLE_SECONDS` tune it; setting them with `process` or
  `firecracker` fails with `TY2010`. Affects handlers that relied on a fresh
  process per request. Migrate: no fix. Reset that state in the handler, or
  set `YON_ISOLATION=process`.
- `@Repository` is now `@Store`, and `@Client` is now `@HTTPClient`
  (ADR 0047). The folders follow: `server/repositories/` is
  `server/stores/`, and `server/remotes/` is the outbound folder
  (`server/clients/` is still accepted). The old names fail with `TY1308`
  and the old folder with `TY1310`. Migrate: `ty migrate fix --write`
  renames the markers, moves the files and renames the moved types.
- `@Store` and `@HTTPClient` are contracts with no implementation (ADR 0022
  and ADR 0048 amendments). A Store names its Entity and declares only
  derived queries or `@Query` methods. An HTTPClient is an interface (or the
  language's contract form) whose methods carry request markers. A method
  body, a field or a constructor is refused with `TY1308`. Affects every
  26.36.01 repository or client class with code in it. Migrate: move the
  logic into a service, and SQL into a service that asks for `DataSource`.
  `ty migrate fix --write` rewrites only stores whose methods are all
  queries and clients whose methods only send requests; it lists the rest.
- `@Relay` and `@Stream` are removed. Call `relay(command, request)` in a
  method instead of marking it `@Relay`; the build refuses the marker. A
  streaming route is one method marked `@StreamRoute(path?)` whose name
  starts with `stream` (`streamTicks`, `stream_ticks`); a generator under
  another route marker is refused. A finished stream now ends with
  `event: end`. Affects relay delegates and streaming routes. Migrate:
  `@Relay` is manual. `ty migrate fix --write` converts `@GetRoute` with
  `@Stream` to `@StreamRoute` and renames the method when that is safe; run
  it again after the route conversion above.
- Root middleware is a class named `Middleware` with `before(request)` and
  `after(request)` (ADR 0044). A missing method lets that phase pass. Any
  other declaration in the file fails with `TY1310`. Affects every project
  with a `middleware.<ext>`. Migrate: `ty migrate fix --write` converts a
  `Handler` class whose HTTP method functions hold the same code; other
  shapes are listed as manual steps.
- An annotated file, plain handler, worker or middleware holds one class and
  its imports (ADR 0044, `TY1310`), and every class file is named after its
  class with the same case (ADR 0067, `TY1318`). Helper functions or extra
  types in these files stop the build. Migrate: `ty migrate fix --write`
  renames a file after its class (or the class after its file); it lists
  each file that needs a split.
- A `relay(command, request)` program now gets only the baseline
  environment, the explicit allowlist, `YON_HANDLER_PROTOCOL` and its
  handler's `NO_PROXY` values. It no longer receives
  `YON_APPLICATION_JSON` or any `YON_RESOURCE_*` value, even when
  allowlisted. Affects relay programs that read them. Migrate: manual;
  read configuration from the program's own files or an explicitly
  allowlisted variable.
- The operating system now confines handler network access (ADR 0113).
  While the outgoing guard runs (`ty serve`, `ty start`, `ty preview`),
  every handler process, relay and Gateway command can connect only to
  its server's outgoing guard and resource door on loopback. macOS uses a
  Seatbelt profile (`sandbox-exec`); Linux 6.7 or newer uses Landlock and
  seccomp. Raw sockets, UDP, DNS, ICMP, non-HTTP clients, Unix sockets to
  local daemons and hosts not in `outgoing.hosts` are refused. `ty start`
  stops with `TY2113` where this can't apply (Windows, Linux older than
  6.7 or without Landlock, gVisor, no working `sandbox-exec`); `ty serve`
  and `ty preview` start with a `handler.egress_unenforced` warning.
  Affects every handler that makes network calls. Migrate: manual. List
  every host the handlers call in `outgoing.hosts`, and use managed storage
  instead of direct database drivers. Where the platform already
  enforces egress, or on Windows and older Linux kernels, set
  `YON_EGRESS_ENFORCEMENT=proxy` (the default is `os`).
- A server `@Topic` model must say who may read its stream: `@Public`,
  `@SignedIn` or `@Allow(Role)`, in its language's spelling (ADR 0061
  amendment). A model without one stops the build with `TY1322`, a marker
  on a `client/` topic model is `TY1308`, and languages that declare the
  same topic must write the same rule. Affects projects with topic models.
  Migrate: manual; `ty serve` or `ty bundle` lists each model with its file
  and line. Add `@Public` to keep the 26.36.01 behaviour (anyone may read
  the stream), or `@SignedIn` or `@Allow(Role)` to guard it. See
  `docs/topics` for each language's spelling. The default 26.36.01 scaffold
  has no topics.
- A handler or middleware call that runs past its deadline (`TY2110`)
  answers `504 Gateway Timeout`, and one cancelled because the server is
  stopping (`TY2111`) answers `503 Service Unavailable` with
  `Retry-After: 1`. Both answered `500` in 26.36.01; other failures still
  answer `500`. Affects clients and monitors that expect `500`. Migrate:
  manual; treat `504` and `503` as retryable.
- `/api.json` and `/api-docs/` list only the routes that a caller without an
  identity may call (`@Public`), and only the models those routes name, the
  rule `OPTIONS` follows. A guarded route's schema now comes from `OPTIONS`,
  for a caller that the route's rule lets in, and a project whose routes are
  all guarded no longer publishes `/api.json` or `/api-docs/` (they answer
  `404`). Affects clients and tools that read guarded routes' schemas from
  `/api.json` without signing in. Migrate: read them with `OPTIONS` and
  credentials. `ty api freeze` and `TY1338` still compare every route, so
  existing `server/api-baseline.json` files are unaffected.

#### Tac pages, components and services

- A page directory holds only `tac.html`, and that file holds only component
  tags, whitespace and comments (ADR 0029). Markup, page state, companions
  and stylesheets next to a page fail with `TY1309`, which names the
  component to move them into. Affects every page with its own markup.
  Migrate: `ty migrate check` reports page companions and page markup
  (`view.page_markup`). `ty migrate fix --write` moves the markup to
  `client/components/page/<name>/` and places `<page-<name> />` in the
  page. In a one-page app it also moves the page's `tac.css` foundation
  rules to `client/shared/styles/` and its `tac.js` to
  `client/shared/scripts/`, listed in `tac.styles` and `tac.scripts`. Other
  cases are manual: move the view to a component and place it in the page.
- Page and component companions (`tac.js`, `tac.ts`, `tac.swift`, `tac.kt`,
  `tac.cs`, `tac.rs`) are no longer discovered (ADR 0030). A component holds
  `tac.html` and optionally `tac.css`; a companion next to it fails with
  `TY1310`. Behavior is a `@Service` class in
  `client/services/<Name>Service.<ext>`, and a view reaches it by its
  instance name (`{basket.count}`, `on:click="basket.add()"`). A Rust service
  shows the page only its `pub` methods (`TY1311`). Migrate: manual.
  `ty migrate check` lists `companion.component`, `companion.controller` and
  `companion.polyglot` with the target file.
- The `this.tac` object and the `@onMount`, `@publish` and `@subscribe`
  decorators are removed. The build refuses the decorators. Use `@OnStart`
  for start-up work, an `@HTTPClient` for requests (mark a public read
  `@Cached`), and application events (`publishEvent(model)` and `@OnEvent`)
  instead of `this.tac.publish` and `this.tac.subscribe`. Migrate: manual,
  together with the companion move.
- `$name` and `$$name` fields are now `@SessionState` and `@LocalState`
  fields (ADR 0059). A `$` or `$$` field is refused with the marker to write.
  A value saved under the old key moves to the new key once. Migrate:
  `ty migrate fix --write` converts the fields, their uses and the view
  expressions.
- Native hosts no longer define `globalThis.tachyonWindow`, and
  `tachyon-env.d.ts` no longer declares it (ADR 0087). Window controls come
  from the injected `WindowControls` browser API. A JavaScript or TypeScript
  service that reads `globalThis` fails with `TY1360`, and one that calls
  `fetch` fails with `TY1361`; use an `@HTTPClient`. Setups and shared
  scripts may still call `fetch`. Migrate: manual.
- `<if :when>`, `<else>` and `<for :each>` are removed (ADR 0015 amendment).
  Write `<logic :if>`, `<logic :else-if>`, `<logic else>` and
  `<loop :for="item of items">`. The build refuses the old tags with
  `TY1367`. `<switch>` and `<case>` do not change. Migrate:
  `ty migrate fix --write` rewrites them (`item in items` becomes
  `item of items`).
- Braces in a plain attribute, such as `class="status-{state}"`, stop the
  build with `TY1313`; before, the braces were written out as text. Braces
  work only in text. Write a bound attribute: `:title="name"`,
  `:class="'status-' + state"`, or `:hidden="condition"`. Regular-expression
  counts, JSON data, `\{` and `<style>` text are still allowed. Migrate:
  `ty migrate fix --write` converts whole-value cases such as
  `title="{name}"`; the rest are manual.
- Client scripts cannot build markup or write styles (`TY1364`): no
  `innerHTML`-style writes, `document.write`, `insertAdjacentHTML`,
  `document.createElement`, node insertion, `element.style` writes or style
  sheet edits, in services, component scripts and `client/shared/`.
  Global stylesheets in `tac.styles` hold foundation CSS only (colors,
  fonts, custom properties, resets; `TY1403`). A third-party script or
  stylesheet that needs an exception is declared in `tac.scriptSources` or
  `tac.styleSources` with package, exact version, license and SHA-256
  (`TY1365`, and `TY1403` on a hash mismatch). Migrate: manual. Move markup
  to `tac.html` and layout to the component's `tac.css`.
- A Tac view that contains a doctype, `<html>`, `<head>` or `<body>` fails
  the build with `TY1307`; 26.36.01 accepted it. Tachyon writes the
  document around every view. Migrate: manual; remove these tags and put
  page metadata in `client/manifest.json` under `tac.metadata`. The same
  tags written as text in `<pre>` or `<code>` still work.

#### Native apps

- Windows release builds no longer open WebView2 DevTools (F12, Inspect).
  Use a debug build to inspect the page.
- A native page can no longer post a form to, or load, a remote page in its
  main frame. Android refuses such loads. Migrate: open remote pages
  externally (links already open in the system browser).

#### New build-refusing rules

- Code that built with 26.36.01 can now fail these checks. `ty migrate check`
  lists every finding. Most rules have a switch, `tac.<rule>.check: false`
  in `client/manifest.json` or `<rule>.check: false` in
  `server/application.json`, for use only while you migrate. There is no
  automatic fix unless noted.
  - Cohesion (ADR 0063, `TY1314`, switch `cohesion`): a public service or
    controller method only coordinates (no branch, loop, `&&`/`||`/`??` or
    `try`); a private method holds at most one decision.
  - Duplication (ADR 0064, `TY1315`, switch `duplication`): a block of 100
    tokens over 10 lines repeated in one language and realm.
  - Safety (ADR 0068): secrets in source (`TY1324`; move them to `.env`),
    `eval`-style code and query text built from pieces (`TY1325`), requests
    to hosts not listed in `tac.outgoing.hosts` or `outgoing.hosts`
    (`TY1326`), a route without `@Public`, `@SignedIn` or `@Allow(...)`
    (`TY1322`, no switch), and a guarded route without exactly one
    `@IdentityProvider` (`TY1323`). `ty migrate fix --write` writes
    `@Public` on the controllers it makes from old route files, which were
    open before; it never adds a rule to another controller.
  - Design (ADR 0069): dependencies point down with no loops (`TY1327`,
    `dependencies`), plain English names (`TY1328`, `names`; add domain words
    to `names.words`), at most 4 parameters and 5 needed classes (`TY1329`,
    `signatures`), and named constants for repeated values (`TY1330`,
    `constants`).
  - Visibility (ADR 0071, `TY1333`, switch `visibility`): a public member
    that nothing outside its class uses. `ty migrate fix --write` makes the
    simple cases private.
  - Front end (ADR 0074): accessible views (`TY1336`, switch
    `accessibility`), and in release builds a per-route budget of 200 KB of
    script and 3584 KB of WebAssembly, Brotli-compressed (`TY1337`,
    `tac.budgets.*`, switch `tac.budgets.check`).
  - Role folders (ADR 0091): every concrete class in a role folder carries
    its role marker (`TY1363`; move plain collaborators to `models/` or
    `shared/`), and a JavaScript, TypeScript or Dart file may not import a
    file of the other realm (`TY1369`; call the server through an
    `@HTTPClient`).

### Added

**Tac**

- Pages compose components, and components use services (ADRs 0029, 0030):
  a page's `tac.html` holds only component tags, a component holds
  `tac.html` and an optional `tac.css`, and behavior lives in one `@Service`
  class in `client/services/`. See `docs/pages`.
- Views read `client/shared/data/<name>.json` as `data.<name>` (ADR 0036),
  write conditionals with `<logic :if>` and loops with `<loop :for>`, and
  render HTML data through an allowlist with `:safe-html`. See
  `docs/templates`.
- `tac.elements` exports a component as a standard custom element for other
  apps (ADR 0086). See `docs/web-components`.
- Services run in the page in every service language: Rust, Go, C#, F#,
  Swift, Kotlin, Java, Dart and C++ as WebAssembly, Ruby on mruby, Lua on
  Lua 5.4 and Python on Pyodide. Release builds use the compiler that gives
  the smallest download (ADR 0081). See `docs/service-languages`.
- Services call Yon through `HTTPClient` contracts in `client/remotes/`
  with `GetRequest` and the other request annotations (ADR 0088).
  `StreamRequest` and `SocketRequest` deliver messages to `@OnEvent`. See
  `docs/calling-the-server`.
- Services reach the browser through injected APIs (ADR 0065): `Clock`,
  `SecureRandom`, `Connectivity`, `PageVisibility`, `Preferences`,
  `PreferenceStrings`, `DocumentTitle`, `Clipboard`, `Share`,
  `Notifications`, `Geolocation` and `WindowControls`, granted in
  `tac.browserApis`. See `docs/platform-apis`.
- Built-in event models such as `ApplicationStateChanged` reach every
  service language; listeners may be async in Swift, Rust, Kotlin, C#, F#
  and Python (ADR 0105).
- `tac.blockingScripts` runs up to 8 local scripts before the first paint.
- Both realms share one annotation model: `Service`, `Store`, `HTTPClient`,
  `Converter`, `Setup`, `Entity` and `Delegate`, one class per file, in role
  folders that may nest (ADRs 0044, 0055, 0067, 0091). Each language uses
  its own syntax, such as `[Service]` in C#, `// @Service` in Go or
  `[[Service]]` in C++. See `docs/annotations`.
- Injection with `Needs`, `NeedsAll`, `Named` and `Lifetime`; a filled
  field is never null (ADR 0057). See `docs/dependency-injection`.
- A `@Delegate` takes on the public methods of the service it needs, and can
  override or overload them (ADR 0112). See `docs/delegates`.
- Behavior annotations: `Cached`, `CacheKey`, `CacheSettings`,
  `UpdateCache`, `ClearCache`, `Retry`, `MaxAtOnce`, `AllOrNothing`,
  `PageDefaults` and `Description` (ADR 0022). See
  `docs/behavior-annotations`.
- `Setup`, `Settings`, `Setting`, `WhenProfile` and `WhenSetting` bind
  configuration (ADR 0092). See `docs/application-configuration`.
- `Checked`, `Check`, `Required`, `NotEmpty`, `Length`, `Min` and `Max`
  check values; Yon answers `422` problem details (ADR 0101). See
  `docs/checking-values`.
- `Converter` contracts copy one model into another, nested models and
  lists included. See `docs/models`.
- Events are models (ADR 0055): `publishEvent(model)` reaches `@OnEvent`
  listeners, with `condition`, `replay`, `Order` and `InBackground`, and
  `OnStart` and `OnStop` hooks. See `docs/application-events`.
- Utility classes in `client/shared/utils/` and `server/utils/`. See
  `docs/utilities`.

**Yon**

- `TAC_ACCESS_LOG=1` writes one `http.access` JSON line per request to
  standard error, and `TAC_HANDLER_STDERR=1` writes each handler call's
  standard error as a `handler.stderr` event.
- WebSocket sends time out after 10 seconds (close `1013`) and stop at once
  when the server stops.
- Seven more Yon languages: Groovy, F#, Dart, Go, Ruby, Lua and C++. See
  `docs/yon-languages`.
- Yon code in every language, C++ included, can call plain helper classes
  in `server/shared/`, and `ty init` and `ty add` accept `cpp` for the
  server.
- Controllers (ADR 0048): a `RequestRoute` class in `server/controllers/`
  with `GetRoute`, `PostRoute`, `PutRoute`, `PatchRoute` and `DeleteRoute`;
  `FromBody`, `FromPath`, `FromQuery`, `FromHeader` and `FromCookie` fill
  parameters, and models come back as JSON. See `docs/controllers`.
- `@Public`, `@SignedIn` and `@Allow(Role)` say who may call a route, and an
  `@IdentityProvider` says who is signed in (ADR 0068). See
  `docs/route-access`.
- `StreamRoute` methods send server-sent events (ADR 0058), and
  `SocketRoute` classes keep a WebSocket open with `OnOpen`, `OnMessage`,
  `OnClose` and `OnError` (ADR 0060). See `docs/streams` and `docs/sockets`.
- `ErrorHandler` classes answer typed errors (ADR 0051). See
  `docs/error-handlers`.
- A bodiless `@Gateway` method is answered by another Yon language
  (ADR 0042). See `docs/relays-and-gateways`.
- `@Schedule` runs a worker on a fixed delay, and `HTTPClient` and
  `restClient` call other servers.
- Warm handlers: `YON_ISOLATION` takes `warm` (the default), `process` or
  `firecracker`, limited by `YON_WARM_MAX_REQUESTS` and
  `YON_WARM_IDLE_SECONDS` (ADR 0111). See `docs/isolation-and-security`.
- `server.environment` passes chosen environment variables to handlers,
  `server.proxies` lists trusted proxies, and `localhost` names such as
  `shop.localhost` work (ADRs 0094, 0096).
- Go apps keep their own Go packages in `server/packages/` (ADR 0095).

**Native**

- `ty bundle --release` builds signed packages from keys in the environment:
  `.dmg`, `.ipa`, APK and App Bundle, MSIX, AppImage and `.deb` (ADR 0104).
  `--arch` builds universal macOS and ARM64 Windows apps. See
  `docs/build-and-deploy`.
- `tac.application.window` sets the window's size, colors and display, and
  `"titleBar": "integrated"` lets the page reach the top edge (ADR 0098).
  See `docs/window-controls`.
- Standard menus on every host; `tac.application.menus` adds items that open
  a page or call an application service (ADR 0097), and
  `tac.application.tray` adds a tray icon (ADR 0107). See
  `docs/native-menus`.
- `tac.application.links` opens the app from URL schemes and verified hosts
  (ADR 0106), and System Back follows the page history (ADR 0109).
- Hosts answer `clipboard`, `share`, `notifications` and `geolocation`, and
  `FilePicker` opens the system picker (ADR 0089). See `docs/devices`.
- `tac.yon.origin` lets native apps reach Yon, with `tac.cache` rules for
  offline reads (ADR 0103).
- Rust services link natively into iOS and Android too. Linked Rust, Swift,
  Kotlin and C# services can wait, reach Yon and hold stream, socket and
  topic clients. See `docs/native-services`.
- Application services live as long as the app, and `tac.application.rust`
  adds crates and helper programs. See `docs/companions`.
- `ty preview --target <native>` runs the real app against the development
  server (ADR 0110).
- `tac.application.build` and `tac.application.minimumSystem`, and a build
  note on what each manifest member does per target (ADR 0102).

**Storage and topics**

- `Store` contracts declare derived queries or `@Query` methods and can
  extend `Repository<Entity, Id>`. See `docs/storage`.
- Tac stores use IndexedDB and caches use the Cache API with no adapter
  code; `@LocalState` and `@SessionState` keep fields (ADR 0059). See
  `docs/browser-storage`.
- Yon owns its datasource and Redis connections (ADR 0027). FYLO is a
  datasource with `patch`, `scroll` and a durable queue (ADR 0032). See
  `docs/server-data`.
- `@Topic` models carry events across processes, Yon languages and open
  pages, with logs in `data/topics/` (ADR 0061). See `docs/topics`.

**Testing**

- `ty test` runs `<Name>Test` units in both realms and every language, with
  `@Test(Setup)`, `TestSetup`, `Fake`, `FreshStart`, `UseProfiles` and
  `TestSettings`; browser units run under Node, Bun or Deno. See
  `docs/test-units`.
- Every language has a JUnit 5-style `Assert`; a test case needs one and
  holds no branch (`TY1339`, ADR 0080).
- `ty test --coverage` counts a method only when a test called it (ADR 0070).
- Kotlin test units can fake browser APIs, window controls included, with
  `BrowserFakes.answer`.

**CLI and tooling**

- `ty init` is a setup wizard that writes examples and tests per language
  and installs missing tools (ADR 0066); `ty add language|target|feature`
  extends an app.
- `ty doctor` lists target and release tools; `ty doctor --install` installs
  them.
- `ty migrate fix --write` rewrites retired forms, and `ty types` writes
  TypeScript globals (ADR 0053).
- `ty api freeze` saves an API baseline that later builds must keep
  (`TY1338`).
- Design, safety and front-end rules are build errors. See **New
  build-refusing rules** under Breaking.
- Simpler-form suggestions (`TY1340`–`TY1355`, `TY1362`). See
  `docs/simpler-forms`.
- `ty preview` keeps the last good build and shows an error box (ADR 0077);
  `ty serve` and `ty preview` restart when `ty` changes.
- Release web builds write Brotli and gzip copies (ADR 0075).
- `ty-windows-arm64.exe`, picked by `install.ps1` on ARM64 Windows.

**Website and docs**

- Every docs section and the first-app tutorial show each language that
  runs them.
- An annotations A–Z table, a glossary and an error codes page. See
  `docs/annotations` and `docs/glossary`.
- The README lists every `ty` command and the environment settings, has an
  "Upgrading a 26.36.01 app" guide, and says how to install a specific
  release or roll back with `TAC_BASE_URL`.
- `docs/CUTOVER.md` lists this release candidate's gates: which passed and
  which are not claimed. No target is promoted to `supported`.
- Feature pages, including a new one for error handlers, checked against the
  code. The language and target pages link `docs/SUPPORT_TIERS.md` for what
  each tier means.
- Website accessibility: the auto theme follows the system light or dark
  setting with the full brand palette, and the theme toggle names its state.
  Code-sample tabs work from the keyboard (arrow keys, Home and End), and copy
  buttons stay visible and announce "Copied" to screen readers. Search has a
  button on phones and tablets and opens as a modal dialog that keeps focus
  and returns it on close; it loads its data only when opened, so the home
  page loads about 376 KB less. The skip link shows clearly in both themes, and
  the header no longer clips on phones. The
  mobile docs drawer moves focus in, closes with Escape and returns focus to
  its toggle. Docs pages
  have one main landmark, the feature filter has a visible border and shows
  and announces how many features match, controls have 44px touch targets, and text meets
  4.5:1 contrast.

### Changed

**Tac**

- Routes load less script: the Tac client runtime and the behavior runtime
  ship without comments, indentation or blank lines, about 1.2 KB less with
  Brotli on every route.
- The service worker no longer leaves a file out of the offline copy
  silently: the build prints an `Offline:` note for each file it cannot
  keep. A cache write that finds 16 writes running now waits for a free slot
  instead of being dropped, so every file of a large page is kept on the
  first visit.

**Yon**

- Yon runs JavaScript handlers with Node by default, and with Bun only when
  Node is not installed. `YON_JAVASCRIPT_RUNTIME` still picks the runtime.
- `server.maxbody` in `server/application.json` raises the request body
  limit from 1 MiB up to 12 MiB. A larger body still gets `413`, now after
  Yon has read up to 16 MiB of it, so a client that is still uploading sees
  the `413` instead of a broken connection.
- When a page and a controller share a path, a browser navigation gets the
  page and a fetch from the page gets the controller. Before, the route
  always answered. Both answers carry `Vary: Sec-Fetch-Mode, Accept`.
- A relay's deadline is the time left in the request, shared by every relay
  in that request, and `request.deadline_ms` shows the time left instead of
  the original value. Handler code cannot change it.
- Every relay failure reaches the handler as `502` with one message,
  `Component invocation failed.`, in every Yon language. Relay output that
  is JSON but not an object is a `protocol` failure everywhere.
- Compiled Kotlin and Rust handlers are cached once per machine in
  `TAC_CACHE_DIR/runtime/handlers-v1`, so another project or a fresh
  checkout reuses them instead of compiling again. The cache keeps the
  newest 512 entries, and `ty cache clean` empties it.
- A server checks its language toolchains at the same time instead of one
  after another, so a Kotlin server starts sooner. Results and the first
  failure reported are unchanged.

**Native**

- The Windows and Linux apps check the web view version when they start and
  explain a missing or older one instead of opening a blank window.

**CLI and tooling**

- `ty doctor` exits with 1 when a toolchain that the project needs to
  build or serve is missing or too old (26.36.01 exited with 0). A
  toolchain needed only to bundle a native target is shown as `info`
  and does not change the exit status.
- `ty doctor` and `ty migrate` are listed in `ty help`, and their help text
  says what they do (`ty migrate fix --write` included).
- The repository is now `d31ma/TACHYON`. The installers, package metadata,
  website links and release signatures use the new name; old URLs
  redirect.
- `ty serve` and `ty preview` start faster. A restart serves the last build
  at once, rebuilds in the background and then reloads open pages.
  Toolchains build in parallel, and `TAC_BUILD_JOBS` limits how many run at
  once. Each build prints its phase timings.
- `ty` help, usage and argument errors are plain text, without colors. The
  wording is unchanged.

**Platform support**

- The iOS app needs iOS 17.4 (was 17.0), the first version that runs CSS
  `@scope`, which component styles compile to. A lower
  `tac.application.minimumSystem.ios` stops the build with `TY1601`.
- The Windows app needs the WebView2 Runtime 118.0 or newer.
- The Linux app keeps its WebKitGTK 2.40 floor, and its `.deb` package
  now declares it. Component styles need WebKitGTK 2.44, and a Linux build
  with component stylesheets prints a note saying so. macOS and Android
  builds note that they need Safari 17.4 or Android System WebView 118.
- macOS (13.0), Android (API 26, AndroidX WebKit 1.14.0) and the Rust
  version (1.97.1) are unchanged.
- Java browser services on Android need Android System WebView (Chrome) 137
  or newer. This release candidate did not test them on Android, because
  the test emulator has WebView 124.
- macOS app bundles that `ty bundle --release` builds are not notarized in
  this release candidate. The Developer ID and notarization gate is an
  accepted exception (see `docs/CUTOVER.md`). The `ty` executable itself
  ships with Sigstore attestations, as before.

### Removed

Each item existed in 26.36.01. The Breaking section gives the replacement
and the migration step.

- `tac.config.js`, `tac.config.mjs`, `tac.config.ts`, `tachyon.json` and the
  `postBundle` hook. Use `client/manifest.json`.
- File routes under `server/routes/`, `OPTIONS.schema.json` route schemas,
  static verb-method handlers and the `@Controller` marker. Use
  `@RequestRoute` controllers in `server/controllers/`.
- The `@Relay` and `@Stream` markers. Use `relay(command, request)` and
  `@StreamRoute`.
- `YonRequest` and `YonResponse`. Each Yon language uses its native request
  and response types.
- The `@Repository` and `@Client` names and the `server/repositories/`
  folder. Use `@Store` in `server/stores/` and `@HTTPClient` in
  `server/remotes/` (`server/clients/` still works).
- Repository and client classes with implementation code. `@Store` and
  `@HTTPClient` are contracts.
- Running PHP handlers with `php`. Yon now starts `php-cgi`.
- The Tac `<if :when>`, `<else>` and `<for :each>` elements. Use `<logic>`
  and `<loop>`.
- Page and component companions (`tac.js`, `tac.ts` and the native
  `tac.swift`, `tac.kt`, `tac.cs`, `tac.rs`), and markup or state in a page
  directory. Use components and services in `client/services/`.
- The `this.tac` object (`fetch`, response caching, `publish`, `subscribe`)
  and the `@onMount`, `@publish` and `@subscribe` decorators.
- `$name` and `$$name` stored fields. Use `@SessionState` and `@LocalState`.
- `globalThis.tachyonWindow` in native hosts. Use the injected
  `WindowControls` browser API.
- `.tachyon/topics` as the topic log folder. Logs live in `data/topics/` and
  move there once on start.
- Runtime decorator shims, generated layer annotations and Rust
  procedural-macro compilation. Compile-time annotations (ADR 0021) replace
  them, without their runtime shims.

### Fixed

**Tac**

- SVG renders correctly: elements are made in the SVG namespace, `viewBox`
  keeps its case, and HTML inside `foreignObject` is HTML. An SVG icon no
  longer shows as an empty box.
- An update keeps the elements whose values did not change, so focus, the
  caret, the scroll position and an open `<select>` survive it. A `<select>`
  whose options come from a loop shows its bound value, not the last option.
- A page served by `ty start` or `ty preview --static` reloads offline. The
  servers sent `Cache-Control: no-store` on every build file, so the service
  worker kept nothing.
- After a deploy, a page that the previous build's service worker served
  reloads once when the new worker takes control, so it no longer mixes old
  and new scripts. It does not reload on a first visit, after the visitor
  has typed into the page, while offline, or twice within 10 seconds.
  On the first deploy after 26.36.01, sites whose static host or CDN cached
  `register-sw.js` get the reload from the following deploy on; `ty start`
  sent `no-store`, so it is not affected.
- The console stays clean: fast navigation no longer logs `AbortError:
  Transition was skipped`, the idle precache no longer shows failed
  `net::ERR_ABORTED` requests, and WebKit and the Linux and Windows hosts no
  longer refuse the inline speculation rules (allowed by hash now).
- The note that the scaffold writes for a new Kotlin service says what the
  page runs: the JavaScript copy written next to the generated service,
  until you delete that copy and the service compiles to WebAssembly with
  garbage collection (WasmGC). It said the service always compiled to
  WasmGC.
- `TY1605`, the build's refusal of a Ruby Delegate that has an `OnStart`,
  `OnStop` or `OnEvent` method, points at the hook and names the Delegate's
  real file, including one in a subfolder. It named
  `client/delegates/<name>.rb`, with no source location.

**Yon**

- On Windows, a PHP handler's relay reads the program's answer through a
  pipe, and a watchdog process ends the program's process tree at the relay
  deadline. The answer used to come back over a loopback socket, which
  Windows resets when the program exits, dropping any answer the relay had
  not read yet; on a busy machine the relay answered `502` "Component
  invocation failed.".
  A Windows PHP relay program must now read its whole request before it
  writes more than one pipe buffer of output, or it waits until the deadline
  (docs/YON_ANNOTATIONS.md).
- On Windows, a Yon Lua Retry delay waits with PowerShell's
  `[Threading.Thread]::Sleep`. `Start-Sleep` took 12 to 23 s to start in a
  handler's environment, so two retries passed the 30 s handler deadline
  (ADR 0085).
- On macOS 26, Java, Kotlin and Groovy handlers start with IPv4 sockets, so
  the sandbox lets them reach the egress guard. They failed with
  `java.net.ConnectException: Operation not permitted`.
- On macOS 26, C# and F# handlers also use IPv4 sockets, so the sandbox lets
  them reach the egress guard and the resource door. Outgoing requests failed
  with `Permission denied`, and gateway calls answered 500.
- The first Lua or Ruby page build downloads its pinned runtime sources with a
  15 s connect timeout, aborts a download that stalls for a minute, and retries
  twice. A source line may list more than one URL, tried in order, and the
  SHA-256 pin still decides what is used. The Lua 5.4.9 line falls back to the
  buildroot and Alpine source mirrors. When `www.lua.org` did not answer, the
  build waited up to five minutes on the connection and then failed.
- A `ty` built without `--features storage-postgres` now names that feature
  when a project asks for PostgreSQL, even with `PGHOST`, `PGPORT` or other
  `PG*` variables set. Before, it reported the ambient `PG*` settings and hid
  the real cause.
- `ty serve`, `ty dev` and `ty start` drain on `SIGTERM`, `SIGINT` or
  Ctrl-C: they stop accepting connections, close streams and WebSockets
  (`1001`), and let running requests finish within
  `TAC_SHUTDOWN_GRACE_SECONDS` (default 10, at most 300) before cancelling
  them. 26.36.01 cancelled running requests at once.
- Handler standard error over 64 KiB per call is cut (the first 48 KiB and
  the last 16 KiB are kept) instead of failing the call with `TY2107`. Relay
  programs and Gateway commands no longer fail a call on long standard
  error; their standard output limit still applies.
- `handler.invocation_failed` events name the `route` and the `handler`.
- A handler or middleware answer no longer sets `content-length`,
  `transfer-encoding` or `connection` on the response, which could stop the
  connection.
- The start-up check no longer reports a working `kotlinc` or .NET SDK as not
  usable (`TY2101`) on a busy machine. Each check has its own time limit.
- A compiled handler whose `rustc`, `kotlinc` or `dotnet` is not on `PATH`
  fails with `TY2112`, which names the missing program, instead of `TY2004`.
- A C# handler build no longer hangs on an old .NET compiler server.
- A PHP relay program no longer inherits the handler's open descriptors, so
  it cannot write into the handler's response.
- A JavaScript relay to a missing program fails at once with `start` instead
  of waiting for the deadline.
- `YON_ISOLATION=firecracker` is refused on Windows with `TY2010` at start-up
  instead of running without its driver checks.

**Native**

- A Windows app embeds its application manifest. Windows no longer asks for
  administrator rights for an app named with `Setup`, `Install`, `Update` or
  `Patch`, and the app stays sharp on scaled displays.
- `http`, `https` and `mailto` links open in the default browser or mail app
  on Windows and Linux, as on the other hosts. Before, nothing opened.
- Linux apps keep their web storage after they quit.
- A native reply over 64 KiB on Linux and Windows fails with a clear error
  instead of "Invalid native response.", and a large Windows page request no
  longer hangs for 10 seconds.
- An app name, bundle ID or route that looks like a host placeholder, such as
  `__ROUTE_DOCUMENTS__`, stays plain text in every host.
- A native build no longer writes temporary folders or type declarations into
  the project source.
- A web bundle file over 16 MiB stops the native build with `TY1605` instead
  of failing silently at run time.
- A Windows build with a C# companion works from a Visual Studio developer
  prompt, and Windows apps no longer ship stray `.lib` and `.exp` files.
- Staging a macOS or iOS app off a Mac no longer fails on `sips`; the app is
  staged without its icon, with a note.

**CLI and tooling**

- A new `ty init` app has an icon on every target and in the browser tab, and
  the browser no longer logs a `/favicon.ico` 404. A native build notes a
  missing or unusable icon instead of leaving it out silently.
- A rebuilt `ty` of the same version no longer reuses the old one's routes.
- `ty migrate check` reports a source file that is not UTF-8 text and checks
  the rest of the project instead of stopping.
- A parameter annotation such as `@CacheKey` in JavaScript or TypeScript no
  longer crashes `ty build`, `serve` or `bundle` when its parameter list opens
  before the class, as in a class passed as an argument; the build reports
  the annotation instead. A parameter annotation in a one-line class now
  works like one in a multi-line class.

### Security

- API docs no longer reveal guarded routes: `/api.json` and `/api-docs/`
  list only `@Public` routes and their models (see Breaking).
- `ty handler invoke --allow-env` refuses Tachyon's own variables
  (`TAC_*`, `TACHYON_*`, `YON_*` and the proxy variables) with `TY2006`,
  as `server.environment` already does; 26.36.01 accepted them.
- Every native host sends a Content-Security-Policy whose `connect-src`
  allows only the bundle and `tac.outgoing.hosts`; before, Apple and Android
  hosts sent none and Windows and Linux allowed any `https:` host.
- Windows release builds turn WebView2 DevTools off, so F12 and Inspect no
  longer open a console in the app; debug builds keep them.
- Native pages post forms only to their own bundle (`form-action 'self'`).
  Android also refuses main-frame loads outside the bundle, and sends
  companion signals, application state and title-bar insets only to bundle
  pages.
- macOS, iOS and Android pass companion signals to the page as checked
  JSON instead of splicing raw text into a script. Bundle assets carry
  `X-Content-Type-Options: nosniff`, and the Apple file-picker route
  answers only the `bundle` host.
- Loopback servers answer only localhost names, IP addresses, the
  `tac.origin` host and `server.hosts`; other `Host` values get `403`
  (`TY1366`), which blocks DNS rebinding.
- `ty start` enforces `outgoing.hosts` at the operating-system level;
  `ty serve` and `ty preview` warn that they do not (ADR 0113). Gateway
  commands go through the outgoing guard too.
- The guard's address filter refuses NAT64, 6to4, Teredo,
  IPv4-compatible and IPv4-translated IPv6 ranges, and the discard,
  documentation, benchmarking, reserved and multicast ranges. The guard
  serves at most 256 connections (`503` beyond), closes tunnels idle for 5
  minutes, and keeps running after an accept error. PHP `file_get_contents`
  and `fopen` HTTP streams go through it.
- The outgoing guard refuses loopback and private addresses behind an
  allowed name, and handlers no longer get `NO_PROXY=127.0.0.1`.
- Browser-sent `Forwarded`, `X-Forwarded-*` and `X-Real-IP` no longer reach
  handlers; only proxies in `server.proxies` may set them (ADR 0096).
- Topic streams (`/.tachyon/topics/<name>`) follow the access rule that the
  server's `@Topic` model declares, with the same guard as routes (`401`
  without an identity, `403` for the wrong role). A topic that no server
  model declares answers `404`, and each client address may hold at most 16
  open topic streams.
- `OPTIONS` on a route path answers after the access check, and lists only
  the verbs and schema of the routes the caller may call.
- A FYLO SQL value can no longer change its query: `$n` counts only outside
  quotes, and each value is written as one literal. A quoted placeholder
  (`'$1'`) and a negative or exponent number are refused with `TY2160`;
  write `WHERE name = $1`.
- A `400` refusal never repeats request values, cookies or authorization
  headers, and values nested over 32 levels are refused.
- Services cannot read the `tac` build settings, such as the outgoing host
  list, through `Settings`.
- `tokio` 1.53.3 (RUSTSEC-2026-0386), `ctutils` 0.4.6 (RUSTSEC-2026-0385),
  and `webpki-roots` 1.0.11 with `rustls-webpki` 0.103.16, which drop the
  distrusted emSign roots (RUSTSEC-2026-0381, RUSTSEC-2026-0382).

### Known limits

This release is a release candidate. `docs/THREAT_MODEL.md`,
`docs/SUPPORT_TIERS.md` and `docs/CUTOVER.md` hold the details.

- The outgoing guard allows any port on a declared host, and takes no
  per-run credential.
- `ty test` units run without egress confinement.
- Topic streams are capped at 128 per server, 32 per topic and 16 per
  client address, so two addresses can fill a public topic and eight the
  whole server.
- The publish workflow's Windows ARM64 job does not run the Sigstore
  client, which has no Windows ARM64 build; the other five jobs verify the
  signature of every asset, the Windows ARM64 executable included.
- `ty` bundles SQLite 3.51.3, which has advisory RUSTSEC-2026-0459
  (CVE-2026-119375). Only an app whose configured SQLite database file can
  be written by a less trusted party is exposed. A later release will update
  SQLite.

### Migration notes

Upgrade a 26.36.01 project in this order. Do it on a branch, so you can
compare and roll back.

1. Commit the project, and copy `.tachyon/topics` somewhere safe if the
   project publishes topic events. The new server moves those logs to
   `data/topics/` on its first start.
2. Install the prerequisites. A project with PHP code needs `php-cgi`
   (Homebrew `php`, the Debian or Ubuntu `php-cgi` package, or
   `php-cgi.exe` from the Windows PHP zip).
3. Install the new `ty` with `install.sh` or `install.ps1` from the latest
   release, then run `ty doctor`.
4. Run `ty migrate check`. Keep the report: it lists every place that the
   Breaking section covers, with the rule name and the action.
5. Run `ty migrate fix` and read the plan. Then run
   `ty migrate fix --write`. It never deletes or overwrites a file. Run it
   again until it reports no more edits, because some conversions (such as
   `@Stream` to `@StreamRoute`) apply to the controllers that the first run
   made.
6. Do the manual steps the fix listed, in this order:
   - configuration values it could not move out of `tac.config.*`, then
     delete the old files;
   - route files it could not convert, and each `OPTIONS.schema.json`
     (write the rules on the model types, then delete the file);
   - repository and client code: move it into services, so that `@Store`
     and `@HTTPClient` stay contracts;
   - `@Relay` methods: call `relay(command, request)` instead;
   - other uses of `YonRequest` and `YonResponse`, and middleware that is
     not yet a `Middleware` class with `before` and `after`;
   - page markup that the fix did not move into a component, and
     companions into `client/services/<Name>Service.<ext>`, replacing
     `this.tac` and the `@onMount`, `@publish` and `@subscribe`
     decorators.
7. Give every controller an access rule (`@Public`, `@SignedIn` or
   `@Allow(...)`). The fix writes `@Public` only on the controllers it made
   from route files. Add one `@IdentityProvider` if any route is guarded.
   Give every server `@Topic` model the same kind of rule; `@Public` keeps
   the 26.36.01 behaviour.
8. Move secrets from source and the manifest into `.env` under `YON_`
   names. If you copied `.env.example` to `.env`, rename the names that
   `ty migrate check` lists for `.env` yourself, using the replacement each
   finding names, and keep the values; the fix never edits `.env`. List
   the outgoing hosts in `tac.outgoing.hosts` and `outgoing.hosts`:
   handlers can now reach only those hosts. Replace direct database
   drivers with managed storage. On Windows, or Linux older than 6.7,
   `ty start` needs `YON_EGRESS_ENFORCEMENT=proxy`.
9. Run `ty serve`. Fix each build error, or, while you migrate, turn one
   rule off for one realm with its `check: false` switch. `TY1322` (route
   access) has no switch. Turn the switches back on before you release.
10. Check handlers that keep module-level state. Handlers now stay warm
    between requests; reset that state, or set `YON_ISOLATION=process`.
11. Run a release build with `ty bundle`, which also checks the per-route
    script and WebAssembly budgets (`TY1337`).
12. Run `ty migrate check` again. Continue until it reports no `changed`
    findings.

Rollback: to go back to 26.36.01 after `ty migrate fix --write`, revert the
source changes too (for example, `git checkout` the commit from step 1).
26.36.01 does not read the new layout, markers or `client/manifest.json`,
and it refuses access markers on a `@Topic` model; if you keep any source
edits, remove those markers before you build with 26.36.01 (the streams
are then open to anyone again).
Copy the saved logs back to `.tachyon/topics` if the new server moved them.
Then install 26.36.01:

```sh
curl -fsSL https://github.com/d31ma/TACHYON/releases/download/v26.36.01/install.sh \
  | TAC_BASE_URL=https://github.com/d31ma/TACHYON/releases/download/v26.36.01 sh
```

```powershell
$env:TAC_BASE_URL = "https://github.com/d31ma/TACHYON/releases/download/v26.36.01"
irm https://github.com/d31ma/TACHYON/releases/download/v26.36.01/install.ps1 | iex
```

PHP projects can keep `php-cgi` installed; 26.36.01 starts `php`.

## [26.45.07] - 2026-11-08 (unpublished)

This release candidate was never made public. Its pull request's CI failed
in the macOS gates job, which ran the runner image's Node 22 instead of a
pinned Node. 26.46.01 replaces it with the same changes and pins Node
24.22.2 in that job.

## [26.45.01] - 2026-11-02 (unpublished)

This release candidate was never made public. Its publish run failed in the
native artifact jobs, whose install, upgrade and rollback drill ran without
Bun. 26.45.07 replaces it with the same changes.

## [26.36.01] - 2026-08-31

### Added

- Reconcile the newer local feature list with the released Rust engine. Kept
  features: publish/subscribe, persistence decorators, bounded browser helpers,
  counted loops, manifest and configuration capture, and target-native
  companions.
- Validate captured request body, header and parameter contracts with a
  bounded external schema validator. Publish deterministic API schemas and an
  interactive reference.
- Restore development `preview`, production `start` with an immutable bundle,
  and single-target `bundle --native`. Existing command aliases still work.
- Before publication, run behavioral schema, companion ABI (application binary
  interface), compiler, browser and website gates against the actual
  downloaded release executables.

### Changed

- Native targets render the bundled Tac application through WKWebView,
  WebView2, WebKitGTK or Android WebView. Native Host contract v3 replaces the
  old widget-tree metadata. Page companions compile for their native OS.
  Browser WebAssembly companions are replaced (ADRs 0018 and 0019).
- Fetch persistence is explicit, has no credentials and is bounded.
  Service-worker caching skips authenticated or private responses and verifies
  packaged static bytes. Native hosts remove old service workers.
- Android pins AndroidX WebKit 1.14.0 for a frame-aware asynchronous bridge
  with no legacy fallback. Pending calls and worker admission are bounded.
- Keep normal JavaScript module helpers. Keep all released guarantees for
  process supervision, source snapshots, routing, environment namespaces and
  atomic builds.

### Fixed

- Keep input focus, caret, selection and the connected editing node during
  reactive updates. This includes inputs without IDs and custom-element slots.
  Website search no longer moves the caret to the end after results update.
- Keep later native field edits when earlier asynchronous replies arrive.
  `Range`/`If-Range` requests skip persistent response caching. Partial
  `206`/`Content-Range` responses cannot become cache hits for complete
  responses.
- Share bounded Foundation JSON parsing and canonical request serialization
  across Apple hosts and Swift companions. This replaces the handwritten value
  scanner and rejects duplicate root keys before dispatch.
- Decode HTML entities once, remove a leading UTF-8 BOM (byte order mark) and
  add mobile viewport metadata. Bound nested empty loops and cancel old
  component mounts.
- Keep native companion state per route, reject bridge requests with a
  mismatched route or origin, and package Android `_id` dynamic route
  directories.
- Serve Linux native stylesheets with their HTTP content type. Route Windows
  WebView2 resources, including dynamic pages and their native companions,
  through the canonical local responder.
- Limit compiled-controller endpoint and stream discovery to the controller,
  so colocated helper types are excluded. Run the published layered examples
  against all eight server runtimes. Report snippets that did not run
  separately.

### Migration notes

- Native companions are trusted application code with OS privileges. They are
  not sandboxed WebAssembly plugins. Automatic native member discovery supports
  fields and methods with zero arguments. Methods with parameters need explicit
  member tables.
- Native-call timeouts stop the wait. They do not stop native execution, and
  they do not undo completed mutations. Keep companions responsive. A hung
  companion may need a relaunch. Android runtimes without the required WebView
  messaging feature report the bridge as unavailable. They do not fall back to
  a legacy interface.
- Existing version tags and assets do not change. See
  [the reconciliation inventory](docs/RECONCILIATION.md) and the
  [client migration guide](docs/CLIENT_RUNTIME_MIGRATION.md) for acceptance
  evidence and compatibility decisions. Before publication, the release
  workflow verifies the restored features in downloaded executables.
## [26.35.07] - 2026-08-30

### Added

- Added required layer contracts: `@Controller`, `@Service`, `@Repository`,
  `@Client`, and `@Component`. Also added supervised `@Relay` migration for
  non-Yon programs, and direct multi-frame `@Stream` responses for JavaScript,
  TypeScript, Python, PHP, Kotlin, and C#.
- Added a structured signal lifecycle for `serve`, `preview`, and
  `bundle --watch`:
  - Signal handlers are registered before the process reports that it is
    ready.
  - Proof of registration is written to stderr as JSON lines that contain no
    private data. It is separate from the application's readiness message on
    stdout.
  - The first signal requests a graceful shutdown. The development server puts
    a time limit on the internal tasks it owns. Preview may wait for open
    connections to close on their own. Bundle watch may finish its synchronous
    fingerprint pass or its current bounded build.
  - Supervisors apply their own grace period.
  - A second signal forces the process to exit. The exit code is `130` for
    Unix `SIGINT` or Windows CTRL-C, `143` for Unix `SIGTERM`, and `131` for
    Windows CTRL-BREAK.

### Changed

- Development topic subscriptions now share one bounded incremental reader
  (tailer) per topic. It has admission limits, replay cursors, removal of slow
  consumers, and clear ownership at shutdown. Compiler, native, WASM,
  post-bundle, and doctor tool processes use one supervisor for the whole
  process tree. It applies a deadline and an output limit.

- Yon handlers, middleware, and workers now use the runtime that the framework
  provides for JavaScript, TypeScript, Python, Java, C#, Kotlin, PHP, or Rust.
  The old `Handler` fallback, `.tachyonrc.interpreters`, shebang handlers, and
  executable handler discovery are removed. `.tachyonrc.workers` is still
  supported.
- The JavaScript and Python runtime overrides are now
  `YON_JAVASCRIPT_RUNTIME` and `YON_PYTHON_RUNTIME`.

### Fixed

- Keep the lexical loop scope through nested iterables, text, dynamic
  attributes, conditions, component props, and event arguments. When a loop
  name repeats, the nearest loop hides the parent name only inside that loop.
  Siblings get the outer binding back. A component owns the evaluated props it
  receives.
- Check every Yon runtime that the project needs before the development server
  prepares output or binds a socket. A missing executable now reports its own
  `TY2112` diagnostic. This happens at startup and also when the executable
  disappears between the check and the invocation. `ty doctor` uses the same
  time-limited checks. It counts the JavaScript / TypeScript and Java / Kotlin
  runtime needs only once, and it never prints configured executable paths.
  The C# readiness check now builds a small project that the framework
  provides against the installed SDK. A runtime-only `dotnet` no longer passes.
- Limited the output of relays for compiled languages, and stopped delegate
  stderr from appearing in responses. Relays, streaming timeouts, and
  subscriber disconnects now apply deadlines and clean up child processes.
  Windows PHP relays do not use non-blocking `proc_open` pipe multiplexing,
  because the platform does not support it.
- End a failed stream with one sanitized SSE error event that includes the
  request ID. This covers failures before the first value and after partial
  delivery.
- Keep every admitted event on finite streams that succeed. Apply JavaScript
  stdout backpressure before asking the generator for the next value.
- Reject symlinked or unreadable non-route layer sources in a predictable way.
  Ignore stereotype names that only appear inside comments and
  language-specific strings.
- Tie one discovery pass to a retained project capability that does not follow
  symlinks. Route graphs, layer checks, Tac pages, components, and shared
  assets, build and native configuration, and Yon execution now read an owned,
  bounded snapshot. They no longer reopen authored paths from the environment.
  Standalone handlers take a snapshot of the complete permitted `server/**`
  dependency boundary before invocation.
- Bind the development server from that same discovery object. Its initial web
  build, route dispatch, selected root middleware, `.tachyonrc` schedules, and
  scheduled-worker sources can no longer mix files from before and after the
  project root is replaced.
- The development server now owns its scheduled-worker and source-watcher
  tasks. Graceful shutdown cancels them and waits a limited time for them to
  finish. Dropping a server that is only bound starts no background work and
  cannot leave a task running on its own.
- Every streaming-handler bridge, hot-update stream, and topic stream is now
  part of the same owned lifetime. Shutdown cancels response producers before
  Axum's graceful wait. It keeps the last part of one bounded deadline to abort
  and join the remaining tasks. So an SSE client that never disconnects cannot
  keep the server or a handler child alive. Finished producer tasks are
  cleaned up continuously instead of piling up until shutdown.
- Reject Firecracker execution of TypeScript and prepared direct-language
  artifacts before the driver starts. This stays in place until a bounded
  artifact-transfer contract exists. That transport still supports JavaScript
  and Python.
- Restricted the project-local `.tachyon/handlers` cache:
  - It uses directory capabilities that do not follow symlinks.
  - Cleanup checks file identity first.
  - Publication is atomic, and runtime copies are private.

  The cache now recovers abandoned locks. It enforces a limit of 256 entries /
  512 MiB. This limit is separate from `TAC_CACHE_DIR`.

## [26.33.02] - 2026-08-11

### Added

- Added a Yon isolation boundary that an environment setting selects.
  Operators can keep the supervised process backend. Or they can send handler
  entry points to a Firecracker control program through framed Handler
  Protocol v1, with limits on CPU, memory, and pool size, and deny-only egress
  settings. A partial or conflicting Firecracker configuration fails closed.
- Added semantic development updates driven by events, through Hot Update
  Protocol v1. They include:
  - bounded merging of filesystem events;
  - the last good diagnostics;
  - CSS and component updates that keep state;
  - a safe full reload when the structure changes.
- Added a weekly fuzz campaign with a persistent corpus, and an acceptance gate
  for semantic hot updates in a real browser.

### Changed

- Made Tac render only on the client. Each route's bootstrap document now
  includes a bounded render plan and `/.tachyon/tac-client.js`. It contains no
  authored subtree rendered on the server. Component companions mount on
  explicit schedules. The old `hydrate` name remains only as a schedule alias.
- Made Yon REST-only. Builds never run Yon, the compiler rejects `yon.html`, and
  Route Manifest v1 no longer lists a build-time view context.
- Put all environment variables that the project owns in one of two
  namespaces. `TAC_` is for client, compiler, packaging, and tooling variables.
  `YON_` is for server and handler variables. The removed `TACHYON_` names do
  not work as aliases.
- Removed old JavaScript implementation paths from the repository. The
  unchanging v26.30.04 behavioral oracle and a full-stack migration fixture
  that cannot run are kept.

### Fixed

- Kept process-group ownership, termination of child processes, pipe
  settlement within the deadline, and bounded reaping. This applies to both
  process and Firecracker-driver handler invocations.
- Kept these guarantees: stable-release recovery, rollback verification after
  a corrupt checksum, exact toolchain setup, website acceptance, native
  execution hardening, and the standalone installer's atomic handling of a
  failed upgrade.
- Generated static assets now win over dynamic route patterns. Mounted website
  components now request rerenders through the current `tachyonComponent`
  boundary.

## [26.33.01] - 2026-08-10

### Fixed

- The Windows installer now decodes byte-array checksum responses as UTF-8. So
  `SHA256SUMS` responses without a file extension still fail closed and can be
  used.
- Atomic Windows upgrades now use a real temporary backup path. So `.NET`
  `File.Replace` keeps the installed executable and does not reject a null
  backup path.
- Made staged-release recovery stronger. It now verifies a deliberately
  corrupted copy of the Windows executable and keeps the canonical checksum
  manifest unchanged.

## [26.32.07] - 2026-08-09 (unpublished)

This release candidate cannot be changed, and it was never made public. Staged
verification of the Windows installer found a defect in atomic replacement.

## [26.31.06] - 2026-08-01

### Removed

- **FYLO is no longer part of Tachyon.** The binary, its vendored shim, and
  every integration point are removed:
  - the browser client that the compiler injected;
  - the `fylo` facade in the polyglot companion preludes and template
    capabilities;
  - the `/_fylo` browser routes;
  - the `db/` scaffold;
  - the installer step that downloaded it.

  The installer now installs only the standalone `ty` binary. Request
  identifiers are generated natively. Legacy `OPTIONS.schema.json`
  request/response validation is not implemented. `ty migrate check` reports
  it with an explicit migration action.
  Consequences a project should plan for:
  - The Rust implementation has no built-in OpenTelemetry. Projects that need
    emitted spans must add instrumentation outside Tachyon or stay on the
    archived release.
  - Yon realtime keeps its append-only topic logs and its SSE cursor based on
    an integer position. It no longer registers clients into a collection,
    because nothing read that collection. Its default root is `.yon-realtime`
    instead of `.fylo-data`.
  - Companion scripts that use a bare `fylo` no longer get an injected
    import. `ty migrate check` no longer reports FYLO constructs.
- Removed from the repository: the JavaScript compiler, server, browser
  runtime, native generators, vendored shims, tests tied to that
  implementation, benchmarks, and old JavaScript TypeScript configurations. The
  ambient type contract for applications now lives under `api/types/`. The
  full-stack migration input that is kept is now separate, non-executable Rust
  test data.

### Added

- Finished the new Rust rewrite (written from scratch) through Phase 7. The
  Rust implementation is now the only stable release candidate. The unchanging
  compatibility oracle stays outside the shipped runtime.
- Added native host generators for iOS, Linux, Windows, and Android, next to
  macOS. They share one staging path that does not depend on the platform and
  one planner. Each target has its own output, and each platform records its
  toolchain.
- Added reproducible gates for each platform:
  - an iOS simulator run;
  - a pinned Linux container that reads accessible names and drives the UI over
    AT-SPI (the Linux accessibility service);
  - an Android emulator run that checks native widget classes through
    `uiautomator`;
  - a Windows UI Automation gate for CI.
- Checked the cutover evidence again by running it on 2026-07-31. This covered
  the compatibility differential (3/3 corpus projects), the release lifecycle
  drill, and the iOS, Android, and Linux native gates. `docs/CUTOVER.md` now
  records the basis for each condition. The basis is either an execution
  result or a named maintainer decision.
- Added build-provenance attestation to the release lifecycle job. Anyone can
  trace a published artifact to the workflow and commit that built it, with
  `gh attestation verify` and without a maintainer key.
- Added ARIA-role adapter mapping for native targets. When the author of an
  unknown tag declares a role, the compiler still knows what the tag is. So a
  design system gets native widgets by naming the roles it must give its users
  anyway. The compiler does not need to know the design system.
- Added browser companions in Dart, Kotlin, Swift, and C#, next to the existing
  Rust one:
  - Each language's own compiler builds its companion.
  - `scripts/wasm/companion-browser-test.mjs` proves all five in a real
    browser, on one page.
  - An authoring prelude includes the ABI. So a companion is plain code in its
    language, not a subset that a transpiler understands.
  - Dart, Kotlin, and C# cannot produce a module that instantiates on its own.
    So ADR 0011 adds a second shape for them.
  - Swift meets the original shape, when it gets an empty WASI (WebAssembly
    System Interface) environment. The island runtime now supplies it.
- Added capability probes to `ty doctor`. Each wasm toolchain is asked to build
  for its wasm target, instead of reporting its version. All of these languages
  ship wasm support separately from the compiler. Before this change, four
  toolchains reported ready on a machine where none of them could build.
- Added `ty migrate check`. It classifies a legacy project against the Rust
  features without running it. It attaches a required action to every finding
  that is unsupported or changed.
- Added a shared compatibility corpus and a browser-based differential. It
  compares route graphs, semantic DOM, and HTTP status across both
  implementations. Declared differences stay visible.
- Added `docs/PARITY_LEDGER.md`. It records every feature as identical,
  equivalent, changed, unsupported, or Rust-only.
- Added four fuzz targets that cover every trust boundary and run under
  `AddressSanitizer`. Also added sanitizer, soak, recovery-drill,
  performance-budget, SBOM (software bill of materials), and auditable-build
  gates.
- Added a weekly long fuzz campaign. It restores the previous corpus and runs
  all four boundaries for two hours in total by default. It publishes the
  changed inputs for audit and uses them in the next campaign.
- Added reproducible release artifacts with checksums for each file. Also added
  a lifecycle drill that covers verification, tamper detection, install,
  upgrade, rollback, and uninstall.

### Changed

- Made automated security qualification of the exact head commit the stable
  release gate. Independent human security review is still welcome, but it no
  longer blocks merge, tagging, or publication.

### Fixed

- Kept the website's mobile navigation, brand link, language controls, and
  footer links at the 44-pixel minimum touch-target size. Removed a nested
  interactive chip from the home link, and respected the bottom safe area.
- Made release archives deterministic under both GNU tar and BSD tar. The
  staged file timestamps are now normalized before archiving.
- Windows upgrades now download and verify into a temporary file in the same
  directory before they replace `ty.exe`. So a network or checksum failure
  keeps the installed binary.
- Unix upgrades now put the verified bytes next to the installed binary before
  the atomic replacement. Release CI tests on Unix and Windows that a failed
  upgrade keeps the installed binary.
- The handler supervisor now owns a POSIX process group or a Windows Job Object
  for each invocation. It ends child processes after every outcome, and it
  limits pipe settlement by the request deadline. It proves that no child PID
  remains after success, timeout, and cancellation.
- Replaced Linux native `file://` WebSurfaces with a private resource scheme for
  each view. The scheme percent-decodes and canonicalizes paths, and it rejects
  symlinks. It limits navigation and loads to the generated surface and
  WebBundle roots. Remote WebSurfaces now enforce the declared effective port
  on every platform.
- Handler and middleware process diagnostics no longer cross the HTTP boundary.
  Public failures include only a request reference.
- Removed the installer's execution of the schema validator and TTID. These were transitive
  dependencies that could change and were not verified. The Rust runtime uses
  neither external binary.
- Removed the last FYLO environment keys, ambient APIs, generated README text,
  and `db/` files from `ty init`.
- Added the migrated website acceptance suite to required CI. Removed its old
  Amplify recipe that built from source. Deployment uses a qualified prebuilt
  `dist/web` archive, because the showcase uses five language toolchains.
- Made the enterprise qualification matrix work on all hosted runners:
  - restored the four-target `cargo-fuzz` package with reviewed seed inputs;
  - normalized extended Windows paths before calling MinGW;
  - compared macOS and web screenshots at the host's real usable viewport;
  - handled Android SDK license-pipe termination explicitly;
  - rebuilt the standard library for Linux sanitizer runs;
  - kept a release artifact for provenance attestation.

- A generated Windows application published its side-by-side manifest to the
  bundle root as `application.manifest`. Windows never reads that file. It
  reads only `<executable>.manifest` next to the executable. So the process ran
  without the Common-Controls v6 activation context that the manifest declares.
  The manifest is now published as `bin/<executable>.exe.manifest`. This alone
  does not fix the open Windows accessibility finding in
  `docs/PHASE_7_EVIDENCE.md`. Every control still appears to UI Automation as a
  generic pane, not as a button, output, or heading. (Superseded: the
  reconciliation of 2026-08-30 put the manifest back at the bundle root as
  `application.manifest`, and the `.exe` embeds none, so Common Controls 6
  and per-monitor DPI awareness again do not apply. SW4 recorded this on
  2026-10-27, with its second effect: with no `requestedExecutionLevel`, an
  app named with `Setup`, `Install`, `Update` or `Patch` asks for
  administrator rights. See `docs/SUPPORT_TIERS.md`, "WebAssembly service
  features in the Windows app". Fixed again by WM1 on 2026-10-27: the `.exe`
  now embeds the manifest as its `RT_MANIFEST` resource, in the
  [Unreleased] entry "A Windows app now embeds its application manifest".)
- The development and preview servers could exit as soon as they reported
  that they were ready. If registering the Ctrl-C handler failed, the shutdown
  future finished at once. So the server completed its graceful shutdown
  before it served anything. It then refused every later connection with no
  diagnostic. The `ty build --watch` loop ended the same way. Now a process
  that cannot detect an interrupt keeps serving, and its supervisor stops it.
- A project whose `client` or `server` path was a regular file reported the
  wrong diagnostic on Windows. When a parent path component is a file, Unix
  reports `NotADirectory`, but Windows reports `NotFound`. `NotFound` is the
  same code that means the source root does not exist. So a broken project
  layout was reported as having no views (`TY1002`), not as an unusable source
  root (`TY1001`). The compiler now finds the blocking parent by checking the
  shape of the path. It no longer trusts a platform-specific error kind.
- The template parser panicked on malformed markup. `<slot>` has no children.
  But recovery from a token error could leave one open on the parser stack.
  Adding the next child then hit an `unreachable!()`. So a `tac.html` that was
  edited by hand or generated could stop the compiler instead of reporting a
  diagnostic. A leaf now rejects the child with `TY1301`. The impossible state
  no longer exists at all, instead of being only unreachable. The
  `template_frontend` fuzz target found this in CI. The crashing input is now a
  regression test and a permanent corpus seed.
- A native build rejected the compiler's own output. Only scripts under
  `/.tachyon/` were removed before native planning. So a route's own
  `/client.js` stayed, and the view contract rejected it with `TY1306`. This
  meant that no page with a companion or an event binding could be built for
  any native target. Every script in generated HTML is generated, because the
  compiler rejects an authored one before rendering.
- Two routes wrote one `WebSurface` over another. Node ids restart at
  `n_000001` for each route, and the payload directory was named after the id.
  So the second route that was built replaced the first route's page. The
  website's home content showed another route's empty surface. The payload
  path now includes the route. The node id stays as the Native UI contract
  specifies.
- A `WebSurface` had a fixed height on every platform: 180 points on iOS,
  macOS, and GTK, and 480 pixels on Android. So a taller subtree was cut off,
  with no scroll bar of its own to show the rest. Each host now uses the
  document's own height.
- A fallback subtree rendered without styles, because the route's stylesheet
  link is removed as a generated asset. The stylesheets that a route links are
  now inlined into the fallback document instead. This also restores the media
  queries that decide which variant of a responsive page is visible.
- The compiler could not plan an island whose content is entirely deferred. An
  `aria-label` written on a component becomes a property of its companion, not
  an attribute of the island. So nothing could name the island, and every
  native build failed with `TY1603`. The name is now read from the props.

- Every gate script in `scripts/` was committed without its executable bit. So
  the release lifecycle drill failed at `build-artifact.sh: Permission denied`
  from a clean checkout. CI's `./scripts/linux/native-test.sh` would have
  failed the same way. This was found by running the drill, not by reading it.

- Accessible names set on generic GTK4 containers never reached AT-SPI. A GTK4
  button with its own label ignored its declared accessible name. Both problems
  were found by running the real accessibility bus, not by inspection.

### Changed

- Switched the repository entry points and CI to the Rust rewrite. Root package
  commands now run the compiled `ty` CLI. A dedicated CI policy job rejects any
  attempt to add back the removed JavaScript runtime or test entry points.
- The compatibility differential now downloads the unchanging v26.30.04
  release executable. It verifies the executable's pinned SHA-256 digest and
  exposes it as `RELEASED_TY_BIN`.
- `NativeBuildOptions` now includes a `NativeTarget`. Native artifacts are
  published under a separate directory for each target.

### Added

- Finished the Rust Phase 4 macOS native vertical slice:
  - Native UI v1;
  - semantic SwiftUI adapters;
  - deterministic accessibility identities;
  - bounded declarative state and actions;
  - route and controller lifecycle logging;
  - atomic `.app` packaging.
- Added a local WebSurface fallback for the smallest subtree, isolated HTTPS
  WebSurfaces with no bridge, Capability Manifest v1 that denies by default,
  and Artifact Manifest v1 with complete digests.
- Added a real macOS application gate. It drives AppKit controls through
  Accessibility and keyboard events. It compares the required semantics with a
  mobile Chromium reference, and it enforces a rough screenshot-layout budget.
- Finished the Rust Phase 3 view pipeline:
  - bounded expressions and escaped bindings;
  - both Tac control syntaxes;
  - recursive components and slots;
  - composed JavaScript/Python Yon context;
  - server-rendered islands;
  - View IR (intermediate representation) and source maps;
  - recovery that reports several diagnostics;
  - verified incremental reuse of routes.
- Added all five island activation policies with deterministic wrappers,
  same-origin generated modules, and failure behavior that keeps the SSR
  (server-side rendering) output. Also added interaction replay and a real
  Chromium acceptance gate.
- Added View Source Map v1, the safe-view/island ADR, the Phase 3 normative
  specification and evidence, and updates to the threat model and support
  documents.
- Finished the Rust Phase 2 Yon handler boundary:
  - typed Handler Protocol v1 request, response, error, and cancellation
    envelopes;
  - deterministic JavaScript and Python handler discovery;
  - a compiled `ty handler invoke` workflow.
- Added supervision that starts one process directly for each request. It has
  strict framed stdout, bounded stderr, deadlines that include queue time,
  cooperative cancellation, forced termination and reaping, concurrency limits,
  crash recovery, and a child environment that denies by default.
- Added shared contract suites that use real runtimes and real binaries. They
  cover:
  - sync and async handlers;
  - Unicode paths and data;
  - all protocol HTTP methods;
  - environment allowlisting;
  - adapter authoring failures;
  - process crashes;
  - malformed and oversized output;
  - stdout smuggling and stderr flooding;
  - concurrency admission.
- Finished the Rust Phase 1 vertical slice:
  - deterministic Tac/Yon static route discovery;
  - bounded HTML validation;
  - Route Manifest v1 output;
  - stable diagnostics;
  - atomic publication;
  - project scaffolding;
  - a development server that is safe on loopback.
- Added acceptance tests on the real binary for initialization, repeatable
  builds, failure recovery, HTTP behavior, security headers, and execution on
  each platform. Also added a Rust coverage ratchet: 80% for the whole
  workspace.
- Set up the base of the new Rust rewrite, written from scratch. It has an
  exact toolchain, safety and lint policies for the whole workspace, and
  executable schema contracts.
- Added product context, six accepted architecture decisions, a first threat
  model, support vocabulary, governance, and release engineering.
- Added separate Rust CI for Linux, macOS, and Windows, with gates for
  dependencies, licenses, schemas, documentation, and immutable actions.

## [26.30.04] - 2026-07-23

### Added

- Generated Linux desktop hosts now provide global shortcuts, window
  click-through that can be turned off again, and declared window capture.
  Each feature is enabled only when the host detects support for it. Wayland
  uses the XDG GlobalShortcuts and Screenshot portals. X11 uses `XGrabKey` as
  a fallback for shortcuts. When the compositor does not support a feature,
  the feature stays blocked. Linux still does not advertise capture
  protection.

### Fixed

- Native controllers without a DOM now run companion `@onMount` handlers only
  after the host bridge is installed. So startup `host.on(...)` subscriptions
  and managed `contentSurface.open(...)` calls work without constructor
  delays.
- When the compiler lowers JavaScript and TypeScript native-provider code, it
  now leaves strings, template text, regular expressions, and comments
  unchanged, byte for byte.
- Controllers produced by compiled `ty` release binaries no longer reference an
  undeclared native-parser constant during render or dispatch. The bundler had
  renamed that constant.
- Linux portal calls now subscribe to the asynchronous response before they
  call the portal method. So an immediate response from the backend cannot be
  lost.

## [26.30.03] - 2026-07-22

### Changed

- On macOS, Windows, and Linux, managed remote content now appears inside the
  primary native window, in its declared 25/75 split. It no longer opens in a
  separate top-level window. So the primary `appWindow` controls apply to the
  trusted controls and the active remote surface together, as one
  presentation.
- Managed-content state and host manifests now report the composed
  presentation contract. Bundles without managed origins leave out the layout
  metadata that they do not support.

### Fixed

- When the active managed surface is closed or replaced, the host now removes
  it from the composed pane and shows another open surface. The native bridge
  stays hidden from remote content.
- The documentation now explains composed presentation compared with detached
  presentation, the platform limitations, and how to use a global shortcut to
  recover a click-through window.

## [26.30.02] - 2026-07-21

### Added

- Native-first macOS, Windows, and Linux hosts now implement:
  - desktop window controls;
  - isolated managed-content surfaces;
  - declared camera and microphone permissions;
  - accurate reporting of capability state.

  macOS and Windows also implement global shortcuts, and window listing and
  capture that require permission.
- Release CI now compiles generated native hosts on macOS, Linux, and Windows.
  This includes the platform-specific managed-content and screen-capture
  adapters.

### Fixed

- Compiled standalone Tachyon binaries now embed the native UI runtime that has
  no DOM. So a newly initialized project can bundle native targets without
  resolving runtime paths that exist only in the source tree.
- Native host manifests now advertise only generated operations. They keep the
  declared permissions and the exact-origin managed-content policies. Remote
  content stays isolated from the native bridge for the whole application.

## [26.29.03] - 2026-07-15

### Added

- Raw native filesystem capabilities now include the opt-in `fs.stat`,
  `fs.mkdir`, and recursive `fs.remove` operations on Android, iOS, macOS,
  Windows, and Linux. Matching companion APIs exist for Dart, Rust, Kotlin,
  Swift, C#, JavaScript, and TypeScript.
- Android native hosts now provide encrypted secret storage through the
  Android Keystore. They also provide local user verification through the
  framework `BiometricPrompt`. So `secrets.*` and `auth.verifyUser` now work
  on Android as well as on Apple hosts.
- `ty bundle --csp-check` (or `TAC_CSP_CHECK=1`) checks the staged JavaScript
  output for constructs that generate code at runtime. A strict CSP (Content
  Security Policy) blocks these constructs. If the check fails, the whole
  bundle fails before it replaces the previous bundle.

### Fixed

- Detection of companion decorators and FYLO auto-imports no longer treats an
  unrelated import, followed later by a `from` expression, as a real binding.
- Element IDs that the author sets no longer break Tac event dispatch or
  value-binding dispatch. Delegated handlers receive the bound element as
  `event.currentTarget`.
- A handler can clear state that its own conditional render pass uses later.
  This no longer leaves the DOM in a broken state before the clean render of
  the state alone can complete.
- An element with several `on:*` handlers now calls only the handler that
  matches the type of the dispatched event.

## [26.28.06] - 2026-07-11

### Added

- Tac components and pages can use portable Dart, Rust, Kotlin, Swift, and C#
  companions, as well as JavaScript and TypeScript. Every language lowers to
  the same reactive controller ABI and the same implicit browser/FYLO/device
  prelude.
- The native capabilities now include:
  - discovery of sandbox paths;
  - secure storage and local user verification on Apple hosts;
  - browser-standard file save;
  - geolocation, notifications, and media capture;
  - inspection of permission state;
  - the buffered `app.ready` host event.
- You can configure the native privacy declarations for camera, microphone,
  location, and notifications with `tachyon.devicePermissions` in
  `package.json`.
- You can enable raw native operations one at a time with
  `tachyon.nativeCapabilities`: `fs.readText`, `fs.writeText`, `fs.readDir`,
  and `shell.exec` (desktop only).
- The website now has a capability atlas split into sections. It has separate
  routes for overview, compose, react, connect, store, observe, and extend. It
  also has a live portable-bridge panel that covers the permission and
  capability model.

### Changed

- Tac companions are now UI controllers that do not depend on a target. They
  are no longer client worker endpoints. The client worker/WASM compiler and
  runtime are removed. Portable companions use normal bundled modules on web,
  desktop, iOS, and Android.
- Native filesystem and shell access is disabled by default. Tachyon validates
  package declarations strictly, filters them by target support, and embeds
  them into generated metadata. The JavaScript facade and the native handler
  each enforce them independently.
- Native external URL operations accept HTTP(S) only. Other rules:
  - Android delivers bridge messages only to the trusted appassets origin and
    the main frame.
  - Apple and Linux bridge scripts run only in the main frame.
  - Android media requests grant only the camera/audio resources that the host
    explicitly recognizes.
- Android native hosts disable application backup and cleartext network
  traffic by default.
- The website's `/atlas` route is now a redirect shell. Each Atlas section
  that search engines can index has its own canonical route and a responsive
  one-column layout on mobile.

### Fixed

- Dart toolchain setup and portable companion diagnostics now report
  unsupported source constructs with deterministic failures that say what to
  fix.
- Native configuration no longer silently drops misspelled permission names or
  capability names. Invalid declarations fail the build before it replaces an
  existing bundle.
- When a page asks for camera or microphone access, Android no longer grants
  unrelated WebView permission resources.

## [26.28.01] - 2026-07-06

### Fixed

- `tac.bundle` no longer clears the whole `dist/` directory. A run replaces
  only the `dist/<target>` directories that it built. It replaces each target
  atomically: stage, remove the old one, rename. Before this fix, building one
  target deleted the output of every other target. This deleted `dist/web`
  while a dev server was using it. It also emptied a generated macOS app that
  was running from `dist/macos`, so its page modules, which load on demand,
  silently returned 404.
- The SPA (single-page application) runtime now requests dynamic-route page
  modules by their `_slug` directory on disk (`pages/docs/_topic/tac.js`).
  Before, it used the `:slug` form from the route manifest. So dynamic routes
  now work on static hosts and native asset loaders, not only on the dev
  server.
- The service worker now registers with `type: "module"`. The built worker is
  an ES module, so before this fix it failed to parse on every production
  origin.
- The generated Android host overrides the default asset-ignore pattern of
  AAPT (Android Asset Packaging Tool). That pattern silently removed `_slug`
  page-module directories from the APK.
- The service worker cache version now includes a content hash of the client
  source tree, as well as the route-manifest hash. So any rebuild with changed
  inputs removes old caches. Before, redeploys with the same version (and
  reinstalls of the native app) served old JS/CSS from the Cache API until the
  user cleared the site data.

### Changed

- The generated macOS host installs a standard menu bar: an App menu with
  About/Hide/Quit, an Edit menu with clipboard actions, and a Window menu.
  Without a main menu, Cmd+C/V/Q do not work in a WKWebView app. App authors
  add custom menus in `buildMainMenu()` in the generated `TachyonApp.swift`.
- The generated macOS host serves the bundle through a `WKURLSchemeHandler` at
  `tachyon://localhost/`. It no longer uses `file://` with workarounds in the
  file-access preferences. The secure-context origin enables Web Workers (Tac
  Workers running wasm) and FYLO persistence backed by OPFS (Origin Private
  File System), as on the iOS and Android hosts. It handles directory indexes
  and deep links in the same way, and external links open in the default
  browser.
- The generated iOS project now really includes its app icon. The xcodegen
  spec now includes the generated `Assets.xcassets` and sets
  `ASSETCATALOG_COMPILER_APPICON_NAME`. So the home-screen icon shows the
  Tachyon mark instead of the blank placeholder.
- On both native hosts, external links open in the system default browser.
  They no longer move the WebView away from the app. The Android host sends an
  `ACTION_VIEW` intent for any origin other than its own. The iOS host's
  `WKNavigationDelegate` passes http/https/mailto URLs outside the
  `tachyon://` scheme to `UIApplication.open`.
- The generated iOS host now includes its web bundle as `WebBundle/` instead of
  `Resources/`. With a top-level folder named "Resources", codesign/installd
  treat the flat iOS bundle as an old-style shallow bundle. Installation then
  fails with a misleading "Missing bundle ID" error. The `.ipa` archive also
  now leaves out AppleDouble sidecar files (`ditto --norsrc`).
- The generated iOS host serves the bundle through a `WKURLSchemeHandler` at
  the `tachyon://localhost/` origin instead of `file://`. The secure-context
  origin enables Web Workers (Tac Workers running wasm) and OPFS-backed FYLO
  persistence inside WKWebView. It handles directory indexes and deep links
  without extensions in the same way as the Android host. Service workers are
  still not available in WKWebView, and the runtime works without them. The
  viewport of the prerendered shell now includes `viewport-fit=cover`, so
  `env(safe-area-inset-top)` works on iOS.
- `tac.bundle --target` accepts lists separated by spaces as well as lists
  separated by commas. Both `--target ios, android` and
  `--target ios android` work. Before, it silently ignored everything after
  the first argument.
- The generated Android host draws behind a transparent status bar:
  - Pages get the status-bar inset as the `--tac-safe-top` CSS variable,
    because WebView cannot read `env(safe-area-inset-top)`.
  - A new `ui.statusBarStyle` bridge capability lets pages switch the system
    icons between `dark-content` and `light-content` when their theme
    changes.
- The generated native icon set (`NativeIconAssets`) now draws the current
  Tachyon mark (slate tile, gradient chevrons) in SVG, Android vectors, and
  raster PNGs. So launcher icons match the PWA (progressive web app) icons.
- The Android host now includes a proper adaptive launcher icon
  (`mipmap-anydpi-v26/ic_launcher` with background, foreground, and
  Android 13+ monochrome layers) instead of a plain drawable. So the icon
  fills the launcher mask from edge to edge, like first-party apps.
- The generated Android host runs without the native action bar
  (`Theme.Material.Light.NoActionBar`) and disables WebView scrollbars. This
  makes it feel like a native app.

### Added

- Yon supports every language. There is no list of supported languages. The
  only limit is the toolchain on the developer's machine. How it works:
  - A `yon.<ext>` handler runs by its extension. The extension names the
    language. An `interpreters` map turns the extension into a run command
    (`.go` → `go run`, `.rb` → `ruby`, …). The map starts with common
    defaults. Each project can add to it or override it under
    `interpreters` in `.tachyonrc`.
  - No shebang and no `chmod` are needed. Write `server/routes/ping/yon.go`,
    declare its HTTP methods in an `OPTIONS.schema.json` next to it, and it
    runs as a process. The handler reads the JSON request on stdin and writes
    the response on stdout.
  - A prebuilt executable or a script with its own shebang also works.
  - The built-in languages only make handlers easier to write: you write
    `class Handler` and do not need the stdin/stdout code. A pluggable
    provider registry (`src/server/process/language-providers.js`) provides
    them. Apps add or override providers in `server/yon.providers.js`. For
    example, an app can point python at a virtualenv interpreter.
  - `HandlerAdapter.supportedLanguages` / `isSupportedLanguage` are removed.
    `HandlerAdapter.knownLanguages` lists the convenience adapters for tools.
    It does not limit which languages can run.

### Removed

- Yon no longer has a step that compiles routes to binaries for deployment.
  `yon.build` (the `yon.build` bin, `src/cli/build.js`) and the
  `native-binary` execution backend are removed. The developer deploys the
  `server/` source as it is, and every route runs as a process. Routes in
  compiled languages compile on the first request and cache the result. So
  production needs the same toolchain that the developer used, not pre-built
  binaries.
- Yon's own WASM execution backend is removed
  (`src/server/process/backends/wasm-compiled.js`), together with the backend
  registry, which is now empty. Every Yon route runs as a subprocess. There is
  no in-process WASM path. The Tac frontend worker compiler is separate and
  does not change.
- Tac workers now support only Rust, JavaScript, and TypeScript. The C#, C++,
  Swift, and Kotlin worker compilers/providers are removed. `yon.<ext>`
  backend handlers in those languages do not change, because they are part of
  Yon, not Tac.
- The generated Android host serves the bundle through `androidx.webkit`
  `WebViewAssetLoader` at the trusted `https://appassets.androidapp.com/`
  origin instead of `file://`. It uses an explicit domain, directory indexes,
  and service-worker interception. Web Workers (Tac Workers running wasm),
  OPFS-backed FYLO persistence, and service workers now all work inside the
  native app. The deprecated file-URL access grants are removed. The iOS host
  README describes the matching `WKURLSchemeHandler` upgrade that iOS still
  needs.
- Production bundles (or `tac.bundle --package`) export native artifacts that
  you can distribute, when the system toolchains exist. `dist/<target>/` then
  contains only the artifact:
  - `dist/android/<App>-<v>.apk` through Gradle. It falls back to signing
    with the debug keystore. Use `TAC_ANDROID_KEYSTORE*` for store signing.
  - `dist/ios/<App>-<v>.ipa` through xcodegen + xcodebuild. It is unsigned by
    default. Use `TAC_IOS_TEAM_ID` to sign it.
  - An ad-hoc-signed `dist/macos/<App>.app` through swiftc.
  - `dist/linux/<App>/` (executable + Resources) through CMake.
  - `dist/windows/<App>/` (`.exe` + Resources) through CMake/MSVC.

  When a toolchain is missing, the build logs a skip and keeps the host
  project that can still be built. `--skip-package` turns packaging off.
- Prerendered shells automatically link a web app manifest when
  `client/shared/assets/manifest.webmanifest` (or `manifest.json`) exists.
  They show its `theme_color` as a `<meta name="theme-color">` tag. With the
  built-in service worker, an app only needs a manifest next to the favicon
  to become an installable PWA.
- Desktop native Tac workers, starting with Rust on macOS, Linux, and Windows.
  `tac.bundle --target <desktop>` compiles `browser/workers/<route>/tac.rs`
  files to native executables. It puts them in the host's
  `Resources/workers/` directory. The generated desktop native hosts expose a
  `tachyon.worker` bridge capability. It starts the executable for each
  request, sends a JSON envelope on stdin, and returns the response envelope
  to the frontend.

- Yon backend routes for Rust/C++/C# now compile to WebAssembly when their
  handler fits the subset that Tachyon supports. Tachyon compiles them
  **in-house** (no rustc/clang/.NET) and runs them **in-process** through a
  pluggable execution backend. Handlers outside the subset automatically use
  the existing subprocess runner instead. Subset handlers can:
  - read request fields (`request.query/path/header("k")`);
  - return structured JSON object literals (`{ "k": value, ... }`).

  Tachyon resolves and builds both in-house, with no JSON library inside the
  wasm module. So wasm routes return validated object responses. The combined
  `/language` example route shows this. Its Rust (`yon.rs`) and C++
  (`yon.cpp`) handlers fit the subset and compile to WebAssembly in
  production.

### Changed

- Tac workers now use five language families, each with explicit targets:
  - JavaScript/TypeScript for web;
  - Swift for macOS/iOS;
  - Kotlin for Android;
  - C# for Windows;
  - Rust for every target.

  `client/workers` no longer accepts C, C++, Go, Python, and Zig.
- The website in the repository (`website/`) changed from a dense capability
  atlas into a minimal marketing landing page. It now has a dark hero section
  with an install command, a picture of the file-system scaffold, a feature
  grid, polyglot code examples in tabs, and a selected interactive demo. The
  demo uses the polyglot, desktop native, and canvas panels. The old atlas
  components are removed.

- Bundled npm dependency modules now live under `/shared/modules/` instead of
  `/modules/` (for example, `/shared/modules/dayjs.js`). This is a clean
  break: the server no longer serves the old `/modules/*` URLs.

### Added

- The app shell now includes an inline `<script type="importmap">`. It maps
  bare npm specifiers (for example, `dayjs`) to their bundled
  `/shared/modules/` URLs. You can add your own mappings in
  `browser/shared/importmap.json`, with `shared/importmap.json` as a fallback.
  Your entries override the generated entries.

### Changed

- Tac worker `OPTIONS.schema.json` now has the same shape as the Yon route
  schema, but it groups response schemas into three status buckets: `ok`
  (2xx), `clientError` (4xx), and `serverError` (5xx). For each HTTP method,
  the keys are `payload` (request) plus the response buckets that apply.
  2xx responses always use the `ok` schema. The worker schema no longer has
  the old `response`/`error` keys, numeric status-code keys, or the `native`
  capability declaration block. Capabilities are now allowed through the
  `TAC_NATIVE_CAPABILITIES` environment variable, a comma-separated
  allowlist. Tachyon reads it at compile time and builds it into the worker
  runtime. Raw OS capabilities (`fs.*`, `shell.*`, `process.*`) still need an
  extra entry in `TAC_DANGEROUS_CAPABILITIES`.
- Tac frontend messaging now uses one signals system. `@publish` and
  `@subscribe` replace the old `@emit`, `@provide`, and `@inject`
  decorators. Field signals keep their value, for context-style values.
  Method signals do not keep a value, for event-style messages.
- Tac method subscribers can now pass `{ onMount: true }`. The subscriber then
  runs once after mount and also reacts to later signal publications.
- `@subscribe` and `@publish` without arguments now use the name of the
  decorated field or method as the default signal name.
- Removed the `@render` decorator from the strict v2 Tac API. Companion state
  now renders again when a field is assigned. This includes `$` and `$$`
  persistent fields.
- Upgraded `@d31ma/fylo` to `^26.23.7`. The new version adds these options to
  `fyloOptions()`, set through environment variables:
  - query caching (`FYLO_CACHE_BACKEND`, `FYLO_CACHE_METHOD`,
    `FYLO_CACHE_TTL`);
  - WORM (write once, read many) mode (`FYLO_WORM=strict`);
  - an RLS (row-level security) switch (`FYLO_RLS`);
  - a local queue switch (`FYLO_QUEUE`);
  - sync-mode selection (`FYLO_SYNC_MODE`).

  The Fylo browser now lets you inspect and restore soft-deleted items
  through `/_fylo/api/deleted` (GET) and `/_fylo/api/restore` (POST).
- Upgraded the `@d31ma/fylo` dependency of the Tachyon website from
  `^26.19.7` to `^26.23.7`.
- Native bundle targets now put their webview host directly in
  `dist/<target>/`. They no longer use a separate `dist/<target>-native/`
  directory. Native targets no longer produce the standalone web bundle. Its
  assets are embedded under the host's `Resources/`. With
  `--skip-native-host`, the build still produces the plain web bundle at
  `dist/<target>/`.

### Removed

- Removed support for Swift (`yon.swift`) and Kotlin (`yon.kt`) backend route
  handlers. These are mobile-first languages. Yon backend routes now target
  server languages only. Removed the compiled-runner Swift/Kotlin adapters,
  the `/languages/swift` and `/languages/kotlin` example routes, and their
  services/repositories.
## [26.23.01] — 2026-06-01

### Added

- Yon realtime SSE mailboxes, turned on by `YON_REALTIME_ENABLED=true`:
  - `/_yon/realtime/clients` issues durable TTID client IDs.
  - `/_yon/realtime/stream` replays queued messages over Server-Sent Events.
  - `/_yon/realtime/messages` stores messages in FYLO's local queue. It also
    wakes any client stream that is connected at that moment.
- The example app now includes a Yon realtime messaging part. It has MVC
  (model-view-controller) routes under `/realtime`, FYLO and validation schemas for demo
  clients and messages, and a Tac companion script. The browser UI markup and
  styles are not done yet.
- The compiler injects a `fylo` browser client into Tac. This client now:
  - caches successful collection reads in IndexedDB;
  - supports explicit cache policies;
  - uses cached reads when the browser is offline;
  - invalidates collection cache entries after FYLO mutations.
- The example users panel now shows every FYLO browser cache policy. It also
  has a temporary create/delete flow that shows invalidation after a mutation.
- `bun serve` now treats an app with `browser/` + `db/` and
  `YON_DATA_BROWSER_ENABLED=true` as having a built-in backend. App authors can
  use the Tac FYLO browser wrapper without adding a `server/` folder.

### Changed

- Upgraded `@d31ma/fylo` to `26.22.7`. Tachyon's FYLO integrations now use the
  root-first constructor API.
- When `NODE_ENV=production`, Yon compiled route adapters cache the build
  artifacts for Java, C#, Dart, C++, Swift, Kotlin, and Rust. The cache key uses
  the route file, the service files in the same language, and the adapter
  cache version.
- Yon now supports C++ backend route handlers through `yon.cpp`, generated
  `YonJson` helpers with no dependencies, and the compiled route adapter.
- Yon now supports Swift (`yon.swift`) and Kotlin (`yon.kt`) backend route
  handlers through the compiled route adapter:
  - Swift handlers put static verbs inside an `enum`/`struct`/`class Handler`
    and exchange JSON through `Foundation`.
  - Kotlin handlers expose verbs from the companion object of a
    `class Handler` (or from a top-level `object Handler`). They receive a
    generated `YonJson` helper with no dependencies.
  - Example routes are under `/languages/swift` and `/languages/kotlin`.
- Yon now supports Rust (`yon.rs`) backend route handlers through `rustc`.
  Rust handlers use `struct Handler; impl Handler { pub fn GET(...) { ... } }`
  and receive a generated `YonJson` helper with no dependencies. An example
  route and service are under `/languages/rust`.
- The navigation rail of the example capability atlas now opens as a
  responsive drawer on mobile. On desktop, the rail stays visible at all times.

### Fixed

- The example FYLO users panel now hides previews that look like API keys
  before it shows them in the browser.

## [26.22.03] — 2026-05-27

### Added

- Query filtering in the style of PostgREST on Fylo browser collection
  endpoints. `GET /_fylo/<collection>/?field=operator.value` supports `eq`,
  `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, and the `not`
  prefix. Reserved parameters:
  - `select`: chooses which fields to return.
  - `order`: sorting.
  - `limit`, `offset`: pagination.
- The browser example dashboard now has a Tac input gallery. It covers native
  input types, `select`, `datalist`, and `textarea`. It shows events as they
  happen and has reactive value bindings that persist.
- The example browser app is now a guided capability atlas. It is one
  application tour that links Tac composition, reactive browser state, Yon
  routes, FYLO storage, telemetry, and WebAssembly companions.
- A Browser studio panel shows canvas drawing driven by Tac. It also shows the
  native semantic `progress`, `meter`, `output`, `time`, and `details`
  elements with scoped responsive CSS.
- Tac templates support `:checked="field"`. Checkbox and radio state stays
  reactive when a component renders again.
- The example app in the repository now takes part in standalone strict type
  checking. Its Tac globals and browser dependencies resolve.

### Changed

- **BREAKING**: Yon route handlers use one class per route. You write
  `export class Handler` with static methods named after HTTP verbs
  (`static async GET(request)`). This replaces the old convention of one
  directory per method (`routes/<path>/GET/yon.<ext>`). All polyglot adapters
  (JS, TS, Python, Ruby, PHP, Java, C#, Dart, C++) support the new pattern.
- Upgraded `@d31ma/fylo` to `^26.22.3`. Removed the `getHistory()` calls.
  Added `findDeletedDocs()` and `restoreDoc()` to the machine-interface demo.
- The Fylo browser query panel now accepts filter input in the style of
  PostgREST. It no longer has SQL/findDocs modes. The `<fylo-browser>` Tac
  component and the plain JS shell both use the new interface.

### Removed

- Removed the `POST /_fylo/api/query` endpoint (SQL and find queries).
  Filtering that uses indexes now uses PostgREST-style query parameters on the
  collection URL. Removed the `fylo.sql()` client method.
- Removed Go and Rust language support from the example routes and handler
  adapters.
- Removed version history display from the Fylo browser. Fylo 26.22.3 removed
  the `getHistory()` API that it used.

### Fixed

- The `HandlerAdapter.hasMethod` regex now matches async generator handlers
  (`static async *GET()`) inside class-per-route definitions.
- `Router.validateSegmentPath` no longer reports false duplicate route errors
  for nested slug routes (for example `/items/_id/detail` versus
  `/items/detail`). It now maps slug segments to a `:` placeholder instead of
  removing them, so comparisons keep the path depth.
- Static prerendering now keeps literal replacement-token text, such as Tac's
  `$` and `$$` persistence sigils, when it inserts rendered pages into the
  HTML shell. It no longer expands this text.
- Browser acceptance tests accept `TACHYON_EXAMPLES_E2E_PORT`. This avoids
  false failures when another process already uses the default example port.
- The example inventory form now binds its CRUD (create, read, update, delete)
  draft state directly to its visible native input. A successful Yon save now
  always clears the field.
- Tac client-side navigation keeps query parameters and URL fragments on the
  first hydration, on intercepted link clicks, and on browser history
  navigation.
- New regression tests for component stylesheets check that template-literal
  characters in author CSS comments stay unchanged in the emitted Tac modules.
- Tac `:value` bindings now render loop-local expressions without evaluating
  their identifier outside the loop scope. This prevents a `ReferenceError`
  during builds such as `<loop :for="option of options"><input :value="option" />`.
- Synthetic value-binding updates keep `$event.target`,
  `$event.currentTarget`, and `$event.type`. DOM-style `@input` and `@change`
  handlers therefore still work together with `:value`.
- `@onMount` registration now waits until the renderer binds the companion.
  Mounted callbacks now run even when an app constructor calls `super(props)`
  without passing on the injected Tac helper argument.

## [26.21.7] — 2026-05-24

### Added

- New `POST /languages/javascript/fylo` example route that runs the full FYLO
  machine-interface suite (about 25 operations). The suite moved off the
  lightweight `/languages/javascript` dashboard endpoint so that diagnostics
  stay fast.

### Changed

- **BREAKING**: Tac page and component templates now use `tac.html`. They can
  have optional sibling files
  `tac.js`/`tac.ts`/`tac.css`/`tac.wasm`/`tac.tac.json` and companions backed
  by source files. The old `index.html` convention for Tac source is removed.
  The bundled `dist/**/index.html` output is unchanged, so static hosting and
  previews work as before. Page and component modules in dist are emitted as
  `tac.js`.
- **BREAKING**: Yon route handlers move to
  `server/routes/<path>/<METHOD>/yon.<ext>`:
  - The HTTP method is now the parent directory of the handler file.
  - Every handler file is named `yon.<ext>`, in every language.
  - The companion `OPTIONS.schema.json` still sits next to the method
    directories.
  - Polyglot class-based handlers (Java, C#, Python, PHP, Ruby) take their
    class name from the `yon` basename. The result is a Pascal-cased `Yon`
    class.
- Repository layout changes:
  - `package-contract/` moved to `tests/package-contract/`.
  - `stress/` moved to `tests/stress/`.
  - Integration test runners and workers moved to
    `tests/integration/helpers/`.
  - The `bun test`, `test:blackbox`, and `stress:tac` scripts were updated to
    match.

  Only the layout changed. Runtime behavior did not change.
- Upgraded `@d31ma/fylo` to `26.21.6`. The parts of FYLO that Tachyon uses
  (`new Fylo(fyloOptions(root))`, `Fylo.uniqueTTID`, `executeSQL`,
  `findDocs`, `inspectCollection`) are unchanged. The upgrade brings upstream
  bug fixes and the `LocalQueue` / `publish` / `consume` queue exports. Example
  apps may use these exports directly.

### Internal

- Replaced the in-place `tsc --noEmit` typecheck with `scripts/typecheck.js`.
  This wrapper runs the same compiler command. On working copies in
  cloud-synced folders, it first copies the selected project into the OS temp
  directory.
  - `tsconfig.src.json` is still the default release gate.
  - Pass `tsconfig.tests.json` or `tsconfig.examples.json` to run those
    scoped projects.
  - Settings: `TACHYON_TYPECHECK_TIMEOUT_MS` (default 2 minutes),
    `TACHYON_TYPECHECK_STAGE=1` or `0`, and `TACHYON_TYPECHECK_KEEP_STAGE=1`.
- The FYLO browser now protects against collection names that look like path
  traversal. It checks them when it resolves REST-style collection URLs and
  event-tail files. An invalid event-tail collection name returns a 200 JSON
  error, in the same style as the existing FYLO browser API.
- Removed the last FYLO browser DOM `innerHTML` rendering for dynamic
  collection, document, history, REST result, encrypted-field, and event
  values. The UI now writes dynamic values with text nodes.
- `tsconfig.json` again inherits from `tsconfig.base.json`. Bun's runtime module
  resolver then finds the `@/*` → `./examples/server/*` path alias that the
  example polyglot route handlers use.
- Updated the fallback handler path in `Pool.prewarmAllHandlers` to the new
  `<METHOD>/<routeFileName>` layout. After the Yon rename, the old `<METHOD>`
  fallback built paths that did not exist, without any error. As a result, most
  polyglot handlers were missing from the pool of warmed processes.
- Restored the static schema map on `Validate`. The schema
  validator needs it to match responses. It had been removed from the working
  tree during a refactor.
- `FyloMachineRepository.ensureDemoSchema()` now writes to a
  `fylo-demo-items` collection instead of `items`. The demo run no longer
  overwrites the real items schema that other example routes validate against.

## [26.21.05] — 2026-05-22

### Added

- Stage 3 decorator versions of the `Tac` runtime helpers:
  `@inject(key, fallback?)`, `@provide(key)`, `@env(key, fallback?)`,
  `@onMount`, and `@emit(name)`.
  - Companion scripts can use them as bare identifiers. When the Tachyon
    compiler sees the `@<name>` syntax in a companion script, it imports them
    automatically. User code needs no `import` line.
  - Outside companion scripts (tests, library code), import them from the new
    `@d31ma/tachyon/decorators` package export.
  - The existing instance-method API (`this.inject`, `this.provide`,
    `this.onMount`, …) is unchanged.
- Automatic binding of props to fields. `Tac` copies values from `this.props`
  onto every instance field with the same name that the subclass declared. The
  constructor no longer needs `this.foo = this.props.foo ?? default`
  assignments.
  - Matching ignores a leading `$` on the field name. So `$`-prefixed
    persistent fields bind to the prop without the prefix
    (`$clicks` ↔ `props.clicks`).
  - A direct match with the prefixed key still wins.
  - Only fields that the subclass explicitly declared take part.
  - `props` and `tac` are skipped, so a malicious prop object cannot overwrite
    framework state.
- New `./decorators` package export that resolves to
  `src/runtime/decorators.js`.

### Changed

- The build-time compiler class from the `./compiler` entry was exported as
  `Tac`. It is now exported as `Compiler`. Its source moved from
  `src/compiler/template-compiler.js` to `src/compiler/index.js`. The
  `./compiler` package export still resolves. If you imported the default
  export under the name `Tac`, rename the binding to `Compiler`. This avoids a
  clash with the runtime `Tac` class.
- Internal reorganization of `src/server/`. Flat files moved into
  subdirectories by responsibility:
  - `route-handler.js`, `browser-env.js`, and `schema-validator.js` are now
    under `src/server/http/`.
  - `openapi.js` is under `src/server/openapi/`.
  - `logger.js`, `console-logger.js`, and `telemetry.js` are under
    `src/server/observability/`.
  - `process-pool.js` is under `src/server/process/`.
  - `yon.js` stays at the `src/server/` root.

  `package.json#main`, `#types`, and `exports."."` now point to the new
  `src/server/http/route-handler.js` path. `exports."./server"` is unchanged.
- Build-time manifests (`route-manifest.json`, `shell-manifest.json`) moved
  from `src/runtime/` to `src/shared/manifests/`. This separates generated
  artifacts from runtime source. The compiler path constants and `.gitignore`
  were updated.

### Removed

- Unused `Compiler` import from `src/cli/serve.js`. The compiler module has
  no top-level side effects, so the import did nothing.
- Unused `layout-manifest.json`. The compiler writes `Compiler.layoutMapping`
  to `shell-manifest.json`. No code ever read or wrote
  `layout-manifest.json`. Removed it from `src/shared/manifests/` and
  `.gitignore`.

### Fixed

- Upgraded `@d31ma/fylo` to `26.21.2`, which brings its schema validator
  package `26.21.02`.
  This version added a `SchemaValidator` API. It requires the `.schema.json`
  extension in schema references and `path:`-prefixed cache keys. Changed
  the validation helper to fill the validator cache with keys in the expected
  format.
- Changed `Router.optionsFileName` from `OPTIONS.json` to `OPTIONS.schema.json`
  so the route handler finds the renamed schema files.
- Renamed route `OPTIONS.json` files to `OPTIONS.schema.json` and schema
  history files (`v1.json`) to `v1.schema.json`. This matches the framework's
  naming convention. Moved the test env file from the root to
  `tests/.env.test`.

## [2.0.0]

Tachyon 2.0 is a full rewrite:

- The framework splits into `Tac` (frontend) and `Yon` (backend and runtime).
- The source moves from TypeScript to JavaScript with strict JSDoc.
- The CLI commands have new names.

If you upgrade from 1.x, expect to change imports, bin names, and any code
that expected the old `dist/layouts/` output.

### Breaking Changes

- Source rewritten from TypeScript to JavaScript + JSDoc. The package `main`,
  `types`, and the `.`, `./server`, `./compiler` exports now resolve to `.js`
  files.
- The `./server` export now points to `src/server/yon.js` (before:
  `process-executor.ts`).
- CLI binaries renamed:
  - `tach.init` → `yon.init`
  - `tach.serve` → `yon.serve`
  - `tach.bundle` → `tac.bundle`
  - `tach.preview` → `tac.preview`
- Removed the `dist/layouts/` output. `shells.json` now describes page shells.
  The runtime uses a single app shell template. It injects the HMR (hot module
  replacement) client only in development.

### Added

- Polyglot backend handlers: any executable file with a shebang can serve a
  route. It receives JSON on stdin.
- Object-oriented companion classes with `export default class extends Tac`.
  Anonymous subclasses are also supported.
- Automatic `sessionStorage` persistence for `$`-prefixed instance fields. The
  key is module path + page/component identity + field name.
- Local-first browser `fetch()` inside Tac page and component scripts. It is
  backed by IndexedDB, caches `GET`/`HEAD` reads, and invalidates the cache
  after mutations.
- Explicit allowlist of browser env values through `TAC_PUBLIC_ENV` and
  `this.env(key, fallback)`.
- A generated OpenAPI 3.1 document at `/openapi.json`. A self-hosted Tachyon
  docs UI at `/api-docs` has request auth, filtering, deep links, cURL
  generation, and live "try it out" requests.
- OpenTelemetry storage in FYLO:
  - Request spans and handler spans are linked.
  - Incoming `traceparent` headers continue the trace.
  - Responses include `Traceparent` / `X-Trace-Id` headers.
  - Settings: `OTEL_ENABLED`, `OTEL_FYLO_ROOT`, `OTEL_SERVICE_NAME`, and
    `OTEL_CAPTURE_IP`.
- Scoped component CSS through `@scope ([data-tac-scope="..."])` wrappers.
- Shared frontend folders: `browser/shared/assets/*`, `browser/shared/data/*`,
  the `browser/shared/scripts/main.js` entry, and `browser/shared/styles/*`.
- Built-in health and readiness endpoints: `/health`, `/healthz`, `/ready`,
  `/readyz`.
- `yon.serve` adapts to the app's folders:
  - With only `browser/`, it bundles and serves the frontend.
  - With only `server/`, it serves the backend routes.
  - With both, it runs the full stack on one port.
- Tac template runtime with `{expr}`, `{!expr}`, `@event`, `:prop`,
  `:value`, `<loop :for>`, `<logic :if>`, and `<my-component lazy />`.
- `Tac` helpers: `env`, `fetch`, `emit`, `inject`, `provide`, `onMount`,
  `rerender`, `isBrowser`, `isServer`, `props`.
- Distributed rate limiting through a `rateLimiter` exported from
  `middleware.js`. Upstash Redis works without extra setup. The in-memory
  limiter is kept for deployments with a single instance.
- Production hardening:
  - Bun password verification for hashed Basic Auth.
  - Limits on request body and parameters.
  - Handler timeouts.
  - Rejection of expired JWTs (JSON Web Tokens) when the token can be decoded.
  - Route request and response validation through `OPTIONS`.
  - Configurable CSP.
  - Optional HSTS (HTTP Strict Transport Security).
- Request context that is aware of proxies. CORS (cross-origin resource
  sharing) checks the origin and rejects a request before the handler runs.
  Document requests are detected with `Sec-Fetch-*` headers, or with
  `Accept: text/html` when those headers are missing.
- Cache headers for runtime assets, chunks, shared assets, and shared data.
- Handler responses get an inferred content type: `application/json` for
  output that looks like JSON, `text/plain` otherwise.
- `tac.bundle --watch` and `tac.preview --watch` rebuild static output as you
  work.

### Changed

- Publishing moved to GitHub Packages under `@d31ma/tachyon`.
- Scaffolds now use JavaScript first, with strict JSDoc. The runtime still
  supports TypeScript companion scripts.

### Fixed

- The `findDocs('otel-spans')` call in the telemetry integration test now
  passes the required `StoreQuery` argument. This keeps `tsc --noEmit` clean
  against `@d31ma/fylo` 2.3.

## [1.11.1] and earlier

See the Git history and GitHub release notes for changes before 2.0.

[Unreleased]: https://github.com/d31ma/TACHYON/compare/v26.46.01...HEAD
[26.46.01]: https://github.com/d31ma/TACHYON/compare/v26.36.01...v26.46.01
[26.45.07]: https://github.com/d31ma/TACHYON/compare/v26.36.01...v26.45.07
[26.45.01]: https://github.com/d31ma/TACHYON/compare/v26.36.01...v26.45.01
[26.36.01]: https://github.com/d31ma/TACHYON/compare/v26.35.07...v26.36.01
[26.35.07]: https://github.com/d31ma/TACHYON/compare/v26.33.02...v26.35.07
[26.33.02]: https://github.com/d31ma/TACHYON/compare/v26.33.01...v26.33.02
[26.33.01]: https://github.com/d31ma/TACHYON/compare/v26.31.06...v26.33.01
[26.32.07]: https://github.com/d31ma/TACHYON/compare/v26.31.06...v26.32.07
[26.31.06]: https://github.com/d31ma/TACHYON/compare/v26.30.04...v26.31.06
[26.30.04]: https://github.com/d31ma/TACHYON/compare/v26.30.03...v26.30.04
[26.22.03]: https://github.com/d31ma/TACHYON/compare/v26.21.7...v26.22.03
[26.21.7]: https://github.com/d31ma/TACHYON/compare/v26.21.05...v26.21.7
[26.21.05]: https://github.com/d31ma/TACHYON/compare/v2.0.0...v26.21.05
[2.0.0]: https://github.com/d31ma/TACHYON/compare/v1.11.1...v2.0.0
[1.11.1]: https://github.com/d31ma/TACHYON/releases/tag/v1.11.1
