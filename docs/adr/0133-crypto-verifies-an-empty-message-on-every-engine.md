# ADR 0133: Crypto Verifies an Empty Message on Every Engine

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0132](0132-tac-services-sign-agree-and-encrypt-with-keys-that-stay-on-the-device.md)
  (`sign` and `verify` refused empty data)

## Context

ADR 0132 made `Crypto.sign` and `Crypto.verify` refuse an empty message,
because WebKit could not verify one. A closer look showed:

- **WebKit's verify answers `false` for every empty message, on purpose.**
  Its Apple bridge to CryptoKit (`CryptoKitShim.swift`) checks
  `if signature.isEmpty || data.isEmpty { return false }` before it asks
  CryptoKit. This is in Safari, in WKWebView on macOS and iOS, and in
  Playwright's WebKit. Checked on 2026-11-14 with RFC 8032 test 1 in
  Playwright's WebKit (Safari 26.5 engine) and in the system WebKit of
  macOS 27.2.1 (Safari 27.2). Every way of passing empty data fails the same
  way (an empty `Uint8Array`, `ArrayBuffer`, `DataView`, or an empty view
  into a larger buffer).
- **WebKit's sign over an empty message is correct.** OpenSSL (through
  Node) accepts the signatures that both WebKit builds made. So signing
  needs nothing.
- Chromium and Firefox sign and verify empty messages correctly.
- WebKitGTK uses libgcrypt and has no such check; it is not tested here.

Verifying uses only public inputs: the public key, the signature and the
message. So a second verifier in the page does not change how keys are
kept, and no key bytes reach page script.

## Decision

1. `sign` and `verify` accept empty data again, on every platform, as the
   approved plan had it. This only widens what the calls accept, so no
   caller breaks.
2. When the message is empty and WebCrypto answers `false`, the Crypto page
   module asks a second, strict verifier. It never asks it for a message
   with bytes, never when WebCrypto answers `true`, and it does not look at
   the browser's name. On Chromium and Firefox the second verifier runs only
   for an empty message whose signature is really wrong, and it agrees.
3. The second verifier is `@noble/ed25519` 3.2.0 (MIT, no dependencies),
   shipped unchanged as `.tachyon/tac-ed25519.js` with a license header.
   The source is the npm registry tarball
   `https://registry.npmjs.org/@noble/ed25519/-/ed25519-3.2.0.tgz`
   (35,047 bytes, integrity
   `sha512-criDgRlnUA09hchYrTy/JUWPIEap5rZxQe6wDWzRx51oWWpDRcUpuNzlgPxDJaOK6AsW9c0wKcj3rKRv6t+bPQ==`,
   checked on download); the shipped file is its `index.js` (39,841 bytes).
   Its SHA-256 is pinned in a test
   (`a4f631d1876b5d9e43a86cd71194ca27b5e1451edf4ba4db9a418f40a475569a`),
   its license is in `crates/tachyon-core/src/template/vendor/` and in
   `NOTICE`. The page loads it with `import()` the first time it is needed,
   once per page. It ships to pages that grant `crypto`, on the web and in
   native builds, and the service worker caches it.
4. **Strict checks, as WebCrypto requires:** `verifyAsync` with
   `{ zip215: false }` (RFC 8032 and FIPS 186-5: canonical encodings,
   `S` below the group order, a small-order public key refused), and the
   page refuses a small-order `R` and a signature that is not 64 bytes
   before it asks the library. A load failure answers `CryptoFailed` with
   reason `failed`, and the next call tries again.
5. The size of `tac-web.js` does not change. `tac-crypto.js` grows by about
   1,000 bytes. The route budget counts `.tachyon/tac-ed25519.js` (39,841
   bytes, about 11 KB compressed) for pages that grant `crypto`, because it
   follows every file a page script names, even one loaded only when
   needed.
6. A native host that answers Crypto itself (ADR 0132, decision 12) must
   accept empty data for `sign` and `verify`.

## Consequences

- One small third-party script is in Tachyon's page code. Cure53 audited
  `@noble/ed25519` version 1 (February 2022); versions 2 and later are not
  audited on their own and are tested against `@noble/curves`, which Cure53
  audited in 2024. The newest line was chosen because it is maintained.
  It only sees public data.
- Upgrading it means a new pinned hash, the same tests, and a note here.
- When WebKit changes its check, nothing in Tachyon needs to change: the
  second verifier then never loads.

## Evidence

- `compiler::page_crypto_tests::the_page_runtime_checks_an_empty_message_with_the_strict_verifier_only_when_webcrypto_says_no`:
  with a WebCrypto that answers `false` for empty data, RFC 8032 test 1
  verifies; a changed signature, another key, a small-order `R` and a short
  signature do not; the verifier is not loaded when WebCrypto says `true` or
  for a message with bytes, and is loaded once otherwise; an empty message
  signs and verifies.
- `compiler::page_crypto_tests::the_shipped_verifier_keeps_the_pinned_noble_bytes_and_its_license`
  and the build test that ships `.tachyon/tac-ed25519.js` only with the
  grant and never as a script tag.
- `bun run test:crypto-browser`: RFC 8032 test 1, a changed signature and
  another key on an empty message, and an empty message signed and verified,
  in Chromium, Firefox and WebKit.
