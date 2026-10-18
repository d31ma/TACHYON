# ADR 0018: Native Hosts Run the Platform's Web Engine

- Status: Accepted; per-route companion selection amended to per-component activation by [ADR 0029](0029-pages-compose-components.md), then to per-service activation by [ADR 0030](0030-components-use-services.md)
- Date: 2026-08-30
- Reconciles: the unshipped native-host decision dated 2026-08-21
- Amends: the native-planning paragraph of ADR 0015

## Context

Two native implementations had grown apart: the released widget planner and
the newer local native implementation.

The planner turned a subset of Tac into platform controls. But it could not
give the same behavior to reactive browser components or to native
companions that the application owns.

A native application must execute the same client render plan as the web
application. It must not use a server-rendered widget tree or a widget tree
that the compiler infers separately.

## Decision

Every native host loads the application's staged web bundle in the web engine
of its platform:

| Platform | Web engine |
| --- | --- |
| macOS, iOS | WKWebView |
| Linux | WebKitGTK |
| Windows | WebView2 |
| Android | Android WebView |

JavaScript and TypeScript browser code stays browser code.

The host provides a small bridge of named JSON capabilities. The bridge is not
a general interface for reflection, shell access, filesystem access, or remote
code execution.

### Navigation and origin rules

- Navigation and messages are limited to the local application origin and the
  main application frame.
- Bundle asset resolution must support root-relative URLs. It must not expose
  arbitrary filesystem paths.
- Untrusted remote pages and subframes cannot inherit native capabilities.

### Companion selection

The host selects compiled page companions for each canonical route. It never
selects them globally from the first page. The compiler generates a bounded
route registry. The host checks every route identifier that arrives over the
bridge against this registry. See
[ADR 0019](0019-companions-compile-for-their-target.md).

## Consequences

- Native UI rendering has the same client-only ownership as the browser.
- Each host builds on its target operating system with that platform's SDK.
- Windows uses MSVC (the Microsoft Visual C++ toolchain) and WebView2, not a
  MinGW widget implementation. The WebView2 runtime and the redistributed
  loader are explicit prerequisites.
- Native companions are trusted application code. The bridge still checks
  origin, route, operation, payload, lifecycle, and capability. These checks
  remain security boundaries.
- This decision supersedes the older Native UI widget planner. Tachyon does
  not keep it as a hidden alternative renderer. Its historical ADRs stay
  available for audit.

## Acceptance

Native gates must do all of the following:

- launch real generated applications;
- exercise DOM events;
- load root-relative assets;
- dispatch two different page companions;
- reject messages from a foreign frame or origin.

Checks on source strings add to the platform build and execution gates. They
do not replace them.

## Amendment (2026-10-18): external links open outside the app

A navigation or new window that leaves the local application origin is still
cancelled on every host. When it is an external link, the host also hands it
to the operating system, so the default browser or mail app opens it:

| Host | How it opens the link |
| --- | --- |
| macOS | `NSWorkspace.shared.open` |
| iOS | `UIApplication.shared.open` |
| Android | an `ACTION_VIEW` intent |
| Windows | `ShellExecuteW` from the WebView2 `NavigationStarting` (top-level frame only) and `NewWindowRequested` handlers |
| Linux | `GtkUriLauncher` from the WebKitGTK `decide-policy` (navigation and new-window decisions) and `create` signals |

Every host uses the same rule for an external link:

- the scheme is `http`, `https` or `mailto`, in any letter case;
- an `http` or `https` link names a host and carries no user name or
  password;
- a `mailto` link has an address part;
- on Apple hosts the request comes from the main frame or asks for a new
  window.

The Windows and Linux hosts check the link in one C helper
(`native_external_link.h`). It also refuses a link longer than 8192 bytes, a
link with a space, control character, non-ASCII byte or `"`, and a link on the
application's own origin (`https://tachyon.local` on Windows). So a page
address that matches no route still opens nothing. A link that the system
cannot open records `link.external.failed` in the host's lifecycle log on
Windows and Linux. Every other scheme, such as `file:`, `javascript:`,
`ftp:` or `ms-settings:`, opens nothing on any host.

Checks: `native_external_link_tests` runs the C helper against hostile links
for both origins, builds the Windows opener with MinGW and runs it under Wine,
and (on the Linux lane) runs the generated GTK host under Xvfb, where a page
script follows links and opens windows and a fake launcher records what the
host handed out.
