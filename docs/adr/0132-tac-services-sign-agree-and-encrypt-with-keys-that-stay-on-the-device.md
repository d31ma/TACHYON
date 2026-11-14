# ADR 0132: Tac Services Sign, Agree and Encrypt with Keys That Stay on the Device

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (the browser
  APIs that need a grant, the built-in events, and the size budget of
  `tac-web.js`) and [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md)
  (a new IndexedDB database, `tachyon-keys`, beside the stores)

## Context

Mesail, a chat and mail app on Tac and Yon, encrypts messages end to end
(CADUCEUS ADR 0011, request P2). Each device needs an identity key that
signs, keys that agree on a shared secret with another device, and a backup
key made from a recovery code. The server and its operators must never see
these keys. Tac services had only `SecureRandom`; a service that wanted
crypto had to bring its own code and hold raw key bytes in WebAssembly
memory, where any bug, log line or dependency could copy them away.

Every platform Tachyon targets already has audited crypto in the page:
WebCrypto (`crypto.subtle`: BoringSSL in Chromium, NSS in Firefox, Apple's
CoreCrypto in Safari and WKWebView, libgcrypt in WebKitGTK). WebCrypto has
Ed25519 and X25519 (Chromium 133 and 137, Firefox 130, Safari 17 and 18),
AES-GCM, HKDF, PBKDF2 and SHA-256. A WebCrypto key can be made
non-extractable: page script can use it but never read its bytes, even
after it is stored in IndexedDB and read back. WebCrypto works only in a
secure page (https, or localhost during development).

Service code in Tachyon talks to the page through a synchronous bridge with
JSON text; slow answers come back as built-in events of at most 65,536 bytes
(ADR 0065). The Peers client (built at the same time, for
peer-to-peer data channels) sends bytes as base64url text without padding,
at most 32,768 bytes per message.

## Decision

1. `crypto` is a new browser API grant in `tac.browserApis`. Every Tac
   service language gets a `Crypto` client (JavaScript and TypeScript
   `@Needs crypto` with the type `TacCrypto`, Rust `crypto: Crypto`, Go
   `crypto Crypto` with `// @Needs`, C# and F# `new()` and `Crypto()`,
   Kotlin, Java, Dart, C++ `Crypto`, Swift `Crypto()`, Ruby `Crypto.new`,
   the Lua table `Crypto`, Python `crypto: Annotated[Crypto, needs]`).
   Natively linked services cannot use it (`TY1317`), like every page
   client.
2. **Keys are names.** The service names each key (1 to 64 letters, digits,
   dots, dashes or underscores). No call returns a private key, a secret
   key or a shared secret. A key may be imported from 32 bytes the service
   already has (`importKey`), and only public keys are ever exported.
3. **The calls.** Every call except `supported` returns a request id at
   once (`crypto-1`, `crypto-2`, ...); the answer event carries it as
   `request`, so many calls can run at once. Bytes are base64url text
   without padding, at most 32,768 bytes per argument (`sealed` may be 29
   bytes longer), the same encoding as Peers, so a service can pass
   `Crypto` output to `Peers.send` unchanged.

   | Call | Answer event |
   | --- | --- |
   | `supported()` (no grant) | at once: true in a secure page with WebCrypto, or when the native app answers Crypto |
   | `check()` | `CryptoSupport { request, x25519, ed25519, aesGcm, hkdf, pbkdf2, keyStore }`, by a real probe of each algorithm; `keyStore` is `browser`, `os` or `none` |
   | `makeKeyPair(key, algorithm, keep)` | `CryptoKeyMade { request, key, publicKey }`; `x25519` or `ed25519`; `publicKey` is the raw 32 bytes |
   | `publicKey(key)` | `CryptoKeyMade` |
   | `sign(key, data)` | `CryptoSigned { request, signature }` (Ed25519); `data` may be empty (ADR 0133) |
   | `verify(publicKey, data, signature)` | `CryptoVerified { request, valid }`; `data` may be empty (ADR 0133) |
   | `agree(key, peerPublicKey, salt, info, newKey, use, keep)` | X25519, then HKDF-SHA-256 straight into a new key: `CryptoKeyMade` with `publicKey` `""` |
   | `deriveKey(fromKey, salt, info, newKey, use, keep)` | HKDF-SHA-256 from a `derive` key into a new key: `CryptoKeyMade` |
   | `importKey(newKey, secret, use, keep)` | `CryptoKeyMade`; `secret` is 32 bytes |
   | `fromCode(newKey, code, salt, use, keep)` | PBKDF2-SHA-256, 600,000 rounds, into a new key: `CryptoKeyMade`; the code is 1 to 1,024 bytes of text, the salt at least 16 bytes |
   | `encrypt(key, data, aad)` | `CryptoEncrypted { request, sealed }` |
   | `decrypt(key, sealed, aad)` | `CryptoDecrypted { request, data }` |
   | `hkdf(input, salt, info, length)` | `CryptoDerived { request, bytes }`: HKDF-SHA-256 over bytes the service already has, 1 to 1,024 bytes |
   | `hash(data)` | `CryptoHashed { request, digest }` (SHA-256) |
   | `forget(key)`, `forgetAll()` | `CryptoForgotten { request }`; forgetting a missing key is not an error |
   | (any later failure) | `CryptoFailed { request, call, reason, message }` |

   Secure random bytes stay `SecureRandom.bytes(n)`; there is no second
   call for them.
4. **Each secret key has one use**: `encrypt` (AES-256-GCM, usages
   `encrypt` and `decrypt`) or `derive` (HKDF-SHA-256, usages `deriveKey`
   and `deriveBits`). A key used for the wrong call answers `invalid`.
5. **AES-GCM nonces are made inside.** Each `encrypt` takes a new random
   96-bit nonce from `crypto.getRandomValues`; the service cannot pass one,
   so it cannot reuse one. `sealed` is `version (1 byte, 1) || nonce (12)
   || ciphertext || tag (16)`. A wrong key, wrong AAD, changed bytes,
   another version or fewer than 29 bytes answer `rejected`. A larger
   backup is encrypted in chunks by the app, each chunk its own call, with
   the chunk number in the AAD.
6. **An all-zero X25519 shared secret is refused** (`invalid`), whatever
   the browser does with small-order points, so every platform behaves the
   same.
7. **Kept or not.** `keep: true` stores the key on the device: a
   non-extractable `CryptoKey` in the IndexedDB database `tachyon-keys`
   (store `keys`, one record per name) of the page's origin, shared by the
   origin's page apps. `keep: false` holds it in the page's memory until the
   page closes. A name is used once across both; `exists` asks the app to
   forget the old key first. Without IndexedDB, `keep: true` answers
   `unsupported` and `check` reports `keyStore: none`.
8. **Failures.** Arguments the page can check at once (base64url, sizes,
   names, algorithm, use, `keep`, the HKDF length, the code and salt sizes)
   are the call's own error, and no event follows. Later failures are
   `CryptoFailed` with one reason word: `unsupported` (no WebCrypto here, or
   the algorithm or the key store is missing), `invalid` (a key of the
   wrong kind, a bad public key, a small-order point), `unknown` (no key
   with that name), `exists`, `rejected`, `locked` (the key store is locked;
   kept for the device lock that comes later, never sent today) and
   `failed`. `message` is a fixed text for the reason, never the browser's
   text.
9. **Calls run one at a time** in the order they were made, per page, so
   two calls on the same key name cannot race. `CryptoFailed` for one call
   does not stop the next.
10. **The code is a page module.** `tac-web.js` gets a small `crypto` entry
    that checks the grant and forwards each call. `.tachyon/tac-crypto.js`
    (about 15 KB before compression) holds the checks, the key store and
    the WebCrypto calls; it ships only to pages whose manifest grants
    `crypto`, on the web and in native builds, and the service worker caches
    it. A build whose `tac.origin` is plain `http` (not localhost) gets a
    note that Crypto needs a secure page.
11. **Other page modules** may use a secret key by name through
    `globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key(name)`, which
    answers a promise of the non-extractable `CryptoKey` of a `encrypt` or
    `derive` key, or `null`. It never gives a key pair's private key. The
    media session's frame encryption (SFrame) uses this to derive its frame
    keys in a worker without key bytes reaching page script.
12. **A native app may answer the calls itself.** When the host lists
    `crypto` in `__tachyonBrowserApiBridge`, the page module still checks the
    arguments and makes the request id, then sends
    `browserApi.crypto.<call>` with the JSON array of the arguments in the
    order above. The host answers `{ "value": { ...the event's fields } }`,
    or `{ "value": { "reason": "<reason>" } }` for `CryptoFailed`; an
    `error`, a missing value or an unknown reason becomes `failed`. The page
    sets `request` itself, so a host cannot answer for another call. No
    host answers Crypto yet; native CryptoKit and the Keychain on Apple,
    and OpenSSL with the Secret Service on Linux, come in later decisions
    and use this path without changing the service API.
13. The size budget of `tac-web.js` (`MOST_RUNTIME_BYTES`) rises from 17,600
    to 18,200 bytes. The `crypto` entry adds about 630 bytes: the list of fifteen
    calls and the `supported` check. The rest of the code is in the module,
    which pages without the grant never load.

## Security

What Crypto protects against:
- **The server and its operators.** They never receive private keys,
  secret keys or shared secrets: no call returns them.
- **Service code and its logs.** No answer carries private or secret bytes,
  so a service bug cannot log or send them.
- **Script injected into the page, or a bad dependency.** While the page is
  open it can **use** the keys (sign, decrypt). It cannot **copy** them: a
  non-extractable `CryptoKey` cannot be exported, also after it is read back
  from IndexedDB.
- **Other origins and other apps.** IndexedDB is per origin; the native
  apps' web view data is per app.
- **Nonce reuse.** The service never chooses a nonce.

What Crypto does not protect against:
- **A copy of the disk** of a Chromium, Firefox, WebView2 or WebKitGTK
  profile taken while the disk is not encrypted: these engines store
  `CryptoKey` bytes in IndexedDB without their own encryption (WebKitGTK:
  checked in the Linux probe of 2026-11-14). Safari wraps them with a key
  from the Keychain. Full-disk encryption (FileVault, BitLocker, LUKS)
  closes this. The approved plan chose this trade over wrapping each key
  under an OS-held key, because page script could unwrap such a key and
  copy it.
- Malware that runs as the same user, or as root, and a jailbroken device.
- A person or script that uses the open, unlocked page.
- Side channels in the platform's crypto (it is the platform's code).
- A plain `http` page: it has no WebCrypto, so every call answers
  `unsupported`.

Tachyon writes no crypto of its own and adds no dependency: it calls the
platform's WebCrypto. Where a platform lacks an algorithm, Crypto answers
`unsupported`; Tachyon does not ship a WebAssembly copy, because key bytes
in WebAssembly memory could be read by page script.

A key derived with `agree`, `deriveKey` or `fromCode` passes through page
memory once as 32 bytes between `deriveBits` and the non-extractable
import, because WebCrypto cannot check an X25519 result for zero or derive
an HKDF key directly in every engine. The page module overwrites those
bytes with zeros at once and never sends them anywhere.

Libraries that need raw keys (MLS, HPKE) keep their own secrets in the
service's memory; Crypto is for keys that must never leave the device: the
device identity key, the backup key and the backup encryption.

## Consequences

- Three calls take more than four arguments (`agree` seven, `deriveKey`
  six, `fromCode` five). The approved plan fixed these lists so that one
  call makes a ready key; splitting them would leave half-made keys in the
  store between calls.
- `sign` and `verify` first refused empty data, because WebKit answers
  `false` when it verifies an empty Ed25519 message. ADR 0133 lifts this:
  an empty message signs and verifies on every engine.
- Every answer is an event. A library that wants a synchronous signer must
  wait for the event (in Go, a goroutine can block on a channel).
- The approved plan put a `request` argument on every call; it became a
  returned request id, like `Peers.open` and `Files.upload`. Ruby's `hash`
  is `hash_data`, because every Ruby object has a `hash` method.
- On the web, kept keys are only as safe at rest as the disk (above).
- Native apps run the same page module with the web view's WebCrypto until
  their own hosts answer Crypto. The macOS web view has WebCrypto on
  `tachyon-app://` (macOS 27.2); iOS, Android and WebView2 are not run yet.
  `target_support.json` records this as `runOwed`.

## Evidence

- `compiler::page_crypto_tests` (Node's WebCrypto with a fake IndexedDB):
  RFC 8032 test 2 verify, RFC 5869 test 1, SHA-256 of `abc`, NIST
  AES-256-GCM test 16 decrypt, PBKDF2-SHA-256 at 600,000 rounds and
  `deriveKey` against Node's own `pbkdf2Sync` and `hkdfSync`, X25519 both
  ways, a small-order key refused, every reason, every argument check,
  the 32,768-byte edge, new nonces, a kept key across a reload of the
  module, `forget` and `forgetAll`, no secret bytes in any event, the hooks,
  the native host path, an insecure page and a page without IndexedDB, and
  a build that ships the module only with the grant (web and Android).
- `tac_web::web_api_tests::crypto_tests`: a service in Rust, Go, C#, F#,
  Dart, Ruby, Lua, Swift, Kotlin, Java and C++ calls `Crypto` and hears
  `CryptoKeyMade`, `CryptoDerived` and `CryptoFailed`;
  `tac_python::browser_api_tests::a_python_web_service_signs_and_derives_with_crypto`
  checks every Python call; the JavaScript `@Needs` test names `crypto`.
- `bun run test:crypto-browser` (`scripts/crypto-browser-test.mjs`):
  Chromium, Firefox and WebKit run the same known answers, both agreement
  sides, sign and verify, 64 new nonces, a 32,768-byte message, a kept key
  across a reload that `exportKey` refuses, `forget` and `forgetAll`; in
  Chromium a plain `http` page answers `unsupported`.
