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
is a built-in element. It shows the named video track in a `<video>` inside
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

## Security

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

## Consequences

- Native permission texts and host rules (Info.plist, entitlements, the
  Android capture row, the Windows microphone permission, the WKWebView
  media capture answer) follow in slice M1's second part, after the
  CodeScanner change lands, because they share its lines.
- Linux (WebKitGTK) uses this same contract: the page captures and the host
  decides the permission per kind (ADR 0131 amendment). Sending tracks waits
  for a WebKitGTK with WebRTC.

## Evidence

- `compiler::page_media_tests`: the page runtime with fake devices (capture,
  names, failures, mute, camera off, following the default device, screen
  sharing, grants and the user-action rule) and module shipping.
- A Playwright smoke run with fake devices in Chromium and Firefox; WebKit
  answers `denied` without a grant and hides labels.
