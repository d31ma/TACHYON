# ADR 0004: Native Rendering and WebSurface Fallback

- Status: Accepted
- Date: 2026-07-26

## Context

Tachyon targets web, macOS, iOS, Android, Windows and Linux. Application
developers write views in HTML. Some HTML elements, Tac components or web
components will not have a native adapter on every platform.

One unsupported subtree must not switch the whole application to a WebView.
That would defeat the native architecture. Treating an unsupported tag as a
native primitive is also wrong: it produces broken behavior.

## Decision

Plan native rendering separately for each View IR subtree.

- Standard elements and components with registered adapters render natively.
- Control tags are already lowered, so they never need adapters.
- An unsupported safe subtree becomes a WebSurface. The WebSurface contains a
  generated local web bundle.
- Supported parents, siblings and unrelated descendants stay native.
- The boundary serializes explicit properties and events through the portable
  controller contract.
- Remote web content always uses `bridge: none`.
- Every capability of a local WebSurface must appear in a capability manifest
  that denies by default. Tachyon checks the capability again on every call.
- Each fallback decision emits a build diagnostic that developers can inspect.
  The decision also appears in the native artifact manifest.

Fallback must select the smallest subtree that keeps semantics and lifecycle
correct. It may select a larger subtree only when layout, state or event
boundaries cannot be separated safely.

## Consequences

- Applications stay mostly native, and more so as adapter coverage grows.
- Unknown custom content behaves in a predictable way.
- Keeping the native and web lifecycles in sync becomes a first-class
  contract.
- Looking the same is not enough. Accessibility and interaction must also
  match.
- Isolation of local WebSurface bundles and their navigation policy become
  security boundaries.

## Rejected Alternatives

- **Application-wide render mode flag**: one unsupported component would
  downgrade the whole application.
- **Drop unsupported nodes**: behavior and accessibility disappear without any
  warning.
- **Render unknown tags as generic labels**: this changes the meaning of the
  content.
- **Give remote WebViews the native bridge**: remote content would gain
  capabilities it must not have. This is not acceptable.

## Acceptance Gate

- Native UI v1 represents native elements, text and WebSurface boundaries;
- its invalid fixture proves that remote content cannot receive a bridge;
- capability manifest v1 proves default deny;
- future platform tests compare semantics, events, accessibility and visuals
  against the web reference.
