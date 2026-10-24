# ADR 0089: Native Capabilities Are Activation-Owned Service Providers

- Status: Accepted. The FilePicker works on macOS for JavaScript, TypeScript, Swift and Rust services, and on Windows for JavaScript, TypeScript and linked Rust and C# services (2026-10-19 amendment). C# and Kotlin services that run as WebAssembly receive the injected providers, but their FilePicker answers `unsupported`; Linux, iOS and Android have no picker.
- Date: 2026-09-30
- Relates to ADR 0025 (NeedsAll), ADR 0030 (native services), ADR 0065 (page APIs), ADR 0087 (WindowControls)

## Decision

A service receives OS capabilities through existing Needs/NeedsAll injection.
Platform imports and calls belong to adapters. There is no second application
container. JavaScript/TypeScript receive a `filePicker` activation resource;
compiled services receive a required `FilePicker` field through their generated
service construction. Native `WindowControls` uses the same construction seam
and retains `tac.application.window.controls` grants.

The additive manifest field `tac.nativeCapabilities` accepts `filePicker` once.
Grants are compiled into the target adapter and page bootstrap from that manifest.
The v3 native-host descriptor keeps its existing schema; it does not duplicate
these manifest grants in an undeclared `nativeCapabilities` field.
It is separate from `tac.browserApis`. Missing grants return `denied` before an
OS picker opens. The host also checks the grant on each read. This grant does
not replace OS user consent or signing entitlements. macOS uses NSOpenPanel's
user-selected file access; a sandboxed distribution also needs the platform's
user-selected read-only entitlement. Packaging does not silently enable a
sandbox or add arbitrary filesystem entitlements.

## FilePicker contract

`available()` returns a boolean. `open({ multiple: false, types: [] })` selects
one file. The only accepted MIME filters are `text/plain` and
`application/json`; an empty list permits any single file, while `readText`
requires UTF-8. `readText(handle)` reads at most 65,536 bytes;
`release(handle)` ends access. JavaScript returns Promises. Swift methods are
synchronous and return Codable FilePickerResult with FilePickerStatus.

Results contain `status`, and optionally `handle`, `name` or `text`. Status is
one of `available`, `selected`, `cancelled`, `read`, `released`, `denied`,
`unsupported`, `unavailable`, `invalid`, `disposed`, `tooLarge`. Failure data
never contains a host path or platform exception text. A selection exposes
only its basename and opaque handle. It grants no directory traversal,
arbitrary path opening, write, persistent bookmark or cross-activation access.

A handle belongs to one owner, expires after 300 seconds, and is released on
owner disposal, navigation or host termination. Limits are eight handles per
owner and 128 per host. One panel may be active; a second request returns
`unavailable`. The picker cancels after 30 seconds. Cancellation returns
`cancelled` rather than throwing. Permission revocation clears the owner's
handles when it next invokes the adapter. Reads open the selected URL without
loading more than the content limit plus one byte.

## Registration and tests

The native host registers its adapter during application startup. The existing
Swift service factory establishes the constructing owner, supplies providers,
and removes their capability lifetime when the activation is disposed.
Each provider retains the activation scope, not an owner-name membership test:
reusing a disposed owner's name cannot revive its earlier providers. Disposal
uses that scope's captured adapter even after another adapter is registered.
TestSetup/fake code can replace the provider transport before constructing a
service. Rust captures its shared provider scope while the normal Default-based
service factory runs. Its native WindowControls uses that scope; the existing
WebAssembly WindowControls keeps its page-host transport. The generated provider keeps that transport, so a later registration
cannot silently replace an existing activation's dependency.

The macOS WK message bridge requires the main frame, local `tachyon-app://bundle`
origin and bounded canonical JSON before dispatch. The native companion adapter
uses its existing owner-thread execution path. A browser tab returns
`unsupported`; no browser picker is substituted. Other native hosts must
register an adapter explicitly before advertising support. macOS window actions
target the AppDelegate-owned window registered with NativeBridge, including when
the app is in the background or a modal picker has focus. They return unavailable
when no owned window exists; they never select another application's key window.

Rust's optional `tac_native_set_capability_adapter` C ABI registers a paired
call/free callback. Call receives borrowed, NUL-terminated operation and JSON
strings for the duration of the call. It returns a host-owned allocation and
byte length; Rust copies at most 1,048,576 UTF-8 bytes and invokes free exactly
once for every non-null answer. Null, empty, excessive or invalid UTF-8 replies
become `unavailable`. Request JSON is limited to 65,536 bytes and operation names
to 64 bytes. The host guarantees that its returned pointer is valid for the
reported length and that the paired free function accepts the original pointer
and length. The macOS host implements both callbacks on the owner thread; no
Rust allocator frees Swift memory. Registration without both callbacks selects
the unsupported adapter. This generated FFI boundary contains the unsafe slice
read; framework implementation remains subject to the repository unsafe ban.

## Support and remaining slices

| Target / service | Contract | Evidence required |
| --- | --- | --- |
| macOS JavaScript/TypeScript | Needs filePicker resource, Promise results | provider and generated app execution, real panel selection/cancel |
| macOS compiled Swift | Needs/NeedsAll FilePicker and WindowControls | generated companion compile/execution with fake, native host execution |
| Browser tab | typed unsupported FilePicker result | provider execution |
| FilePicker on iOS / Android / Linux | typed unsupported; no platform picker rollout | no OS picker interaction claim |
| Windows JavaScript/TypeScript, linked Rust and C# | `IFileOpenDialog` behind the same adapter (2026-10-19 amendment) | adapter and Wine dialog runs; MSVC and WebView2 page runs owed |
| macOS compiled Rust | NeedsAll default providers, captured registration and native WindowControls | compiled fake/unsupported/disposal, static-library host link and actual owned-window title/resize/grant test |
| Rust WebAssembly | existing page WindowControls, unsupported native FilePicker | generated wasm32 compilation |
| Swift / C# / Kotlin WebAssembly | injected FilePicker that answers unsupported; existing page WindowControls | `ty bundle` web and non-linking native targets |
| Native C# / Kotlin | Needs/NeedsAll captured providers, typed unsupported FilePicker (linked C# on Windows reaches the Windows picker since the 2026-10-19 amendment), Windows/Android WindowControls registration | actual compiled activation/fake/disposal tests, Windows stub-host dispatch execution and complete Android API35 typecheck |

Later C# and Kotlin registration tests and four native example checks passed.
They replace the earlier pending C# and Kotlin registration status. Windows
desktop and Android device interaction are still not tested. (Later, an ignored
Windows test that needs MSVC was added. It compiles the generated Windows
control code and drives a real Win32 window with it: title, resize,
fullscreen, minimize, a refused close and a refused call from another thread.
It does not open the full app or WebView2.) On macOS, a test
selects a file, revokes access, grants it again, and checks that the old file
handle is refused.

The platform matrix is not expanded merely because the macOS source compiles.
An OS interaction claim requires the actual native picker, selection, bounded
read and cancellation checks on that OS. UI automation permissions are recorded
as blockers if the host does not permit that check.

## Amendment (2026-10-04): the page answers `unsupported` through the same field

A Swift, C#, Kotlin or Rust service that runs as WebAssembly in the page
receives its `FilePicker` field the same way a linked service does. The build
gives the field the typed provider, and every call answers `unsupported`
(`available()` is false; options that break the contract still answer
`invalid`). It is no longer a property that the page must pass, so a
`NeedsAll` service with a `FilePicker` field builds for the web and for every
target where the language is not linked: Swift on Linux, Windows and Android,
C# off Windows, Kotlin off Android. `WindowControls` in the page keeps its
page-host transport. The Kotlin component types read the same provider list,
so a Kotlin `FilePicker` field also builds for `--target android --native`.

A Lua service has no provider for either capability. A `---@type FilePicker`
or `---@type WindowControls` annotation in a Lua service fails the build with
`TY2015`, like the other Lua resource refusals, instead of leaving the field
nil at run time. Go, Java, Ruby, Python and F# page services still have no
FilePicker provider; their `NeedsAll` field remains a page property.

## Amendment (2026-10-18): the Linux host runs the controls on its own window

An ignored Linux test,
`linux_window_control_tests::linux_generated_host_drives_granted_window_controls_and_refuses_a_denied_one`,
compiles the generated GTK host with a Rust page service that has a
`WindowControls` field and grants `title`, `resize` and `minimize`. The page
calls the service through the real WebKit bridge. The test checks that
`granted` answers true only for those three, that a title with non-ASCII
text reaches the GTK window, that a resize to 800x600 changes the window's
real size, that a size under 64 is refused, and that minimize sets the
window's minimized state. A `close` without its grant answers `denied` and
the window stays open. The host also refuses `window.close` and
`window.fullscreen` when they are called on it directly. A title longer than
1024 bytes is refused by the host, and the Rust field reports that as
`unavailable`.

On 2026-10-18 it passed under Xvfb with the xfwm4 window manager on WSL
Ubuntu 24.04 (aarch64, GTK 4.14.5, WebKitGTK 2.52.6). Without a window
manager, X11 has nothing to minimize the window, so the test fails there. A
physical Linux desktop, Wayland and x86_64 are not covered. The Linux
`FilePicker` still answers `unsupported`.

## Amendment (2026-10-18): a test holds the macOS handle limits

`MacOsFilePicker` reads the time through its `clock` field, and `keep` stores
a selected file without the panel. `open` still checks every limit before it
shows the panel, so `keep` adds no path around them. The macOS test
`native_file_picker_limit_tests` compiles the picker with a caller that
fills all 128 handles (16 owners with 8 each) and checks that the next
`open` answers `unavailable` without adding a handle, that a ninth handle for
one owner is refused, and, with a fake clock, that a handle still reads at
299 seconds and at 300 seconds answers `invalid` and is removed.

## Amendment (2026-10-19): the Windows host opens the file dialog

The Windows host answers `filePicker.` calls itself instead of
`unsupported`. The page reaches it through the WebView2 message bridge, and
linked Rust and C# services reach it through
`tac_native_set_capability_adapter`, which the host already registered. The
adapter keeps the macOS rules: the owner must be a UUID, the grant comes from
`tac.nativeCapabilities` and is checked before the dialog opens and again
when it closes, at most 8 handles per owner and 128 per app, a handle ends
after 300 seconds, one dialog at a time, and the dialog closes itself as
cancelled after 30 seconds. `readText` reads at most 65,537 bytes and
answers `tooLarge` past 65,536, and `invalid` for text that is not UTF-8.
Answers carry the file name and an opaque handle, never the path. Every
call must come from the window's thread; another thread gets `unavailable`.
Navigation, window close and the owner's `dispose` remove the owner's
handles and close an open dialog. The page also sees
`__tachyonNativeFilePicker` and the grant list, as on macOS.

The dialog is `IFileOpenDialog` with `FOS_FORCEFILESYSTEM`,
`FOS_FILEMUSTEXIST`, `FOS_NOCHANGEDIR` and `FOS_DONTADDTORECENT`, and one
selection. `text/plain` becomes the filter `*.txt;*.text`, and
`application/json` becomes `*.json`. The contract has no save operation, so
the host uses no `IFileSaveDialog`.

WebView2 does not allow a modal dialog inside one of its event handlers. So
`WebMessageReceived` now only queues the page message and posts
`TACHYON_WM_PAGE_MESSAGE`; the window procedure answers queued messages in
order. While a dialog is open, the host holds new page messages and
companion task resumes, and answers them in order after the dialog closes.
This keeps a linked service from being called again while it waits for its
own `open`. Navigation drops messages that are still queued. The page waits
40 seconds instead of 10 for `filePicker.open` and `companion.invoke`,
because a dialog can stay open for 30 seconds.

`windows_file_picker_tests` holds the evidence. Generated-source tests
check the routes, the grant, the queue and the disposal. On 2026-10-19 the
picker, built with MinGW-w64, ran under Wine 9.0 and Xvfb in an x86_64
cloud container. With a test chooser it checked every status, the escaped
UTF-8 text, the 65,536-byte limit, the handle limits and the 300-second
expiry with a fake clock, a call from another thread, and that page messages
and a task resume wait for the dialog and keep their order. It also drove
Wine's real `IFileOpenDialog`: a helper typed a file name into the dialog,
the file was selected and read back, and an unanswered dialog closed as
`cancelled`. The whole generated host also passed a MinGW-w64 syntax check
against the WebView2 1.0.3240.44 headers. The MSVC build and a run of the
Windows dialog are `#[ignore]` tests for Windows. A picker opened from a
WebView2 page needs a signed-in desktop (SD1). On 2026-10-24 (SD1) the MSVC
dialog run passed on the signed-in Windows 11 ARM64 desktop: the typed file
was selected and read, and an unanswered dialog closed after its patience.
That run found that a patience close landing while the dialog was still
starting was ignored, so the host now repeats `IFileDialog::Close` every
250 ms and posts Cancel to the dialog window until `Show` returns.

## Amendment (2026-10-19): the Linux host opens the GTK file dialog

The Linux host now answers `filePicker` calls itself instead of answering
`unsupported` (plan item FP3). JavaScript and TypeScript page services reach
it through the host bridge, and linked Rust services through the same C
capability adapter as `WindowControls`. The host follows the macOS rules:

- Without `tac.nativeCapabilities: ["filePicker"]` every call answers
  `denied` and the caller's handles are released before any dialog opens.
- `open` shows `GtkFileDialog` for one file. GTK uses the desktop portal's
  file chooser when one runs (as on GNOME and KDE) and its own dialog
  otherwise. The types are only `text/plain` and `application/json`, at most
  two. One dialog is open at a time; a second `open` answers `unavailable`.
  The dialog is cancelled after 30 seconds, when its owner is disposed, when
  the window closes, and when the app quits.
- A selection gives an opaque handle and the file name. There are at most
  128 handles, 8 for one owner, and each ends after 300 seconds.
- `readText` reads at most 65,536 bytes; a larger file answers `tooLarge`
  and text that is not UTF-8 answers `invalid`.
- `dispose` releases one owner's handles. A new page and shutdown release
  them all.

The page shim now sets `__tachyonNativeFilePicker` and the manifest grants
on Linux too, so `available()` is true there when the grant is present.
Swift, C# and Kotlin services still run as WebAssembly on Linux; their
`FilePicker` fields keep answering `unsupported` until FP5 routes them
through the page bridge.

The ignored test
`linux_file_picker_tests::linux_generated_host_opens_the_gtk_file_dialog_for_a_page_and_a_rust_service`
compiles the generated host with a JavaScript page provider and a Rust
service, and drives the real dialog with XTest key presses (Ctrl+L, paste
the path, Return, or Escape). The page chose, read, released and cancelled;
the Rust service chose and read a JSON file; a second dialog was refused
while one was open. The same run checks the limits, the 300-second lifetime
with a fake clock, another owner's handle, a file over 65,536 bytes, a file
that is not UTF-8, the missing grant, and that a new page releases every
handle. On 2026-10-19 it passed under Xvfb in an x86_64 Ubuntu 24.04 cloud
container (GTK 4.14.5, WebKitGTK 2.52.6): three runs through the desktop
portal's chooser (xdg-desktop-portal 1.18.4 with xdg-desktop-portal-gtk
1.15.1, xfwm4), and one run each through GTK's own dialog
(`GDK_DEBUG=no-portals`) with and without a window manager. The same day it
passed on WSL Ubuntu 24.04 (aarch64, GTK 4.14.5, WebKitGTK 2.52.6, Xvfb with
xfwm4) once through GTK's own dialog and once through the portal's chooser.
The portal run needs the session bus started inside `xvfb-run`; started
outside it, the portal backend has no display and GTK falls back to its own
dialog. One WSL run in that broken-portal setup, while a gvfs volume monitor
timed out for about 54 seconds, reached the 30-second limit before a dialog
showed and answered `cancelled`; the rerun passed. Not covered: a physical
Linux desktop, Wayland, and a person choosing the file.

## Amendment (2026-10-24): WebAssembly services reach the desktop picker through the page

Swift, C# and Kotlin services run as WebAssembly in the page wherever the
desktop app does not link the language: C# and Kotlin on macOS, Swift and
Kotlin on Windows, and all three on Linux. Their `FilePicker` fields used to
answer `unsupported` there (FP5). They now reach the same host picker as a
page service.

The `FilePicker` API in these languages is synchronous, and the page bridge
is not. So the page runtime (`tac-native.js`) sends each call as a
synchronous `POST` to the app's own origin, at
`/__tachyon/capability/filePicker.<operation>`, with the header
`X-Tachyon-Capability` naming the same capability. The host shows the
dialog, then answers the request with the call's JSON answer. Each host
accepts only a `POST` from its own trusted page, with a matching header, an
operation of 64 bytes or less, and a body of 65,536 bytes or less:

- macOS: the `tachyon-app` scheme handler answers on the main thread and
  drops the answer for a request WebKit has stopped.
- Linux: the WebKitGTK `tachyon` scheme handler answers from an idle
  callback.
- Windows: `WebResourceRequested` takes a deferral and posts
  `TACHYON_WM_CAPABILITY`, and the window procedure answers it, because
  WebView2 forbids modal UI inside its events. One call runs at a time; a
  call while page messages are busy answers `unavailable`, and the window
  closing completes a waiting call with `unavailable`.

Each host's page script sets `__tachyonNativeFilePickerRoute`; without it,
the page answers `unsupported` as before, so a browser tab and the iOS and
Android apps are unchanged. Without the grant the page answers `denied`
before any request. `available` answers in the page. The page gives each
(language, owner) pair its own owner id, so two languages never share
handles, and `dispose` sends nothing for an owner that never called.

C# and Kotlin keep their `NativeCapabilities.Call` hook; in the WebAssembly
build it sends through `TacWebHost` `nativeCapability.call`. Swift's
WebAssembly prelude keeps the owners (`TacPageCapabilities`) and sends the
same way. The same work fixed C# type filters: `JsonArray.Add` of a string
needed reflection that trimmed .NET removes, so `open` with types failed
with "Native companion operation failed", linked or not. It now adds a
`JsonValue`.

On 2026-10-24, `scripts/linux/page-services-native-test.mjs` built the
`native-capabilities` feature with `ty` for each of Swift (6.3.3, WebAssembly
SDK), C# (.NET 10.0.302) and Kotlin (2.1.10) and ran the packaged Linux app
under Xvfb with xfwm4 in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5,
WebKitGTK 2.52.6). In each, the service opened GTK's dialog, which took a
typed path, and read the file; a second `open` was cancelled with Escape and
answered `cancelled`. The generated Windows host compiled with MinGW
against the WebView2 1.0.3240.44 headers, and the Wine picker checks
passed. Owed: a Swift and a Kotlin field from a WebView2 page on a
signed-in Windows desktop.

Also on 2026-10-24, on a Mac (macOS 26.6.2), `ty` built the same feature
for macOS with the C# and with the Kotlin service, both running as
WebAssembly. In each app the service opened `NSOpenPanel`, which took a
typed path, and read a UTF-8 text file back with its non-ASCII characters
intact; a second `open` was cancelled with Escape and answered `cancelled`
(`Cancelled` in C#). WKWebView's `WKURLSchemeHandler` receives the
synchronous request with its body, so the page route needs no change on
macOS. A synchronous request to a custom scheme is proven on WebKitGTK and
WKWebView. The same day, on the Surface (MSVC 19.44 ARM64), `ty build
--target windows` with the Kotlin service stopped with C2197: MSVC's
`shlwapi.h` replaces the `COBJMACROS` `IStream_Read` macro with its own
three-argument function, so the page route now calls `Read` through the
stream's vtable. MinGW keeps the macro, which is why the cloud build
passed.

Later on 2026-10-24, `scripts/windows/page-services-native-test.mjs` gained
FilePicker callers (`TAC_WINDOWS_PICKER_CALLERS`, default JavaScript,
Kotlin and C#, with Swift added when `TAC_SWIFT_WASM_SDK` is set) and
`scripts/windows/page-steps-probe.ps1` a `pick` step: it clicks the control,
waits for the app's `IFileOpenDialog` window, types the path into its file
name box and presses Enter, or presses Escape. Started through
`scripts/windows/desktop-run.ps1` in the signed-in desktop session of the
Windows 11 ARM64 Surface (build 26200.9711, WebView2 Runtime
155.0.4283.62, MSVC 14.44 ARM64, WebView2 SDK 1.0.3485.44) on develop
`e6d26623`, it built the `native-capabilities` feature for Windows with each
service. The JavaScript page service, the Kotlin service running as
WebAssembly (which sends through the page route) and the natively linked
C# service each opened the dialog, read the chosen file and answered
`cancelled` when the next dialog was closed with Escape. WebView2's
`WebResourceRequested` handler receives the synchronous request with its
body. Swift on Windows was not built there, because that machine has no
Swift toolchain.

On develop `0727eb7e`, `scripts/linux/page-services-native-test.mjs` also
passed on WSL Ubuntu 24.04.5 (aarch64, GTK 4.14.5, WebKitGTK 2.52.6, Xvfb
with xfwm4): the Swift (6.3.3), C# (.NET 9.0.317) and Kotlin (2.1.10)
WebAssembly services each chose, read and cancelled through GTK's dialog.
It ran with `GIO_USE_VFS=local` and `GIO_USE_VOLUME_MONITOR=unix`, because
GVfs there waited 25 seconds for UDisks2 and the first run's dialog reached
the 30-second limit and answered `cancelled`.

## Amendment (2026-10-24): the iOS and Android apps open the document picker

FP4 and FP2 give the mobile hosts the desktop picker contract: UUID owners,
the manifest grant, 8 handles per owner and 128 per app, 300-second handles,
a 64 KiB UTF-8 `readText`, `text/plain` and `application/json` filters (an
empty list allows any file), one picker at a time that is cancelled after
30 seconds, and disposal when the owner goes away, the page navigates or the
app closes.

**iOS.** `capabilities/IosFilePicker.swift` presents
`UIDocumentPickerViewController(forOpeningContentTypes:asCopy: false)` over
the root view controller and keeps the picked URL with its security-scoped
access until the handle is released or expires. The picker answers through a
completion, so the page bridge (`WKScriptMessageHandlerWithReply`) and the
page route (`capabilities/IosCapabilityRoute.swift`, the iOS copy of the
macOS `/__tachyon/capability/` route) reply once it closes, without
blocking the main thread. Linked Swift and Rust services call the host
synchronously, as on macOS, so `callNow` runs the main run loop until the
picker answers; the 30-second timer runs in that loop, so the call always
returns. C# and Kotlin services run as WebAssembly on iOS and reach the
picker through the page route.

**Android.** `native_file_picker_android.java` starts
`ACTION_OPEN_DOCUMENT` (`CATEGORY_OPENABLE`) with `startActivityForResult`
on the UI thread and waits for `onActivityResult` on the calling thread:
the bridge worker for the page, linked Kotlin and linked Rust services, or
the web view's request thread for the page route. A call on the UI thread
answers `unavailable`. The display name comes from `OpenableColumns` with
path separators replaced. `shouldInterceptRequest` cannot read a request
body, so the page runtime now sends each route request a second time,
URI-encoded, in an `X-Tachyon-Capability-Request` header, which the Android
route reads; the other hosts still read the body. The page bridge waits 60
seconds for `filePicker.open` (10 for other calls), and a FilePicker answer
may be up to 786,432 bytes, the limit the service decoders already use, so
a full 64 KiB read fits. Swift and C# services run as WebAssembly on
Android and reach the picker through the page route.

Tests: `ios::tests::the_ios_host_opens_the_document_picker_for_the_page_linked_services_and_webassembly_services`,
`android::tests::android_opens_the_storage_access_framework_for_the_page_linked_services_and_webassembly_services`
and `android_file_picker::tests` check the generated hosts, and
`scripts/native/file-picker-runtime-test.mjs` the route header. The
simulator and emulator gates are `scripts/ios/file-picker-test.mjs` (OCR
and simulated taps, a file seeded into the simulator's On My iPhone
storage) and `scripts/android/file-picker-test.mjs` (DevTools for the page,
`uiautomator` for the picker, a file pushed to Downloads). Until they run,
the iOS and Android cells of `tac.nativeCapabilities` are `runOwed`.
