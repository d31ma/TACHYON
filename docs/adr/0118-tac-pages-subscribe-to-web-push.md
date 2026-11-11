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
2. Every Tac service language gets a `Push` client, injected the way it gets
   `Notifications`, with the same four calls on one contract (the page's
   `push` browser API):
   - `supported` answers at once: true only in a secure web page whose
     browser has the Push API;
   - `subscribe(applicationServerKey)` needs a click or key handler. It takes
     the server's VAPID public key as base64url (65 bytes that start with
     `0x04`), asks for notification permission, and answers with the
     `PushSubscribed` event;
   - `subscription` answers with `PushSubscribed`, or with `PushUnsubscribed`
     and an empty endpoint when there is none;
   - `unsubscribe` answers with `PushUnsubscribed`.
   A failure after the call started (permission denied, the browser refused)
   sends `BrowserCallFailed` with `api` `push`, as every browser API does. A
   call the page refuses at once (no grant, no user action, a bad key, no
   Push API) fails the way every browser call fails in that language.

   | Language | How a service gets it | Calls |
   | --- | --- | --- |
   | JavaScript, TypeScript | `@Needs push` (`TacPush`) | `supported()`, `subscribe(key)`, `subscription()`, `unsubscribe()` |
   | Rust | a `push: Push` field filled by `Default` | the same names; each returns `Result` |
   | Go | a `// @Needs` field `push Push` | `Supported() (bool, error)`, `Subscribe(key) error`, `Subscription() error`, `Unsubscribe() error` |
   | C#, F# | `new()` field, `let push = Push()` | `Supported()`, `Subscribe(key)`, `Subscription()`, `Unsubscribe()` |
   | Kotlin, Java, Dart, C++ | `Push()`, `new Push()`, a `@Needs()` field, `[[Needs]] Push&` | `supported()`, `subscribe(key)`, `subscription()`, `unsubscribe()` |
   | Swift | `Push()` | `supported()`, `subscribe(applicationServerKey:)`, `subscription()`, `unsubscribe()`, all `throws` |
   | Ruby | `Push.new` | `supported`, `subscribe(key)`, `subscription`, `unsubscribe` |
   | Lua | the global table `Push` | `Push:supported()`, `Push:subscribe(key)`, `Push:subscription()`, `Push:unsubscribe()` |
   | Python | a `needs` field `push: Annotated[Push, needs]` (ADR 0121) | `supported()`, `subscribe(key)`, `subscription()`, `unsubscribe()`; a refusal raises `RuntimeError` |

   A Swift, Kotlin, C# or Rust service that is linked into a native app as
   native code cannot use `Push`, like every page client: it belongs to the
   page.
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

- A Tac app in any service language can subscribe to Web Push and send the
  subscription to its Yon server with `@HTTPClient`. The server encrypts and sends the messages
  itself; Tachyon does not send push messages.
- Safari on iPhone and iPad offers Web Push only to an app added to the Home
  Screen, from iOS 16.4. `Supported` answers false elsewhere on those
  devices.
- Pages that do not grant `push` load no new bytes except the small entry in
  `tac-web.js`.
- Native push stays a separate, later decision.

## Amendment (2026-11-11): open pages hear a push (T4a)

### Context

Mesail treats a push as a poke: when a message arrives, the open app must
sync at once (CADUCEUS ADR 0006). Before this amendment the push worker only
showed the notification, so an open page learned about new mail only at its
next poll or socket message.

### Decision

1. `PushReceived { tag, path, data }` is a new built-in event model, all text,
   in the shared event table (`web_event_models.rs`), so every Tac service
   language gets the type and can listen with `@OnEvent`.
2. When a message arrives, the push worker still shows the notification, and
   it also posts `{ tachyon: "PushReceived", tag, path, data }` to every open
   window of its own origin (`clients.matchAll` with `includeUncontrolled`,
   then a second origin check of each window's URL). The scope
   `/.tachyon/push/` holds no page, so "the app's pages" means every window
   of the worker's origin. The worker never posts to another origin.
   - `tag` is the message's `tag`, or empty;
   - `path` is the path the notification click opens, after the checks of
     decision 5 (`/` when the message has none or a bad one);
   - `data` is the message's optional `data` member: a string as it is, any
     other JSON value as its JSON text, or empty. A plain-text message has
     no `data`; its text stays the notification body.
3. `tac-web-push.js` listens for service-worker messages and starts the
   message queue. It accepts a message only when its `source` is the push
   worker of the page's own origin (`scriptURL` equals
   `<origin>/.tachyon/tac-push-sw.js`), its kind is `PushReceived`, and all
   three fields are strings. It then publishes the event through the page's
   shared page-event entry, which publishes only names on its allow list
   (`FileDropped`, `PushReceived`, `BrowserCallFailed`). So the payload gets
   the same checks as every other page event: the 64 KiB and structure limits
   of the event bus, and each listener's model check.
4. The page-event entry of `tac-web.js` was named `filesDropped`; it is now
   `pageEvent`, because files and push both use it. `tac-web.js` stays within
   its 13,500-byte budget.
5. Nothing changes for native builds: they ship no push files, so they never
   send `PushReceived`.

### Consequences

- An open page syncs on a push without waiting for a poll. A closed app still
  only shows the notification; the next page start syncs as before.
- Every open window of the app hears the push, each once. An app that should
  sync from one window only must choose that window itself.
- Messages that arrive before the page's services start are not kept.
