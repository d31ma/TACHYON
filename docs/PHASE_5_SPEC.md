# Phase 5 Specification: Platform Expansion

This document is normative for Phase 5: it states the rules that Phase 5 must
follow. It defines three things:

- how one resolved Native UI v1 document becomes an application on each
  supported platform;
- what each platform host must guarantee;
- what evidence moves a platform from *buildable* to a higher tier in
  [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md).

Phase 5 adds no new view syntax, no new adapter and no new capability. It adds
platform hosts and the evidence that proves they work.

## 1. Target Model

A native build selects exactly one target:

| Target    | Directory          | Host technology                     | Toolchain                |
| --------- | ------------------ | ----------------------------------- | ------------------------ |
| `macos`   | `dist/macos/`      | `SwiftUI` over `AppKit` controls    | `xcrun swiftc`           |
| `ios`     | `dist/ios/`        | `SwiftUI` over `UIKit` controls     | `xcrun -sdk iphonesimulator swiftc` |
| `linux`   | `dist/linux/`      | GTK4 widgets, `WebKitGTK` surfaces  | `cc` + `pkg-config`      |
| `windows` | `dist/windows/`    | Win32 common controls               | `mingw-w64` `gcc`        |
| `android` | `dist/android/`    | Platform Android views              | Gradle + Android SDK     |

### 1.1 Target Isolation

- Each target publishes only under its own directory.
- A failed build for one target never changes the output of another target.
- A failed build publishes nothing under its own directory. It keeps the
  previous complete application.

### 1.2 Shared Lowering

Every target uses the same fully resolved Phase 3 web output and the same
`NativePlanner`. Build one project for two targets. The two Native UI v1
documents must be identical byte for byte, except for:

1. the `target` discriminator, and
2. the human-readable `reason` on a `web_surface`, which names its platform.

No other difference is allowed. These parts do not depend on the platform:
adapter selection, node identities, accessible names, roles, properties,
events and initial state.

## 2. Platform-Neutral Staging

Before any toolchain runs, every target stages the same resource tree:

```text
<stage>/native-index.json          inspectable route index
<stage>/native-ui/<key>.json       inspectable Native UI v1 documents
<stage>/web-surfaces/<id>/index.html
<stage>/web/                       resolved Phase 3 web output
<stage>/capability-manifest.json   Capability Manifest v1
<stage>/project/                   generated host source
```

Each target then copies this tree into the platform bundle under fixed names:

```text
NativeIndex.json
NativeUI/<key>.json
WebSurfaces/<id>/index.html
WebBundle/
CapabilityManifest.json
```

The build writes Artifact Manifest v1 last. It covers every other published
file. It records the name and version of the real platform toolchain.

## 3. Host Requirements

Every generated host must:

1. reject a Native UI v1 document when its `contract_version` is not `1` or
   its `target` is not the platform of the host;
2. render every adapter listed in §4 with the platform's own control for that
   adapter, never with an emulated or web-drawn substitute;
3. pass the accessible name of each node to the platform accessibility API;
4. pass the identity of each node to the testing identifier of the platform,
   when the platform has one;
5. apply `increment`, `toggle`, text input and disclosure state exactly as the
   Phase 4 declarative ABI (application binary interface) defines them;
6. record these events to a bounded log for each application:
   `controller.created`, `controller.mounted`, `controller.active`,
   `controller.suspended`, `controller.destroyed`, `route.opened`,
   `route.failed`, and the `state.*` and `websurface.*` events;
7. expose **no** bridge of any kind to `WebSurface` content;
8. fail closed: show a visible error state instead of a partial view.

### 3.1 Lifecycle Delivery

A host records a lifecycle event only when its platform really delivers the
matching notification.

- The host records `controller.destroyed` when the application ends
  gracefully.
- Some platforms end applications without a graceful notification. The main
  examples are iOS and Android when the system kills the application. These
  platforms do not have to record `controller.destroyed`. A missing record is
  not a defect.

## 4. Adapter Coverage

| Adapter              | macOS          | iOS            | Linux           | Windows        | Android      |
| -------------------- | -------------- | -------------- | --------------- | -------------- | ------------ |
| `layout.column`      | `VStack`       | `VStack`       | `GtkBox`        | stacked frame  | `LinearLayout` |
| `layout.app_bar`     | `HStack`       | `HStack`       | `GtkBox`        | stacked frame  | `LinearLayout` |
| `layout.list`        | `VStack`       | `VStack`       | `GtkBox`        | stacked frame  | `LinearLayout` |
| `layout.list_item`   | `VStack`       | `VStack`       | `GtkBox`        | stacked frame  | `LinearLayout` |
| `text.heading1..6`   | `Text`         | `Text`         | `GtkLabel`      | `STATIC`       | `TextView`   |
| `content.text`       | `Text`         | `Text`         | `GtkLabel`      | `STATIC`       | `TextView`   |
| `control.button`     | `NSButton`     | `UIButton`     | `GtkButton`     | `BUTTON`       | `Button`     |
| `control.text_field` | `NSTextField`  | `UITextField`  | `GtkEntry`      | `EDIT`         | `EditText`   |
| `content.output`     | `Text`         | `Text`         | `GtkLabel`      | `STATIC`       | `TextView`   |
| `control.disclosure` | `DisclosureGroup` | `DisclosureGroup` | `GtkExpander` | `BUTTON` + toggle | `Button` + toggle |
| `navigation.link`    | `Button`       | `Button`       | `GtkButton`     | `BUTTON`       | `Button`     |
| `content.image`      | `Label`        | `Label`        | `GtkImage`      | `STATIC`       | `TextView`   |
| `content.divider`    | `Divider`      | `Divider`      | `GtkSeparator`  | `SS_ETCHEDHORZ`| `View`       |
| `web_surface`        | `WKWebView`    | `WKWebView`    | `WebKitWebView` | placeholder §6 | `WebView`    |

## 5. `WebSurface` Isolation

On every platform that embeds a web view:

- the surface uses a temporary data store that is not saved;
- scripting is on only for a `local_bundle` surface, never for `remote_url`;
- a `local_bundle` surface can navigate only inside the bundle root;
- a `remote_url` surface can navigate only to the declared HTTPS host;
- the host cancels every other navigation;
- the host exposes no script message handler, no JavaScript interface and no
  native object.

## 6. Declared Platform Reductions

Each reduction below is deliberate. Each has a named viability gate: the
condition that must be met before the reduction can be removed. Each is also
listed in `SUPPORT_TIERS.md`. No reduction may be widened silently.

| Reduction | Platform | Gate |
| --- | --- | --- |
| `WebSurface` renders an accessible placeholder that opens its content in the default browser instead of embedding a web view. | Windows | `WebView2` redistributable, COM binding, and offline-build viability. |
| Native UI v1 is lowered to a generated control table at build time. It is not parsed at run time. | Windows | Kept until a Windows JSON dependency is justified. The published `NativeUI/*.json` remains the contract. |
| The application is built for the simulator SDK, not for a device. | iOS | Device provisioning profiles and distribution signing. |
| Accessible roles come from the implicit role of the widget. Only names and hidden state are set explicitly. | Linux | GTK4 does not allow a role to be assigned after construction. |
| A control's accessible name is its window text. A declared `aria-label` that differs from the visible text is not separately exposed. | Windows | A UI Automation provider implementation for the generated controls. |
| The APK is a debug build signed with the debug key. | Android | Release keystore management and Play signing. |

## 7. Promotion Evidence

A platform moves up one tier at a time. Each tier requires the evidence
below. Record it in [`PHASE_5_EVIDENCE.md`](PHASE_5_EVIDENCE.md), together with
the commands that reproduce it.

| Tier | Required evidence |
| --- | --- |
| `buildable` | The generated host compiles for the platform, with warnings treated as errors. |
| `simulator-tested` | The application installs and launches on a first-party simulator or emulator, renders its native view, and records its lifecycle. |
| `native-tested` | Every §4 adapter renders. Every interaction in §3.5 succeeds. Accessible names are readable through the platform accessibility API. The `WebSurface` subtree stays isolated. |
| `supported` | `native-tested`, plus install, upgrade, rollback and uninstall tests on real hardware, plus a published support window. |

Cross-compilation proves only `buildable`.

## 8. Failure Behavior

| Code | Condition |
| --- | --- |
| `TY1601` | Application configuration or entry route is invalid for the target. |
| `TY1602` | The resolved view cannot be planned. |
| `TY1603` | Accessibility or declarative state is invalid. |
| `TY1604` | A `WebSurface` boundary is invalid or exceeds its budget. |
| `TY1605` | Staging, host generation, toolchain execution, or publication failed. |

Every failure keeps the previous published application for that target. No
failure leaves partial output.
