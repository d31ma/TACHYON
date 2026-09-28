# ADR 0075: Release Builds Pre-Compress and the Server Negotiates

- Status: Accepted
- Date: 2026-09-28
- Works with: ADR 0074 (front-end and API rules: route size budgets can
  count the compressed bytes that are sent)

## Context

`ty start`, `ty preview` and the static preview sent every file without
compression. The WebAssembly services are large: on the Tachyon website the
Go module is 3.2 MB, the .NET runtime 1.5 MB and the Ruby runtime 1.1 MB. A
browser can unpack Brotli and gzip, and every common web server and CDN can
send a file that was compressed ahead of time. Compressing each answer while
it is sent costs CPU on every request and cannot use the slowest, best
Brotli level.

## Decision

1. A release build of the web target (`ty build`, `ty bundle`; profile
   Release, no native target) writes `<file>.br` (Brotli, quality 11,
   window 22) and `<file>.gz` (gzip, best level) next to each file with the
   extension js, mjs, cjs, css, html, json, webmanifest, svg, wasm, map, xml
   or txt that has at least 1024 bytes. A copy is kept only when it is
   smaller than the file. Other formats (images, fonts, archives) are
   already compressed and get no copy.
2. The output is deterministic (fixed levels, gzip time 0), so the same
   sources give the same bytes. The copies are written after the build
   digest, so `BuildResult::sha256` does not change.
3. The build is incremental: when the previous output holds the same bytes
   for a file, its old copies are reused, not encoded again. Encoding runs
   on at most four threads.
4. The server uses tower-http `ServeDir` with `precompressed_br` and
   `precompressed_gzip`. It reads `Accept-Encoding`, honours `q=0`, and picks
   Brotli, then gzip, then the plain file. It sets `Content-Encoding`,
   `Vary: Accept-Encoding`, and the `Content-Length`, `Last-Modified` and
   `ETag` of the file that it sends, so each copy has its own ETag. Range
   and HEAD requests describe the copy that is sent.
5. `ty start` and the static preview use the copies. A server that watches
   sources (`ty preview`, `ty serve`) sends plain files: it adds the live
   reload script to each page, and its development builds write no copies,
   so hot updates stay fast.
6. Native targets get no copies: a WebView reads files from the app bundle,
   and copies would only make the bundle larger.
7. The encoders are the pure-Rust crates `brotli` 9 and `flate2` 1.1 with
   `miniz_oxide`. Their licences pass `cargo deny`, and NOTICE holds the
   Brotli licence text.
8. `BuildResult::compression()` reports how many files have a copy and
   their raw and Brotli sizes. `ty build` prints it. The build keeps the
   copies in memory before the route size budgets run, so the budgets
   (ADR 0074) can also count the Brotli bytes that a browser downloads
   (`CompressedFiles::sent_bytes`).

## Consequences

- The Tachyon website sends 2.7 MB instead of 11 MB (75% less). The Go
  module goes from 3.2 MB to 684 KiB, the .NET runtime from 1.5 MB to
  461 KiB, the Ruby runtime from 1.1 MB to 347 KiB and the Dart module from
  61 KiB to 21 KiB.
- The first release build takes longer, because Brotli at quality 11 is
  slow. The next builds reuse the copies of unchanged files.
- The published folder holds about twice as many files. A host that does
  not know the copies still sends the plain files.
- The service worker is unchanged: the browser unpacks each answer before
  the worker or the cache sees it, and the worker's asset list names only
  the plain files.
