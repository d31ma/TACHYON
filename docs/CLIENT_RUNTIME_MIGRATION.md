# Client runtime and native service migration

This guide describes the current rewrite contract. A version label alone does
not prove a release. Use the acceptance gates at the end of this guide to check
the exact binary.

## Injected HTTP clients and named resources

Application HTTP calls use `@HTTPClient` together with request method annotations.

- Client is a Tachyon role marker. It is not a Spring annotation.
- The class name and the filename of a client end in Client.
- Clients live in `client/remotes/`. This folder sits beside
  `client/services/`, which holds the services that use the clients. The
  older `client/clients/` folder still compiles; see `REMOTES_MIGRATION.md`.
- `@Needs` injects a concrete client by the name of the field. You do not
  write a constructor or a `Needs` list by hand.

The example below shows two files:

- `client/remotes/CatalogClient.js` defines the client.
- `client/services/CatalogService.js` defines the service. The component
  `client/components/catalog/panel/tac.html` addresses this service as
  `catalog`. The page `client/pages/catalog/tac.html` places that component as
  `<catalog-panel />`.

```javascript
@HTTPClient
@CacheSettings({"name":"catalog","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
class CatalogClient {
  @Cached
  @GetRequest("/api/catalog")
  async list() {}

  @ClearCache({"method":"list"})
  @PostRequest("/api/catalog")
  async save(@RequestBody item) {}
}

@Service
export default class CatalogService {
  @Needs
  catalogClient;
  items = [];

  @OnStart
  async initialize() {
    const items = await this.catalogClient.list();
    if (!Array.isArray(items)) throw new TypeError('Expected a catalog list');
    this.items = items;
  }

  async save(item) {
    await this.catalogClient.save(item);
  }
}
```

How a client method works:

- The method body describes optional query data, headers and JSON body data.
- The generated adapter sends the HTTP request. It returns parsed JSON, not a
  Response.
- The call rejects on HTTP errors, invalid JSON, redirects and responses that
  are too large.
- A 204 response or a HEAD response returns null.
- The compiler validates each path. A path must be an origin-relative HTTP(S)
  endpoint.
- The client sends no credentials. Tachyon copies no server configuration and
  no secrets into a client.

How services own their clients:

- Each browser service receives its own client graph and its own context.
- Tachyon aborts active operations when it disposes the owner or replaces the
  owner during a hot update. A prototype-scoped service is disposed together
  with its component instance. A singleton is disposed when the user leaves the
  page.
- A page that the browser suspends in the back/forward cache (bfcache) keeps
  its services.
- Constructors and field initializers must not use injected dependencies. Use
  OnStart or event methods after construction instead.
- A service field can be declared, left undefined and named after another
  service (`catalog;`). That field receives the page singleton of that service.
- A dependency cycle between services fails at runtime.

The former `this.tac` object and the `TacRuntime` type are removed. Inject only
the browser resources that you need. Put `@Needs` on a field named after
each resource. The field names are the Spring resource names (ADR 0038; ADR means architecture
decision record):

| Field name | Resource |
| --- | --- |
| `view` | the element of the owner |
| `lifecycle` | the `AbortSignal` of the owner |
| `resourceLoader` | the resource loader |
| `taskScheduler` | the task scheduler |
| `applicationEventPublisher` | the application event publisher |

A field with one of these names that has no initial value also receives the
resource without the annotation.

- Resource fields are non-enumerable. Because of this, hot-state snapshots stay
  cloneable.
- After state changes in the background, call `render()` on the injected
  taskScheduler.
- Templates and local event listeners rerender automatically.
- Tac stays client-rendered.

## Declarative JSON response caching

`@HTTPClient` reads are sent `no-store`, so they are always fresh. To keep a
read, mark the client class `@CacheSettings` and the read `@Cached`, like
Spring's `@Cacheable`. Mark the write that changes the data `@ClearCache` or
`@UpdateCache`, as in the example above (ADR 0087).

- The former `cache` option of `@HTTPClient` (`cache-first`, `network-first`,
  `reload`, `no-store`) and its IndexedDB store `tachyon-fetch-cache` are
  removed. The build refuses `{"cache": ...}` and names `CacheSettings` and
  `Cached`.
- The `cacheManager` resource that cleared that store is removed too. A
  `@Needs cacheManager` field is refused; use `@ClearCache` instead.
- In JavaScript and TypeScript a `Cached` GET is also a public read, so the
  service worker may answer it by the manifest's `tac.cache` rules. Add a rule
  for a path whose reads must work offline.
- Never cache secrets or data that belongs to one user.
- Client parsing limits every response to 1 MiB. It also applies the operation
  deadline of the behavior context.

## Separate service-worker assets and declared public reads

The service worker follows the path rules in the manifest's `tac.cache` for eligible public
reads. It is a separate layer from the behavior cache that `Cached` uses, on
purpose.

- Examples written only for the worker policy can use the standard browser
  fetch internally to test those rules. Application HTTP examples should use
  Client.
- A successful write still invalidates the affected worker reads before it
  returns its response.

Worker cache rules:

- Credentialed requests, Authorization requests and no-store requests bypass
  the worker cache and evict matching worker entries.
- Private, no-store and Vary responses evict earlier entries.
- The worker requests packaged assets anonymously. It checks them against the
  SHA-256 fingerprints that the compiler emits.
- Arbitrary HTML and URLs that look like file extensions are not trusted
  assets.
- Worker API bodies are limited to 256 KiB. Assets are limited to 4 MiB.
  Storage is limited to 256 entries and 32 MiB.
- Native hosts do not register this browser worker.

Warming and clearing the worker cache:

- `precache(urls)` warms eligible same-origin public assets in the worker
  cache.
- It accepts at most 512 URLs. Each body is at most 4 MiB. Each warm-up has a
  budget of 30 seconds and 32 MiB.
- Component branches that were not taken are warmed when the browser is idle.
- `clearAssetCache()` removes the static worker caches. It does not remove
  IndexedDB data or persisted fields.

## Kept fields: `@SessionState` and `@LocalState`

In the example below, `draft` uses sessionStorage, so it lasts for the
current tab. `theme` uses localStorage, so it survives browser sessions. The
file is `client/services/PreferencesService.js`. The markers replace the old
`$` and `$$` prefixes, which Tachyon now refuses (ADR 0059);
`ty migrate fix --write` converts them. The markers work in JavaScript,
TypeScript, Dart, Swift, Kotlin, C#, Rust and Go Tac services.

```javascript
@Service
export default class PreferencesService {
  @SessionState
  draft = '';
  @LocalState
  theme = 'light';
}
```

How values are read and written:

- The stored value replaces the declared default before OnStart runs.
- Tachyon writes a value when you assign it. It does not write the initial
  declaration.
- Changing a nested part of an object does not run the persistence setter.
  Reassign the object after you change it.
- Assigning `undefined` removes the stored key.
- Keep values JSON-serializable and small. The limit is 64 KiB after
  serialization.
- These APIs are not secure storage.

How keys are built:

- Persisted fields belong to services.
- For a singleton service, the key combines the instance name and the pathname:
  `tac:service:preferences:/settings:draft`.
- For a prototype-scoped service, the key combines the component name, the
  instance name and the pathname: `tac:basket-panel.basket:/shop:draft`.
- A value saved under the old key (`...:$draft` or `...:$$theme`) moves to
  the new key the first time the service starts.
- To tell apart two instances of the same component, give each a `persist-id`
  prop. It replaces the pathname in the prototype key.

Failures and other tabs:

- If storage is denied or holds invalid JSON, the field keeps a usable
  in-memory default.
- Tachyon does not automatically send storage changes to instances that are
  already mounted in other tabs.

## Document-local application events

The example below is the file `client/services/CounterService.js`.

```javascript
/**
 * @typedef {Object} CounterValue
 * @property {integer} count
 */
/**
 * @typedef {Object} CounterResult
 * @property {integer} doubled
 */
@Service
export default class CounterService {
  applicationEventPublisher;
  $count = 0;
  displayedCount = 0;

  @OnStart
  initialize() { this.announce(); }

  /** @param {CounterValue} value */
  @Order(-1)
  @OnEvent({"replay":true,"condition":"value.count >= 0"})
  changed(value) { this.displayedCount = value.count; }

  announce() {
    /** @type {CounterValue} */
    const value = { count: this.$count };
    this.applicationEventPublisher.publishEvent(value, true);
  }

  increment() {
    this.$count += 1;
    this.announce();
  }

  async calculate() {
    /** @type {CounterResult} */
    const result = { doubled: this.$count * 2 };
    this.applicationEventPublisher.publishEvent(result);
  }
}
```

Publishing:

- To publish, call the injected publisher. The compiler binds it to the
  `applicationEventPublisher` field, which has no initial value.
- `@EventPublisher` is removed (ADR 0031).
- A method that throws never reaches the publish call.

Listening:

- Each listener receives one isolated, finite JSON value. The value is limited
  to 64 KiB.
- Listeners run in `@Order` order.
- An optional compiled `condition` filters the events a listener receives.
- Retention and replay are false by default.
- There is no longer an imperative getter for the retained value.
- Listeners register before initialization. They unregister when the owner is
  disposed or replaced by a hot update.
- A failure exposes a generic error marker. It never exposes payloads or
  exception text.
- Delivery is not over the network, not across tabs, not durable and not
  transactional.

See [application events](APPLICATION_EVENTS.md) for lifecycle ordering,
asynchronous work and the limits of each language.

The retired spellings `@onMount`, `@publish` and `@subscribe` fail with
`TY1306` and a migration message. Tachyon does not pass them to a JavaScript
runtime. It also does not silently treat them as syntax that does nothing.

This does not forbid the optional polyglot behavior annotations that ADR 0022
accepts.

- In JS/TS, the global behavior annotations provide adapter-backed caching,
  persistence, outbound HTTP, pagination, configuration and explicit
  constructor generation. This works in Tac and in Yon.
- Use the injected `lifecycle` signal to own the cancellation of the browser
  context.
- Native services instead own their signals explicitly and dispose their
  contexts. See [the shared contract](BEHAVIOR_ANNOTATIONS.md).
- This does not add server-side rendering (SSR).

## Counted loops remain bounded

```html
<loop :for="let i = 0; i < 3; i++"><span>{i}</span></loop>
<loop :for="let i = 6; i >= 0; i -= 2"><span>{i}</span></loop>
<loop :for="const item of items"><span>{item.name}</span></loop>
```

The counted form requires:

- exactly one `let` binding;
- a comparison against that same binding;
- a step of `++`, `--`, `+= positiveMagnitude` or `-= positiveMagnitude` that
  moves in the correct direction.

Compilation fails for a zero step, a negative literal step, mismatched
bindings and extra statements.

At runtime:

- Bounds or steps that are not finite produce no iterations.
- A loop fails with a bounded error when a step makes no numeric progress or
  when the loop runs more than 10,000 iterations.
- A separate budget of 100,000 iterations applies to all loops together. This
  includes nested empty loops that emit no nodes.
- Nested loops can see the lexical locals of outer loops.

The `<if>`, `<else>` and `<for>` elements were removed (TY1367). Write
`<logic :if>`, `<logic :else-if>`, `<logic else>` and `<loop :for>`;
`ty migrate fix --write` rewrites the old elements. Iterable bindings without a
declaration are still accepted.

## Browser Wasm to target-native services

The former Wasm component ABI (application binary interface) is retired.
Behaviour is a service (ADR 0030). Keep browser behaviour in
`client/services/<Name>Service.js` or `.ts`. Add a native implementation of the
same service beside it. Add one file for each language the service must reach:

| Native source | Targets |
| --- | --- |
| `client/services/<Name>Service.swift` | macOS and iOS |
| `client/services/<Name>Service.kt` | Android |
| `client/services/<Name>Service.cs` | Windows |
| `client/services/<Name>Service.rs` | macOS, Linux, Windows |

- The platform-specific language wins over the Rust desktop fallback. On each
  target, one compiled implementation answers a service.
- Where no host links a Rust, C#, Swift or Kotlin service, and for every Go
  and Java service, the service runs as WebAssembly, including on the web
  (ADR 0035, ADR 0037, ADR 0045, ADR 0078, ADR 0082, ADR 0083). So every
  service language reaches every target.
- Python is not a Tac service language. Dart is: it compiles to JavaScript
  for every target (ADR 0033).
- A native `tac.*` file beside a component view is `TY1310`.

### The native bridge

- Native hosts render the same Tac document in their platform web view.
- Native services run in the host. They reach OS APIs through the SDK of their
  language.
- The bridge serves only the packaged local application. It does not serve
  remote pages or subframes.
- Every `init`, `get`, `set`, `call` or `dispose` operation in the bridge
  protocol carries the canonical route from the compiler and the instance name
  of the service (`service: "basket"`). It also carries a fresh owner identity
  for each activation.
- The browser activates a native service once for each browser-side service
  instance. For a singleton, that is once per page. For a prototype-scoped
  service, that is once per component instance.
- An unknown service answers `TY_NATIVE_SERVICE`. Calls and events from a stale
  owner answer `TY_NATIVE_OWNER`.

The former fixed JavaScript host verbs are not compatibility APIs of this
architecture. These verbs are `host.on`, `shortcuts.register`, and
`contentSurface.open/state`. Move those operations into a target-native service
that uses the platform SDK. Deliver local native events with an injected
ApplicationEventPublisher and an OnEvent listener.

The injected `WindowControls` browser API (ADR 0087) covers only the window
controls that the application declares. It replaces the former
`globalThis.tachyonWindow` global, and it does not replace a registry for
arbitrary device capabilities.

### Native calls from the browser

- Native methods are asynchronous in the browser. Use
  `{await basket.method()}` in a template, or await the returned Promise in your
  own code.
- Initialization reads field values. It never calls methods to find out what
  they return.
- A field assignment goes through the host before the view rerenders.
- An explicit method call refreshes the field snapshot after it returns.
- Each field has a revision number. Because of this, a late reply cannot undo
  later typing. A refresh skips fields that have queued writes.
- Discovering native methods must not cause side effects.
- Lifecycle hooks and listener hooks are not ordinary bridge methods.
- Synchronous native publications wait in a queue until the current method
  returns. This keeps native borrow safety and owner identity intact.
- This annotation API does not cover background publication without an owner.

Limits and timeouts:

- Native assignments allow at most 128 queued writes.
- The browser waits at most ten seconds for each native call.
- This timeout does not preempt native code and does not roll back mutations.
  Native work that the application owns may still finish after the caller stops
  waiting.
- Keep native services responsive. Move long-running work off the interaction
  path of the host.
- A hung service may need an application relaunch.
- Tachyon does not automatically replay failed mutations.

### Android

- Android pins AndroidX WebKit `1.14.0`. It uses the frame-aware asynchronous
  message bridge of that library. There is no fallback to the legacy
  JavaScript interface.
- If the runtime has no `WEB_MESSAGE_LISTENER`, the app shows a refusal page
  at startup and loads no page. The page names the installed System WebView.
  Update Android System WebView from Google Play, then open the app again.
- The bridge allows 128 pending document calls, each with a ten-second
  deadline.
- It uses one worker with a queue of 128 requests.
- Navigation rejects pending document calls and discards stale queued work.
- Destroying the activity shuts down the worker.
- These controls do not force an already-running native service to stop.

### macOS and iOS

macOS and iOS share the same bounded Foundation JSON parser and the same
canonical request serialization. This replaces the handwritten Swift value
scanner. Before Foundation decodes a request, a byte-level guard does these
checks:

- it enforces the 64 KiB size limit and the 64-level nesting limit;
- it rejects duplicate root keys.

### Arguments

Browser event handlers receive an implicit DOM event before the arguments you
write. Native methods receive only the JSON arguments you write. For example,
`on:click="counter.setCount(7)"` calls the native method with `7`, not with a
DOM event. Explicit expressions such as `$event.target.value` still pass their
resolved value.

Automatic discovery of native members currently supports fields and public
methods with zero arguments. Parameterized methods are not bound
automatically. So the `setCount(7)` example needs a native member table that
you write explicitly and that accepts that argument. Until typed parameter
discovery exists, use a field assignment or an explicit table.

## Acceptance against the installed or published binary

```sh
TAC_BIN=/absolute/path/to/ty node scripts/runtime-browser-test.mjs
TAC_BIN=/absolute/path/to/ty node scripts/authored-node-browser-gate.mjs
TAC_BIN=/absolute/path/to/ty node scripts/service-worker-browser-test.mjs
TAC_BIN=/absolute/path/to/ty node scripts/storage-browser-test.mjs
TAC_BIN=/absolute/path/to/ty node scripts/native/companion-test.mjs
TAC_BIN=/absolute/path/to/ty node scripts/website-browser-test.mjs
TAC_BIN=/absolute/path/to/ty bash scripts/compat/verify-ledger.sh
```

Requirements of the gates:

- Browser gates need Node and Playwright Chromium. The HTTPS offline case also
  needs OpenSSL.
- The desktop native service gate needs Rust and Python 3. It does not need a
  GUI SDK. It stages two routes with the selected binary. It compiles the
  emitted Rust service with the host toolchain. Then it runs the generated ABI,
  including native OS access and publication.
- Platform UI gates are separate. Cross-compiling an archive does not prove
  native interaction.
- Historical ADR and evidence documents describe older architectures. They do
  not describe the support level of this architecture.
