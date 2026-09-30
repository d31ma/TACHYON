# ADR 0089: Native capabilities are activation-owned service providers

- Status: Accepted contract; macOS FilePicker and Swift WindowControls slice in progress
- Date: 2026-09-30
- Relates to ADR 0025 (NeedsAll), ADR 0030 (native services), ADR 0065 (page APIs), ADR 0087 (WindowControls)

## Decision

A service receives OS capabilities through existing Needs/NeedsAll injection.
Platform imports and calls belong to adapters. There is no second application
container. JavaScript/TypeScript receive a `filePicker` activation resource;
compiled Swift receives a required `FilePicker` field through its generated
service construction. Native `WindowControls` uses the same construction seam
and retains `tac.application.window.controls` grants.

The additive manifest field `tac.nativeCapabilities` accepts `filePicker` once.
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
TestSetup/fake code can replace the provider transport before constructing a
service. The generated provider keeps that transport, so a later registration
cannot silently replace an existing activation's dependency.

The macOS WK message bridge requires the main frame, local `tachyon-app://bundle`
origin and bounded canonical JSON before dispatch. The native companion adapter
uses its existing owner-thread execution path. A browser tab returns
`unsupported`; no browser picker is substituted. Other native hosts must
register an adapter explicitly before advertising support.

## Support and remaining slices

| Target / service | Contract | Evidence required |
| --- | --- | --- |
| macOS JavaScript/TypeScript | Needs filePicker resource, Promise results | provider and generated app execution, real panel selection/cancel |
| macOS compiled Swift | Needs/NeedsAll FilePicker and WindowControls | generated companion compile/execution with fake, native host execution |
| Browser tab | typed unsupported FilePicker result | provider execution |
| iOS / Android / Linux / Windows | unsupported until platform adapter registered | separate OS interaction and grant tests |
| Native Rust / C# / Kotlin | provider registration and injection pending | compiled activation and fake-backed service tests before enabling |

The platform matrix is not expanded merely because the macOS source compiles.
An OS interaction claim requires the actual native picker, selection, bounded
read and cancellation checks on that OS. UI automation permissions are recorded
as blockers if the host does not permit that check.
