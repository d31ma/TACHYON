# ADR 0118: Tac Pages Subscribe to Web Push

- Status: Accepted
- Date: 2026-11-10
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (the browser
  APIs that need a grant, and the size budget of `tac-web.js`)

## Context

Mesail, a mail app written in Go on Tac and Yon, must tell a user about new
mail while its page is closed. Browsers do this with Web Push: the page asks
`PushManager` for a subscription, the server stores it, and later the server
sends an encrypted message (RFC 8291, with a VAPID key, RFC 8292) to the
subscription's endpoint. The browser wakes a service worker, which shows a
notification. Tac services had no way to make a subscription, and the
Tachyon service worker only caches files.

Native apps run Tac pages in the platform web view (ADR 0018). WKWebView,
WebView2, WebKitGTK and the Android WebView have no Push API. Native push
(APNs, FCM, WNS) needs a different design and server, and is out of scope.

## Decision

1. `push` is a new browser API grant in `tac.browserApis`. Without it every
   push call throws the usual "not granted" error.
2. Go services get the `Push` type, injected like `Clock` or
   `Notifications`:
   - `Supported() (bool, error)` answers at once;
   - `Subscribe(applicationServerKey string) error` needs a click or key
     handler. It takes the server's VAPID public key as base64url (65 bytes
     that start with `0x04`), asks for notification permission, and answers
     with the `PushSubscribed` event;
   - `Subscription() error` answers with `PushSubscribed`, or with
     `PushUnsubscribed` and an empty endpoint when there is none;
   - `Unsubscribe() error` answers with `PushUnsubscribed`.
   A failure after the call started (permission denied, the browser refused)
   sends `BrowserCallFailed` with `api` `push`, as every browser API does.
   JavaScript and TypeScript get the same calls as the `push` resource
   (`@Needs push`). The other Tac languages have no `Push` type yet.
3. `PushSubscribed { endpoint, p256dh, auth, expirationTime }` and
   `PushUnsubscribed { endpoint }` are built-in event models. The keys are
   base64url, as `PushSubscription.toJSON()` gives them, and
   `expirationTime` is milliseconds since 1970, or 0.
4. A web build that grants `push` ships two more files, and only then:
   - `/.tachyon/tac-web-push.js`, a page module loaded before the runtime,
     which does the subscription work;
   - `/.tachyon/tac-push-sw.js`, a second service worker registered with the
     scope `/.tachyon/push/`. No page is inside that scope, so it never
     handles `fetch` and never changes the caching of `tachyon-sw.js`.
   `tac-web.js` keeps only a small `push` entry that checks the grant and
   calls the module.
5. The push worker shows each message with `showNotification`. The message
   is JSON `{"title", "body", "path", "tag"}`; plain text becomes the body,
   and a missing title becomes the host name. A click focuses a window that
   is already on that path, or opens a new one. The path is used only when it
   starts with one `/`, has no `\` and stays on the worker's origin;
   otherwise the click opens `/`. The worker loads no other script.
6. The worker registration script still removes every worker on loopback
   hosts and in native apps, except the push worker on a loopback host, so
   a push subscription survives a reload during development.
7. A native build never ships the push files. There, `Supported` answers
   false, and the other calls fail with "Web Push needs a web browser". The
   manifest reach table records this as `adapted` on macOS, Windows, Linux,
   iOS and Android (ADR 0102).
8. The size budget of `tac-web.js` grows from 11,600 to 12,700 bytes for the
   `push` entry and its fallback.

## Consequences

- A Go Tac app can subscribe to Web Push and send the subscription to its
  Yon server with `@HTTPClient`. The server encrypts and sends the messages
  itself; Tachyon does not send push messages.
- Safari on iPhone and iPad offers Web Push only to an app added to the Home
  Screen, from iOS 16.4. `Supported` answers false elsewhere on those
  devices.
- Pages that do not grant `push` load no new bytes except the small entry in
  `tac-web.js`.
- Native push stays a separate, later decision.
