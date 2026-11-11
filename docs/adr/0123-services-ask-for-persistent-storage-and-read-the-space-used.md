# ADR 0123: Services Ask for Persistent Storage and Read the Space Used

- Status: Accepted
- Date: 2026-11-11
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (the browser
  APIs that need a grant, and the size budget of `tac-web.js`)

## Context

Mesail, a mail app on Tac and Yon, keeps mail on the device so that it works
offline (CADUCEUS ADR 0006, request T9). A browser may delete an origin's
storage when the device runs low on space, unless the origin is
"persistent". The web gives two calls for this: `navigator.storage.persist()`
asks the browser to keep the storage, and `navigator.storage.estimate()`
tells how many bytes the origin uses (`usage`) and may use (`quota`). Tac
services had neither. Mesail needs both: it asks once to keep its mail, and
it shows the user how much space the app uses before it saves a large
attachment (the blob store of the next slice, T2, uses the same numbers for
its quota check).

`navigator.storage` exists only in a secure context. Browsers differ:
Chromium decides `persist()` by itself, Firefox asks the user, Safari has
it since 15.2. Native apps run Tac pages in the platform web view (ADR
0018). Their storage belongs to the app, and the operating system deletes it
only when the app is uninstalled (or offloaded on iOS); no browser policy
deletes it. The Apple shells load pages from `tachyon-app://`, which is not a
secure context, so WKWebView has no `navigator.storage` there.

## Decision

1. `persistentStorage` is a new browser API grant in `tac.browserApis`.
   `persist` needs it, because Firefox shows the user a question. `estimate`
   needs no grant: it reads only the page's own numbers.
2. Every Tac service language gets a `StorageSpace` client, injected the way
   it gets `Geolocation`, with two calls on one contract (the page's
   `storageSpace` browser API). Both start the work and return at once; the
   answer comes later as a built-in event, as for every slow browser call
   (ADR 0065):
   - `estimate` answers with `StorageEstimated { usage, quota, persisted }`
     (bytes, bytes, and whether the storage is kept);
   - `persist` answers with `StoragePersisted { persisted }`.
   A failure after the call started sends `BrowserCallFailed` with `api`
   `storageSpace`. A `persist` call without the grant fails at once, the way
   every browser call fails in that language.

   | Language | How a service gets it | Calls |
   | --- | --- | --- |
   | JavaScript, TypeScript | `@Needs storageSpace` (`TacStorageSpace`) | `estimate()`, `persist()` |
   | Rust | a `storage_space: StorageSpace` field filled by `Default` | `estimate()`, `persist()`; each returns `Result` |
   | Go | a `// @Needs` field `storageSpace StorageSpace` | `Estimate() error`, `Persist() error` |
   | C#, F# | `new()` field, `let storageSpace = StorageSpace()` | `Estimate()`, `Persist()` |
   | Kotlin, Java, Dart, C++ | `StorageSpace()`, `new StorageSpace()`, a `@Needs()` field, `[[Needs]] StorageSpace&` | `estimate()`, `persist()` |
   | Swift | `StorageSpace()` | `estimate()`, `persist()`, both `throws` |
   | Ruby | `StorageSpace.new` | `estimate`, `persist` |
   | Lua | the global table `StorageSpace` | `StorageSpace:estimate()`, `StorageSpace:persist()` |
   | Python | a `needs` field `storage_space: Annotated[StorageSpace, needs]` (ADR 0121) | `estimate()`, `persist()` |

   A Swift, Kotlin, C# or Rust service that is linked into a native app as
   native code cannot use `StorageSpace`, like every page client (`TY1317`).
3. **In a web browser** the page answers from `navigator.storage`:
   `usage` and `quota` from `estimate()`, `persisted` from `persisted()`,
   and `persist()` as the browser answers it. On an insecure page, where
   `navigator.storage` does not exist, `estimate` answers `quota` 0 and
   `persisted` false, and `persist` answers false. Nothing fails there: a
   service can always ask.
4. **In a native app** the shell answers through the host bridge, on every
   target, with the new host calls `browserApi.storageSpace.estimate` and
   `browserApi.storageSpace.persist`:
   - `persisted` is true, and `persist` answers true (the host refuses it
     without the grant, as the page does);
   - `quota` is the free disk space for the app's data:
     `volumeAvailableCapacityForImportantUsage` of the home folder on macOS
     and iOS, `StatFs.getAvailableBytes` of the files folder on Android,
     `GetDiskFreeSpaceEx` of `%LOCALAPPDATA%` on Windows, and the free space
     of the user data folder (GIO, `statvfs`) on Linux;
   - `usage` comes from the web view's `navigator.storage.estimate()` where
     the origin is secure (Android, Windows, Linux). On the Apple shells it
     is the sum of the bytes that Tachyon's own stores report (below), 0 for
     now.
   These calls are not in the per-target "bridged" list of ADR 0065: that
   list replaces the whole browser call with the host's answer, while
   `estimate` joins the page's `usage` with the host's `quota`.
5. **Reuse.** The page keeps the space code in `tac-web.js` for later
   slices: `TacWebApis.space()` gives `{ usage, quota, persisted }` as a
   promise, and `TacWebApis.countSpace(counter)` adds a function that returns
   the bytes a Tachyon store keeps. The counters are summed into `usage` only
   where `navigator.storage` is missing, because `estimate()` already counts
   IndexedDB. The blob store (T2) registers its index there and checks its
   quota with `space()`.
6. The size budget of `tac-web.js` rises from 13,500 to 15,500 bytes, once,
   for these calls and the shared space code.

## Consequences

- Services can ask to keep their offline data and can show or check the
  space used, in every Tac language, on the web and in the five native
  shells.
- `persisted: true` in a native app means "no browser policy deletes this";
  the operating system still deletes it on uninstall, and iOS may offload an
  app that the user has not opened for a long time.
- Safari rounds `usage` and updates it slowly, and its `quota` is an
  estimate, not a promise; a write can still fail with a quota error. Apps
  must handle that error, not trust the numbers alone.
- Chromium may answer `persist()` with false until the user engages with the
  site (bookmarks it, installs it, allows notifications). The service hears
  the answer and may ask again later.
- On the Apple shells `usage` stays 0 until a Tachyon store reports its
  bytes (the T2 blob store will). WebKit gives no size for the data of a
  custom scheme.

## Evidence

- `tac_web::web_api_tests::storage_space_tests`: the page runtime on an
  insecure page, with a counter, in a secure page, through a native host and
  through a native host without `navigator.storage`; and a service in Rust,
  Go, C#, F#, Swift, Kotlin, Java, Dart, Ruby, Lua and C++ that calls both and
  hears both events.
- `tac_python::browser_api_tests::a_python_web_service_estimates_and_persists_storage`
  and `compiler::web_api_tests::a_javascript_service_needs_every_curated_browser_api`.
- `native::browser_api_bridge`: every shell's source answers both host
  calls. The Windows (Wine and MSVC) and Linux (WebKitGTK) browser API
  fixtures call both, with and without the grant.
