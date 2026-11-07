# ADR 0105: Swift and Rust Browser Listeners May Be Async

- Status: Accepted
- Date: 2026-10-19
- Amended: 2026-10-24 (every language with async methods, and linked
  services; LS3)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (an async listener in
  a Swift or Rust browser service)
- Works with: [ADR 0035](0035-rust-services-reach-the-web-as-webassembly.md),
  [ADR 0082](0082-swift-services-reach-the-web-as-webassembly.md),
  [ADR 0022](0022-shared-behavior-adapters.md) (one Setup per page program)

## Context

A Swift or Rust service in the browser answers its HTTP clients and stores
only asynchronously. Its methods may be `async`, and the page runs each one
as a task of the service. Its `@OnEvent` methods could not be `async`: the
compiler refused every async native event method before the toolchain ran.

So a Swift or Rust listener could not read through a client. The cookbook
list could only copy the name from the event, while the Dart list reloads
every saved user through its client.

## Decision

- An `@OnEvent` method of a Swift or Rust service that runs in the browser
  may be `async`. A Rust listener may also return `BehaviorResult<()>`.
- The compiler gives such a listener a task entry. The service's event
  operation starts the task and answers `{"task": id}`, as an async method
  call does.
- The page sends the event of a task listener through the service's task
  queue. The listener tasks of one service run one after another, in the
  order their events arrive. A Rust listener task holds the service, so the
  page shows the fields it read before the task started, as it does for a
  Rust async method.
- The listener's promise goes back to the page publisher, which marks the
  page changed when the task ends and counts a failed task as a failed
  listener.
- Removing the service cancels the running task and the waiting ones.
- An async listener cannot return an event. It publishes through
  `applicationEventPublisher`. The compiler refuses a returned model.
- A Swift or Rust listener of a native app stays synchronous. Kotlin, C# and
  F# browser listeners stay synchronous too.

## Consequences

- The Swift and Rust cookbook lists reload every saved user through their
  own `UsersClient`, from the shared UsersSetup adapter, like the Dart list.
- `scripts/async-listener-browser-gate.mjs` builds the Swift and Rust
  cookbook example and checks in a browser that the list shows every saved
  name.

## Amendment (2026-10-24): every language and linked services (LS3)

The decision above left Kotlin, C# and F# listeners, and every listener of a
native app, synchronous. They were refused with TY1405. That rule is
replaced.

- An `@OnEvent` method may be async in every Tac service language that has
  async methods: Swift `async`, Rust `async fn`, Kotlin `suspend`, C# `Task`
  or `ValueTask`, and F# `Task<unit>`, `ValueTask` or `Async<unit>`. The
  compiler wraps an F# `Async` in a task. An F# listener needs its return
  type written out, or it stays synchronous.
- The rule is the same in the browser and in a native app. An async listener
  still cannot return an event. A C# `async void` listener is refused,
  because nothing can wait for it. Lifecycle methods stay synchronous.
- In the browser, Kotlin, C# and F# listeners run as tasks through the
  service's task queue, as Swift and Rust ones do.
- In a native app the listeners of one service run one after another. A
  later listener, async or not, waits until the running async one ends.
  - Swift and Kotlin wait for the listener inside the call.
  - C# chains the listener after the one before it and finishes it on the
    thread pool under the owner lock, then drains the events it published.
  - Rust queues the listener behind the running one and finishes it through
    the host's task dispatch (`host-resume`), without a task answer to the
    page. While a Rust task holds the service, a call or field read answers
    `{"code":"BUSY"}` instead of failing, and the native page bridge retries
    it for up to ten seconds. This also covers a Rust async method.
  - On macOS and iOS, a linked Rust listener that waits for HTTP reaches
    `tac.yon.origin` through the hosts' task dispatch and `URLSession` sender
    since LR1 (2026-10-24, ADR 0103 amendment); before that it answered
    `HTTP_MISSING` there.
  - Disposing the owner cancels the running listener and drops the waiting
    ones.

Runs on 2026-10-24 (develop `e0bbb86a` plus LS3):

- `scripts/async-listener-browser-gate.mjs` passed for Swift, Rust, Kotlin,
  C# and F# in Chromium, Firefox and WebKit (30 checks), in the Linux cloud
  container. The C# and F# cookbook lists now reload every saved user through
  their `UsersClient`, like the other languages.
- `native_rust_async_listeners_run_in_order_through_the_host_dispatch`
  (Linux) checks the order, the `BUSY` answers and the cancel on dispose.
- `a_linked_swift_async_listener_finishes_before_the_next_listener` builds
  a staged Swift companion with Swift 6.3.3 on Linux and runs it.
- `native_csharp_async_listeners_wait_for_each_other_in_source_order` runs a
  staged C# companion under .NET 10 on Linux.
- `kotlin_native_suspend_listeners_run_in_source_order` runs an Android-style
  Kotlin companion on the JVM.
- Host runs, each with a 400 ms async listener and a plain one, two presses,
  and the page showing `a1s1` and then `a1s1a2s2`:
  - macOS 26.6.2 on the Mac (Xcode 27.1, Swift 6.4, Rust 1.97.1, develop
    `cb18afdb8`): `swift_listener_tests` and `native_rust_task_tests`
    passed, and `scripts/native/async-listener-app-test.mjs` passed with a
    linked Swift listener in the macOS app.
  - iOS 26.5 simulator (iPhone 17 Pro, same Mac and commit): the same script
    passed in the simulator app.
  - Android 15 emulator (API 35, arm64): the script passed with a linked
    Kotlin `suspend` listener in the packaged app, after a fix. A Kotlin
    suspend method or listener that really suspends now holds the call until
    it resumes, as Swift's does. Before, it answered `HTTP_MISSING` at once,
    and the listener finished later with no render.
  - Windows 11 Home ARM64 on the Surface (build 26200, MSVC 19.44.35229
    ARM64, Rust 1.97.1, .NET 10.0.302, develop `519f3bee`, no local
    changes): `csharp_native_task_tests` and `native_rust_task_tests`
    passed over SSH, and `scripts/windows/async-listener-native-test.mjs`
    passed with a linked C# `Task` listener in the WebView2 app on the
    signed-in desktop. The first run, on `ddf568e8`, needed a fix: a
    finished C# listener task now sends the owner's `changed` signal, so
    the page reads the fields again. Before, the page kept the old values.
    On 2026-10-25 (SW1, Windows 11 25H2 build 26200, WebView2 Runtime
    155.0.4283.62) the same script passed with a linked Rust `async fn`
    listener as well: the page showed `[a1s1]` with the app's process ID,
    then `[a1s1a2s2]`.
- Linux (x86_64 Ubuntu 24.04 cloud container, GTK 4.14.5, WebKitGTK
  2.52.6, Xvfb): `scripts/linux/async-listener-native-test.mjs` passed with
  a linked Rust `async fn` listener in the Linux app (`[a1s1]`, then
  `[a1s1a2s2]`). A finished Rust listener task now sends the owner's
  `changed` signal too.
- macOS 26.6.2 and the iOS 26.5 simulator, 2026-10-25 (MA1, branch
  `100f335e`, Xcode 27.1): `TAC_LISTENER_LANGUAGE=rs
  scripts/native/async-listener-app-test.mjs` passed with a linked Rust
  `async fn` listener in the macOS app and in the simulator app. Each press
  waited about 430 ms, and the listener ran in the app's process.
- Android 15 emulator, 2026-10-25 (MA2, API 35 arm64, WebView 124):
  `TAC_ANDROID_LISTENER_LANGUAGE=rs scripts/native/async-listener-app-test.mjs
  --target=android` passed with a linked Rust `async fn` listener in the
  packaged app. Each press waited about 430 ms, and the listener ran in the
  app's process.
