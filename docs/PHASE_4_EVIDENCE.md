# Phase 4 macOS Native Vertical-Slice Evidence

## Result

Phase 4 was completed as a development milestone on 2026-07-26. It is not a
preview and not a supported release. The native-tested claim covers only macOS
arm64. Native hosts for Android, iOS, Windows and Linux are still Phase 5 work.

The normative contract is [`PHASE_4_SPEC.md`](PHASE_4_SPEC.md). The repeatable
host gate is `bun run test:phase4-macos`. The gate records its run artifacts and
their SHA-256 digests under `target/phase4-evidence/`. Git ignores these
artifacts.

## Implemented Evidence

- `ty build PROJECT --target macos` reads the fully resolved Phase 3 HTML and
  publishes `dist/macos` atomically.
- Native UI v1 records these things:
  - fixed semantic adapters;
  - deterministic identities that are local to each route;
  - accessibility roles and names;
  - controller properties and events;
  - explicit boundaries for local or remote WebSurfaces.
- Custom design-system elements select those adapters through explicit
  semantic roles, not through tag names from a specific framework. The app bar
  in the acceptance fixture declares `role="banner"`. An unrecognised element
  without a supported role stays an isolated WebSurface subtree.
- The generated SwiftUI/AppKit host compiles with the selected Xcode Swift
  toolchain. It gets an ad-hoc code signature. It loads only the Native UI and
  route-index resources that are bundled with it.
- Scalar state written in HTML supports text input and output, numeric
  increment, boolean toggle and disclosure state. It needs no Swift or
  JavaScript controller written by the application author.
- Local fallback is limited to its subtree. It is constrained by a CSP (Content
  Security Policy), it is not persistent and it has no bridge. Remote fallback
  requires HTTPS, disables JavaScript and allows navigation only to the
  declared host. It never has a bridge.
- Capability Manifest v1 denies every capability that is not declared.
  Artifact Manifest v1 records:
  - the release and source identity;
  - the target;
  - the Rust and Swift toolchains;
  - the contract versions;
  - the path, size and SHA-256 digest of every output.
- If planning or host generation fails, the previous complete application
  bundle stays in place.

## Real Application Acceptance

The committed `phase4-macos` fixture builds a native application that is 420
points wide. Its height is from 600 through 780 points, which is safe for
hosted CI (continuous integration) runners. It has a native app bar, heading,
text, button, output, text field, disclosure, footer and one isolated
custom-element WebSurface.

The gate does these steps in order:

1. It launches the compiled `.app`.
2. It reads the operating system's Accessibility hierarchy for the app.
3. It presses the AppKit button.
4. It types `Ada` with CoreGraphics keyboard events.
5. It opens the disclosure.
6. It captures the native window.
7. It quits the app normally.

The evidence proves these results:

- The output changes from `0` to `1`.
- The named text field receives `Ada` and logs `state.input`.
- The disclosure shows its native content.
- The required names are present for the button, text field, heading, group
  and WebSurface.
- The lifecycle order includes `created`, `mounted`, `active`, the state
  transitions and a synchronous `destroyed`.

The same resolved bundle also runs in real Chromium, at the exact size of the
captured native window. Its required role and name pairs match Native UI v1 for
main, heading, button, status, textbox, disclosure and fallback. The latest
local comparison passed with these values:

| Measure | Value | Limit |
| --- | ---: | ---: |
| Vertical transport distance | `0.0768` | `0.16` |
| Centroid distance | `0.0711` | `0.22` |
| Edge-density ratio | `1.1419` | `4.0` |

## Adversarial and Recovery Evidence

Tests reject all of these:

- configuration that is malformed, too large or a symlink;
- missing entry routes;
- unsupported targets;
- interactive controls that are not accessible;
- duplicate or undeclared state;
- increments that are not numeric;
- invalid verbs or names;
- unsafe remote URLs;
- too much nesting;
- too many WebSurfaces;
- compiler syntax that was not resolved;
- generated symlinks;
- unsafe output paths;
- artifacts that do not match their schema.

The Native UI, Capability Manifest and Artifact Manifest outputs validate
against their canonical Draft 2020-12 schemas. The capability test proves
`default_policy: deny`, an empty capability set and
`remote_content_bridge: false`. The rollback test makes a rebuild fail on
purpose. It then checks that the executable bytes are the same before and
after that rebuild.

## Repository Gates

The completion run includes these checks:

- formatting;
- checking all targets and all features;
- strict Clippy;
- all 112 Rust tests on the macOS host;
- rustdoc with no warnings;
- the same 90% coverage ratchet for lines, functions and regions. It applies
  to target-neutral Rust, the Native UI planner and native configuration;
- the dependency, license and source policy;
- the compatibility differential against the released binary;
- real Chromium islands;
- the real macOS parity gate;
- a run in a pinned Linux container;
- the Windows GNU cross-check;
- a smoke test of the release binary, which reported `ty 0.0.0-phase4`.

The pinned `rust:1.97.1-bookworm` Linux x86_64 container passed all 109 tests
that apply to that host. The two tests it omitted are the explicit macOS
bundle and rollback tests. The historical JavaScript suite passed all 637
tests at the Phase 4 checkpoint. The team removed that suite when Rust became
the only implementation in the repository.

The LLVM coverage percentage excludes `native/compiler.rs` and
`native/macos.rs`. Those files contain the paths for the external Swift
compiler, signing and process orchestration. Two mandatory tests cover them
instead: an in-process Rust packaging test and the gate that launches the
real application. The Native UI inputs to those paths are still inside the
90% ratchet. The passing report was 91.44% lines, 90.96% functions and
91.38% regions.

These results qualify only a Phase 4 development milestone. These items are
still promotion gates:

- production signing and notarization;
- installers;
- upgrades;
- rollback across releases;
- entitlements;
- sandbox review;
- live adversarial testing against remote origins;
- fuzzing;
- soak testing;
- automated security qualification.
