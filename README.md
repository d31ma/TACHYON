# Tachyon

Tachyon is a full-stack framework. It supports many programming languages (it
is polyglot), and the file system defines its routes. Application developers
write standard HTML. Depending on the target, Tachyon then does this:

- It compiles Tac into web applications that render in the browser, and it
  dispatches Yon REST endpoints.
- It packages those views in native web-view hosts for macOS, iOS, Android,
  Linux, and Windows.

Target-native services give access to each platform's own SDKs.

`Tac` is the view and interaction layer. `Yon` is the server route and handler
layer. Both use the standalone `ty` executable.

> [!IMPORTANT]
> This branch contains the Rust rewrite, written from scratch. Engineering
> phases 0–7 are implemented. The rewrite is not yet a supported Tachyon
> release. [docs/CUTOVER.md](docs/CUTOVER.md) and
> [docs/SUPPORT_TIERS.md](docs/SUPPORT_TIERS.md) record the exact promotion
> evidence and the external gates that remain.

## Why Tachyon

- HTML is the only view language that application code uses.
- Files and directories define routes, components, handlers, and middleware.
- Tac templates support bounded expressions, conditionals, loops, components,
  and slots. The browser renders them. Tac has no server-side rendering (SSR)
  mode.
- Yon handlers return an HTTP status, headers, and a body. Tachyon does not
  render Yon templates. It does not run handlers during a build.
- Browser services are written in JavaScript, TypeScript, Dart (compiled to
  JavaScript), Rust, C#, F# or Go (all four compiled to WebAssembly), or Ruby (on
  Tachyon's mruby WebAssembly runtime, which the developer's machine builds
  once with Ruby and Emscripten 6.0.10 or newer; `ty doctor` lists the
  tools). Swift, Kotlin, C#, and Rust implement the same services for native
  targets.
- Content that is only data lives in `client/shared/data`. A view reads it as
  `data.<name>`.
- Yon runs JavaScript, TypeScript, Python, Java, Groovy, C#, Kotlin, PHP,
  Rust, Dart, Go, Ruby, F# and Lua. Routes expose plain `Handler` types or annotated controllers.
  Go annotations are comments such as `// @Service` (ADR 0050), Ruby
  annotations are comment lines such as `#[Service]` (ADR 0054), F#
  annotations are F# attributes such as `[<Service>]` (ADR 0073), and Lua
  annotations are comment lines such as `-- @Service` above a class table
  (ADR 0085). Programs in
  other languages can still be called through an explicit
  `relay(command, request)` call.
- One project produces deterministic web and native artifacts. Deterministic
  means the same input always gives the same output.
- Tachyon sets explicit limits on generated output, child processes, input
  sizes, build hooks, and native fallback boundaries.
- Public manifests, schemas, diagnostics, and handler envelopes are versioned
  contracts. They live in [api/](api/).

## Quick start

### Install a release

On macOS or Linux (x64 or arm64), run:

```sh
curl -fsSL https://github.com/d31ma/Tachyon/releases/latest/download/install.sh | sh
```

On Windows (x64 or arm64), run this in PowerShell:

```powershell
irm https://github.com/d31ma/Tachyon/releases/latest/download/install.ps1 | iex
```

Each installer does these steps:

1. It downloads the `ty` binary for your system from the latest release.
2. It checks the binary against the release's `SHA256SUMS` file. If the
   checksum is missing or wrong, it stops and installs nothing.
3. It installs `ty` and replaces an older `ty` safely:
   - macOS and Linux: into `/usr/local/bin` if you can write there, otherwise
     into `~/.local/bin`. If that folder is not on your `PATH`, the installer
     tells you the line to add.
   - Windows: into `%LOCALAPPDATA%\Tachyon`, which it adds to your user
     `PATH`. Open a new terminal afterwards.

Two environment variables change the defaults:

| Variable | What it changes |
|---|---|
| `TAC_INSTALL_DIR` | The folder that receives `ty`. |
| `TAC_BASE_URL` | Where the installer downloads the binary and `SHA256SUMS` from. |

### Build from source

The rewrite also builds from source with stable Rust 1.97.1:

```sh
cargo build --release --locked --bin ty
./target/release/ty init hello --name "Hello"
cd hello
../target/release/ty serve
```

Open `http://127.0.0.1:8080/`. The generated project has an HTML page at
`client/pages/tac.html`. The compiler produces a render plan for it. The
external Tac runtime uses that plan to build the page's DOM in the browser.

Build and preview static output in production style:

```sh
ty bundle
ty preview
```

The build writes the web bundle to `dist/web/`. Static `preview` serves an
existing bundle on `127.0.0.1:3000`. Add `--watch` to rebuild it when source
files change. Dynamic web preview also dispatches Yon and uses the private
application configuration. Scaffolded projects configure `127.0.0.1:8080`.

`ty serve` and dynamic `ty preview` start fast. When the output folder already
holds a build, they serve it at once and rebuild in the background; open pages
reload when the new build is ready. Each build prints how long every phase took.
The WebAssembly languages build in parallel: `TAC_BUILD_JOBS` sets how many
toolchains run at once (the default is a third of the CPU cores, from 1 to 4).
The development server builds C# services without the slow native relink;
`ty bundle` keeps the release settings. For a preview that runs for a long
time, use a release `ty` (`cargo build --release --bin ty`).

A watching `ty serve` or `ty preview` notices when the `ty` file on disk
changes, for example after `cargo build`. Open pages show "Tachyon was updated
on disk. Restarting the preview…", the server restarts itself with the new
`ty` and the same arguments, and the pages reload. If the new `ty` does not
start, the old server keeps running and the pages say why; it tries again when
`ty` changes once more. `--no-self-restart` keeps the old `ty` running.

## Project shape

```text
client/
  manifest.json
  pages/
    tac.html
  components/
    product-card/
      tac.html
      tac.css
  services/
    CatalogService.js
  remotes/
  stores/
  setups/
  entities/
  delegates/
  shared/
    assets/
    data/
    scripts/
    styles/
server/
  application.json
  routes/
    products/
      yon.js
  services/
  stores/
  remotes/
  delegates/
  workers/
    cleanup/
      yon.py
middleware.py
.tachyonrc
```

Some files in this tree have a special role:

- `client/manifest.json` is the web app manifest and the whole Tac
  configuration (ADR 0052). Its `tac` member holds the public Tac settings
  and Tachyon's own settings: `origin`, `application`, `scripts`,
  `blockingScripts`, `styles`, `cache`, `metadata` and `sitemap`.
- `server/application.json` holds Yon's settings that are not secret. It has
  no root key. Secrets go in the ignored `.env` file.
- `.tachyonrc` holds only worker intervals.

A project needs only the files it uses. A client-only, server-only, or
full-stack project is valid.

## Tac views

A `client/pages/tac.html` file defines a page. Nested directories define URL
paths. A directory whose name begins with `_` defines a dynamic segment.

A page only combines components. Its `tac.html` contains component tags,
nested as needed, plus whitespace and comments. A page directory contains only
that file. Everything a page shows lives in a component under
`client/components/<name>/`. There, `tac.html` is the view and `tac.css` is
the optional style.

- Markup, a script, or a stylesheet beside a page fails with `TY1309`. The
  error names the component to move it into.
- Behaviour is a service in `client/services/`. A script beside a component
  view fails with `TY1310`. The error names the service file to move it into.
- A public method of a service or a controller only coordinates. A decision,
  a loop or `try` in it fails with `TY1314`; it moves into a private method
  that holds at most one of them (ADR 0063).
- A block of code that repeats (100 tokens over 10 lines, even with other
  names) fails with `TY1315`. It moves into a static method of a utility
  class in `client/shared/utils/` (Tac) or `server/utils/` (Yon) (ADR 0064).
- Dependencies point down: a loop of classes that need each other, a store
  that needs a service, or any class that needs a controller fails with
  `TY1327` (ADR 0069).
- Names are plain English: a clipped word such as `usr` or `cfg` fails with
  `TY1328`, and the help names the whole word.
- A method takes at most 4 parameters and a class needs at most 5 other
  classes (`TY1329`).
- A text or number written twice, or a number in a condition, needs a name:
  a class constant (`TY1330`).

For example, the page `client/pages/tac.html` combines two components:

```html
<site-shell>
  <catalog-grid />
</site-shell>
```

The component view `client/components/catalog/grid/tac.html` lists the
products:

```html
<main>
  <h1>Catalog</h1>
  <loop :for="product of catalog.products"><product-card :name="product.name" /></loop>
</main>
```

The service `client/services/CatalogService.js` holds the products:

```javascript
@Service
export default class CatalogService {
  products = [{ name: 'Anvil' }, { name: 'Bellows' }]
}
```

`client/manifest.json` declares site-wide scripts and stylesheets once, in
`tac.scripts` and `tac.styles`. The build links them into every document.
Each script in `tac.scripts` is a deferred module. A script that must run
before the first paint, such as one that applies a saved theme, goes in
`tac.blockingScripts`: the build links it as a classic script before the Tac
runtime. Files under `client/shared/` are published at stable
`/shared/...` URLs. Limits apply to the number of files and to their bytes.
Tachyon rejects symlinks and anything that is not a regular file.

### Expressions and control flow

Text can include (interpolate) bounded expressions in braces. Braces work
only in text: an attribute takes a value through a `:` name, and the build
refuses braces in a plain attribute with `TY1313`.

```html
<h1>{title}</h1>
<p :class="'status-' + state">{message}</p>
```

Structural control tags are compiler syntax. The compiler resolves them before
output. A browser or native renderer never receives them as unknown elements.

```html
<logic :if="products.length > 0">
  <ul>
    <loop :for="product of products">
      <li>{product.name}</li>
    </loop>
  </ul>
</logic>
<logic else>
  <p>No products yet.</p>
</logic>
```

`logic` and `loop` are the only conditional and iteration tags. The old
`<if>`, `<else>` and `<for>` elements were removed (TY1367); run
`ty migrate fix --write` to rewrite them. Counted loops also work: `<loop :for="let i = 0; i < 3; i++">{i}</loop>`. The compiler validates
comparisons, step direction, and iteration limits. See
[the client runtime migration guide](docs/CLIENT_RUNTIME_MIGRATION.md).

### Components, services, slots, and browser mounting

A directory under `client/components/` defines a Tac component. It has a
`tac.html` view and an optional `tac.css` style. The build resolves component
tags. A component tag can receive static props or bounded expression props.
Slots are scoped. Cycles fail before publication.

Behaviour is a service (ADR 0030; ADR means architecture decision record). A
service is one class in `client/services/<Name>Service.<ext>`:

- The class name is exactly the file name without its extension.
- The class has the `@Service` annotation and is the default export.
- `js` and `ts` are the browser languages.
- `swift` (Apple), `kt` (Android), `cs` (Windows) and `rs` (desktop)
  implement the same service for a native target. They compile into the host,
  not into WebAssembly.

A `tac.js`, `tac.ts` or native `tac.*` file beside a component view fails
with `TY1310`. `ty doctor` reports the toolchains you need.

A view refers to a service by its instance name. The instance name is the
class name without the `Service` suffix, with a lowercase first letter. For
example, `BasketService` is `basket`, and `UserListService` is `userList`. An
event handler or an assignment names either a bare handler or one
`service.member`.

The component view `client/components/basket/panel/tac.html` uses the
`basket` service:

```html
<input on:input="basket.query = $event.target.value" />
<button on:click="basket.add()">Add</button>
<output>Count: {basket.count}</output>
```

The service is in `client/services/BasketService.js`:

```javascript
@Service
export default class BasketService {
  count = 0
  query = ''

  @OnStart
  initialize() {}

  add() { this.count += 1 }
}
```

By default, a service is a page singleton:

- Tachyon constructs it when the first component that names it renders.
- Tachyon initialises it once, when that component activates.
- Every component that names it shares it.
- Tachyon disposes it when the user leaves the page.

`@Lifetime({"value":"prototype"})` gives each component instance its own service
instead. Tachyon constructs it with that instance's properties and disposes it
with that instance.

A service's collaborators live in `client/remotes/`, `client/stores/`,
`client/setups/`, `client/entities/` and `client/delegates/`.
A `@Needs` field named after a collaborator receives it, as it does for a
Yon service (ADR 0038). Yon uses exactly the same folders: `server/services/`,
`server/remotes/`, `server/stores/`, `server/setups/`,
`server/entities/`, `server/delegates/`. Every route shares them (ADR 0031).
The older `clients/` folder still works in both realms. New apps use `remotes/`
for outbound HTTP, socket and gateway classes. Annotations and class names do
not change. See [the migration guide](docs/REMOTES_MIGRATION.md).


Data types without a role live in `client/models/` or `server/models/`: in
TypeScript, Rust, C#, F#, Dart and Kotlin in both realms, Swift in the client,
and Java, Python and PHP in the server. A models file holds data, not
behavior. Other files import a model in their language's usual way, for
example `import type { X } from '/models/X.ts'`, `use crate::models::X;` or
`from models import X`. The framework adds the models files of a language to
every program it builds in that language, and never routes or ships a models
file on its own (ADR 0041, ADR 0043).

Tac is owned entirely by the client. Component mounting can be scheduled as
`load`, `idle`, `visible`, `interaction`, or `never`. A component that names a
service defaults to `load`. A mount policy on a component that names no
service fails with `TY1405`. The legacy `hydrate=` attribute is still accepted
as another name for mount scheduling. It does not enable SSR.

```html
<basket-panel mount="interaction"></basket-panel>
```

A service declares what it needs. It does not receive a runtime object:

- A `@Needs` field named `view`, `lifecycle`, `taskScheduler` or
  `resourceLoader` receives the owner's object of that kind.
  These are the names Spring uses.
- `@OnStart` runs once the owner's DOM exists.
- `@OnStop` runs on disposal.
- To publish an event, a class declares an uninitialised
  `applicationEventPublisher` field and calls `publishEvent(model)` with a
  value of a model type. The model's type name is the event name (ADR 0055).
- `@OnEvent` (or `@OnEvent({"condition":...})`) receives one payload of the
  model type of its parameter.
  `@Order(n)` sets the order of listeners. `@InBackground` can defer a listener.
- A field marked `@SessionState` persists per browser tab. A field marked
  `@LocalState` persists across sessions.
  Neither is secure storage.

There is no `this.tac` object and no implicit `mount()` hook. See
[application events and lifecycle](docs/APPLICATION_EVENTS.md) and
[the API and migration guide](docs/CLIENT_RUNTIME_MIGRATION.md) for cache
privacy rules, bounds, cleanup, and native asynchronous calls.

State belongs to a service, never to a page or a component view. Loop bindings
and the properties a parent passes stay unqualified. The compiler rejects a
`<script>` block in a page with `TY1309`, because a page has no owner to
receive it. Arbitrary inline JavaScript, script tags with attributes, and
executable state initializers fail with `TY1306`. The compiler never evaluates
them as authored source.

Nested loops use lexical scope. An inner loop can read every binding of its
parent loops. It can read them in its iterable, text, dynamic attributes,
conditions, component props, and event arguments. When a loop reuses a
binding name, the new binding hides the old one only in the nearest scope. A
following sibling sees the outer value again. Components receive evaluated
props and own those values. They do not inherit the caller's loop scope.

```html
<loop :for="account of accounts">
  <loop :for="role of rolesFor(account)">
    <account-role :label="format(account, role)" />
    <button on:click="select(account.id, role.id)">Select</button>
  </loop>
</loop>
```

## Yon REST endpoints

A `server/controllers/<Name>Controller.*` file defines REST endpoints.
`RequestRoute` gives the base path. Method annotations select HTTP verbs and
optional paths. The method returns either a value that can
be serialized to JSON, or an explicit response descriptor:

```javascript
@RequestRoute('/products')
@Public
class ProductsController {
  @GetRoute
  async getProducts() {
    return { products: await loadProducts() }
  }
}
```

Tachyon turns ordinary values into JSON responses. To return HTML, the handler
must provide the body and explicitly select `text/html`:

```javascript
@RequestRoute('/products')
@Public
class ProductsController {
  @GetRoute
  getProducts() {
    return {
      status: 200,
      headers: { 'content-type': 'text/html; charset=utf-8' },
      body: '<main><h1>Products</h1></main>',
    }
  }
}
```

Yon sends the HTML body unchanged. It performs no interpolation, component
expansion, control-tag evaluation, or server-side rendering. Tachyon rejects
`yon.html` with `TY1008`. This stops a project from gaining build-time handler
execution by accident.

A controller uses compile-time annotations, without a runtime annotation package:

- `GetRoute`, `PostRoute`, `PutRoute`, `PatchRoute` and `DeleteRoute`
  select the verb. Method names start with that verb, such as `getProducts`.
- `FromPath`, `FromQuery`, `FromBody`, `FromHeader` and `FromCookie`
  select argument values. Types validate arguments and model responses.
- Its providers are `@Service`, `@Store`, `@HTTPClient`, and `@Delegate`
  classes from the shared `server/<role>/` folders.
- `NeedsAll` infers the required dependencies. You write no
  constructor and no `Needs` list.

Tachyon checks role annotations before build or execution. A type and its file
must have the suffix that matches the role, and the file must be in the shared
role folder. Controllers live directly in `server/controllers/`. Old
`server/routes/`, `yon.*` entries and `OPTIONS.schema.json` are refused.
`OPTIONS` and `/api.json` publish schemas generated from the method types. See
[Yon annotations](docs/YON_ANNOTATIONS.md) for the annotation names in each
language, scope semantics, and composition boundaries.

Every supported Tac and Yon source language also supports optional
[behavior annotations](docs/BEHAVIOR_ANNOTATIONS.md): `Setup`,
`NeedsAll`, `CacheSettings`, `Cached`, `PageDefaults`, `Entity`, `Table`
and outbound `HTTPClient` with `GetRequest` and the other request annotations.

- Adapters that the application selects own storage and transport.
- The names are global in every language, so no source file imports them
  (ADR 0046).
- Other languages use markers that the compiler recognizes, plus native
  execution adapters.
- You install no package.
- Tac stays client-only and never receives server credentials.
- Native page contexts have explicit cancellation and disposal. Synchronous
  runtimes check deadlines cooperatively.

The [annotation guide](docs/BEHAVIOR_ANNOTATIONS.md) covers cache mutations,
explicit typed configuration, validation, and authorized Store queries
with derived method names.
[Project structure](docs/BEHAVIOR_PROJECT_STRUCTURE.md) explains the shared role folders and the source-local annotation boundary.
The [Spring review](docs/SPRING_ANNOTATION_REVIEW.md) separates the implemented
features from the deferred transaction and container capabilities.

`relay(command, request)` runs an explicit program without a shell. Its
stdout, stderr, and deadline are bounded, and Tachyon cleans up its process
tree. `.tachyonrc.interpreters`, shebang handlers, and executable-handler
fallbacks are removed. `.tachyonrc.workers` remains. Middleware and workers
must use one of the eleven Yon languages.

A generator HTTP method is a stream. In JavaScript, TypeScript, Python, PHP,
Kotlin, C#, and Ruby, Yon sends each yielded value as a bounded server-sent
event.
Java and Rust still send a single response. When a subscriber drops or a
deadline passes, Tachyon ends the whole handler process group.
`YON_JAVASCRIPT_RUNTIME`, `YON_PYTHON_RUNTIME` and `YON_RUBY_RUNTIME` override
the runtimes.

## Server runtime

`ty serve` builds the web output, dispatches Yon handlers, and watches project
sources by default. If a rebuild fails, the last good output stays available.

```sh
ty serve
ty serve --port 9000
ty serve --host 127.0.0.1 --no-watch
```

The default bind address is loopback. A non-loopback address requires the
explicit `--allow-non-loopback` flag.

`ty serve` first runs bounded capability probes for every runtime that the
discovered Yon entry points need. It runs them before it prepares output,
binds a socket, reports readiness, registers routes or middleware, or starts
workers. This also applies to `--no-bundle`. A static-only project needs no
Yon runtime.

- JavaScript and TypeScript share one JavaScript-runtime probe.
- Java and Kotlin share one Java-runtime probe.
- A missing executable is `TY2112`.
- A capability that is installed but unusable is still a bounded `TY2101`
  readiness failure.
- The C# probe resolves the installed SDK. It then builds a minimal project
  that the framework owns, with isolated temporary CLI and NuGet state. So an
  installation with only the runtime cannot report readiness.

`ty doctor` uses the same set of probes. It reports configured overrides by
environment-variable name, never by executable path.

The runtime supports:

- exact and dynamic filesystem routes;
- standard HTTP methods, bodies, headers, and route parameters;
- before and after middleware, through the same supervised protocol;
- scheduled workers declared in `.tachyonrc`;
- bounded, append-only topic logs, exposed as server-sent events;
- defensive response headers, and static serving that resists path traversal;
- hot updates of semantic CSS and services for development pages, with a safe
  reload fallback.

Topic subscriptions have these limits:

- at most 128 clients in total;
- at most 32 clients per topic;
- at most 32 active topics;
- 256 replay records kept for each active topic;
- at most 64 KiB per record;
- at most 16 MiB per log.

An `EventSource` without a cursor starts at the oldest kept record. When the
browser reconnects, it sends `Last-Event-ID` and resumes at the next record.
An explicitly stale cursor ends the stream with a named `topic-error` event.
Its JSON payload includes `code`, `message`, `category`, `guidance`, and
`terminal: true`. Clients should parse that event and call `close()`. For
`TY_TOPIC_CURSOR_STALE`, they should recreate the `EventSource` without
`?position=` to recover at the replay floor (the oldest kept record). When
capacity is full, the server returns HTTP 503. Close the attempted
subscription and retry with bounded backoff.

When a handler or middleware invocation fails, the response has a bounded
public message and an `x-tachyon-request-id` header. The server writes a
redacted, structured failure event with the same request ID. The ID sorts by
time. Operators can use it to match a failure that the client sees, without
exposing child stderr or internal diagnostics over HTTP. A `TY2112` event adds
only the logical `runtime_family` and `failure_kind: "not_found"`. It does not
include the executable path, the operating-system error, environment values,
source, request body, or child output.

OpenAPI generation and built-in telemetry from the legacy implementation are
deliberately out of scope.
[docs/PARITY_LEDGER.md](docs/PARITY_LEDGER.md) gives the exact compatibility
decision and migration action for each public feature.

## Building targets

```sh
ty bundle --target web
ty bundle --target macos
ty bundle --target ios
ty bundle --target android
ty bundle --target linux
ty bundle --target windows
ty bundle --target all
```

Non-web targets host the same compiled Tac HTML, CSS, and JavaScript in a
platform web view. There is no render-mode flag. There is no version of the
document built from native controls. Target-native services give operating
system (OS) access through a route-scoped `companion.invoke` bridge. Only the
packaged local application can use it.

| Target | Host | Output |
| --- | --- | --- |
| Web | bootstrap HTML, client render plans, CSS, modules, service worker | `dist/web/` |
| macOS | AppKit/WKWebView | `dist/macos/*.app` |
| iOS | UIKit/WKWebView | `dist/ios/*.app` |
| Android | Android WebView | `dist/android/*/*.apk` |
| Linux | GTK4/WebKitGTK | `dist/linux/` |
| Windows | Win32/WebView2 | `dist/windows/` |

Application authors still write semantic HTML and accessible custom elements:

```html
<design-button role="button" aria-label="Save">Save</design-button>
<design-app-bar role="banner" aria-label="Primary navigation">...</design-app-bar>
```

HTML and custom elements render through the platform's browser engine. Roles
are only accessibility semantics. They do not select adapters. The packaged
bundle owns navigation. Remote pages must never receive the application
bridge.

The native application identity lives in `client/manifest.json`
([ADR 0052](docs/adr/0052-the-manifest-is-the-tac-configuration.md)). The
standard members name the app (`name`, or `short_name`), choose the entry
route (`start_url`) and give the icons (`icons`). The `tac.application` member
holds what the standard lacks: the bundle `id`, the `version` and the
`window`:

```json
{
  "name": "Catalog",
  "start_url": "/",
  "tac": {
    "application": {
      "id": "com.example.catalog",
      "version": "1.0.0",
      "window": { "width": 1024, "height": 768 }
    }
  }
}
```

The manifest is sent to every browser, so these values are public. Never put
a secret in it. A `tachyon.json` or `tac.config.js` file stops the build with
a message that says where each value moved; `ty migrate fix --write` moves
the plain values for you.

A platform output is only as supported as its published evidence. A
successful cross-build is `buildable`. It is not `native-tested` or
`supported`. See [docs/SUPPORT_TIERS.md](docs/SUPPORT_TIERS.md).

## CLI reference

| Command | Purpose |
| --- | --- |
| `ty init [directory] --name <name>` | Create an app in a missing or empty directory. In a terminal it asks which parts, Tac and Yon languages, targets and examples you want; `--realm`, `--tac`, `--yon`, `--target` and `--features` give the same answers. It then checks the tools that the app needs and offers to install each one (`--yes`, `--skip-tools`) (ADR 0066) |
| `ty add language client\|server <language>`, `ty add target <target>`, `ty add feature <example>` | Add a language, a target or an example, with its tests and the same tool check |
| `ty serve [project]` | Build, serve, dispatch handlers, watch, and hot-update |
| `ty bundle [project] --target <target>` | Build web or native artifacts |
| `ty native-bundle [project] --target <target>` | Build a selected native host |
| `ty preview [project] --target <target>` | Serve the embedded web bundle of an existing target |
| `ty cache [status\|clean]` | Inspect or remove the cache that an earlier installation left |
| `ty types [project]` | Write TypeScript globals for the composed JavaScript and TypeScript classes into `.tachyon/types/` (ADR 0053) |

`ty cache` manages the installation and runtime cache that `TAC_CACHE_DIR`
selects, including the shared copies of compiled Kotlin, Rust, Dart, Go and
Groovy handlers in `runtime/handlers-v1` (ADR 0037 amendment). It does
**not** inspect or clear a project's compiled handler cache at
`.tachyon/handlers`. For that cache, use the safe recovery procedure in
[the engineering standards](docs/ENGINEERING_STANDARDS.md#handler-cache-operations).

Useful options:

```text
--diagnostic-format human|json
--out-dir <project-relative-directory>
--no-incremental
--no-bundle
--no-watch
--no-self-restart
--skip-package
--watch
--host <address>
--hostname <address>
--port <0..65535>
--allow-non-loopback
```

`--target` accepts one target, a comma-separated list, or `all`. Aliases such
as `browser`, `darwin`, and `win32` are still accepted. Multi-target output
always goes exactly to `dist/web`, `dist/macos`, `dist/ios`, `dist/android`,
`dist/linux`, and `dist/windows`. `TAC_DIST_PATH` selects another output root.
Existing automation can also use `TAC_BUNDLE_TARGET`, `TAC_PREVIEW_TARGET`,
`TAC_TARGET`, `YON_HOST`, `YON_HOSTNAME`, `YON_PORT`, and `YON_SKIP_BUNDLE`.
Tachyon rejects `TAC_RENDER_MODE` and `--render-mode`. Native hosts run the
same Tac document in their platform web view and do not select a rendering
mode.

The public command names from the latest standalone binary are still
accepted. Internal qualification commands such as `doctor`, `migrate`, and
`handler invoke` are intentionally left out of normal help until cutover.

### Long-running command readiness and shutdown

For `serve`, `preview`, and `bundle --watch`, standard error is a stream of
compact JSON lifecycle events. Parse it one line at a time. The
`runtime.signal_handlers_ready` event only proves which operating-system
signal handlers were installed. Despite its name, it does **not** prove that
the application is ready. Before you send traffic, wait for the command's
readiness line on standard output. For a deployed service, also wait for a
successful health probe.

The first supported signal emits `runtime.shutdown_requested` and requests a
graceful shutdown. The development server puts limits on the internal tasks it
owns. However, preview may wait for connections to drain cooperatively, and
`bundle --watch` may finish its synchronous fingerprint pass or its current
bounded build. Supervisors should enforce their own grace period. A second
signal during graceful shutdown emits `runtime.shutdown_forced` and exits
immediately with one of these codes:

- `130` for Unix `SIGINT` or Windows CTRL-C;
- `143` for Unix `SIGTERM`;
- `131` for Windows CTRL-BREAK.

This interface cannot observe Unix `SIGKILL`, Windows `TerminateProcess`,
out-of-memory termination, or host shutdown.
[CLI Signal Lifecycle](docs/SIGNAL_LIFECYCLE.md) describes the complete
structured event and privacy contract.

## Configuration and hooks

The optional root `client/manifest.json` is the web app manifest. A browser
installs the app from it. Its `tac` member is Tac's public configuration. The
build links it from every page and publishes it at `/manifest.webmanifest`.

The optional root `server/application.json` belongs only to Yon. The whole
file is the `yon` namespace, so it has no root key.

- `YON_PROFILE` selects an additional server profile.
- CLI endpoint arguments take precedence over file values.
- Secrets are literal `YON_` assignments in the ignored `.env` file at the
  project root. Underscores map to nested paths.
- The private resolved Yon snapshot never enters client bundles. Builds never
  open `.env`.
- Datasource and Redis settings configure adapters that the application owns.
  They do not open connections automatically.
- `yon.datasource.driver` is `postgresql`, `mysql`, `mariadb`, `sqlite`,
  `sqlserver` or `fylo`. The FYLO driver takes `fylo:///absolute/root`. It
  serves repositories from a local document store instead of SQL
  ([ADR 0032](docs/adr/0032-fylo-datasource.md)).
- Restart the server to apply changes to private configuration.

See [Application configuration](docs/APPLICATION_CONFIGURATION.md) for the
complete precedence rules, language examples, and deployment boundaries.

When the manifest sets `tac.origin`, the build writes `sitemap.xml` and
`robots.txt` itself. The sitemap lists every page route without a path
variable, except pages whose metadata sets `noindex`. A page route with a
path variable is listed once for each value that `tac.sitemap` names in a
data file under `client/shared/`:

```json
{
  "tac": {
    "origin": "https://example.com",
    "metadata": {
      "defaults": { "siteName": "Catalog", "image": "/shared/logo.png" },
      "routes": {
        "/": { "title": "Catalog", "canonical": "/" },
        "/products/_id": { "title": "Product", "noindex": true }
      }
    },
    "sitemap": {
      "/products/_id": { "data": "/shared/data/products.json", "values": "/products", "field": "id" }
    }
  }
}
```

`values` is a JSON pointer to a list in the data file. `field` names the
member of each listed object that fills the path variable; leave it out when
the list holds strings. A `canonical` or `image` that starts with `/` is
joined to `tac.origin`.

## Determinism and failure behavior

- Discovery and route ordering are canonical.
- Builds publish through a staging directory and an atomic swap.
- Incremental state is not trusted. Tachyon reuses it only after it verifies
  its digest.
- A failed build keeps the previous complete output.
- Repeated clean builds are byte-identical for identical inputs and
  toolchains.
- Artifact Manifest v1 records the target, inputs, outputs, toolchains,
  contracts, and digests.
- Diagnostics use stable `TY####` codes and can be emitted as JSON.
- During target builds, each real platform toolchain compiles the generated
  native source. Tests of the source shape do not replace that compilation.

## Security boundary

By default, Tachyon uses:

- loopback networking;
- escaped interpolation;
- no shell dispatch when the framework invokes its own tools;
- strict local asset schemes;
- bounded external input.

The native bridge is reserved for the packaged local document. Remote content
and subframes never get it. Native service code runs with the OS privileges of
the host process. It is not a sandbox, and it is not a fixed allowlist of
device APIs. [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) documents the
threats and the remaining risks.

Handler child processes, warm or not, still have the developer
account's normal access to the file system and the network. The process
supervisor controls the protocol and the lifecycle. It is not an
operating-system sandbox. Deploy handlers inside the isolation boundary that
your environment requires.

### Environment-selected Yon isolation

Tachyon is serverless-first, so by default Yon keeps a handler's process warm:
the process that answers one request stays running and answers later requests
for the same handler, one at a time. An always-on server that wants a clean
process for every request can choose process mode instead:

```sh
export YON_ISOLATION=process
ty serve
```

| `YON_ISOLATION` | What runs a request |
| --- | --- |
| `warm` (default) | A warm handler process that serves many requests, one at a time |
| `process` | A new process for every request, stopped after the response |
| `firecracker` | The operator's Firecracker control program, described below |

In warm mode, `YON_WARM_MAX_REQUESTS` (default 1000) sets how many requests a
process serves before Yon replaces it, and `YON_WARM_IDLE_SECONDS` (default 60)
sets how long an unused process waits before Yon stops it. Yon also replaces a
warm process after a crash, a timeout, a cancellation or a protocol error.
Module-level state can survive between the requests one warm process serves;
a handler must not depend on it. Sockets always keep their own process. See
[ADR 0111](docs/adr/0111-yon-keeps-handler-processes-warm-by-default.md).

Production operators can send JavaScript and Python Yon execution through a
qualified Firecracker control program. Only the environment selects this. The
application's project files cannot override it:

```sh
export YON_ISOLATION=firecracker
export YON_FIRECRACKER_DRIVER=/usr/local/libexec/ty-firecracker
export YON_FIRECRACKER_POOL=production
export YON_FIRECRACKER_VCPUS=1
export YON_FIRECRACKER_MEMORY_MIB=256
export YON_FIRECRACKER_EGRESS=deny
ty serve
```

Firecracker mode uses the same framed Handler
Protocol, deadlines, cancellation, bounded diagnostics, and response
validation. It currently rejects TypeScript with `TY2010` before it starts the
driver. It also rejects the prepared Java, PHP, Kotlin, C#, and Rust paths
with `TY2010`. The reason is that the driver contract has no boundary for
transferring their artifacts. On Windows, `YON_ISOLATION=firecracker` fails
with `TY2010` at startup, because Firecracker needs a Linux host with KVM and
Tachyon cannot check the driver's file permissions there. The configured
control program and host must be
qualified separately. Setting the environment variable alone does not prove
hardware isolation. See
[ADR 0014](docs/adr/0014-environment-selected-yon-isolation.md).

## Migration from the released implementation

Run the compatibility analyzer before you switch a project:

```sh
ty migrate check path/to/project
ty migrate check path/to/project --json
```

The report classifies each finding as `supported`, `changed`, or
`unsupported`. It names a required action for every result that is not
supported. The neutral compatibility corpus compares route graphs, HTTP
status, and semantic browser output against the latest standalone binary. It
does not compare generated artifacts, because their internal shape is
intentionally different.

Tachyon's own breaking changes have automatic fixes:

```sh
ty migrate fix path/to/project
ty migrate fix path/to/project --write
```

The first command prints every change and every manual step. `--write`
applies the changes. The command deletes imports from `tachyon:behaviors`,
and turns an old root middleware class `Handler` into class `Middleware` with
`before(request)` and `after(request)` when its HTTP method functions all hold
the same code. It lists the rest as manual steps: imports that rename the
names, other middleware, and files that break the one-class rule, with the
split to do.

It also moves old configuration into `client/manifest.json` (ADR 0052). From
`tachyon.json` it adds `name` and `start_url` when the manifest has none, and
`id` and `version` as `tac.application`; it deletes `tachyon.json` only when
every value moved and the file did not change after planning. From
`tac.config.js` it copies static fields that are plain data (`application`,
`scripts`, `styles`, `cache`, `metadata`, `origin`). It never runs the file
and never deletes it: computed values, `postBundle` and the file itself are
manual steps with the exact place each value goes. A value that differs from
one the manifest already has is a manual step, never an overwrite.

## Verification

The canonical local gate is:

```sh
cargo fmt --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
cargo deny check advisories bans licenses sources
```

The matching `docs/PHASE_*_EVIDENCE.md` files record the browser,
compatibility, native, recovery, soak, performance, and release lifecycle
commands, with their results. Some claims need hardware or credentials that
are not available. Those claims stay open. Nobody infers them from reading the
source.

## Architecture and contribution

Start with:

- [CONTEXT.md](CONTEXT.md) for product language and invariants;
- [docs/architecture/OVERVIEW.md](docs/architecture/OVERVIEW.md) for the data
  path and dependency direction;
- [docs/PARITY_LEDGER.md](docs/PARITY_LEDGER.md) for compatibility decisions;
- [docs/ENGINEERING_STANDARDS.md](docs/ENGINEERING_STANDARDS.md) for the test,
  security, and review standard;
- [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow;
- [docs/RELEASE_ENGINEERING.md](docs/RELEASE_ENGINEERING.md) for release and
  rollback policy.

A rollback across the mandatory-layer migration is one coordinated restore.
Restore the matching `ty` binary, the Yon handler sources, and `.tachyonrc`
together. If you restore only one of them, removed interpreter, shebang, or
`Handler` fallback assumptions can come back. That is not a supported
rollback.

The Rust implementation was written from scratch. The former JavaScript
framework and its test tree are removed. Compatibility gates compare Rust with
the immutable v26.30.04 release binary, using neutral fixtures in `corpus/`.
No Rust crate imports or copies private legacy internals.

## License

Tachyon is licensed under the terms in [LICENSE](LICENSE).
