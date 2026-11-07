# ADR 0013: Semantic Hot Updates Without a Production Client Renderer

- Status: Superseded by ADR 0015 for Tac rendering and hot-update ownership
- Date: 2026-08-09

## Context

The Rust development server rebuilt changed source, but it offered only live
reload:

- It sampled every source tree every 400 milliseconds.
- It published an integer generation number.
- Each open page polled every 500 milliseconds and then called
  `location.reload()`.

Incremental compilation reduced the compiler's work. It did not keep browser
state.

Vite solves the general JavaScript case with native ESM (ECMAScript modules),
module graphs and acceptance boundaries that the framework owns. Tachyon has
more specific semantic information: styles, HTML templates, Tac islands, Yon
routes and View IR. A Tachyon update can therefore name the framework boundary
that is safe to replace. It does not have to treat every source file as an
arbitrary JavaScript module.

ADR 0009 rejects a production client renderer. Hot updates must not reverse
that decision without saying so. They must also not make application
correctness depend on a development reconciler.

## Decision

Introduce Hot Update Protocol v1. It is a contract from the server to the
client, and it exists only in development.

The development server uses:

- file system events from the operating system;
- a bounded queue of 1,024 events;
- a quiet period of 75 milliseconds.

One save in the editor produces one deterministic change set in canonical
order. Some cases widen the update to a full reload: queue overflow, watcher
uncertainty, mixed change kinds and unknown paths. In these cases the server
never guesses a narrower boundary.

A successful build publishes one of three actions over a same-origin SSE
(server-sent events) stream:

- `css` replaces the generated same-origin stylesheets after their new bytes
  load.
- `island` replaces only the named hydrated Tac component boundaries. It
  imports the companion by its digest address and restores compatible
  declared state.
- `reload` performs an ordinary document reload. It is used when structure,
  routing, Yon context, configuration or another unsupported boundary changed.

A failed build publishes `diagnostics`. The last good artifacts and DOM keep
running. The client shows Diagnostics v1 in a bounded, text-only overlay. The
next successful action clears the overlay. The client does not evaluate
source. A protocol message never carries executable text that a developer
wrote.

An island may implement `hotState()`, `restoreHotState(state)` and
`hotDispose()`. These methods give it an explicit contract for state and
cleanup. Without them, the runtime uses `structuredClone` on the enumerable
fields that are not functions. If that default clone fails, the runtime keeps
an empty object. The runtime also clones the result of an explicit
`hotState()` before it restores it. The client claims no JSON limit and no
depth limit for component state. Before replacement, the runtime aborts the
existing hydration `AbortSignal`. Listeners and asynchronous work can then
stop.

The replacement client also saves state that the browser owns. It takes a
snapshot of at most the first 2,048 elements that have an `id`. It saves:

- input and textarea values;
- the checked state of inputs;
- whether each `<details>` element is open;
- scroll offsets that are not zero;
- focus, by element id.

It does not keep text selection or the selected `<option>` state. It restores
these properties only after the new companion has activated. It never
transfers markup, attributes, event listeners, arbitrary DOM properties or
contenteditable HTML.

The server sends `X-Accel-Buffering: no`. No WebSocket upgrade is required.
The existing `/.tachyon/live` integer endpoint and the
`/.tachyon/live-reload.js` asset path stay available for compatibility. The
client uses `/.tachyon/hot` instead and no longer polls.

The server injects the hot-update client only while `ty dev` watches source.
`ty build` never writes it. It is never bundled into production output and
never used for navigation. Changes to static HTML and templates deliberately
cause a reload. They will keep doing so until a future fragment contract,
owned by the compiler, can prove stable identities. Native View IR updates need
a host transport that a separate decision accepts. This decision does not
claim them.

## Consequences

CSS and hydrated component behavior update without destroying unrelated page
or island state. A companion-only edit also keeps native form, disclosure,
scroll and focus state inside the replaced boundary. Compiler errors no longer
break the document or stay hidden in the terminal. Server-side and structural
changes keep an explicit and correct reload fallback.

The development runtime now owns a small island replacement path and a state
lifecycle. It does not own general DOM reconciliation, emulation of the import
graph or preservation of arbitrary side effects. An application with state
that is not JSON must declare its hot-state methods if it wants to keep that
state.

The event watcher adds the `notify` dependency, which has a permissive
license. It also adds platform-specific transitive dependencies under the
CC0-1.0 and ISC licenses. Source queues, protocol collections, diagnostic
text and SSE broadcast history are bounded. A client that
disconnects and misses an update receives a reload when it reconnects. It does
not receive a partial replay.

## Rejected Alternatives

- Porting the legacy SPA (single-page application) renderer would reverse ADR
  0009 and ship development complexity in production.
- Reproducing Vite's general ESM graph would duplicate a mature tool. It would
  also not use Tachyon's HTML and island semantics.
- Applying arbitrary HTML diffs could lose custom-element state, duplicate
  listeners and get out of sync with the server document.
- WebSockets add a two-way protocol and reverse-proxy configuration. This
  one-way update stream does not need them.
- Treating every successful build as able to hot-update would hide unsafe
  widening. It would also make state retention nondeterministic.

## Acceptance Gate

- [x] Filesystem events replace source-tree polling without publication loops.
- [x] Hot Update Protocol v1 has a canonical schema and positive/negative fixtures.
- [x] CSS changes replace styles without losing island state.
- [x] Island companion changes dispose, replace, reactivate, and restore declared state.
- [x] Island replacement retains bounded native form, disclosure, scroll, and focus state.
- [x] Invalid source retains the last-good page and displays structured diagnostics.
- [x] Structural changes use the full-reload fallback.
- [x] The client and update stream are absent when source watching is disabled.
- [x] Generated production output does not contain the hot-update client.
