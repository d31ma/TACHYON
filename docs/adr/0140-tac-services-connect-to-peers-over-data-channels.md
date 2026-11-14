# ADR 0140: Tac Services Connect to Peers over Data Channels

- Status: Accepted
- Date: 2026-11-14
- Amends: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md) (a new
  browser API grant, its built-in events and the size budget of `tac-web.js`),
  [ADR 0124](0124-sockets-and-streams-reconnect-by-a-declared-policy-and-reach-named-origins.md)
  (peer traffic is a named exception to "a page reaches only declared hosts")

## Context

Mesail, a mail app on Tac and Yon, moves chat between people to their own
devices, encrypted end to end, so that no server can read it (CADUCEUS ADR
0011, request P1). Devices talk over WebRTC data channels: the app carries the
connection offers, answers and network candidates over its own server
("signalling"), and a TURN relay carries the traffic when two devices cannot
reach each other directly. Tac services had no way to reach WebRTC.

WebRTC needs a STUN server (to learn the public address) and a TURN relay.
Browsers have no default servers, and Mesail's installs run their own relay
on-premise (CADUCEUS ADR 0007), so the servers must come from the app.

A data channel is not limited by the page's content security policy
(`connect-src`): the browser connects to whatever address the other side
offers. So the "a page reaches only declared hosts" rule of ADR 0124 cannot
hold for the other device.

## Decision

1. `peers` is a new browser API grant in `tac.browserApis`. Every call needs
   it except `supported`.
2. **Servers come only from the manifest.** `tac.peers` declares them:

   ```json
   "peers": {
     "iceServers": ["stun:turn.example.com:3478", "turn:turn.example.com:3478?transport=udp", "turns:turn.example.com:5349"],
     "relayOnly": false
   }
   ```

   - There is no default server. With an empty list only direct connections
     (on one network) work.
   - The build checks each entry: `stun:host:port`, `turn:host:port`
     (optionally `?transport=udp` or `tcp`) or `turns:host:port` (optionally
     `?transport=tcp`); at most 8 entries of at most 256 characters. Each host
     must be in `tac.outgoing.hosts`, except `localhost` and loopback
     addresses. A failure stops the build with `TY1601`.
   - `relayOnly: true` sets `iceTransportPolicy: "relay"`: all traffic goes
     through the TURN relay, and the other side never learns this device's
     addresses. It needs at least one `turn:` or `turns:` entry.
   - `tac.peers` without the `peers` grant stops the build.
   - The page reads the list from a `tachyon-peers` meta tag that the build
     writes. A service never passes a server address.
3. **Credentials come from the app at run time.** `open(username,
   credential)` gives the TURN servers (never the STUN servers) a username and
   a credential, which the app gets from its own server, ideally short-lived
   (the TURN REST style of coturn). Credentials are never in the manifest, so
   no page carries a long-lived relay password.
4. Every Tac service language gets a `Peers` client, injected the way it gets
   `Passkeys`, on one contract (the page's `peers` browser API):

   | Call | Answer at once | Later |
   | --- | --- | --- |
   | `supported()` (no grant) | true when the page has `RTCPeerConnection` | |
   | `open(username, credential)` | a connection id, such as `peer-1` | `PeerCandidate`, `PeerState` |
   | `offer(connection)` | nothing | `PeerDescription { kind: "offer" }` |
   | `answer(connection, offerSdp)` | nothing | `PeerDescription { kind: "answer" }` |
   | `accept(connection, answerSdp)` | nothing | `PeerFailed` only on failure |
   | `addCandidate(connection, candidateJson)` | nothing | `PeerFailed` only on failure |
   | `restart(connection)` | nothing | a new offer (ICE restart) |
   | `openChannel(connection, label, ordered)` | a channel id, such as `channel-1` | `PeerChannelOpened` |
   | `send(connection, channel, data)` | true when queued, false when more than 1 MiB waits | `PeerChannelReady` after a false |
   | `closeChannel(connection, channel)` | nothing | `PeerChannelClosed` |
   | `close(connection)` | nothing | `PeerState { state: "closed" }` |

   | Language | Field | Calls |
   | --- | --- | --- |
   | JavaScript, TypeScript | `@Needs peers` (`TacPeers`) | the names above |
   | Rust | a `peers: Peers` field filled by `Default` | snake_case (`add_candidate`, `open_channel`, `close_channel`), each returns `Result` |
   | Go | a `// @Needs` field `peers Peers` | `Supported`, `Open`, `Offer`, `Answer`, `Accept`, `AddCandidate`, `Restart`, `OpenChannel`, `Send`, `CloseChannel`, `Close`; `PeerMessage.Bytes()` decodes the data |
   | C#, F# | `new()` field, `let peers = Peers()` | PascalCase |
   | Kotlin, Java, Dart, C++ | `Peers()`, `new Peers()`, a `@Needs()` field, `[[Needs]] Peers&` | camelCase |
   | Swift | `Peers()` | camelCase with labels (`open(username:credential:)`), all `throws` |
   | Ruby | `Peers.new` | snake_case; `send` is `send_data`, because `send` is a method of every Ruby object |
   | Lua | the global table `Peers` | snake_case, `Peers:open(...)` |
   | Python | `peers: Annotated[Peers, needs]` | snake_case |

   Natively linked services cannot use `Peers`, like every page client
   (`TY1317`).
5. **Channels.** `ordered: true` is ordered and reliable. `ordered: false` is
   unordered with no retransmits (`maxRetransmits: 0`), for data where the
   newest value matters, such as "typing". A label is 1 to 64 letters, digits,
   dots, dashes or underscores. A page holds at most 32 connections and 16
   channels per connection.
6. **Bytes and back-pressure.** `data` is base64url text without padding,
   like the other binary values of Tac clients that are not files. A message
   holds at most 32,768 bytes, so a `PeerMessage` stays far below the 64 KiB
   event limit and works between every browser. `send` answers false, and
   sends nothing, when more than 1 MiB would wait in the channel; the page
   then sends `PeerChannelReady` once the waiting bytes fall below 256 KiB. A
   text message from the other side arrives as its UTF-8 bytes. A message
   larger than 32,768 bytes from the other side closes that channel and sends
   `PeerFailed` with reason `tooLarge`.
7. **Nine built-in events** in a new `peer` group, each sent only to the
   service that opened the connection and only on a page that grants `peers`:
   `PeerDescription { connection, kind, sdp }`, `PeerCandidate { connection,
   candidateJson }` (`""` when there are no more), `PeerState { connection,
   state }` (`new`, `connecting`, `connected`, `disconnected`, `failed`,
   `closed`), `PeerNegotiationNeeded { connection }` (the connection changed,
   for example its first channel was opened after it connected, and the app
   should send a new offer), `PeerChannelOpened { connection, channel, label, ordered, remote
   }`, `PeerChannelReady { connection, channel }`, `PeerMessage { connection,
   channel, data }`, `PeerChannelClosed { connection, channel }` and
   `PeerFailed { connection, call, reason, message }`.
8. **Errors.** A mistake the service can see at once (no grant, no WebRTC, an
   unknown connection or channel, a bad label, bad base64url, data over 32,768
   bytes, a description or candidate over 65,536 bytes, a closed channel, a
   limit) is the call's own error. A failure that comes later is `PeerFailed`
   with one reason: `invalid` (a description or candidate the browser
   refused), `tooLarge` or `failed`. `message` is a fixed text, never the
   browser's own. A closed connection sends nothing more.
9. **No WebRTC, no connection.** Where the page has no `RTCPeerConnection`,
   `supported` answers false and `open` fails with a text that ends in
   `(unsupported)`.
10. **WebKit and the transport hint.** WebKit refuses every TURN URL with a
    `?transport=` part (checked with Playwright's WebKit build 2336). When the
    browser refuses the server list with a `SyntaxError`, the page tries once
    more with the `?transport=` parts removed, so WebKit uses its default
    transport: UDP for `turn:`, TCP and TLS for `turns:`. An app that must
    work where UDP is blocked lists a `turns:` server.
11. **A module that ships only when granted.** The connections, channels,
    limits and reasons are in `.tachyon/tac-peers.js` (web and native builds,
    cached by the service worker). `tac-web.js` holds only a one-line `peers`
    entry through the shared module loader (`TacWebApis.calls`). The entry
    and its line in `TacWebApis.apis` add 167 bytes; `tac-web.js` had only
    48 bytes left, so `MOST_RUNTIME_BYTES` rises from 18,400 to 18,600.
12. **Pages close their connections** on `pagehide`.
13. **Room for media.** A later client for audio and video calls (Mesail's
    request P5) adds tracks to the same connection ids instead of a second
    connection object. Renegotiation already uses the calls above: `offer`
    again on an open connection (the ICE restart test proves it), `answer` on
    a connection that has its own offer out (the browser rolls its own offer
    back, as perfect negotiation needs; Tachyon adds no state check), and
    `PeerNegotiationNeeded`. Other page modules reach a connection through
    `Symbol.for('tachyon.tac.peers.hooks')`: `connection(id)` gives the
    `RTCPeerConnection` or null, `opened(hook)` is called with the id and the
    connection right after it is made and before any offer, and
    `closed(hook)` before it closes. These hooks are for Tachyon's own page
    modules, not for services.

## Security

- **The other device's address is not declared.** This is a named exception
  to ADR 0124: with the `peers` grant, a page may exchange data with any
  device that the app's own signalling introduces. Tachyon itself contacts
  only the declared STUN and TURN hosts. `relayOnly` keeps the page's own
  traffic on the declared relay.
- **The app must encrypt end to end.** WebRTC encrypts each channel with
  DTLS, but the DTLS fingerprints travel in the descriptions, which the app's
  signalling server carries; that server could place itself in the middle.
  Content must be encrypted by the app itself (Mesail uses MLS).
- **Addresses.** Without `relayOnly`, the other side learns this device's
  public address; browsers hide local addresses behind mDNS names.
- Tachyon never logs, prints, puts in an error or event message, or sends in
  telemetry: descriptions, candidates, TURN credentials or message data.
  `PeerFailed.message` is never the browser's `error.message`.
- iOS and macOS ask the person for local network access when WebRTC uses the
  local network. The native shell work that answers this comes with the
  native evidence (below).

## Consequences

- Services open device-to-device connections in every Tac language, with the
  signalling and the encryption left to the app.
- The servers are visible in the manifest and checked at build time, so no
  public server can be used by mistake.
- WebKit ignores the transport hint; Chromium and Firefox use it.
- **Native apps** use the web view's own WebRTC. WebView2 and the Android
  WebView have it; WebKitGTK as Linux distributions build it does not, so the
  Linux app answers `unsupported`. Whether WKWebView offers
  `RTCPeerConnection` on the `tachyon-app://` scheme is not yet checked in a
  built app. These runs are owed (`target_support.json`).

## Evidence

- `compiler::page_peers_tests`: the page runtime with a fake WebRTC: the
  server list and relay policy from the meta tag, credentials only on TURN
  servers, the retry without `?transport=`, offer, answer, accept, candidates
  both ways, ICE restart, state, reliable and unordered channels, a channel
  the other side opened, back-pressure with one `PeerChannelReady`, the
  32,768-byte limits both ways, text as UTF-8, every refusal and limit, no
  browser text in any event, nothing after close, no grant, no module and no
  WebRTC. A build ships the module and the server list only with the grant
  (web and native), and refuses `tac.peers` without the grant and an
  undeclared TURN host. `native::peers_member` checks every URL form.
- `tac_web::web_api_tests::peers_tests`: a service in Rust, Go, C#, F#,
  Swift, Kotlin, Java, Dart, Ruby, Lua and C++ opens a connection and a
  channel, sends bytes and hears `PeerDescription`, `PeerMessage` and
  `PeerFailed`. `tac_python::browser_api_tests::a_python_web_service_connects_to_a_peer`
  and `compiler::web_api_tests::a_javascript_service_needs_every_curated_browser_api`.
- `scripts/peers-browser-test.mjs` (`bun run test:peers-browser`): real
  connections between two browser contexts, with signalling carried by the
  test: Chromium with Chromium, Firefox with Firefox and Chromium with Firefox
  directly; and Chromium, Firefox and WebKit with themselves and Chromium
  with WebKit through a local coturn relay with short-lived credentials and
  `relayOnly`. Each pair connects, sends 10 MiB on an ordered channel with
  back-pressure (checked by SHA-256), uses an unordered channel, refuses a
  32,769-byte message, survives an ICE restart, and closes.
  `TAC_PEERS_REQUIRE_TOOLCHAINS=1` makes a missing `turnserver` fail the
  test; without it the relay pairs are skipped with a note. It passed on the
  macOS test machine on 2026-11-14 (Playwright Chromium 1234, Firefox 1538,
  WebKit 2336, coturn 4.19.0).
- Owed: a connection in each native app (WKWebView on macOS and iOS,
  WebView2, the Android WebView), and a connection between two physical
  devices on different networks through a real TURN relay, recorded in
  `docs/SUPPORT_TIERS.md`.
