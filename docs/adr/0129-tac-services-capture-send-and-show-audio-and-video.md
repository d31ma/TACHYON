# ADR 0129: Tac Services Capture, Send and Show Audio and Video

- Status: Accepted (slice M1 of 6; the number is the next free one at merge)
- Date: 2026-11-14
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (two
  more browser grants, built-in events that any of three grants sends, and
  the size budget of `tac-web.js`),
  [ADR 0030](0030-components-use-services.md) (one built-in element,
  `media-view`),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
- Relates to: the Peers ADR (P1) and the CodeScanner ADR (P4, which owns the
  `camera` grant)

## Context

Mesail, a chat and mail app on Tac and Yon, adds audio and video calls of up
to 50 people, screen sharing on computers, and recording on the recorder's
own device (CADUCEUS ADR 0012, requests P5 and P7). Calls of two people go
device to device over WebRTC (`Peers`, P1); larger calls go through
Mesail's own forwarding server (an SFU).

Tac services in Tachyon's fourteen languages had no way to list cameras and
microphones, capture them, share a screen, or show a video. Tac components
hold no script (ADR 0030), so a view cannot set `video.srcObject` itself.

Every browser engine Tachyon runs on already does the hard work:
`getUserMedia`, `getDisplayMedia`, WebRTC and `MediaRecorder`. On macOS
27.2, WKWebView treats `tachyon-app://bundle` as a secure context, and
exposes `navigator.mediaDevices` once the app's Info.plist has a microphone
or camera usage text (a probe on 2026-11-14). iOS was not probed yet.

## Decision

Tachyon adds a page client named `Media`. Slice M1 (this text) covers
devices, capture and the `media-view` element. Later slices add sending on
`Peers` connections (M2), frame encryption (M3, its own ADR), calls in the
background on phones (M4) and recording (M5, its own ADR).

### Grants

- New grants `microphone` and `screen` in `tac.browserApis`. The `camera`
  grant is the CodeScanner's (P4); one grant serves both.
- `Media` loads when any of the three is granted. Each capture call needs
  its own grant: `captureMicrophone` needs `microphone`, `captureCamera`
  needs `camera`, `captureScreen` needs `screen` and a click or key
  handler.
- `tac.application.permissions.microphone` and `.screen` give the reason
  text (1 to 200 characters), like `camera` and `geolocation`.

### One contract for every service language

| Call | Answer |
| --- | --- |
| `supported(feature)` | at once: `capture`, `screen`, `send`, `simulcast`, `frames`, `speakers`, `background`; any other word answers false, and so does a page that grants no media API. It tests the real browser objects, never a platform name. |
| `devices(request)` | one `MediaDeviceFound { request, id, kind, label, chosen, index, count }` per device (`microphone`, `camera`, `speaker`); one event with `count: 0` when there is none |
| (a device is plugged in or removed) | `MediaDevicesChanged {}` to every service that listed devices |
| `chooseSpeaker(device)` | `MediaSpeakerChosen { device }` |
| `captureMicrophone(track, device, processing)` | `MediaTrackStarted { track, kind, device, label, width, height }`; `processing` turns on echo cancellation, noise suppression and automatic gain |
| `captureCamera(track, device, quality)` | the same; `quality` is `low` (360p), `medium` (720p, the default) or `high` (1080p) |
| `captureScreen(track, withAudio)` | the same; a second track `<track>.audio` when the browser gives sound |
| `stopTrack(track)` | `MediaTrackEnded { track, reason: "stopped" }` |
| `mute(track, muted)` | at once. A microphone sends silence and stays open. A camera closes (its light goes off) and opens again on unmute. |
| (a device stops by itself) | `MediaTrackEnded { track, reason }`: `removed`, `ended` (the person stopped sharing) or `failed` |
| (a failure) | `MediaFailed { track, call, reason, message }`; `reason` is `unsupported`, `denied`, `notFound`, `busy` or `failed`, and `message` is a fixed text |

Names (`track`) are 1 to 64 letters, digits, `.`, `_` or `-`; a page holds
at most 8 local tracks. A bad name, a name in use, or an unknown track fails
the call at once.

A track captured from the default device (`device: ""`) follows the system
default: when the person plugs in a headset, Tachyon captures the new
default under the same name, stops the old track, and sends
`MediaTrackStarted` again. A muted track stays muted.

### `media-view`

`<media-view track="{call.mainTrack}" fit="cover" mirror="{call.isSelf}" label="{call.name}">`
is a built-in element. It shows the named local or remote video track in a `<video>` inside
a closed shadow root, and nothing when the name is empty or unknown. `fit`
is `cover` or `contain`; `mirror="true"` flips one's own camera; `label`
becomes its accessible name. It never plays sound.

### Page side

`.tachyon/tac-media.js` and `.tachyon/tac-media-view.js` ship only when a
media grant is in the manifest, load before the runtime, and are cached by
the service worker. `tac-web.js` gains only a small `media` forwarder (714
bytes); the feature checks and the grant choice live in the media module.
Its budget (`MOST_RUNTIME_BYTES`) rises from 16,500 to 16,900 bytes for
that forwarder.

### Native apps

- **Reasons.** `tac.application.permissions.microphone` is required for iOS,
  macOS and Android builds (`TY1601`), like `camera`. iOS and macOS write it
  as `NSMicrophoneUsageDescription`; that text is also what makes WKWebView
  show `navigator.mediaDevices` at all. macOS adds the hardened-runtime
  entitlement `com.apple.security.device.audio-input`. The `screen` reason
  is optional; no system shows it yet.
- **Apple.** Both shells answer WebKit's media capture request
  (`WKUIDelegate`, `requestMediaCapturePermissionFor`) through
  `AppleMediaCapture`: it allows a request only for `tachyon-app://bundle`
  with no port, and only when every kind asked for (camera, microphone) is
  granted; anything else is denied. With a media grant the web view plays
  media without a tap, and on iOS inline.
- **Android.** The build adds `RECORD_AUDIO` and `MODIFY_AUDIO_SETTINGS`.
  The WebView host grants `RESOURCE_AUDIO_CAPTURE` only to the app's own
  pages, only with the `microphone` grant, and only after the runtime
  permission, with the reason as its rationale (one row next to the
  camera's).
- **Windows.** The WebView2 host allows the `MICROPHONE` permission only for
  the app's own page with the grant. An MSIX package declares
  `<DeviceCapability Name="microphone"/>` and `webcam` for the granted
  kinds. The host does not yet answer `ScreenCaptureStarting` (a newer
  WebView2 interface): the page's own `screen` check and WebView2's picker
  stand until it does.

### Calls on phones (slice M4)

- `tac.media.background` (default `false`) keeps call audio running when a
  phone app leaves the screen. iOS writes `UIBackgroundModes` `audio`;
  Android declares a foreground service of type `microphone` (and `camera`
  when that grant is on), with `FOREGROUND_SERVICE` permissions.
  `tac.media.callNotice` (1 to 100 characters) is the text of Android's
  ongoing call notification, required on Android when `background` is
  true (`TY1601`).
- `Media.callActive(on, title)` answers `MediaCallActive { on, background }`.
  On iOS it sets the audio session to `playAndRecord` in `voiceChat` mode
  with Bluetooth, and turns it off again. On Android it sets
  `MODE_IN_COMMUNICATION` and starts or stops the foreground service; the
  notification shows `title`, or `callNotice` when `title` is empty. On the
  web and on computers it only answers, with `background` false.
- On phones the host lists the speakers: the earpiece (iPhone only), the
  loudspeaker and connected Bluetooth or wired headsets on iOS, and the
  communication devices on Android 12 and newer (earpiece and loudspeaker
  before). `chooseSpeaker` routes call audio to one of them, and
  `supported("speakers")` answers true.
- The app decides when a call starts and ends; this is the single place where
  a later change can report calls to CallKit and ConnectionService.



- Deny by default: each capture call checks its own grant in the page.
- **Services never get audio or video samples.** Tracks are names; no call
  returns frames or sound. A track leaves the page only through a
  connection the app opens (M2) or a recording the person saves (M5).
- Screen sharing needs a click and shows the system picker; the person
  chooses what is shared.
- Device labels are shown only after the person allowed a device (a browser
  rule).
- A JavaScript or TypeScript service shares the page and could call
  `navigator.mediaDevices` itself once the person allowed it; WebAssembly
  services cannot.

### Sending and receiving on a Peers connection (slice M2)

`Media` sends its local tracks on the connections of `Peers` (ADR 0140),
named by the id that `Peers.open` answers (`peer-1`, ...). Tachyon does no
signalling: after a change, Peers sends `PeerNegotiationNeeded` and the app
makes the next offer.

| Call | Answer |
| --- | --- |
| `send(connection, track, stream, simulcast)` | `MediaSending { connection, track }`, then Peers' `PeerNegotiationNeeded`. Tracks with the same `stream` name go out in one stream. `simulcast: true` sends a camera in three layers (`q` 1/4 size up to 150 kbit/s, `h` 1/2 up to 500 kbit/s, `f` full up to 1.5 Mbit/s); a screen track is sent with the `detail` hint and no simulcast. |
| `replace(connection, track, withTrack)` | at once, with no new offer; both tracks have the same kind |
| `unsend(connection, track)` | `PeerNegotiationNeeded` again |
| (the other side sends a track) | `MediaTrackArrived { connection, track, kind, stream }`. The track is named `<connection>.<mid>`; `stream` is the sender's stream id (with an SFU, the SFU's). Remote audio plays at once on the chosen speaker; a `media-view` shows remote video by its name. A page receives at most 128 remote tracks. |
| (a remote track stops or starts getting media) | `MediaTrackMuted { track, muted }` |
| (the other side stops sending it) | `MediaTrackEnded { track, reason: "remote" }`, once negotiation is stable again; closing the connection ends its tracks with `reason: "closed"` |
| `stats(request, connection)` | one `MediaStats { request, connection, track, layer, direction, kind, bitrate, packetsLost, lossPercent, jitterMs, roundTripMs, framesPerSecond, width, height, index, count }` per sent or received track, one per simulcast layer; `bitrate` (bits per second) is the change since the last `stats` call, 0 on the first. One event with `count: 0` when nothing flows. |
| `speaking(on)` | every 250 ms Tachyon reads the audio level of each remote track (the receiver's synchronization sources) and each local microphone. A track talks after 2 samples at 0.05 or more and stops after 4 samples under 0.02: `MediaSpeaking { track, speaking }`. `MediaLoudest { track }` when another talking track has been the loudest for 2 samples. Few events, never a stream of levels. |
| (the browser blocks remote audio until a click) | `MediaAudioBlocked {}` once; `resumeAudio()` from a click plays it |

Remote events go to the service that last named the connection in a `Media`
call, otherwise to the last service that called `Media`. A service that
only receives calls `stats` or `speaking` once to receive them. Local
tracks that change (a camera turned off and on, a new default microphone)
are sent again on every connection without a new offer; a camera that is
off sends no frames. With `encryptFrames` on, every track that `send` adds
or that arrives is encrypted (ADR 0135).

## Consequences

- iOS is not probed yet; a capture on a phone, a Mac with a microphone, an
  Android device and a Windows PC is still owed (`target_support.json`).
- Linux (WebKitGTK) uses this same contract: the page captures and the host
  decides the permission per kind (ADR 0131 amendment). Sending tracks waits
  for a WebKitGTK with WebRTC.

## Evidence

- `compiler::page_media_tests::the_page_runtime_sends_receives_measures_and_hears_tracks_on_a_peer_connection`:
  send, simulcast layers, shared streams, replace, unsend, camera off and
  on, remote tracks and their players, mute, blocked audio and
  `resumeAudio`, the speaker for remote audio, stats rows and bitrate, a
  remote track leaving after negotiation, talk detection with its delays,
  and a closed connection, against fake WebRTC objects.
- `scripts/media-calls-browser-test.mjs` (`bun run test:media-calls-browser`):
  two pages on real `Peers` connections in Chromium, Firefox and WebKit, and
  one connection with 50 audio and 9 video tracks (`SUPPORT_TIERS.md`,
  "Calls between two pages and 59 tracks on one connection").
- `compiler::page_media_tests`: the page runtime with fake devices (capture,
  names, failures, mute, camera off, following the default device, screen
  sharing, grants and the user-action rule) and module shipping.
- A Playwright smoke run with fake devices in Chromium and Firefox; WebKit
  answers `denied` without a grant and hides labels.
