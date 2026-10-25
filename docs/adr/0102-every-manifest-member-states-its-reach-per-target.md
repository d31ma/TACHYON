# ADR 0102: Every Manifest Member States Its Reach per Target

- Status: Accepted
- Date: 2026-10-18
- Related: [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
  (the manifest is the one public configuration file),
  [ADR 0098](0098-an-integrated-title-bar-is-an-opt-in-window-setting.md)
  (the first fallback note for one member)

## Context

A developer configures an app once, in `client/manifest.json`, and builds it
for six targets: Web, macOS, Windows, Linux, iOS and Android. The targets are
not the same. A survey of the targets on 2026-10-18 found 243 places where a
declared member did nothing on a target and the build said nothing. For
example, `theme_color` reached no native window, `tac.application.window`
width and height reached only macOS, and `tac.cache` did nothing in a native
app. Only one member, an integrated title bar, printed a note when a target
could not build it (ADR 0098).

A developer cannot trust a manifest that is ignored without a word.

## Decision

1. One table in the framework,
   `crates/tachyon-core/src/native/target_support.json`, lists every framework
   member of `tac`, the standard manifest members that a native build reads,
   and each native-facing capability (window controls, each browser API,
   menus, the title bar, application lifetime, links, stored data, the
   Content-Security-Policy and reaching Yon). `TargetSupport` reads it.
2. For each of the six targets, an entry is one of:
   - **honored**: the target uses the member natively, with a short "how";
   - **adapted**: the target uses the closest thing it has, with one note;
   - **refused**: the build stops with a TY code and an alternative;
   - **not applicable**: the member has no meaning there, with one note.
3. A build checks only the members that the manifest declares. A path such as
   `tac.application.window.width` is declared when it is present, and an item
   such as `tac.browserApis[geolocation]` when the list holds that name.
   - Each declared refused member stops the build with its code.
   - Each declared adapted or not-applicable member prints `Note: <note>`
     once. Two members with the same note print it once.
   - A member that is not declared prints nothing.
4. The table seeds today's behavior. It changes no build result except one:
   `tac.application.rust.executables` on iOS and Android is now refused with
   `TY1601`, because a mobile app may not start other programs. It was ignored
   before. Menus on iOS and Android and application lifetime services on Web,
   iOS and Android stay refused by their existing checks; the table records
   them.
5. `ty bundle --explain-targets` prints the table for every target, or for the
   targets that `--target` names, and marks each member that the manifest
   declares. The website page "Settings and configuration" shows the same text
   in one tab per target. A test keeps those tab files equal to the table.
6. Every later change to how a target treats a member changes its entry in the
   same commit. A test requires an entry for every name in
   `TacMember::FRAMEWORK_MEMBERS`, all six targets in every entry, and a real
   TY code in every refusal.

## Consequences

- A developer sees what each target does with their manifest when they build,
  and can ask for the whole list before they build.
- The integrated title bar note now comes from the table. Its text names the
  plan item that will build the title bar on that target.
- The website tabs and `--explain-targets` cannot drift from the build,
  because all three read the same table.
- Notes add lines to build output. A note is printed only for a declared
  member, and only once per build.
- A new manifest member cannot be added without stating its reach on every
  target.

## Amendment (2026-10-19): run owed and not applied

The platform survey of 2026-10-19 held the table to a stricter rule: a target
supports a member only when a run on that target proves it. The table used
`adapted` both for owed runs and for members that a target ignores, and
`honored` for members checked only in code. Two states join the four above:

- **run owed** (`"reach": "runOwed"`, with `how` and `run`): the target
  supports the member in code, and `run` names the run on that target that
  has not happened yet. A declared run-owed member prints one note:
  `<member> is supported on <target>, but this run is still owed: <run>.`
  When the run passes, the entry becomes `honored` in the same commit as
  the run record.
- **not applied** (`"reach": "notApplied"`, with `note`): the target does
  not use the member yet, and the note names the plan item that will apply
  it. A declared not-applied member prints its note, like an adapted one.
  `adapted` now means only that the target uses the closest thing it has.

A test now rejects an `honored` entry whose text says a run is owed, checked
only in code, or not run. The table also states two design limits as not
applicable: native apps register no service worker, and the macOS and
Windows apps need a signed-in desktop session.

## Amendment (2026-10-19): honored needs a recorded run (DOC3)

The platform survey of develop `50acf161` found 29 cells that the table
marked `honored` on code alone, with no run on that target on record. A
member is now `honored` on a native target only when a recorded run on that
target proves it; otherwise it is `runOwed` and names the run. These moved
from `honored` to `runOwed`:

- external `http`, `https` and `mailto` links on macOS, Windows, iOS and
  Android (only Linux ran them);
- `Content-Security-Policy` and `tac.outgoing` on macOS, Windows and iOS
  (only Android and Linux ran a refusal);
- `tac.application.permissions` on macOS, iOS and Android, whose prompt runs
  were already `runOwed` under the browser API entries;
- `tac.application.rust` vendored crates on macOS, iOS and Android;
- `tac.dotnet` on macOS, Linux, iOS and Android (no native host loaded an
  ahead-of-time C# build);
- `tac.application.minimumSystem` on macOS and iOS (the 15.2 and 18.2
  WasmGC floors);
- `tac.application.window.minWidth` and `minHeight` on macOS.

Web entries stay `honored`: the browser itself follows links, and the web
gates run in three engines. Linux "reaching Yon" is now `adapted`, like the
other native hosts: its pages load from `tachyon://app`, so same-origin
requests reach no Yon, and calls go to `tac.yon.origin` instead. The native
service worker notes now say that without `tac.yon.origin` a native
`tac.cache` has no effect.

## Amendment (2026-10-24): iOS simulator runs (RM2)

RM2's runs on the iOS 26.5 simulator move these iOS cells to `honored`:
`tac.application.minimumSystem` (Kotlin, Dart and Java apps built at 18.2),
`tac.application.rust` (a vendored crate), `tac.cache` and stored data (kept
across a relaunch), `tac.application.permissions` and the `share`,
`notifications`, `geolocation` and `preferenceStrings` browser APIs,
`tac.python`, `tac.dotnet` (ahead of time), `tac.outgoing` and
Content-Security-Policy, external links, background and foreground, and
byte-identical rebuilds. `docs/SUPPORT_TIERS.md` ("iOS simulator runs") lists
each run. iOS release packaging and page accessibility stay `runOwed`.

## Amendment (2026-10-24): the macOS runs (RM1)

RM1 ran the macOS `runOwed` cells on a signed-in Mac (Apple M4, macOS 26.6.2,
Xcode 27.1, develop `4c052177`; `docs/SUPPORT_TIERS.md`, macOS runs owed,
RM1). These macOS cells moved to `honored`, each with its recorded run:

- `tac.application.window.controls` and `minWidth`/`minHeight`: granted
  close, fullscreen, maximize, minimize and title from a page,
  `scripts/window-grant-macos-test.mjs`, and a drag that stopped at the
  minimum size;
- background and foreground: `scripts/application-state-macos-test.mjs`
  (a page service and linked Rust and Swift services);
- external links: `http`, `https`, `mailto` and an off-bundle navigation
  handed to the default apps; a link with a user name opened nothing;
- `tac.browserApis` clipboard, share and geolocation, and
  `tac.application.permissions`: the pasteboard host test, the share picker
  from a page, and the location prompt showing the manifest reason;
- `tac.cache`, `tac.browserApis` preferenceStrings, `tac.outgoing` and
  `Content-Security-Policy`: `scripts/native/page-checks-macos-test.mjs`;
- `tac.python` and `tac.application.minimumSystem`:
  `scripts/native/page-services-macos-test.mjs` (Python, and Dart, Kotlin
  and Java built at 15.2);
- `tac.application.rust`: a universal app whose Rust service answered the
  page from vendored crates in both slices;
- byte-identical rebuilds: `a_packaged_macos_app_is_byte_identical_when_built_twice`.

Still `runOwed` on macOS: `tac.browserApis` notifications (the prompt opened
from a page; a notification needs someone to allow it) and `tac.dotnet` (an
ahead-of-time C# build needs a signed `--release` build).

## Amendment (2026-10-24): macOS notifications after an allowed prompt (RM1)

This amendment narrows "Still `runOwed` on macOS: `tac.browserApis`
notifications" in the RM1 amendment above, which stays as the history of this
decision. On the same Mac, a copy of the RM1 browser API app under a new id
asked for notification permission from a page, and Samuel allowed the
prompt. The page heard `granted`, and its notification was delivered and
presented as a banner while the app was in front (`docs/SUPPORT_TIERS.md`,
macOS runs owed, RM1). The support table marks `tac.browserApis`
notifications `honored` on macOS. `tac.dotnet` stays `runOwed`.

## Amendment (2026-10-24): page accessibility on iOS and Android (AX1)

The `page accessibility (roles and names through the system accessibility
API)` cells for iOS and Android move from `runOwed` to `honored`.
`scripts/ios/accessibility-audit-test.mjs` (XCUITest on the iOS 26.5
simulator) and `scripts/android/accessibility-test.mjs` (UiAutomation on the
Android 15 emulator) read a page's heading, image, text field, button and link
by role and name, and drive them through the accessibility API.
`docs/SUPPORT_TIERS.md` ("Page accessibility on iOS and Android") records the
runs. It also records their limits: the iOS audit API does not look inside web
view content, and Chromium's Android heading is not marked as a heading.

## Amendment (2026-10-25): cells narrowed to their runs (JS1)

The platform survey of develop `4243f2b6` found cells whose `how` text named
callers that never ran on that target. Under the DOC3 rule above, a cell
whose claim is wider than its run is `runOwed`, and its `run` names the
callers still to run. These cells moved from `honored` to `runOwed`:

- `tac.application.window.controls` on macOS (C# and Kotlin WebAssembly
  callers; RM1 ran a page and a page service) and on Windows (linked Rust
  and WebAssembly Swift callers inside WebView2);
- `tac.nativeCapabilities` filePicker on macOS (a linked Rust field) and on
  Windows (linked Rust, which ran only under Wine, and WebAssembly Swift);
- `tac.go` on macOS and Android (no Go service ran in either app);
- `tac.browserApis` geolocation on macOS (the prompt was answered Don't
  Allow, so no position reached a page);
- background and foreground on Windows (a linked C# listener);
- reaching Yon on Windows (a linked Rust service's held stream and socket
  clients; only the C# ones ran).

This narrows the RM1 amendment above for window controls and geolocation on
macOS; that amendment stays as the history of the decision. The host threads
that run these callers move each cell back to `honored` with its run.
`docs/SUPPORT_TIERS.md` ("Support table narrowed to its runs") lists what
ran and what is still owed for each.

On 2026-10-25 (MA1) C# and Kotlin WebAssembly window-control callers, a
linked Rust file picker field and a Go service ran in the macOS app, so the
macOS window controls, file picker and `tac.go` cells are `honored` again.
