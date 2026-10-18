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
