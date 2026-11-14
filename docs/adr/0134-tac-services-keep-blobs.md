# ADR 0134: Tac Services Keep Blobs

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md) (a second
  IndexedDB database, for binary values), [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (a new grant and the size budget of `tac-web.js`) and [ADR 0116](0116-go-tac-services-pick-hold-and-upload-files.md)
  (a held file can be kept, and the upload code is shared)

## Context

Mesail, a mail app on Tac and Yon, works offline (CADUCEUS ADR 0006, request
T2). Mail has attachments of a few megabytes each. A Store row (ADR 0059,
ADR 0125) holds text and numbers, at most 64 KiB per field, so it cannot
hold them. A held file (ADR 0116) lives only in page memory, and a
WebAssembly HTTP answer is cut at 1 MiB. So a service had no place to keep
binary data across a reload, and no way to fetch a large body without the
bytes passing through the service.

The page has three places for binary data: IndexedDB `Blob` values, the
Cache API, and the origin private file system (OPFS). OPFS is missing in
WKWebView, which runs the macOS and iOS apps. The Cache API keys entries by
URL and has no quota check per app. IndexedDB `Blob` values work in
Chromium, Firefox, Safari 14 and later, and in every platform web view, and
all five native shells already keep IndexedDB across a relaunch (ADR 0059
amendments).

## Decision

1. `blobs` is a new browser API grant in `tac.browserApis`. The page refuses
   every `Blobs` call without it, and a page without it does not load the
   blob code.
2. Every Tac service language gets a `Blobs` client, injected the way it gets
   `Files`. Each call checks its input at once and throws on a programming
   error (a bad key, a bad range, an undeclared host). The work then runs in
   the page, and the answer comes later as a built-in event (ADR 0065):

   | Call | Answer |
   | --- | --- |
   | `saveHeld(fileId, key)` | `BlobSaved { key, bytes, media, sha256 }` |
   | `download(url, key, headers)` | `BlobSaved` |
   | `list(prefix)` | one `BlobFound { key, bytes, media, sha256, saved, index, count }` per blob, sorted by key, then `BlobsListed { prefix, count, bytes, usedBytes, maxBytes }` |
   | `read(key, offset, count)` | `BlobChunkRead { key, offset, size, data }`, base64, at most 32,768 bytes, the same shape as `FileChunkRead` |
   | `remove(key)` | `BlobRemoved { key, bytes }`; `bytes` is 0 when there was no blob |
   | `open(key)` | `BlobOpened { key, url }`, a `blob:` URL for an `img`, `video` or link in a view |
   | `url(key)` | the URL that `open` made, at once, or empty text |
   | `close(key)` | nothing; the URL stops working |
   | `upload(key, method, url, headers)` | the upload id at once, then `BlobUploaded { upload, key, status, body, truncated, error }` |
   | `cancel(upload)` | nothing; the upload ends with `error` `CANCELLED` |

   Every failure after the start sends `BlobFailed { call, key, reason,
   message }`. The reason is one word: `quota`, `size`, `missing`,
   `network`, `status`, `unavailable` or `failed`; the message is a fixed
   text for the reason, never the browser's own text.
3. The bytes never pass through the service. `saveHeld` keeps a file that
   `Files` holds (so it also needs the `files` grant). `download` makes a
   `GET` to a path on the page's origin or to an https host in
   `tac.outgoing.hosts`, with the same header list as an upload (ADR 0116),
   and streams the body into the store; there is no 1 MiB limit. `upload`
   sends a blob as the body, with the same rules as `Files.upload`. The
   plan for this slice named an `@HTTPClient` return type `BlobRef` instead;
   we chose calls on `Blobs`, as ADR 0116 did for uploads, because they need
   no change to the HTTP client code of 14 languages and give the same
   result.
4. The quota comes from the manifest:
   `tac.storage.blobs { "maxBytes": ..., "maxBlobBytes": ... }`, in bytes.
   The defaults are 536,870,912 (512 MiB) for all blobs and 67,108,864
   (64 MiB), or `maxBytes` when that is smaller, for one blob. The build
   stops with `TY1601` when `maxBlobBytes` is 0 or more than `maxBytes`, or
   `maxBytes` is more than 1 PiB, and on an unknown member. A save that
   would pass `maxBytes` fails with reason `quota` before anything is
   written; the check and the write run in one IndexedDB transaction, so
   two tabs cannot both pass it. A blob over `maxBlobBytes` fails with
   reason `size`; a download stops reading as soon as it passes that size.
   A browser that refuses the write (`QuotaExceededError`) also gives
   `quota`. Tachyon never removes a blob by itself: the app chooses what to
   remove, with `list` and `remove`.
5. Keys are app text of 1 to 512 bytes with no control character and no
   `..`. A save under an existing key replaces the blob, and the quota
   counts only the new size. `saved` is milliseconds since 1970, and
   `sha256` is the hex digest of the bytes.
6. Storage: the IndexedDB database `tachyon-blobs` has two object stores
   with the key as the store key: `blobs` holds the `Blob` values, and
   `index` holds a small row per blob (`key`, `bytes`, `media`, `sha256`,
   `saved`, `keyId`). Listing and the quota read only `index`. A browser
   that refuses a `Blob` value (WebKit in a private session answers
   `UnknownError`) gets the same bytes as an `ArrayBuffer`; a read turns
   it back into a `Blob` of the kept media type.
7. The seal hook for sealed tables (T6 and T7, a later ADR): the page asks
   `globalThis[Symbol.for('tachyon.tac.blobs.seal')]` for an object with a
   `keyId`, `seal(blob)` and `open(blob, keyId)`. Without one, blobs are
   kept as they are, with an empty `keyId`. A blob kept with a `keyId` that
   no seal on the page can open fails with reason `unavailable`; it is
   never shown as plain bytes. `bytes`, `sha256` and the quota count the
   plain value.
8. The code lives in `.tachyon/tac-blobs.js` (with `tac-sha256.js`,
   `tac-random-id.js` and the new `tac-transfer.js`, the upload and host
   check code that `tac-files.js` now shares), which ships only to pages
   that grant `blobs`, before the service worker is built so that offline
   pages keep it. The page also gets `<meta name="tachyon-blobs">` with the
   two limits. `tac-web.js` gains only the `blobs` entry: one helper now
   routes `files`, `blobs`, `media` (ADR 0129) and `crypto` (ADR 0132) to
   their modules, so the
   runtime stays under its budget of 18,200 bytes.
9. Space: the blob store adds its bytes to the counters that
   `StorageSpace.estimate` uses where the page has no
   `navigator.storage.estimate` (the Apple shells, ADR 0123).
10. The Content-Security-Policy of the server and of the native hosts allows
    `blob:` in `img-src` and a new `media-src 'self' blob:`. Both policies
    are written without reading `tac.browserApis`, so the rule holds for
    every page. A `blob:` URL can only be made by script that already runs
    in the page, from bytes it already has, and it never reaches the
    network; `img-src` already allows `data:`, which has the same reach.
11. Native shells run the same page code. Their web views keep IndexedDB
    in the app's own data, which the operating system keeps until the app
    is removed. No host code changes.

## Consequences

- An app can keep attachments offline, show them in a view, and send them
  again, in every Tac service language, with no bytes in the service.
- The app owns eviction. `BlobsListed` gives the used bytes and the quota,
  and `BlobFound` gives the size and the time of each blob.
- A browser may still delete an origin's storage when the device runs low,
  unless the origin is persistent: apps that keep blobs should call
  `StorageSpace.persist` (ADR 0123). Safari asks the user above about 1 GB
  in some versions; measure on iOS before raising `maxBytes` that high.
- Each `read` of a sealed blob opens the whole blob. Large sealed blobs are
  a later concern of the sealed tables slice.
- Out of scope: blobs shared between origins, streaming a blob into a
  `video` element by ranges (only whole-blob URLs), encryption (the seal
  hook only), automatic eviction, download progress, and cancelling a
  download.

## Evidence

- `compiler::page_blobs_tests`: the page module against an in-memory
  IndexedDB: keep, download, list, read, open, upload, the quota from the
  manifest and from the browser, the size limit, missing blobs, the seal
  hook, refused keys, hosts and headers, and the manifest limits.
- `tac_web::web_api_tests::blobs_tests`: a service in Rust, Go, C#, F#, Swift,
  Kotlin, Java, Dart, Ruby, Lua and C++ calls `Blobs` and hears `BlobSaved`
  and `BlobChunkRead`; `tac_python::browser_api_tests` does the same for
  Python, and `compiler::web_api_tests` for JavaScript.
- `scripts/blobs-browser-test.mjs`: real IndexedDB `Blob` values in
  Chromium, Firefox and WebKit: keep, list, read, reload, quota, and a
  `blob:` URL shown in an `img`.
