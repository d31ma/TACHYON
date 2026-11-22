# Which bun checks to run for each changed file

About 70 `bun run test:*` checks in `package.json` are not part of nextest.
They run in full only in the local release gates and
on GitHub CI for release pull requests. So a change that touches a file listed
below runs the matching checks itself, in its own gates and in its merge batch
recheck. Paths under `template/` mean `crates/tachyon-core/src/template/`; paths
under `src/` mean `crates/tachyon-core/src/`.

Rule: if the slice changes a path in the left column, run the right column in the
slice's own gates (and in the batch recheck when any slice of the batch changes it).
Use a release ty built with `--features shipped` for tier 3 (`TAC_BIN=<path>`).

## Tier 1: node only, seconds (always run when touched)

| If you change | Run |
| --- | --- |
| `template/application-events.js` | `test:application-events` (byte budget, now 20,480) and `test:streams-browser` |
| `template/tac-stream-report.js` | `test:streams-browser` and `test:application-events` |
| `template/tac-stream-query.js` | `test:streams-browser` and `test:application-events` |
| `template/tac-client.js` | `test:native-task-answers`, `test:native-bridge-bounds` (and tier 3 below) |
| `crates/tachyon-core/src/behavior/runtime.js` | `test:behavior-runtime` |
| `scripts/python-tac-public-assets.mjs` | `test:python-tac-public-assets` |
| `scripts/site-deploy.mjs` | `test:site-deploy` |
| `scripts/ci/install-fylo.mjs` | `test:install-fylo` |
| `scripts/release/server-probe.mjs`, `scripts/supervised-command.mjs`, `scripts/cargo-artifacts.mjs` | `test:server-probe` |
| `crates/tachyon-core/src/behavior/polyglot/{java,kotlin}_*`, `tests/fixtures/behavior/Jvm*` | `test:behavior-jvm` (needs javac, kotlinc) |
| `crates/tachyon-core/src/behavior/polyglot/{cs,swift}*` | `test:behavior-native-runtime` (needs dotnet, swiftc) |

## Tier 2: Playwright only, no ty build, a few minutes

| If you change | Run |
| --- | --- |
| `template/tac-safe-html.js` | `test:safe-html-browser` |
| `template/tac-focus.js` | `test:focus-browser` |
| `template/tac-crypto.js`, `template/vendor/noble-ed25519.js` | `test:crypto-browser`, `test:passkeys-browser` |
| `template/tac-passkeys.js` | `test:passkeys-browser` |
| `template/tac-media.js` | `test:media-frames-browser`, `test:media-calls-browser` |
| `template/tac-media-frames.js` | `test:media-frames-browser` |
| `template/tac-peers.js`, `template/tac-media-view.js` | `test:media-calls-browser` |
| `template/tac-code-scanner.js`, `template/vendor/jsQR.js` | `test:code-scanner-browser` |
| `template/tac-{files,blobs,transfer,sha256,random-id}.js` | `test:blobs-browser` |
| `template/tac-web.js` | `test:focus-browser`, `test:crypto-browser`, `test:passkeys-browser`, `test:code-scanner-browser`, `test:blobs-browser` |
| `template/tac-peers.js` | `test:peers-browser` with `TAC_PEERS_REQUIRE_TOOLCHAINS=1` (needs coturn, a local TURN relay server) |

## Tier 3: needs a built ty and Playwright (run the matching gate)

| If you change | Run |
| --- | --- |
| `template/tac-client.js` | `test:tac-client`, `test:runtime-browser`, `test:tac-events`, `test:dom-keep-browser`, `test:page-apps-browser` |
| `template/tac-shell.js`, `compiler/shell_layout.rs`, `tests/fixtures/tac-page-apps/**`, `scripts/page-apps-browser-gate.mjs` | `test:page-apps-browser` |
| `template/tac-storage.js`, `template/tac-preference-*.js`, `src/storage/**`, `src/storage.rs` | `test:storage-browser` |
| `template/tac-web-push.js`, `template/tac-push-sw.js`, `src/web_api_clients.rs` | `test:web-apis-browser` |
| `template/tac-yon-reads.js`, `template/tac-http.js` | `test:yon-reads-browser`, `test:yon-reads-web-browser` |
| `template/authored-root-update.js`, `tests/fixtures/tac-authored-node-retention/**` | `test:authored-node-browser`, `test:dom-keep-browser` |
| `template/html_names.rs`, `tests/fixtures/tac-svg-namespace/**` | `test:svg-namespace-browser` |
| `template/{bound_*,braced_attribute}.rs`, `tests/fixtures/tac-element-properties/**` | `test:element-properties-browser` |
| `template/component*.rs`, `tests/fixtures/tac-component-scope/**` | `test:component-scope-browser` |
| template logic/loop compile, `tests/fixtures/tac-logic-controls/**` | `test:logic-controls-browser` |
| `template/expression.rs` | `test:island-expression` |
| `template/{event_*,tac-event-options.js}`, `src/events/**` | `test:events-browser`, `test:tac-events` |
| `src/compiler/exported_elements.rs` | `test:web-components-browser` |
| `src/topics.rs`, `src/topic_*.rs`, `src/server/topic_hub.rs`, `src/handler/topic_bus/**` | `test:topics-browser` |
| `src/hot_update*`, `src/server/hot_update_hub.rs` | `test:hot-update-browser` |
| `src/scaffold/**`, `src/scaffold.rs`, `crates/tachyon-cli/src/init_*.rs` | `test:scaffold-browser` |
| `src/tac_cpp/**`, `src/yon_cpp/**` | `test:cpp-services-browser` |
| `src/tac_lua/**`, `src/lua_source/**` | `test:lua-null-arguments-browser` |
| `src/tac_dart/**` | `test:first-app-dart-browser` |
| `src/yon/**` | `test:yon-origin-browser`, `test:yon-reach-browser` |
| `src/companion/**`, `src/companion.rs` | `test:native-companion` |
| service worker / offline code in `src/compiler.rs`, `src/server/**`, `compiler/shell_layout.rs` | `test:preview-offline-browser`, `test:service-worker-browser` |
| `src/server/**` (host, cookies) | `test:localhost-subdomain-browser` |
| `src/native/capabilities/WindowControls.*`, `template/tac-native.js` | `test:window-controls-browser` |

Every tier-3 path above that has a fixture folder: changing the fixture also means running its gate.

## Release only (too broad for per-merge; stay in release-extras)

`test:website`, `test:website-browser`, `test:website-lua-browser`, `test:compat`,
`test:standalone-rust`, `test:branded-browsers`, `test:held-clients-browser`,
`test:wasm-offline-browser`, `test:page-program-browser`, `test:cached-client-browser`,
`test:async-listener-browser`, and the macOS native gates (`test:phase4-macos`,
`test:native-icons`, `test:window-grant-macos`, `test:application-state-macos`,
`test:page-checks-macos`, `test:page-services-macos`, `test:macos-release`,
`test:wasm-features-app`), and the native Peers checks (`test:peers-macos`, `test:peers-ios`,
`test:peers-android`, `test:peers-windows`; release-extras' macos, ios, android and windows
groups). Exception: a slice that changes `src/native/**` for macOS
runs `test:phase4-macos` before merging.

Android foldables: `test:foldable-android` runs on the `tachyon-fold` emulator (Pixel 9 Pro
Fold, android-35), never on `tachyon-gate`. It folds and unfolds with `adb emu fold` and
`adb emu unfold`, and fails on a letterboxed or pillarboxed window, a page that reloads or
loses its state on a fold, a restart that does not reopen the open page, or a manifest
without `resizeableActivity` and the size `configChanges`. Pass `--evidence=<folder>` to
keep the screenshots. A slice that changes `src/native/android.rs`,
`src/native/android_window_settings.*` or `src/native/native_orientation.rs` runs it
before merging (`mac-heavy --vm=android`).

**macOS GUI and TCC gates** (TCC is the macOS privacy permission system): run
`test:phase4-macos` and every other macOS GUI or TCC gate as
`MAC_HEAVY_COORDINATOR=tachyon-build mac-heavy --owner <name> --gui -- bun run test:<gate>`,
in slice gates and in batch rechecks, without `--touch`. The `--gui` flag (Platform,
live since 2026-11-15 02:11) keeps the macOS Accessibility permission. It checks the
screen before it takes any lock, so it holds no slot while it waits. It waits up to
30 minutes for an unlocked, awake screen, then exits with code 75.

- While the screen is locked, run the gate once with
  `JOBQ=off MAC_HEAVY_COORDINATOR=tachyon-build mac-heavy --owner <name> --hold-light -- bun run test:<gate>`
  instead, so the build, launch and probe steps still run.
- Only a failure at "native rendered Accessibility controls did not become ready" with
  no app window may be deferred. It also needs a control run on `develop` from the same
  night that fails the same way.
- Run the deferred part later, while the user is at the Mac.

Note: for "inline project" tests (the script writes its own app), the mapping is a
best guess from what the test exercises; fixture-based rows are exact.
