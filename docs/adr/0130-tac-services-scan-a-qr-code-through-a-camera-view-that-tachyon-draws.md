# ADR 0130: Tac Services Scan a QR Code Through a Camera View That Tachyon Draws

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (the browser
  APIs that need a grant, the built-in events, and the size budget of
  `tac-web.js`)

## Context

Mesail, a chat app on Tac and Yon, links a new device to an account by
showing a QR code on one device and scanning it with the camera of the other
(CADUCEUS ADR 0011, request P4). Tac services had no way to use the camera.
Drawing the QR code needs no Tachyon change: Mesail draws an SVG `data:`
image, which the page's content security policy already allows.

A camera is sensitive. The browser asks the person before a page may use it,
and a page that holds a camera stream can read every frame. Mesail needs only
the text of one QR code, never the frames.

Browsers differ. `getUserMedia` needs a secure page. `BarcodeDetector` reads
QR codes in Chrome on macOS, Android and ChromeOS, but not in Firefox, Safari,
or Chrome and Edge on Windows and Linux, so a decoder must ship with
Tachyon. The native shells load their pages from their own origins.
`https://appassets.tachyon.local` (Android) and `https://tachyon.local`
(Windows) are secure pages, and their web views can open the camera when the
host allows it, as they already do for geolocation. On macOS 27.2.1 a probe
found that WebKit treats `tachyon-app://bundle` (macOS and iOS) as a secure
page too, and shows `navigator.mediaDevices` only when the app has a camera or
microphone usage text; iOS was not checked. A WKWebView camera also needs the
host to answer WebKit's media capture permission request, which the shells do
not do yet. Mesail asked for the platform's own scanner on Apple. The Linux
web view (`tachyon://app`) had no camera access then (ADR 0131 adds it).

## Decision

1. `camera` is a new browser API grant in `tac.browserApis`. It takes a
   reason in `tac.application.permissions.camera` (1 to 200 characters of
   plain text), like `geolocation`. iOS, macOS and Android builds that grant
   `camera` without a reason stop with `TY1601`.
2. A new page client, `CodeScanner` (field `codeScanner`), exists in every
   Tac service language with three calls:
   - `supported()` answers at once and never asks the person. It is false
     where no camera can open: no camera API, not a secure page, no
     `videoinput` device, or (on Apple) no video device.
   - `scan()` must run from a click or key handler. It shows a camera view
     that Tachyon draws, looks for one QR code, then stops the camera,
     closes the view and answers with a built-in event.
   - `cancel()` closes an open scan; with none open it does nothing.
3. Two new built-in events, sent only to the caller of `scan()`:
   - `CodeScanned { text }`: the text of the code. It is untrusted input.
   - `ScanFailed { reason }`: `cancelled` (Cancel, Escape, `cancel()` or the
     page went to the background), `denied` (the person or the system refused
     the camera), `unsupported` (no camera or camera API) or `failed`
     (anything else). The browser's own error text never reaches the app.
   A second `scan()` while one is open, or a page that did not load the
   scanner module, sends `BrowserCallFailed`, as for any programming error.
4. On the web, Android and Windows the view is a `<dialog>` in a closed
   shadow root, opened with `showModal()`. It shows the live picture, a
   square where to hold the code, the reason when the app gives one, and a
   Cancel button that has the focus. The frames go only to a canvas inside
   the scanner; the service gets only the text. Codes that hold no text are
   skipped and the scan goes on.
5. The scanner reads frames every 150 ms at most 640 pixels wide. It uses
   `BarcodeDetector` when it reads `qr_code`, and otherwise jsQR 1.4.0
   (Apache-2.0), vendored unchanged with a SHA-256 pin and shipped with its
   license notice.
6. The scanner lives in `.tachyon/tac-code-scanner.js` (about 3 KB Brotli)
   and the decoder in `.tachyon/tac-qr-decoder.js` (about 33 KB Brotli), next
   to it. Both ship only to apps that grant `camera`, the service worker keeps
   them for offline pages, and the decoder loads only on the first scan where
   the browser cannot read QR codes itself. The route budget (`TY1337`)
   counts both on every route of a granting app.
7. Native shells:
   - macOS and iOS answer `codeScanner` in the host
     (`browserApi.codeScanner.scan` and `.cancel`) with AVFoundation and the
     Vision framework, in a native sheet (macOS) or a full-screen view (iOS)
     with the reason and a Cancel button. The reason becomes
     `NSCameraUsageDescription`, and macOS adds the hardened-runtime
     entitlement `com.apple.security.device.camera`. The host publishes
     whether a video device exists, so `supported()` answers at once. One
     code path serves both systems; VisionKit's `DataScannerViewController`
     is iOS-only, so it is not used. The native path does not depend on
     WebKit's media capture in the web view, so it works whether or not the
     page itself may open the camera. A later change may move Apple to the
     web view path once the shells answer WebKit's media capture request
     and iOS is checked; the service API and events stay the same.
   - Android grants `RESOURCE_VIDEO_CAPTURE` in `onPermissionRequest` only to
     the app's own pages, only when `camera` is granted, and only after the
     `CAMERA` runtime permission, whose rationale is the reason. The manifest
     gets `android.permission.CAMERA` and an optional
     `android.hardware.camera.any` feature. Every other web view permission
     is denied.
   - Windows allows `COREWEBVIEW2_PERMISSION_KIND_CAMERA` only for the app's
     own pages when `camera` is granted. The microphone stays denied.
   - Linux runs the same page path as Android and Windows: WebKitGTK's
     `getUserMedia`, allowed by the host only for the app's own pages when
     `camera` is granted (ADR 0131 amendment).
8. `tac-web.js` grows by about 640 bytes, so `MOST_RUNTIME_BYTES` rises from
   16,900 to 17,600. The module lookup that Passkeys used is shared with the
   scanner to keep the growth small.
9. The `camera` grant and its reason are shared with Media
   ([ADR 0129](0129-tac-services-capture-send-and-show-audio-and-video.md)):
   `Media.captureCamera` needs the same grant, and an app gives one camera
   reason for both.

## Consequences

- Services in all fourteen languages can link a device by QR code without
  touching the camera. The same events arrive on every target.
- Apps that grant `camera` load about 36 KB more (Brotli) on each route.
  Apps that do not grant it load nothing more than the 640 bytes in
  `tac-web.js`.
- A JavaScript or TypeScript service runs on the same page, so after the
  person allows the camera it could call `navigator.mediaDevices` itself.
  That is true of every browser API. The promise of this decision is that the
  `CodeScanner` client never hands frames out. Services in WebAssembly
  languages cannot reach `navigator` at all.
- The view's Cancel label and the default label "Scan a QR code" are English.
  A later change can add translations.
- Out of scope: other code types, several codes at once, continuous
  scanning, a torch, choosing a camera, and scanning an image file.
- Evidence still owed by hand: a real QR read on an iPhone, a Mac with a
  camera, an Android phone and a Windows PC with a camera. The test machines
  and the iOS simulator have no camera, and `windows-vm` has none either.
