# ADR 0089: Native Capabilities Are Activation-Owned Service Providers

- Status: Accepted. The FilePicker works on macOS for JavaScript, TypeScript, Swift and Rust services. C# and Kotlin services receive the injected providers, but their FilePicker answers `unsupported`; Windows and Android have no picker.
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
| FilePicker on iOS / Android / Linux / Windows | typed unsupported; no platform picker rollout | no OS picker interaction claim |
| macOS compiled Rust | NeedsAll default providers, captured registration and native WindowControls | compiled fake/unsupported/disposal, static-library host link and actual owned-window title/resize/grant test |
| Rust WebAssembly | existing page WindowControls, unsupported native FilePicker | generated wasm32 compilation |
| Swift / C# / Kotlin WebAssembly | injected FilePicker that answers unsupported; existing page WindowControls | `ty bundle` web and non-linking native targets |
| Native C# / Kotlin | Needs/NeedsAll captured providers, typed unsupported FilePicker, Windows/Android WindowControls registration | actual compiled activation/fake/disposal tests, Windows stub-host dispatch execution and complete Android API35 typecheck |

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
