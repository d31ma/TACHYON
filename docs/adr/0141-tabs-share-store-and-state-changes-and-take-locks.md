# ADR 0141: Tabs Share Store and State Changes, and Take Locks

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md)
  (a change in one tab no longer waits for a reload to reach another tab)
- Builds on: [ADR 0125](0125-store-tables-load-by-partition.md) (partitions
  decide what a tab reads again) and
  [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (the synchronous page bridge, built-in events and the caller rule)

## Context

ADR 0059 kept one copy of the Store and of `@LocalState` per tab: "a change
in one tab reaches another tab only after that tab reloads". Mesail, a
messaging application on Tachyon, keeps its conversations and its outgoing
queue in Store tables and expects people to open it in several tabs
(CADUCEUS ADR 0006, request T11). Two problems follow:

1. A tab shows old rows and old settings after another tab changed them.
2. Every tab sends the outgoing queue, so a message can go out twice.

The browser has the parts: `BroadcastChannel` (every supported browser and
web view, also without a secure context), the window `storage` event for
`localStorage`, and Web Locks (`navigator.locks`, only in a secure context).

## Decision

1. **Store rows reach the other tabs.** After a write reaches IndexedDB, the
   page posts `{ changes: [{ table, ids }] }` on the `BroadcastChannel`
   `tachyon-store`. This covers every writer: the memory copy of WebAssembly
   and Dart services, the asynchronous JavaScript and TypeScript adapter,
   transactions, and `deleteBy` queries. The message carries table names and
   ids, never row data.
2. **A tab reads the rows again.** A tab that knows the table (it mirrors
   it, a service declared it, or it loaded a partition of it) reads those ids
   from IndexedDB and applies them to its copy: an eager table, every loaded
   partition (a row can move between partitions), and a partition that is
   still loading. A partition the tab has not loaded stays on disk. The read
   waits for the tab's own writes that are already queued, and an id that
   the tab itself wrote but has not stored yet is skipped: that local write
   is newer and reaches the other tabs when it is stored. So the last write
   to reach IndexedDB wins in every tab, as it does on disk.
3. **Services hear `StoreChanged { table, partition }`**, a new built-in
   event (sender `Page`, no grant) published to the services of the other
   tabs after their copy changed. `partition` is the partition value as text,
   or empty for a table without partitions. One event is sent per changed
   partition: the partition of each changed row, and each loaded partition
   that held a changed row. A row deleted in a partition this tab has not
   loaded sends no event for that partition. The tab that wrote hears
   nothing new; it already has the rows.
4. **`@LocalState` follows the other tabs.** The page listens to the window
   `storage` event. When another tab writes the key of a `@LocalState` field
   (`tac:<scope>:<field>`), every live owner of that field in this tab gets
   the new value through the same setter a service uses, in every language,
   and the view renders again. The value is not written back, so the change
   does not echo. A removed key keeps the current value. `@SessionState` is
   per tab and does not follow.
5. **A `Locks` page client** in every Tac language (Rust, Go, C#, F#, Swift,
   Kotlin, Java, Dart, Ruby, Lua, Python, C++, JavaScript, TypeScript), behind
   the grant `locks` in `tac.browserApis`:
   - `request(name)` answers at once. When this tab holds the lock, the page
     sends `LockGranted { name }` to the caller. A second request for a name
     this tab already holds or waits for shares that one request; a held
     lock answers `LockGranted` again at once.
   - `release(name)` frees the lock. Called while the request still waits,
     it stops the request, which sends nothing.
   - A closed tab frees its locks (the browser does this).
   - Names are text of 1 to 256 characters; the page asks the browser for
     `tachyon:<name>`, so no name hits the Web Locks names that start with
     `-`. A bad name is an error at the call, as for every module client.
   - Only exclusive locks. Shared locks, `ifAvailable` and `steal` are not
     offered.
6. **Without Web Locks the page grants every lock at once.** That is the
   macOS and iOS app scheme `tachyon-app://`, which is not a secure context,
   and a web page on plain `http` outside localhost. A native app has one
   window (decision 9 of the offline-first plan), so one page holds every
   lock anyway. The build prints a note when `locks` is granted and
   `tac.origin` is not `https`.
7. **Code layout.** The tab channel lives in `tac-storage.js`
   (`TacEntityTabs`) and the storage listener in `tac-client.js`
   (`StoredTabs`), because both are part of every page that has a Store or
   stored fields. The lock code is `.tachyon/tac-locks.js`, shipped only to
   pages that grant `locks`, as the entry `PageModule::LOCKS` beside passkeys
   and crypto. `tac-web.js` only names the two calls through
   `TacWebApis.calls`, as Blobs and Peers do, so it grows by one line and
   `MOST_RUNTIME_BYTES` does not change.

## Consequences

- Mesail takes one lock, for example `outbox`, in each tab; only the tab
  that hears `LockGranted` sends the queue. When that tab closes, another
  tab gets the lock.
- Two tabs that write the same row at once both see the last stored write.
  Tachyon does not merge rows; the server stays the source of truth.
- Each Store write costs one small `BroadcastChannel` message; each other
  tab reads only the written ids.
- The sealed tables of the next slice change the same mirror: the rows a tab
  reads again in `TacEntityTabs.rows` must be opened there too.
- A native app with several windows would not share locks between them.
  If that is needed later, the shell must coordinate its windows.

## Evidence

- `scripts/tabs-browser-test.mjs` (`bun run test:tabs-browser`, in CI) opens
  two tabs in Chromium, Firefox and WebKit: a Store row and its delete reach
  the other tab with `StoreChanged`; a loaded partition gets new rows and a
  partition that was never loaded stays on disk; a `@LocalState` value
  follows and a `@SessionState` value does not; a lock waits in the second
  tab until the first releases it, a waiting request released early sends
  nothing, and closing a tab frees its lock.
- `compiler/page_locks_tests.rs`: the page runtime with a fake Web Locks
  (grant, wait, cancel, bad names, no module, no grant, no Web Locks), and
  a build ships the module only when `locks` is granted.
- One service test per language: `tac_web/locks_*tests.rs` (and Python in
  `tac_python/browser_api_tests.rs`) take and release a lock;
  `store_partition_*_tests.rs` hear `StoreChanged` and read the other tab's
  row; each stored-state test follows a `@LocalState` change of another tab.
