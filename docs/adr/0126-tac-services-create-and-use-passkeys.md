# ADR 0126: Tac Services Create and Use Passkeys

- Status: Accepted
- Date: 2026-11-12
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (the browser
  APIs that need a grant, the built-in events, and the size budget of
  `tac-web.js`)

## Context

Mesail, a mail app on Tac and Yon, signs people in with passkeys through its
own server (CADUCEUS ADR 0008, request T13). A passkey is a WebAuthn
credential: the server sends options with a random challenge, the browser
asks the person to confirm with Touch ID, Face ID, Windows Hello, the Android
lock screen or a security key, and the page sends the signed answer back to
the server, which checks it. Tac services had no way to reach WebAuthn
(`navigator.credentials.create` and `get`).

WebAuthn has binary fields (`ArrayBuffer`), but services in Tachyon's
fourteen languages talk to the page with JSON. WebAuthn Level 3 defines a
JSON form of the options (`PublicKeyCredentialCreationOptionsJSON`, base64url
for binary fields) and of the answers (`RegistrationResponseJSON`,
`AuthenticationResponseJSON`), with `PublicKeyCredential.parse...FromJSON`
and `toJSON()` in Chromium 129+, Firefox 119+ and Safari 18.4+. Servers and
their libraries already speak this JSON.

Browsers require a secure page for WebAuthn, bind every passkey to the
page's origin and the relying party id, and (Safari) require a click or key
press for `create` and a modal `get`. Conditional mediation lets the browser
offer passkeys in the autofill of an input with
`autocomplete="username webauthn"` without a prompt. The native shells load
pages from their own origins (`tachyon-app://bundle`,
`https://appassets.tachyon.local`, `https://tachyon.local`, `tachyon://app`),
never from the app's server host, so WebAuthn in a web view cannot make a
passkey for that host; native apps need the platform passkey APIs, which come
in a later decision.

## Decision

1. `passkeys` is a new browser API grant in `tac.browserApis`. Every call
   needs it except `supported`.
2. Every Tac service language gets a `Passkeys` client, injected the way it
   gets `Push`, on one contract (the page's `passkeys` browser API):

   | Call | Needs a click or key handler | Answer |
   | --- | --- | --- |
   | `supported()` | no; no grant | at once: true in a secure page with `PublicKeyCredential`, false in a native app |
   | `check()` | no | `PasskeySupport { platform, conditional }` |
   | `create(optionsJson)` | yes | `PasskeyCreated { credentialJson }` |
   | `get(optionsJson)` | yes | `PasskeyAsserted { credentialJson, conditional: false }` |
   | `offer(optionsJson)` | no | `PasskeyAsserted { credentialJson, conditional: true }` when the person picks a passkey in autofill |
   | `cancel()` | no | nothing |

   | Language | Field | Calls |
   | --- | --- | --- |
   | JavaScript, TypeScript | `@Needs passkeys` (`TacPasskeys`) | `supported()`, `check()`, `create(optionsJson)`, `get`, `offer`, `cancel()` |
   | Rust | a `passkeys: Passkeys` field filled by `Default` | the same, each returns `Result`; `supported` gives `Result<bool, String>` |
   | Go | a `// @Needs` field `passkeys Passkeys` | `Supported() (bool, error)`, `Check() error`, `Create(optionsJSON string) error`, `Get`, `Offer`, `Cancel()` |
   | C#, F# | `new()` field, `let passkeys = Passkeys()` | `Supported()`, `Check()`, `Create(optionsJson)`, `Get`, `Offer`, `Cancel()` |
   | Kotlin, Java, Dart, C++ | `Passkeys()`, `new Passkeys()`, a `@Needs()` field, `[[Needs]] Passkeys&` | `supported()`, `check()`, `create(optionsJson)`, `get`, `offer`, `cancel()` |
   | Swift | `Passkeys()` | `supported()`, `check()`, `create(optionsJson:)`, `get(optionsJson:)`, `offer(optionsJson:)`, `cancel()`, all `throws` |
   | Ruby | `Passkeys.new` | `supported`, `check`, `create(options_json)`, `get`, `offer`, `cancel` |
   | Lua | the global table `Passkeys` | `Passkeys:create(options_json)` and the others |
   | Python | a `needs` field `passkeys: Annotated[Passkeys, needs]` | `supported()`, `check()`, `create(options_json)`, `get`, `offer`, `cancel()` |

   Natively linked services cannot use `Passkeys`, like every page client
   (`TY1317`).
3. **JSON text in and out.** The options and the credential stay WebAuthn
   Level 3 JSON **text** in every language. Services pass them between the
   page and the server and need no WebAuthn model. The page uses the
   browser's own `parseCreationOptionsFromJSON`, `parseRequestOptionsFromJSON`
   and `toJSON()` where they exist, and its own small converter elsewhere
   (base64url for `challenge`, `user.id` and the credential ids; the same
   member names and values as `toJSON()`). Extensions pass through: their
   inputs in the options, their results in `clientExtensionResults`.
4. **Four built-in events** in a new `passkey` group of the event table, each
   sent only to the calling service and only on a page that grants
   `passkeys` (sender `WebEventSender::Grant("passkeys")`):
   `PasskeySupport { platform, conditional }`, `PasskeyCreated {
   credentialJson }`, `PasskeyAsserted { credentialJson, conditional }` and
   `PasskeyFailed { call, reason, message }`.
5. **One reason word per failure.** Every failure after a call starts sends
   `PasskeyFailed` with one `reason` the app can switch on:

   | Reason | When |
   | --- | --- |
   | `cancelled` | the person closed the prompt, the prompt timed out, or no passkey was chosen (`NotAllowedError`, an `AbortError` that Tachyon did not cause, no credential) |
   | `exists` | a credential in `excludeCredentials` is already on this device (`InvalidStateError`) |
   | `unsupported` | no WebAuthn here: a native app, an insecure page, no conditional mediation for `offer` (`NotSupportedError`) |
   | `refused` | the relying party does not match this page (`SecurityError`) |
   | `busy` | a `create` or a modal `get` is already open |
   | `invalid` | the options are not a JSON object, have a bad base64url field, miss a required member, or are larger than 65,536 bytes |
   | `failed` | anything else |

   `message` is a fixed text for the reason, with the browser's error name in
   parentheses when it is a plain word. The grant and user-action errors
   stay synchronous, as for every client. `BrowserCallFailed` is sent only
   when the passkeys module did not load (a programming error), never for a
   ceremony.
6. **One ceremony at a time.** A `create` or `get` while another `create` or
   modal `get` is open answers `busy` and never reaches the browser. A
   `create`, `get` or `offer` stops a pending `offer` first; a stopped offer
   sends nothing. `cancel()` stops the pending call, which sends nothing
   either: the service asked for it.
7. **A module that ships only when granted.** The ceremonies, the converter
   and the reason table are in `.tachyon/tac-passkeys.js`, which a build
   writes and the page loads only when `tac.browserApis` grants `passkeys`
   (web and native builds), and which the service worker caches for offline
   pages. `tac-web.js` holds only the small `passkeys` entry, so its budget
   (`MOST_RUNTIME_BYTES`) rises from 15,500 to 16,500 bytes; most of that is
   the entry's six calls and their grant check.
8. **Native shells answer later.** Until the platform passkey APIs are
   built, a native page answers `supported()` false, `check()` with
   `PasskeySupport { platform: false, conditional: false }`, and `create`,
   `get` and `offer` with `PasskeyFailed` reason `unsupported`.
9. **A build note.** A build that grants `passkeys` with a `tac.origin`
   that is not `https` (other than `http://localhost` and
   `http://127.0.0.1`) prints a note: WebAuthn works only in a secure page.

## Security

- Tachyon never logs, prints, puts in an error message or sends in
  telemetry: the options, the challenge, `user.id` or the user handle,
  credential ids, `credentialJson`, `clientDataJSON`, attestation or
  signature bytes. `PasskeyFailed.message` is never the browser's
  `error.message`, because some browsers put the relying party or the URL in
  it.
- The relying party and origin checks on the web are the browser's: WebAuthn
  binds the credential to the page origin and `rp.id`. Tachyon does not
  weaken them: no iframe, no `allow` attribute, no
  `publickey-credentials-get` delegation, and no `Permissions-Policy` that
  changes `publickey-credentials-*`.
- A ceremony needs the person: `create` and a modal `get` need a click or
  key handler, and the browser shows its own prompt. `offer` shows nothing
  until the person picks a passkey in autofill.
- One ceremony at a time per page, so a page cannot stack prompts.
- `credentialJson` is untrusted input for the server. The server must check
  the challenge, the origin, the relying party id hash, the user presence
  and user verification flags, and the signature counter. Tachyon does not
  invent its own checks that the server must make.

## Consequences

- Services sign people in and add passkeys in every Tac language, with the
  JSON that server libraries already read.
- An app tells "the person closed the prompt" (`cancelled`) apart from a
  real error, and "this device already has a passkey" (`exists`) apart from
  both.
- `NotAllowedError` means both "closed" and "timed out"; both are
  `cancelled`.
- Safari refuses `create` and a modal `get` outside a click or key handler;
  the user-action check stops such a call at once in every browser, so the
  mistake shows during development.
- Security keys over USB or NFC are not a separate choice; the browser's
  own sheet may still offer them, and Tachyon does not refuse them. Related
  Origin Requests (`/.well-known/webauthn`) are not part of this decision.
- Native apps get passkeys only with the platform APIs of a later decision.

## Evidence

- `compiler::page_passkeys_tests`: the page runtime with a fake WebAuthn:
  `check` with and without `getClientCapabilities`; `create` and `get` with
  the browser's JSON helpers and with Tachyon's own converter (byte for byte
  against the expected JSON); every `DOMException` name to its reason; bad
  JSON, a non-object, a bad base64url field, a bad base64url length, a
  missing `user` and 65,537 bytes as `invalid`; `busy`; a replaced and a
  cancelled call that send nothing; `offer` without conditional mediation;
  the grant, user-action, native and insecure refusals; no browser message
  in any event. The build ships the module (web and native) only with the
  grant, caches it in the service worker, and prints the note for an
  `http` origin.
- `tac_web::web_api_tests::passkeys_tests`: a service in Rust, Go, C#, F#,
  Swift, Kotlin, Java, Dart, Ruby, Lua and C++ that asks `supported`, calls
  `create` and `get`, and hears `PasskeyCreated` and `PasskeyFailed`.
  `tac_python::browser_api_tests::a_python_web_service_creates_and_uses_passkeys`
  and `compiler::web_api_tests::a_javascript_service_needs_every_curated_browser_api`.
- `scripts/passkeys-browser-test.mjs` (`bun run test:passkeys-browser`): in
  Chromium with the DevTools virtual authenticator (internal transport, user
  verification on): `check`, `create`, `get` (the converter writes the same
  JSON as `toJSON()`), `exists`, `refused`, `cancelled` by timeout, `busy`,
  `cancel`, the conditional `offer` (the virtual authenticator answers it),
  and an offer that a modal `get` stops without an event. In Chromium,
  Firefox and WebKit:
  `supported`, `check`, the grant and user-action errors, `invalid` options,
  and the converter against the browser's own `parse...FromJSON`. Firefox
  and WebKit have no virtual authenticator, so they run no ceremony.
- Owed, by hand: one real `create` and `get` in branded Safari with Touch ID,
  and in Chrome on macOS or Windows with the platform authenticator, recorded
  in `docs/SUPPORT_TIERS.md`.
