# ADR NNNN: Tac Services Record Calls to a File on the Device

- Status: Accepted (the number is the next free one at merge)
- Date: 2026-11-21
- Amends: [ADR 0134](0134-tac-services-keep-blobs.md) (one more module behind
  the module forwarder), [ADR 0129](0129-tac-services-capture-send-and-show-audio-and-video.md)
  (`Recorder` records the tracks that `Media` holds)
- Relates to: Mesail ADR 0012 (P7, recording on the device)

## Context

Mesail lets a person record a call on their own device: the sound of
everyone, and one video that the app switches to the person speaking or to
a shared screen. The recording must be written to a file the person picks
while it runs, so a long call does not fill memory, and it must never leave
the device by itself.

Every engine Tachyon runs in has `MediaRecorder`. Native encoders
(AVAssetWriter, MediaMuxer, Media Foundation) cannot reach the engine's
tracks without copying every frame out of the web view.

## Decision

### Calls

| Call | Answer |
| --- | --- |
| `supported()` | at once: true when the page can record and save here |
| `start(recording, audioTracks, videoTrack, fileName, format)` | from a click or key handler. Opens the save dialog with `fileName`, then `RecordingStarted { recording, format, fileName }`. `audioTracks` is `Media` track names separated by spaces; `videoTrack` may be `""` (audio only); `format` is `mp4`, `webm` or `""`. |
| `addAudio(recording, track)`, `removeAudio(recording, track)` | at once; for people who join or leave |
| `showVideo(recording, track)` | at once; the next frames show that video; `""` shows black |
| `pause(recording)`, `resume(recording)` | `RecordingPaused`, `RecordingResumed` |
| `stop(recording)` | `RecordingStopped { recording, bytes, seconds }` after the last bytes are on disk |
| (while it runs) | `RecordingProgress { recording, bytes, seconds }` every 5 seconds |
| (a failure) | `RecordingFailed { recording, call, reason, message }`: `unsupported`, `cancelled` (the save dialog was closed), `denied`, `full`, `invalid`, `unknown` or `failed`; what was written stays in the file |

A page makes one recording at a time. A track that ends leaves the mix and
the recording goes on; an ended video shows black. `Recorder` needs the
`files` grant and a media grant (`microphone`, `camera` or `screen`).

### How it works

- The recorder runs in the page with the engine's `MediaRecorder`. One
  `AudioContext` mixes the audio tracks into one track. The video is a
  1280×720 canvas at 30 frames a second that draws the chosen track
  letterboxed, so `MediaRecorder` always sees the same video track.
- The format is always one that `MediaRecorder.isTypeSupported` accepts:
  the app's `format` first, then MP4 (H.264, AAC), then WebM (VP8, Opus).
  Without one, `start` answers `unsupported`.
- `MediaRecorder` gives a piece every second, and each piece is written to
  the file at its place, so memory does not grow with the length.
- Where the file goes, in this order: the native host (see below; macOS
  since slice M5b1, other native shells use the web sinks until M5b2 and
  L4), the save picker
  (`showSaveFilePicker`, Chromium and WebView2), or a file in the page's
  private file system that the browser downloads at stop (Firefox, Safari).
  Without one of them, `start` answers `unsupported`.
- WebM from `MediaRecorder` has no usable duration in its header, so
  players cannot seek. Tachyon puts an 8-byte `Duration` element into the
  first piece's `Info` element (or reuses the one Firefox writes) and writes
  the real length there at stop.
- **Tachyon never uploads a recording.** `Recorder` makes no network call
  and its file is not a `Files` handle, so `Files.upload` cannot reach it.
  The notice to the other people in the call is the app's.
- The code ships in `.tachyon/tac-recorder.js` only to pages that grant
  `files` and a media grant. `tac-web.js` gains one module entry and
  `start` in its user-action list (54 bytes).

### The native host sink (slice M5b1)

A host that lists `recorder` in `__tachyonBrowserApiBridge` saves the file
itself. The page sends three calls through the normal host call, and each
call stays under the host call's 64 KiB limit:

| Call | Values | Answer |
| --- | --- | --- |
| `browserApi.recorder.open` | `recording`, `fileName`, `format` (`mp4` or `webm`) | the system save dialog with `fileName`; `{ fileName }` with the name the person chose, or an error that ends in `(cancelled)`, `(invalid)` or `(denied)` |
| `browserApi.recorder.write` | `recording`, `position`, `data` (base64, 1 to 49,152 bytes) | the bytes are written at `position`; `(full)` when the disk is full, `(unknown)` for a recording that is not open |
| `browserApi.recorder.close` | `recording` | the file is flushed and closed |

- The page splits each piece into chunks of 45 KiB, in order, so the file
  is written while the call runs and the WebM duration can be written at
  its place at stop.
- The host answers only the app's own page, only with the `files` grant
  and a media grant, and writes only the file the person chose in the
  dialog. It checks the recording name, the file name and the format again.
- macOS (M5b1): `NSSavePanel` as a sheet on the app window, then a
  `FileHandle` on the chosen file. A page that reloads and opens the same
  recording name again replaces its old handle.

## Consequences

- The macOS save dialog came in slice M5b1. The iOS app folder and the
  export picker, Android `ACTION_CREATE_DOCUMENT` and Windows come in slice
  M5b2, and the Linux host sink in slice L4. Until then the recorder is a run
  still owed on those targets in `target_support.json`: a web view
  with a save picker or a writable private file system records with the web
  sinks, and one without them answers `unsupported`. On Linux, WebKitGTK
  can record captured tracks; call tracks wait for its WebRTC.
- A private window may refuse the private file system; `start` then fails
  and nothing is saved.
- MP4 files from `MediaRecorder` are fragmented; players play them, and
  their length shows once the file is read to the end.

## Evidence

- `compiler::page_recorder_tests`: the module against fake objects: the
  format order, refused names and file names, one recording at a time, the
  audio mix and its changes, the video switch, pieces written at their
  places, progress, pause and resume, a track that ends, the WebM duration
  inserted and kept, a full disk, a closed save dialog, the private file and
  its download, and no place to save.
- `native::target_support_tests`: `tac.recorder` is honored on the web and
  a run still owed on the five native targets.
- `compiler::page_media_tests::only_a_build_that_grants_a_media_api_ships_the_media_modules`:
  the module ships only with `files` and a media grant.
- `scripts/recorder-browser-test.mjs` (`bun run test:recorder-browser`):
  about 5 seconds with three microphones mixed and a video switch, in
  Chromium (MP4 through the save picker, replaced by a test file handle),
  Firefox (WebM in the private file, downloaded, with its duration) and
  WebKit (MP4 in the private file, downloaded; a persistent profile, because
  Playwright's private sessions refuse the private file system).
- `tac_web::recorder_tests` and `tac_python::browser_api_tests`: a service in
  every language starts, switches, pauses and stops a recording.
