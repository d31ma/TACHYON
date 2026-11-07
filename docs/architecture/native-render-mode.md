# Native application rendering

Status: platform web-view hosts, Native Host contract v3 (ADRs 0018 and 0019).

Every target uses the same client-rendered Tac application. The compiler does
not:

- lower a separate widget tree;
- render native pages on the server;
- choose a fallback for each subtree.

JavaScript and TypeScript services run in the platform web engine. Native
services (`client/services/<Name>Service.swift|kt|cs|rs`) compile to their
selected native target (ADR 0030).

## Pipeline

1. Capture one immutable project snapshot. Validate routes and configuration.
2. Compile bounded client render plans, modules, styles, and shared assets.
3. Select one native language per service from the canonical target table.
4. Emit bounded dispatch tables for each service, and service code for the
   native target.
5. Stage the platform web-view host and the local bundle resources.
6. When packaging is requested, build with the target SDK. Then publish the
   complete artifact and an accurate host manifest atomically.

Each platform uses its own web view:

| Platform | Web view |
| --- | --- |
| macOS and iOS | WKWebView |
| Linux | WebKitGTK |
| Windows | WebView2 |
| Android | Android WebView |

Root-relative assets and canonical static and dynamic routes resolve inside
the staged bundle.

Automatic service member discovery currently exposes supported fields and
public methods with zero arguments. A method that takes arguments needs an
explicit native member table. The transport forwards only JSON arguments that
the author wrote, never an implicit DOM event. Unsupported automatic bindings
must not be mistaken for generated methods. Native methods return browser
Promises.

## Security boundary

Only the main application frame at the exact local bundle origin may call
declared capabilities. Navigation checks and per-message checks of origin and
frame are separate requirements. A canonical route in a request must belong to
the registry that the compiler generated, and it must match the active
application page. A request names a service by its instance name. An unknown
service answers `TY_NATIVE_SERVICE`.

Native services are trusted application code. They have the operating-system
privileges of the host. They are not sandboxed plugins. These never get the
bridge:

- remote pages;
- subframes;
- arbitrary file URLs;
- unknown routes;
- unknown operations;
- undeclared members.

A string that comes from the user never becomes executable script without
JSON encoding.

## Bundle and migration

Each target publishes under `dist/<target>/`. This is also true when several
targets build together. `tachyon.host.json` declares:

- schema and bridge version 3;
- bundle rendering;
- the entry route and document;
- the selected native services;
- window controls;
- the supported host capabilities.

Artifact metadata no longer claims to have produced a Native UI v1 document
that does not exist.

The earlier widget planner and WebAssembly companion contracts are superseded.
Use [ADR 0019](../adr/0019-companions-compile-for-their-target.md) for companion
migration and [RECONCILIATION.md](../RECONCILIATION.md) for release qualification.
`--render-mode` stays rejected. The renderer is an architectural contract, not
an undocumented compatibility switch.
