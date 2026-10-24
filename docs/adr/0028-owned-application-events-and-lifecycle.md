# ADR 0028: Owned Application Events and Lifecycle

- Status: Accepted, with `EventPublisher` removed and the Spring event model
  adopted by [ADR 0031](0031-realms-mirror-each-other.md); implementation
  qualification in progress
- Date: 2026-09-15
- Amends: ADR 0020/0022 lifecycle conventions, ADR 0027 browser owner API
- Preserves: client-only Tac, REST-only Yon, explicit remote topic/SSE delivery
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Decision

Tachyon replaces the public browser `this.tac` object with method annotations
and named resources that are bound to an owner. PostConstruct and PreDestroy
are instance hooks with zero arguments. EventPublisher publishes the return
value of a method that succeeds. EventListener receives exactly one local
event payload. These modifiers do not change class, filename or folder
naming. The existing role naming rules stay.

Extraction works from source only, has limits, and the language backends
share it. Its metadata is separate from cache/HTTP behavior contexts.
Application events never require an authored Behaviors context, constructor
or Inject list. Unknown forms and unsupported asynchronous declarations fail
before execution. They never silently become annotations that do nothing.
Tachyon adds no Spring runtime and no reflection package.

In browser JS/TS, Resource fields select one of these: element, signal,
cacheManager, resourceLoader or renderScheduler. Resources are:

- injected after construction;
- excluded from RequiredArgsConstructor dependency inference;
- non-enumerable in state snapshots;
- bound to the current owner.

No resource grants native capabilities, server configuration, credentials or
cross-origin transport.

Listeners register before initialization. Tac initializes after its owner DOM
exists. This is a view lifecycle on purpose. It does not copy a Spring bean
constructor literally. Stable component roots keep their owner identity
across rerenders. Replacing an owner runs cleanup and creates a new lifetime.

Native companions carry a fresh activation ID through calls and
publications. Old (stale) owners cannot affect a replacement page. Native
publication queues drain after the current call releases its mutable borrow.
The initial native feature set is synchronous. It adds no background-thread
delivery that lacks an owner.

Yon owns events inside the provider graph of one invocation. It does not own
them globally across requests, workers or processes. Generated containers
register participating listeners, initialize dependencies, dispatch, drain
owned work and clean up in reverse order. The existing supervisor deadline
stays the final limit. An event does not prove a transaction commit, a
successful durable delivery, or a database/cache rollback. A forced
termination cannot guarantee that user cleanup hooks run.

Payloads are finite JSON snapshots. Each payload is limited to 64 KiB
(kibibytes) and cloned for each listener. Retention and replay are
local-memory policies that an application must opt into. Tachyon limits each
of these separately: names, listeners, queued work, recursion and operation
lifetime. Listener failures stay observable. They do not replace a
successful publisher result. Tachyon adds no implicit network forwarding,
cross-tab bus, event broker or durable queue.

## Upstream review

Reviewed Spring Framework source on 2026-09-15:

- [CommonAnnotationBeanPostProcessor](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/annotation/CommonAnnotationBeanPostProcessor.java)
  supports Jakarta PostConstruct, PreDestroy and named Resource injection.
- [EventListener](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/event/EventListener.java)
  separates method listeners, ordering and asynchronous failure behavior.
- [ApplicationEventPublisher](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/context/ApplicationEventPublisher.java)
  is a publishing interface. It is not an EventPublisher annotation.
- [CacheManager](https://github.com/spring-projects/spring-framework/blob/main/spring-context/src/main/java/org/springframework/cache/CacheManager.java)
  and [ResourceLoader](https://github.com/spring-projects/spring-framework/blob/main/spring-core/src/main/java/org/springframework/core/io/ResourceLoader.java)
  informed the separation of caching and resource loading. Tachyon's browser
  service methods are separate contracts. They do not implement these Java
  interfaces.

EventPublisher is specific to Tachyon. DOM access and rendering have no
direct Spring equivalent. The implementation was written independently. It
does not copy Spring internals. It does not claim API or performance parity
with Spring.

## Acceptance

Acceptance requires:

- source diagnostics;
- both TypeScript decorator modes;
- browser lifecycle and HMR (hot module replacement) interaction tests;
- native activation and stale-owner tests;
- real Yon execution across all eight languages.

Migrate the runnable website examples and verify them against the new
compiler. Record the actual execution evidence and limitations separately
from this decision. Do not infer release qualification from source staging
or compilation alone.

## Amendment (2026-10-18): the application goes to the background and comes back

A service can now hear when the whole app goes to the background and when it
comes back. Document visibility (`VisibilityChanged`, ADR 0065) stays as it
is: it is about the page, not the app.

1. **One built-in event model.** `ApplicationStateChanged { state }`, where
   `state` is `background` or `foreground`, joins the built-in event models
   (`route_types::WebEventModels`). Like the others, it is never declared in
   `client/models/`, and `WebEventSources` writes its type for each language.
   A listener is a normal `@OnEvent` listener with a parameter of this type.
   Python services (ADR 0093) get it too, although they have no browser
   APIs: when a unit names it, the bridge unit declares it as a dataclass.
2. **What sends it.**
   - macOS: `NSApplication` did resign or become active, did hide or unhide,
     and `NSWindow` did minimize or restore (`AppleApplicationState`).
   - iOS: `UIApplication` did enter the background and will enter the
     foreground. The host asks for a short background task, so the page and
     the services can finish hearing the change.
   - Windows: `WM_ACTIVATEAPP`, and `WM_SIZE` with `SIZE_MINIMIZED`, or a
     restored size while the window is in front.
   - Linux: GTK `notify::is-active` and `notify::suspended` on the window. The
     app is in the background when no app window is active, or every active
     window is suspended. The check waits for the next idle moment, so moving
     the focus to an app dialog does not send `background`.
   - Android: `onStop` and `onStart`. The plan named `onPause` and
     `onResume`, but `onPause` also runs when a permission dialog covers the
     activity, while the app is still in front.
   - A browser tab has no application state. The page sends `background`
     when the tab is hidden and `foreground` when it is shown.
   Each host keeps the last state and sends only changes. It records
   `application.background` and `application.foreground` in its lifecycle
   log, and sends the current state again when a page finishes loading
   (except Windows, which sends only changes).
3. **How it reaches the page.** The native shim adds
   `__tachyonApplicationState(state)`. The host calls it (Windows posts
   `{ applicationState }` to the page, and the injected script calls it). It
   ignores an unknown or repeated state and dispatches `tac.application.state` on
   `window` with `detail.state`, so plain page scripts can listen too. The
   page runtime (`TacWebApis.watch`) publishes the model for each known state
   and keeps the last one, so a `replay` listener hears the current state
   first. A native page starts in `foreground`.
4. **How it reaches natively linked services.** A natively linked Rust,
   Swift, C# or Kotlin service may name `ApplicationStateChanged`: `TY1317`
   no longer refuses this one model, because it belongs to the app, not to
   the page. When the page binds a native owner, it also registers that
   owner's listeners for this model, and each delivery is an `event` call to
   the owner (`op: "event"`, which every host now passes on). Swift and
   Kotlin on native targets describe only these listeners; other page events
   still do not reach native listeners. Native Swift and Kotlin companions
   get the model's type when their source names it.

Checks: `application_state_tests` (the shim and the page runtime in Node, and
an Android host and Kotlin companion compiled against Android platform 35),
host source tests for all five hosts, a MinGW-w64 compile of the Windows
state code, and `scripts/linux/application-state-test.sh`, which builds a
Linux app with a page service and a linked Rust service and, under Xvfb and
xfwm4, moves the focus to another window and back and minimizes the app. The
page and the Rust service each heard `foreground`, `background`,
`foreground` and `background` (2026-10-18, x86_64 Ubuntu 24.04 cloud
container, GTK 4.14.5, WebKitGTK 2.52.6). Not run yet: the macOS and iOS
hosts (the Swift is not compiled in this run), the Windows host inside
WebView2 on a signed-in desktop, and Android on an emulator.

## Amendment (2026-10-24): background and foreground on the iOS simulator (RM2)

`scripts/ios/page-checks-test.mjs` sent the app to the background on the iOS
26.5 simulator by launching another app, and brought it back by launching it
again. A linked Swift service's `ApplicationStateChanged` listener and a page
service both heard `background` and then `foreground`, and the host telemetry
recorded each switch. Disposal on termination was not seen: `simctl
terminate` kills the app, so neither `applicationWillTerminate` nor a Swift
`@OnStop` runs.

## Amendment (2026-10-24): background and foreground ran on Android

This amendment supersedes "and Android on an emulator" in the "Not run yet"
list of the 2026-10-18 amendment above, which stays as the history of this
decision.

`scripts/android/page-checks-test.mjs` ran on the Android 15 (API 35, arm64) emulator on 2026-10-24, on develop `4c052177` and again after a rebase onto `259818ee` (RM3, `docs/SUPPORT_TIERS.md`, "Android runs owed, RM3"). A JavaScript
page service and a linked Kotlin service each heard `foreground` (replayed),
then `background` when Home stopped the activity (`onStop`), then
`foreground` when it started again (`onStart`). The support table marks the
Android background and foreground signal `honored`.

## Amendment (2026-10-24): the macOS host on a Mac (RM1)

This amendment narrows "Not run yet: the macOS and iOS hosts" above, which
stays as the history of this decision. `scripts/application-state-macos-test.mjs`
builds a macOS app with a page service, a linked Rust service and a linked
Swift service, and drives it through Accessibility with
`scripts/application-state-macos-control.swift`. On a Mac (macOS 26.6.2,
Xcode 27.1, develop `4c052177`) all three heard `background` and
`foreground` when another app became active and the app came back, when the
app was hidden and shown, and when its window was minimized and restored.
