# ADR 0116: Tac Services Pick, Hold and Upload Files

- Status: Accepted
- Date: 2026-11-10
- Amends: [ADR 0089](0089-native-capabilities-are-activation-owned-service-providers.md)
  (the Go answer to `TY1407`)
- Relates to: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md),
  [ADR 0094](0094-local-host-names-and-host-validation.md),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)

## Context

A Go Tac service runs in the page as WebAssembly. It had no way to let the
user choose a file. `FilePicker` (ADR 0089) needs an activation-owned native
provider, and Go has none, so a Go service that named `FilePicker` stopped the
build with `TY1407`. Applications written in Go, such as Mesail, still need to
attach files, show their name, size and type, check them by hash, and send
them to storage.

Three limits shape the design:

- every call between Go and the page is JSON text of at most 64 KiB, and an
  event to a service has the same limit;
- the page must not send file bytes to a host the project did not declare;
- the shared browser runtime has a size budget, and most pages never use
  files.

## Decision

Tachyon adds a browser API named `files`. A project grants it with `"files"`
in `tac.browserApis` in `client/manifest.json`. A service gets it as the type
`Files`, the way its language gets other browser APIs (in Go, a `// @Needs`
field).

### One contract for every service language

Every call takes only text, numbers and true/false values, and every answer
is a flat event model with text, number and true/false fields. Tachyon's
shared web event models already reach every service language, so each
language gets the same models in its own style, and each binding is a thin
call to the page.

### Picking and dropping

- `Pick(accept, multiple)` opens the browser's file chooser. It needs a user
  action, like the clipboard. `accept` is the HTML list, such as
  `".pdf,image/*"`, with at most 32 entries.
- The answer is one `FilePicked { id, name, size, media, sha256, index, count }`
  per file, sent to the calling service. A cancelled chooser answers
  `FilePickCancelled {}`.
- An element with `data-tac-drop-zone="<name>"` is a drop zone. While files are
  dragged over it, it carries `data-tac-drag-over`. Each dropped file arrives
  as `FileDropped`, with the `FilePicked` fields plus `zone`, published to
  every listener, because a drop has no calling service.
- `media` is the media type, such as `image/png` (not `type`, which is a
  reserved word in Rust and F#). `id` is a random UUID. `sha256` is the lowercase hex SHA-256 of the whole
  file, computed in the page in 1 MiB steps, because WebCrypto has no
  streaming digest.

### Holding and reading

- The page holds each chosen file by its id, at most 256 files and at most 32
  from one choice or drop. The bytes stay in the page. They leave it only
  through an upload.
- `Read(id, offset, count)` reads 1 to 32,768 bytes and answers
  `FileChunkRead { id, offset, size, data }`. `data` is base64 text, so the
  largest chunk fits the 64 KiB event limit. `size` is the whole file's size,
  so a reader knows when it is done. Go's `FileChunkRead.Bytes()` decodes it.
- `Release(id)` forgets the file. A later call with that id fails.

### Uploading

- `Upload(id, method, url, headers)` starts an upload and returns its upload
  id at once. `headers` is text with one `name: value` per line.
  `Cancel(upload)` stops it.
- The answer is `UploadFinished { upload, id, status, body, truncated, error }`.
  `body` holds at most 8,192 characters of the response; `truncated` says when
  there was more. `error` is empty when the server answered, or `UPLOAD_NETWORK`,
  `CANCELLED` or `DEADLINE` (15 minutes).
- The URL is a path that starts with `/`, resolved against `tac.yon.origin` or
  the page's origin, or an `https` address whose host is listed in
  `tac.outgoing.hosts` (`*.example.com` means its subdomains). `http` is
  allowed only for local hosts (ADR 0094). Everything else is refused at once
  with `UPLOAD_ORIGIN`. A URL with a user name or password is refused.
- The method is `PUT` (the default) or `POST`. Headers are limited to
  `authorization`, `content-type`, `content-md5`, `content-disposition`,
  `content-encoding`, `cache-control` and names that start with `x-amz-`,
  `x-goog-` or `x-ms-`, at most 32 headers in 8 KiB, with no line breaks
  (`UPLOAD_HEADER`). This covers presigned storage uploads.
- The page sends the file with `fetch`, `credentials: 'omit'`,
  `redirect: 'error'` and no referrer. A redirect could carry the bytes to an
  undeclared host, and cookies could carry the user's session to a storage
  host, so both are refused. `fetch` reports no upload progress, so Tachyon
  sends no progress events.
- The page's Content Security Policy still limits `connect-src`; the runtime
  check gives a clear code before the browser would block the request.

### Shipping

- The files runtime (`tac-files.js` with its SHA-256) is a separate module,
  `/.tachyon/tac-files.js`. A page loads it, with a
  `<meta name="tachyon-outgoing-hosts">` tag, only when the manifest grants
  `files`. A page that grants nothing gets nothing new.
- The shared runtime gains only a small `files` entry that checks the grant and
  forwards the call.

### Native apps

Native hosts run the same page in the platform web view ([ADR 0018](0018-native-hosts-run-the-platform-javascript-engine.md)), so they
use the web view's file chooser and drops:

- macOS: the host now answers the web view's open-panel request with
  `NSOpenPanel`. Before, a file input did nothing in the macOS app.
- Android: the host now answers `onShowFileChooser` with the system chooser
  intent. Android web views get no file drops.
- iOS, Windows and Linux: the web views show their own chooser. iOS takes drops
  on iPad only.

### TY1407 for Go

`FilePicker` stays a native-provider capability (ADR 0089), and Go still has
no provider for it. `Files` is the Go way to pick files, so `TY1407` for Go
now says to use `Files` instead. Java keeps its old help text.

### Languages

Every Tac service language gets `Files` with the same five calls and the same
five event models, in its own style:

| Language | Field | Calls |
| --- | --- | --- |
| JavaScript, TypeScript | `@Needs files` (`TacFiles` in TypeScript) | `pick`, `read`, `release`, `upload`, `cancel` |
| Rust | `files: Files` | `pick`, `read`, `release`, `upload`, `cancel`, each a `Result` |
| Go | `// @Needs` field `files Files` | `Pick`, `Read`, `Release`, `Upload`, `Cancel`, each with an `error` |
| C#, F# | `new()` / `Files()` | `Pick`, `Read`, `Release`, `Upload`, `Cancel` |
| Swift | `Files()` | `pick(accept:multiple:)`, `read(id:offset:count:)`, `release(id:)`, `upload(id:method:url:headers:)`, `cancel(upload:)`, each `throws` |
| Kotlin, Java, Dart, C++ | `Files()`, `new Files()`, `@Needs()`, `[[Needs]] Files&` | `pick`, `read`, `release`, `upload`, `cancel` |
| Ruby, Lua | `Files.new`, the global `Files` table | `pick`, `read`, `release`, `upload`, `cancel` |
| Python | `files: Annotated[Files, needs]` (ADR 0122) | `pick`, `read`, `release`, `upload`, `cancel`; a refusal raises `RuntimeError` |

`Files` is one of the browser API types (`WebApiClients::TYPE_NAMES`), so a
native service that is linked into an app cannot use it (`TY1317`), as with
the other browser APIs.

## Consequences

- Go applications can attach, hash, read and upload files with no JavaScript.
- Uploads reach only declared hosts or the page's own origin.
- A page holds file bytes in memory until it releases them or closes.
- Hashing reads the whole file once before the chooser answers, so a very
  large file answers later.
- There is no byte-level upload progress.
- The native file choosers and drops follow each platform's web view; they
  have not yet run in a native release gate.
