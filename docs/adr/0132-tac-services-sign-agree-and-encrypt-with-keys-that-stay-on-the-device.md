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
    keys in a worker without key bytes reaching page script. The same hooks
    object has `prfName(name)` and `prf(name, { first, second })`, which the
    passkeys module uses to turn a passkey's PRF results into not-kept
    `derive` keys (ADR 0126, amendment P3 of 2026-11-14).
12. **A native app may answer the calls itself.** When the host lists
    `crypto` in `__tachyonBrowserApiBridge`, the page module still checks the
    arguments and makes the request id, then sends
    `browserApi.crypto.<call>` with the JSON array of the arguments in the
    order above. The host answers `{ "value": { ...the event's fields } }`,
    or `{ "value": { "reason": "<reason>" } }` for `CryptoFailed`; an
    `error`, a missing value or an unknown reason becomes `failed`. The page
    sets `request` itself, so a host cannot answer for another call. The
    Linux app answers Crypto this way with OpenSSL and the Secret Service
    (ADR 0131 amendment), and the macOS and iOS apps with CryptoKit and the
    Keychain (amendment P2b below); both use the same path without changing
    the service API. Where the host answers, the frame-encryption hook
    (`key(name)`) finds no `CryptoKey` in the page and answers `null`; the
    host hands SFrame its frame keys in a later change (ADR 0131).
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
- **A copy of the disk** of a Chromium, Firefox or WebView2 profile, or of
  WebKitGTK outside the Linux app, taken while the disk is not encrypted:
  these engines store `CryptoKey` bytes in IndexedDB without their own
  encryption (WebKitGTK: checked in the Linux probe of 2026-11-14). The
  Linux app keeps its keys in the Secret Service instead. Safari wraps them with a key
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
  their own hosts answer Crypto. The macOS and iOS hosts answer it since
  the P2b amendment below; Android and WebView2 are not run yet. `target_support.json`
  records this as `runOwed`.

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

## Amendment (2026-11-14): the Apple apps answer Crypto with CryptoKit and the Keychain (P2b)

### Context

This decision gave Tac services a `Crypto` client. In a browser the page's
WebCrypto does the work, and kept keys are non-extractable `CryptoKey`s in
IndexedDB. Item 12 defined a native seam: when the host lists
`crypto` in `__tachyonBrowserApiBridge`, the page module checks the
arguments and sends `browserApi.crypto.<call>` with a JSON array of them to
`__tachyonNativeHostCall`; the host answers `{ "value": { ...fields } }` or
`{ "value": { "reason": "<reason>" } }`. Until now no Apple host answered,
so the macOS and iOS apps used the web view's own WebCrypto.

That had three problems on Apple:

- WKWebView keeps a kept `CryptoKey` in its IndexedDB, wrapped with a key
  from the login Keychain. The approved plan asks for keys that never
  leave the device: not in iCloud Keychain, not in a backup, not on a new
  phone.
- WebKit's own Ed25519 verify answers `false` for every empty message
  (ADR 0133). The page works around it with a second verifier.
- Older WebKit and some iOS versions do not treat `tachyon-app://` as a
  secure context, so they have no `crypto.subtle` at all (ADR 0065). The
  probe of 2026-11-14 found it present on macOS 27.2.1; iOS and older
  macOS versions were not checked.

Apple's CryptoKit has every operation the contract needs:
`Curve25519.Signing` (Ed25519), `Curve25519.KeyAgreement` (X25519),
`AES.GCM`, `HKDF<SHA256>` and `SHA256`. CommonCrypto has PBKDF2-SHA-256
(`CCKeyDerivationPBKDF`). The Keychain can hold an item "this device only".

### Decision

1. **The macOS and iOS hosts answer every Crypto call themselves.** Both
   list `crypto` in `__tachyonBrowserApiBridge`, so the page module sends
   all fifteen calls over the native seam of ADR 0132 and never uses the web
   view's WebCrypto for them. The service API, the events, the reasons and
   the byte format do not change. The same seam carries the Linux host's
   answers (ADR 0131), and both hosts are checked with the same page
   scripts (`native_crypto_checks.rs`).
2. **Only the app's own page reaches it.** The message handler accepts
   the call only from the main frame of `tachyon-app://bundle`, as for
   every host call. A call reaches CryptoKit only when the manifest grants
   `crypto`; otherwise the host answers the same "not granted" error as
   for the other browser APIs. The host checks every argument again (key
   names, base64url text, sizes, algorithms, uses, `keep`, the HKDF length),
   because page script can call the bridge directly.
3. **The work.** Ed25519 sign and verify (an empty message works, as
   ADR 0133 requires), X25519 into HKDF-SHA-256 straight into a new key
   (an all-zero shared secret answers `invalid`), HKDF-SHA-256 from a
   `derive` key, PBKDF2-SHA-256 at 600,000 rounds, AES-256-GCM with a
   random 96-bit nonce made by CryptoKit and the sealed format
   `version (1) || nonce (12) || ciphertext || tag (16)`, and SHA-256. The
   work runs on one serial queue off the main thread; answers come back on
   the main thread.
4. **Where keys live.** A key with `keep: false` lives in the host's memory
   until the app stops. A key with `keep: true` is a generic-password item
   in the data-protection Keychain (`kSecUseDataProtectionKeychain`),
   service `dev.tachyon.crypto.keys`, account = the key's name,
   `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, and not
   synchronizable. So it is not in iCloud Keychain, it is not restored from
   a backup onto another device, and it cannot be read before the first
   unlock after a restart (`locked`). The item holds one byte for the kind
   (`x25519`, `ed25519`, `encrypt`, `derive`) and the 32 key bytes. Key
   bytes never reach the page, the service or a log. `check` reports
   `keyStore` `os`.
5. **A reinstalled app starts clean.** iOS keeps Keychain items after an
   app is removed. The first time a Crypto call reaches the Keychain in a
   new install (no `dev.tachyon.crypto.installed` mark in the app's
   `UserDefaults`), the host deletes every item of its service, then sets
   the mark. The device copy of the data starts clean in the same way.
6. **No Keychain entitlement, no kept keys.** The data-protection Keychain
   needs a signed app with an application identifier. An ad hoc signed
   macOS app (every development build, and the macOS test machine, which
   has no Apple signing identity) gets `errSecMissingEntitlement`. Then
   `check` reports `keyStore` `none`, `keep: true` answers `unsupported`
   (as a browser without IndexedDB does), `keep: false` keys work, and the
   app writes once to its standard error: "this build has no Keychain
   entitlement". The host never falls back to the file-based login
   Keychain, which Time Machine copies and which would ask for the user's
   password after every rebuild (the hub's decision for the user,
   2026-11-14, choice A).
   - **Signed macOS releases keep keys.** A `--release` build that grants
     `crypto` and names a Developer ID profile in
     `TAC_MACOS_PROVISIONING_PROFILE` (ADR 0106) now checks the profile's
     team and app ID, embeds it, and signs the app with
     `com.apple.application-identifier` and
     `com.apple.developer.team-identifier`. The Associated Domains
     capability is required only when `tac.application.links` lists hosts.
     Without the profile the release builds as before and answers
     `keyStore` `none`.
   - **iOS** device builds are always signed with `application-identifier`
     from their profile, so they keep keys. Simulator builds are signed ad
     hoc; the simulator check records what they answer.
7. **Frame encryption fails closed on a native Crypto host.** SFrame
   (ADR 0135) takes its frame keys from the page hook `key(name)`, which
   gives a non-extractable `CryptoKey`. Where the host answers Crypto
   (macOS, iOS and Linux), keys stay in native code and the hook has none.
   There `Media.useFrameKey` and `Media.encryptFrames(connection, true)`
   answer at once with an `unsupported` error, and turning encryption off
   still works. Media is never sent without SFrame by mistake: the frame
   worker also drops every outgoing frame while it has no send key. The
   follow-up "native hosts hand SFrame its frame keys" is part of the same
   release (the hub's decision for the user, 2026-11-14). In it, the one
   exception to "no key bytes in the page" stays narrow: only per-frame
   media keys, imported as non-extractable, and only into the media
   worker.
8. **Android keeps app data out of backups and device transfers.** The
   Android app already had `android:allowBackup="false"`, but for apps that
   target Android 12 or later that does not stop a device-to-device
   transfer. Every Android app now also has
   `android:dataExtractionRules="@xml/tachyon_data_extraction_rules"`,
   which excludes every domain (`root`, `file`, `database`, `sharedpref`,
   `external` and the `device_` ones) from both `cloud-backup` and
   `device-transfer`. The WebView's IndexedDB, where the Android app keeps
   its `CryptoKey`s, is in the app's private folder, so kept keys no longer
   reach a new phone.

### Security

- **Same promises as above**, now with the platform key store on Apple:
  no call returns a private key, a secret key or a shared secret; script
  injected into the page can use a key while the page is open, but cannot
  copy it, because the bytes never enter the web view.
- **Secure Enclave:** it holds only P-256 keys, so X25519 and Ed25519 keys
  are Keychain items protected by the device's data protection, not
  hardware keys.
- **Verify follows CryptoKit.** CryptoKit refuses non-canonical `S` values;
  like Chromium's WebCrypto, it does not refuse a small-order public key.
  The page's extra strict check of ADR 0133 runs only when WebCrypto
  answers, not over the native seam.
- **What this does not protect against:** malware running as the same
  user, a jailbroken device, and a person using the unlocked app (as above).

### Consequences

- `target_support.json`: `tac.browserApis crypto` is `honored` on macOS
  (the native check ran) and stays `runOwed` on iOS until the simulator
  check runs.
- **Owed:** kept keys across a relaunch in a signed macOS release (a
  Developer ID profile and signing identity are not on the test machine),
  and on a signed iOS device by hand.
- Android apps lose device-to-device transfer of their private data. The
  server stays the source of truth (ADR 0059), so a new phone syncs again.
- About 400 lines of Swift ship in Apple apps that grant nothing more than
  before; the Crypto code runs only for apps that grant `crypto`.

### Evidence

- 2026-11-14, macOS 27.2.1 (Falcos-Mini-2, ad hoc build): an ad hoc signed
  tool got `errSecMissingEntitlement` (-34018) from the data-protection
  Keychain, and CryptoKit verified an Ed25519 signature over an empty
  message.
