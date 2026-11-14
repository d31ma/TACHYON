# ADR 0131: The Linux App Reaches Devices, Screens, Files and Keys Through WebKitGTK, the Portals and the Secret Service

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0129](0129-tac-services-capture-send-and-show-audio-and-video.md)
  and [ADR 0130](0130-tac-services-scan-a-qr-code-through-a-camera-view-that-tachyon-draws.md)
  (their Linux rows), [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the Linux reach rows), [ADR 0104](0104-release-packages-are-opt-in-and-signed-from-the-environment.md)
  (Linux package dependencies)

## Context

Mesail, a chat app on Tac and Yon, needs its Linux desktop app before its
first release (CADUCEUS ADR 0011 section 10 and ADR 0012, request P8). The
Linux app must support the same page clients as the other platforms:
`Peers`, `Crypto`, `CodeScanner`, `Media` and `Recorder`, with the same API.

The Linux app is a GTK 4 window with a WebKitGTK 6.0 web view on the
`tachyon://app` origin, which the host registers as secure. On 2026-11-14 a
probe ran a page on that origin in six distribution images in the Linux
lane: Debian 13 and testing, Ubuntu 24.04 and 26.04, Fedora 43 and 45, with
WebKitGTK 2.52.6 to 2.54.2. It found:

- no `RTCPeerConnection` in any of them, even with the `enable-webrtc`
  setting on. WebKitGTK 2.54 removed its experimental GStreamer WebRTC, and
  the WebKitGTK project plans WebRTC on Google's libwebrtc, on by default,
  in 2.56 (spring 2027);
- everything else the clients need: a secure context, WebCrypto with
  X25519 and Ed25519, `getUserMedia` (with a permission request the host
  decides), `getDisplayMedia`, `MediaRecorder` (MP4 or WebM by system),
  WebCodecs, the Camera and FileChooser portals, and the Secret Service;
- no `setSinkId`, no `showSaveFilePicker`, no private file system and no
  `BarcodeDetector`;
- that WebKitGTK writes a saved `CryptoKey` to IndexedDB without
  encryption (its key wrapping is not implemented), although page script
  cannot export it.

Before this decision, the Linux host turned on no media setting and denied
every media permission request, while WebKitGTK's own default left
`navigator.mediaDevices` on. A page could list a camera but never open it.

## Decision

1. **WebRTC comes from WebKitGTK.** The Linux shell carries no WebRTC
   engine of its own. When a system's WebKitGTK has WebRTC (2.56 or newer,
   built with it), `Peers` and the call parts of `Media` work there through
   the same page modules as on the web; until then their `supported()`
   checks answer false, and the app shows its "not available on this device"
   state. The shell checks for a working `RTCPeerConnection`, never for a
   version number.
2. **Media capture is on only with a media grant.** The host sets
   WebKitGTK's `enable-media-stream` to true when the manifest grants
   `camera`, `microphone` or `screen`, and to false otherwise, so a page
   without a media grant has no `navigator.mediaDevices` at all.
3. **One permission decision per capture kind.** The host answers
   WebKitGTK's `permission-request` for user media only on the app's own
   page, and only when every kind the request asks for is granted: an audio
   device needs `microphone`, a video device `camera`, a display `screen`.
   The device list permission (labels) needs any media grant. Everything else
   is denied, as before.
4. **Devices through what WebKitGTK can open.** WebKitGTK uses the Camera
   portal and PipeWire where they run, and opens devices directly where they
   do not. Tachyon allows both. `supported()` answers true only when a device
   is really listed, so a system with no camera answers false.
   Screen sharing needs a ScreenCast portal (GNOME, KDE Plasma and
   wlroots desktops have one). When the app grants `screen`, the host asks
   the session bus once at start, for at most 2 seconds, whether that portal
   answers. When it does not, or when `screen` is not granted, the host
   removes `getDisplayMedia` from the page, so `supported("screen")` answers
   false and the app shows its "not available" state instead of a failed
   picker.
5. **The QR scanner runs in the page**, as on Android and Windows: the same
   camera view and decoder as the web. WebKitGTK has no `BarcodeDetector`,
   so the shipped decoder always reads the code.
6. **Keys stay native.** `Crypto` on Linux runs in the host, like Apple's:
   OpenSSL 3 does the work, and the Secret Service (GNOME Keyring or KDE
   Wallet, through libsecret) keeps the saved keys. Key bytes never cross
   the bridge. For SFrame, the host derives each epoch's frame key and hands
   only that key to the page's frame worker.
7. **The recorder writes through the host.** The page records with
   WebKitGTK's `MediaRecorder` and sends each piece to the host, which writes
   it to the file the person picked in a `GtkFileDialog` (the FileChooser
   portal where one runs).
8. **Supported systems:** Ubuntu 24.04 and 26.04, Debian 13, and the two
   current Fedora releases, on GNOME and KDE Plasma. Other systems should
   work; they are not tested.
9. **Packages** keep `Depends` small: libsecret and OpenSSL for `Crypto`.
   The portal, PipeWire and the GStreamer plugins are `Recommends`, so apt
   installs them by default and the app still installs without them.

This change builds decisions 2 to 5. `Crypto`, the recorder sink, the
WebRTC check and the package changes land with their page clients.

## Consequences

- On a Linux PC, an app can scan a QR code and capture the camera and the
  microphone (and the screen, on a desktop with the ScreenCast portal), each
  with its grant. An app without a media grant cannot open a camera,
  microphone or screen in its page.
- Peer connections and calls on Linux wait for WebKitGTK 2.56 on each
  system. If a distribution builds WebKitGTK without WebRTC, those features
  stay unavailable there.
- The Secret Service has no per-app access control on Linux: other programs
  of the same user can read its items. Same-user malware is out of scope on
  every platform.
- The Linux lane runs in containers with WebKit's mock capture devices; a
  real camera, the desktop portals' dialogs and KDE Wallet are checked by
  hand on a physical Linux PC.
