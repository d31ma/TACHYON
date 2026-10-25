# ADR 0059: Tac Stores Use IndexedDB and Caches Use the Cache API

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (Tac data access
  and caching need no adapter code in JavaScript and TypeScript)
- Preserves: [ADR 0055](0055-annotation-arguments-use-types.md) (typed
  arguments), [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule
  table) and [ADR 0057](0057-needs-all-requires-its-fields.md) (NeedsAll)

## Context

A Tac Store, a Cached method and a saved service field all keep data in the
browser. Before this decision a Tac application wrote its own entity and
cache adapters for `@Entity` and `@CacheSettings`, even when the data never
left the browser. Saved fields used name prefixes: `$name` for sessionStorage
and `$$name` for localStorage. A prefix is not an annotation, so the rule
table, the TypeScript hover text and the other Tac languages did not know it.

## Decision

1. `@Entity({"adapter": "indexeddb"})` needs no adapter code in a JavaScript
   or TypeScript Tac service. Tachyon keeps one IndexedDB object store per
   Table in the database `tachyon-store`, keyed by the Id field.
   - Indexes come from the compiled Store metadata: every field that a
     derived `findBy`/`countBy` method or a `@Query` compares or orders by
     gets a single-field index. The runtime compares the wanted stores and
     indexes with the database and upgrades the database version when some
     are missing. An upgrade only adds; it never deletes a store, a row or an
     index.
   - If a stored Table uses another key than the entity's Id field, Tachyon
     stops with a message and deletes nothing.
   - A query uses the index of its first equality or range comparison on a
     string or number value, then filters and sorts the rest in memory. A
     query without a usable index reads the whole store. Both reads stop with
     a clear message above 10,000 rows. Pages sort by the requested fields and
     then by the Id field, so a page is stable.
   - `@AllOrNothing({"adapter": "indexeddb"})` collects the writes of the
     method and saves them in one IndexedDB transaction on commit. Reads in
     the method see the collected writes. A failure saves nothing.
   - A Store on this adapter needs no `authorizeEntity`, because the data
     never leaves the browser. An `authorizeEntity` that the Setup declares
     still runs.
2. A `@CacheSettings` cache that the Setup does not configure uses the
   browser Cache API in a JavaScript or TypeScript Tac service.
   - Each cache is the Cache Storage cache `tachyon-cache:<name>`. An entry is
     a synthetic JSON response under `/.tachyon/cache/<name>/<key>`, with the
     expiry and the last write time in headers. These caches are separate from
     the service worker caches (`tachyon-static-*`).
   - Before the first render, Tachyon loads every saved entry that has not
     expired into memory (at most 10,000 per cache, waiting at most two
     seconds). Reads are answered from memory, so they are synchronous for
     the runtime. Writes and deletions go to the Cache API in the background,
     in order, with only the newest change per key waiting.
   - Tachyon enforces `ttlMs`, `maxBytes` (the size of one value, as in Yon)
     and `maxEntries`: the least recently used entry goes first. The order of
     use is kept in memory; after a reload it starts from the write order.
   - Where the Cache API is missing (an insecure origin such as a plain-http
     local network address, or a native web view without a secure scheme),
     the cache lives in memory only, and the console says so once. It is
     never an error.
   - WebKit storage that is not kept on disk (a Safari private window, a
     web view with a non-persistent data store, and Playwright's default
     WebKit context) drops the records of a Cache Storage cache once no open
     page holds that cache. So there, a reload starts the cache empty and the
     method runs again; IndexedDB rows stay. A persistent WebKit profile,
     Chromium and Firefox keep the entries. Tachyon's iOS and macOS shells
     use the default (persistent) data store.
   - Tabs do not share memory. A change in one tab reaches another tab only
     after that tab reloads (best effort).
   - A cache on the built-in storage needs no `authorize` hook, for the same
     reason as the Store. A declared hook still runs.
3. `@LocalState` on a Tac service field keeps it in localStorage, and
   `@SessionState` keeps it in sessionStorage. The rules of the old prefixes
   stay: the value is read when the service is created, written on every
   assignment, stored as JSON of at most 64 KiB, and namespaced by the
   service and the page (`tac:<scope>:<field>`). A value saved under the old
   key (`tac:<scope>:$$field` or `$field`) moves to the new key once.
   - The markers work in JavaScript, TypeScript, Dart, Swift, Kotlin, Java,
     C#, F# (`[<LocalState>]` above a settable `member val`, read through the
     F# service's C# view), Rust, Go and Ruby (`#[LocalState]` above
     `attr_accessor :name`; the page name is lowerCamel): the compiler reads them into the page plan (`stored`), and the
     client runtime binds the field, also through the native and WebAssembly
     field bridges. They are bare, belong on fields, exclude each other and
     `@Needs`, and are Tac only (rule table rows).
   - A class field named `$name` or `$$name` in a JavaScript, TypeScript or
     Dart Tac service is refused, and the message names the marker to write.
     `ty migrate fix --write` changes the fields, their uses in the class and
     the view expressions (`editor.$$theme` becomes `editor.theme`).
4. A WebAssembly, Dart or Ruby Tac service reaches the same built-ins through one
   synchronous storage host on the page (`host` in `tac-storage.js`), because
   its methods answer the page synchronously. The service's behavior runtime
   uses the host when its Setup names no adapter of that name, and it skips
   the authorize hooks in the same way. Languages: Go, Rust (a
   `tac_host_storage` WebAssembly import) and C# (a `storage` JSImport that
   sets `BehaviorStorage.Host`), and Dart (the compiled program's `main`
   sets `BehaviorStorage.host` through `dart:js_interop`), and Ruby (mruby
   WebAssembly, ADR 0054).
   - Ruby uses the pinned runtime without a rebuild. The only host import of
     the mruby build, `TachyonWeb.host_emit`, is synchronous and hands the
     page a pointer to the Ruby string. For a storage request the Ruby
     prelude sends a string that starts with `tachyon-storage:ask` and ends
     with 4,096 spaces; `tachyon-ruby.js` writes the answer over that string
     and pads it with spaces, and the Ruby side parses it as JSON. A larger
     answer is kept by the page, which writes `{"more": bytes}`; the prelude
     then sends a `tachyon-storage:take` string of that size.
   - An Android Kotlin, Swift, native Rust or native C# companion has no
     storage host, so its Setup must name every cache and entity adapter that it
     uses. The native build stops when a Cached, UpdateCache or ClearCache
     method uses a cache, or a Store uses an entity adapter, that no Setup
     in the service names (for example "Android Kotlin services need a cache
     adapter in their Setup"). It does not wait for a failure at run time.
   - Cached reads are served from the memory copy that Tachyon loads before
     the first render, so they need no wait.
   - Before a WebAssembly or Dart service module starts, Tachyon copies every Table of
     `tachyon-store` into memory (only when the database exists). Reads,
     derived queries and pages are answered from that copy with the same
     rules as the IndexedDB adapter. Writes change the copy at once and go to
     IndexedDB in the background, in order; an AllOrNothing method keeps its
     writes apart and saves them in one IndexedDB transaction on commit.
   - A Table with more than 10,000 rows is not copied; a WebAssembly service
     that uses it stops with a message. Writes of JavaScript services on the
     same page update the copy; the copy is not shared between tabs.
   - A storage failure keeps its message through the behavior runtime, as in
     JavaScript.

## Consequences

- A Tac application can cache and store browser data with annotations alone.
- The Store and Cache built-ins exist in the JavaScript runtime and, through
  the storage host, in Go, Rust, C#, Kotlin and Swift WebAssembly services
  (Kotlin since ADR 0078, Swift since ADR 0082) and in Dart and Ruby
  services. Native companions (Swift,
  Android Kotlin, native Rust and C#) still
  configure their own adapters: they answer the page asynchronously from
  another process, so the synchronous page host does not reach them. The
  native build refuses such a service when its Setup does not name the
  adapter.
- The HTTP response cache of `@HTTPClient` stayed on its own IndexedDB
  database (`tachyon-fetch-cache`) until ADR 0087 removed it: a client read
  is now kept by `@Cached` in the behavior cache.
- Browser storage is not a secret store. None of these keeps credentials.
- Evidence: `scripts/storage-browser-test.mjs`, run in Chromium, Firefox and
  WebKit with a persistent profile (reload and tab behavior of
  both markers, adoption of old keys, Cache API persistence, TTL, LRU
  eviction, ClearCache, the insecure-origin fallback, IndexedDB derived,
  Query and paged reads, AllOrNothing, the schema upgrade and the refused key
  change, and Go, Ruby and C# WebAssembly services whose Cached value and
  Store row survive a reload, whose ClearCache reads again and whose refused
  AllOrNothing saves nothing; the C# service also runs with denied storage);
  unit tests in `behavior/stored_state.rs` and
  `migrate/fix/stored_fields.rs`; the rule-table completeness tests;
  `tac_web` and `tac_dart` tests that run a Go and a Rust WebAssembly service
  and a compiled Dart service in Node, and a `tac_ruby` test that runs a Ruby
  service on the mruby runtime in Node (with an answer larger than 4,096
  bytes), against
  `tac-storage.js` (synchronous Cached reads, ClearCache, Store save and
  find, a refused AllOrNothing method); a C# runtime test with a fake page
  host; `behavior/polyglot/native_storage.rs` and
  `native/storage_tests.rs` (a Kotlin, Swift or native Rust service whose
  Setup does not name its adapter stops the native build).

## Amendment (2026-10-18): native methods save stored fields, and `/index.html` is `/`

A native companion's fields reach the page through the native field bridge,
which reads them again after every native method. The page now saves its
`@LocalState` and `@SessionState` fields after that refresh, as it already did
after an assignment, so a value a native method sets survives a reload.

The saved-field key uses the page path with a trailing `/index.html` written as
`/`. The iOS, Android and macOS hosts open their first page as `/index.html`,
and a link back to the same route opens `/`; both now read the same key. A
value saved before this change under a `/index.html` key is not moved.

Tests: `scripts/runtime-browser-test.mjs` (a native field set by a method
survives a reload and is read at `/stored/index.html`), and the iOS simulator
gate `scripts/ios/native-test.mjs` (the root route's field set by a Swift
method is read again after the app navigates away and back).

## Amendment (2026-10-18): native hosts keep web storage across relaunch

- The Linux host now gives its web view a persistent WebKit network session.
  WebKit keeps the application's IndexedDB, localStorage and Cache Storage in
  `$XDG_DATA_HOME/<application id>/webkit` and its HTTP cache in
  `$XDG_CACHE_HOME/<application id>/webkit` (`~/.local/share` and `~/.cache`
  when the variables are unset). Before, the session was ephemeral, so Store
  rows, `@Cached` values, `@LocalState` fields and `preferenceStrings` were
  lost when the application quit. The application id is the
  `tac.application.id` from the manifest, so two applications never share
  data.
- WebKitGTK saves a localStorage change to disk a moment after the write,
  and its helper processes end with the application. A value written just
  before a quit was lost in 4 of 10 runs. So when the Linux application
  quits, the host hides its window and keeps WebKit running for 750 ms
  before it stops the services. IndexedDB rows and Cache API entries were
  already on disk when their writes finished.
- The Cache API accepts only `http:` and `https:` request URLs. The macOS,
  iOS and Linux hosts serve the page from a private scheme (`tachyon-app:`
  and `tachyon:`), so every `cache.put` failed there and a `@Cached` value
  lived in memory only, without a notice. The runtime now keys a cache entry
  with `https://cache.tachyon.invalid` when the page is not on `http:` or
  `https:`. The key is only a name inside the page's own Cache Storage, which
  WebKit still separates by the page's origin; nothing is fetched from it.
  Pages on `http:` and `https:` keep their own origin as the key, so entries
  already written in a browser stay valid.
- Every host gate now quits the application, starts it again and reads back
  an IndexedDB row, a localStorage value and a Cache API entry written before
  the quit.
- On iOS the default data store does not keep Cache API entries for the
  `tachyon-app:` page, which corrects the claim above for iOS. On the iOS 26.5
  simulator the cache's name is kept but its entries are not: WebKit writes
  no record file for them. So after a relaunch an iOS `@Cached` method runs
  again, as in a non-persistent WebKit store. Store rows and `@LocalState`
  fields are kept. The iOS gate records this limit and fails only if a row or
  a localStorage value is lost.
- On Android, Store rows and `@LocalState` fields survive `am force-stop`.
  A Cache API entry written about 6 s before the force-stop was missing
  after the relaunch in 2 of the 3 runs that checked it, so a `@Cached`
  method may run again after the system stops the app. The Android gate
  records this and fails only if a row or a localStorage value is lost.

## Amendment (2026-10-19): the iOS Cache API limit is not proven yet

The amendment above states as fact that iOS does not keep Cache API entries
across a relaunch. That is not proven yet. The iOS run wrote the check's old
raw cache entry, which has no `x-tachyon-expires-at` header. Tac's start-up
cache sweep deletes such an entry, so the empty cache after the relaunch may
come from the sweep and not from WebKit. The macOS gate had the same problem
and kept its entry once the check wrote it in Tac's format
(`docs/SUPPORT_TIERS.md`, "Web storage across relaunch"). The iOS limit waits
on a rerun of `scripts/ios/native-test.mjs` on the simulator with that check
(plan item RM2). Until then, treat the iOS Cache API limit as unproven.

## Amendment (2026-10-19): the three-browser storage gate after ST1, with Ruby in WebKit

`scripts/storage-browser-test.mjs` ran again after the persistent-session
change of ST1 (499a6246), in Chromium 141, Firefox 153 and WebKit 26.5
(Playwright 1.62.1), in an x86_64 Ubuntu 24.04 cloud container with `ty`
built from develop `d7110f74`, and again on `162064a0`. Every toolchain was
present, so no service was
skipped: the Ruby (mruby built with Emscripten 6.0.10), Go, Rust, C#, F#,
Kotlin, Swift and Java services all ran in all three engines, including Ruby
in WebKit, which had no recorded run before. All checks passed. With every
service on its one route, the fixture page loads 3607 KB of WebAssembly, so
its manifest now sets `tac.budgets.webassembly` to 6144 KB; the gate's own
budget check still uses the default limits.


## Amendment (2026-10-24): iOS keeps Cache API entries across a relaunch (RM2)

The rerun owed by the amendment of 2026-10-19 passed. `scripts/ios/native-test.mjs`
wrote the check's Tac-format entry (cache `tachyon-cache:relaunch`, an
`x-tachyon-expires-at` one hour ahead) on the iOS 26.5 simulator, terminated
the app, and a new process read `local=kept,store=kept,cache=kept` in 2 of 2
runs (Xcode 27.1, develop `4c052177`). WebKit wrote the entry as a Cache
Storage record under the app's `WebsiteData`, and the same file was read after
the relaunch. The iOS limit described in the 2026-10-18 amendment does not
exist: the earlier result came from Tac's start-up sweep deleting the raw
entry. iOS keeps IndexedDB rows, LocalState values and Cache API entries on
disk, like macOS and Linux, and the iOS gate now requires `cache=kept`.

## Amendment (2026-10-24): Windows keeps web storage across a relaunch (SD1)

`scripts/windows/desktop-run.ps1` ran `scripts/windows/relaunch-storage-test.mjs`
from SSH in the signed-in desktop session of a Windows 11 ARM64 machine
(WebView2 Runtime 155.0.4283.62). The first launch wrote an IndexedDB row, a
localStorage value and a Cache API entry, the app quit, and a new process read
all three back. WebView2 keeps the data in `<App>.exe.WebView2` (ADR 0097).
With the Linux, macOS and iOS runs above, every desktop and Apple host keeps
the three across a relaunch; Android keeps rows and localStorage values, and a
Cache API entry written just before the system stops the app is not always
kept.
