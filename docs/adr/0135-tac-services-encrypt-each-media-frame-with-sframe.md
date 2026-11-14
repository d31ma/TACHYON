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
