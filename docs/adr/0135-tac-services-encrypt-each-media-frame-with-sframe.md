# ADR 0135: Tac Services Encrypt Each Media Frame with SFrame

- Status: Accepted (the number is the next free one at merge)
- Date: 2026-11-14
- Amends: [ADR 0129](0129-tac-services-capture-send-and-show-audio-and-video.md) (four
  more `Media` calls and three built-in events)
- Relates to: [ADR 0132](0132-tac-services-sign-agree-and-encrypt-with-keys-that-stay-on-the-device.md)
  (frame keys come from `Crypto`), the Peers ADR (P1)

## Context

Mesail's group calls go through a selective forwarding unit (SFU) that the
app's operator runs (CADUCEUS ADR 0012). WebRTC's own transport encryption
ends at the SFU, so the SFU could see and hear every call. Mesail wants the
SFU to forward frames it cannot read: each frame encrypted on the sending
device and decrypted on the receiving device, with keys from the
conversation's MLS group. RFC 9605 (SFrame) defines that frame format.

Every engine Tachyon runs WebRTC on lets a page change each encoded frame
in a worker: `RTCRtpScriptTransform` (Chromium since 2025, Firefox 117,
Safari 15.4; WKWebView on macOS 27.2 has it too). Tac services in fourteen
languages cannot run code in that worker, and Tac components hold no
script (ADR 0030).

## Decision

Tachyon ships one SFrame transform, `.tachyon/tac-media-frames.js`, and the
service only gives keys.

### Calls (part of `Media`)

| Call | Answer |
| --- | --- |
| `useFrameKey(keyId, key)` | `MediaFrameKeyReady { keyId }`. `keyId` is the SFrame key id (a whole number from 0 to 2^53-1); `key` is the name of a `Crypto` key with use `derive` (ADR 0132), made for example with `Crypto.importKey` from an MLS exporter secret. |
| `sendFrameKey(keyId)` | from the next frame, this device encrypts with that key. The key then no longer decrypts (RFC 9605: a key is for sending or for receiving, never both). |
| `forgetFrameKey(keyId)` | the worker drops the key. |
| `encryptFrames(connection, on)` | every sender of the `Peers` connection encrypts and every receiver decrypts, also for tracks that arrive later; video prefers VP8. `false` takes the transforms off. |
| (a frame with an unknown key id) | `MediaFrameKeyMissing { track, keyId }`, at most every 5 seconds per track and key; the frame is dropped |
| (a frame that fails to decrypt) | `MediaFrameRejected { track, count }`, at most every 5 seconds per track; the frame is dropped |

Failures send `MediaFailed`: `unsupported` (no `RTCRtpScriptTransform`, or
the page does not grant `crypto`), `unknown` (no such `Crypto` key or
connection), `invalid` (a bad key id), `failed` (the send key used every
counter value).

### Format

- Cipher suite `AES_128_GCM_SHA256_128` (0x0004). Key and salt come from the
  base key with HKDF-SHA-256 and RFC 9605's labels; the nonce is the salt
  XOR the counter; the header (key id and counter) is in clear and part of
  the additional data.
- The counter starts at 0 for each key and only goes up. A key whose counter
  would pass 2^64-1 sends nothing more, so a nonce is never used twice.
- The VP8 payload header stays in clear: 10 bytes of a key frame and 3 bytes
  of other frames, as Jitsi and LiveKit do, so an SFU can see key frames and
  switch simulcast layers. Those bytes are RFC 9605's metadata, inside the
  additional data, so changing one makes the frame fail. Audio is encrypted
  whole.
- The AES-GCM and HKDF work is the engine's WebCrypto in the worker. Tachyon
  writes only the header, the key schedule and the nonce, and tests them
  against all 289 header vectors and the 0x0004 vector of RFC 9605
  Appendix C.
- Without a send key, the sender drops frames: it never sends a plain frame.

## Security

- Key bytes never reach page script or the service. The page asks P2's
  `Crypto` page hook for the non-extractable `CryptoKey` and posts it to the
  worker; the worker derives a non-extractable AES-GCM key and the salt.
- What SFrame does not hide: who sends to whom, when, and how much; the size
  of each frame; the clear VP8 header bytes; and the RTP audio-level header
  extension (the SFU can see who speaks, not what is said).
- The SDP and DTLS fingerprints still travel through the app's server. With
  SFrame, a server in the middle still cannot read media.
- RFC 9605 asks that a failed frame take as long as a good one; the engine's
  WebCrypto decides that, not Tachyon.

## Consequences

- `Media.send` (slice M2) attaches the transform to the tracks it adds when
  the connection encrypts; until then, `encryptFrames` covers the
  transceivers a connection already has and every track that arrives.
- H.264 or AV1 with SFrame needs another way to keep the codec header in
  clear; until then encrypted calls use VP8.
- The worker ships with the media files (about 7 KB) to every page that
  grants a media API.

## Evidence

- `compiler::page_media_frames_tests`: every RFC 9605 header vector and the
  0x0004 vector; clear VP8 bytes; no send key, no frame; a changed clear
  byte, a cut header, an unknown key and an exhausted counter drop the frame.
- `compiler::page_media_tests`: keys go to one worker as `CryptoKey` objects,
  transforms on every transceiver, VP8 first, the 5-second report limit.
- `scripts/media-frames-browser-test.mjs` (`bun run test:media-frames-browser`):
  two pages per case over a real `RTCPeerConnection` in Chromium, Firefox and
  WebKit. The same key decodes video; the wrong key decodes nothing and
  reports rejected frames; no key reports the missing key; a receiver
  without the transform cannot decode the sent frames.

## Amendment (2026-11-21): the Apple apps hand SFrame its frame keys (P5K)

### Context

Since ADR 0132's P2b amendment, the macOS and iOS apps keep `Crypto` keys in
the Keychain and use them with CryptoKit; no key bytes enter the web view.
The page hook `key(name)` therefore has no `CryptoKey` to give the media
worker, so on these hosts `useFrameKey` and `encryptFrames(connection, true)`
answered `unsupported`. Calls between an Apple app and a browser could not
use SFrame.

### Decision

1. **The native host derives the frame key.** For a key id, the host takes
   the `Crypto` key named in `useFrameKey(keyId, key)` (use `derive`) from
   the Keychain and derives the SFrame key (16 bytes) and salt (12 bytes)
   with HKDF-SHA-256 and RFC 9605's labels, exactly as the worker does with
   a page key. The base key never leaves the Keychain.
2. **Only the media worker can open it.** For each handover the worker
   makes a new P-256 key pair inside the worker (private key
   non-extractable, never posted) and sends the page only its public key.
   The host seals the key and the salt to that public key with HPKE
   (RFC 9180, base mode, DHKEM(P-256, HKDF-SHA256), HKDF-SHA256,
   AES-256-GCM; CryptoKit's `.P256_SHA256_AES_GCM_256`). HPKE `info` is
   `"Tachyon SFrame frame key 1"` with the key id (8 bytes), and each sealed
   part carries `"key"` or `"salt"` with the key id as associated data. The
   page passes on only `{ enc, key, salt }`. The worker uses its key pair
   once and then drops it, also when opening fails; it takes the SFrame key
   with `unwrapKey` as a non-extractable AES-GCM key, so the key bytes never
   exist as a JavaScript value anywhere.
3. **The flow.** `Media.useFrameKey(keyId, key)` on a host that sets
   `__tachyonNativeFrameKeys` (macOS and iOS): the page asks the worker to
   bind (`{ kind: 'bind', keyId }`), the worker answers
   `{ kind: 'bound', keyId, publicKey }`, the page calls the Crypto hook
   `frameKey(name, keyId, publicKey)` (native call `browserApi.crypto.frameKey`)
   and posts `{ kind: 'wrapped', keyId, wrapped }`. The worker answers
   `ready` as before, or `refused`.
4. **Fail closed.** No key for that name answers `MediaFailed` `unknown`.
   Every other failure answers `unsupported`: a key of the wrong use, a bad
   worker key or key id, a refused unwrap, a worker that cannot make its key
   pair, and macOS before 14 or iOS before 17 (CryptoKit has no HPKE there).
   The worker then holds no key for that id, so the sender drops its frames:
   media is never sent without encryption.
5. **Other native hosts.** Linux, Android and Windows do not set the flag.
   Where they bridge `Crypto` (Linux), SFrame keeps P2b's answer:
   `unsupported` at once. **SFrame is unsupported on the Linux native host
   until Linux calls ship (WebKitGTK 2.56).** The Linux host answers
   `browserApi.crypto.frameKey` with `unsupported`. Browsers keep the
   page-key path unchanged.

### Security

- What this protects: raw frame keys and salts never sit in page
  JavaScript or the web view's page memory; the base secret never leaves
  the Keychain; a sealed key cannot be opened twice, by another key id, or
  by another call, because the worker's key pair is used once.
- What this does not protect against: script that already runs in the page.
  The page carries the messages between the worker and the host, so such
  script can give the host its own public key and open the frame keys. The
  native host has no way to check that a public key comes from the worker:
  a worker in WKWebView cannot reach the native message handlers, and the
  URL scheme handler cannot tell a worker's request from the page's. Such
  script also already holds the plain camera and microphone tracks; frame
  keys protect media from the SFU, not from the page.
- The ADR 0132 exception to "no key bytes in the page" stays narrow: only
  per-frame media keys, sealed to the media worker, opened there as
  non-extractable keys.

### Consequences

- The worker file grows from 7,930 to 14,359 bytes (2,211 to 3,521 bytes
  gzipped); it ships only with the media files and loads only when a page
  encrypts frames. `tac-media.js` grows by about 1.7 KB, `tac-crypto.js` by
  about 0.3 KB; `tac-web.js` and the event bus do not change. About 50 lines
  of Swift ship in every Apple app that grants `crypto`.
- Apps built for macOS 13 or iOS 15 and 16 answer `unsupported` for frame
  keys on those systems.

### Evidence

- `compiler::page_media_native_keys_tests`: the worker opens RFC 9180
  Appendix A.3 (and the test's own HPKE sender matches it); a native-sealed
  key makes the same frames as the page-key path; the key is
  non-extractable; a new key pair per handover, only the public key posted;
  a replayed package, another key id, changed bytes, a wrong worker key or
  a bad `enc` are refused, and the sender then sends nothing.
- `compiler::page_media_tests`: on a native frame-key host the page binds,
  passes the worker's key to the hook and posts only the wrapped package;
  `unknown`, `unsupported` (older system), a wrong use, a refused unwrap and
  a refused bind fail closed; a host that bridges Crypto without the flag
  (Linux) answers `unsupported` at once.
- `native::host`, `native::browser_api_bridge`, `native::apple_browser_apis`:
  only macOS and iOS set the flag; every host answers `frameKey`; the Swift
  refuses before macOS 14 and iOS 17.
- `native_crypto_host_tests` (macOS live check; iOS inside a
  `mac-heavy --vm=ios` job): see SUPPORT_TIERS, "SFrame frame keys in the
  Apple apps".
