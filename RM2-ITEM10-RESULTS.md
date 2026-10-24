# RM2 item 10: iOS 17.0.1 and iOS 18 simulator runtimes (Mac, 2026-10-24 evening)

Samuel downloaded `/Volumes/FORGE/iOS_17.0.1_Simulator_Runtime.dmg` (7,210,691,623 bytes) and `/Volumes/FORGE/iOS_18_Simulator_Runtime.dmg` (8,640,300,544 bytes). Both files stay where they are.

Base: develop `4eac45df9`. Commit `971b22d3a` adds `TAC_IOS_RUNTIME` to `scripts/ios/native-test.mjs`, `page-services-test.mjs` and `rust-crate-test.mjs`, so they can boot an older installed runtime. `ty` is a debug build of that commit (Xcode 27.1, macOS 26.6.2).

Evidence is in `/Volumes/FORGE/Worktrees/rm2-ios-sim/rm2-evidence/ios17/`, logs in `/Volumes/FORGE/Logs/TACHYON/rm2-ios/ios17-*.log`. Neither is committed.

## iOS 17.0.1 (21A342)

Installed with `xcrun simctl runtime add`.

| Check | Result | Device | Command | Observed |
| --- | --- | --- | --- | --- |
| Install, launch and linked Rust (`ty build --target ios`) | PASS | iPhone SE (3rd generation) | `TAC_IOS_RUNTIME=17.0 node scripts/ios/native-test.mjs` | The WKWebView rendered both routes, with input and focus. Swift methods and publish worked, and the clipboard bridge too. The linked Rust service ran in the app process (`rust-start.json` origin `ios`), and its sync member `double()` took the tally from 21 to 42 (`ios:42`). Relaunch storage gave `local=kept,store=kept,cache=kept`. The window-settings step that follows (it runs on the newest runtime, iOS 26.5) failed once at load average 120 with "theme_color fills the status bar: 247,237,172" and passed when run alone |
| Linked Rust with vendored crates | PASS | iPhone 15 Pro | `TAC_IOS_RUNTIME=17.0 node scripts/ios/rust-crate-test.mjs` | The page clicked Greet and the Rust service answered `hello from ios` (`greeted.png`) |
| `ty bundle --target ios` apps at the 17.0 floor (Go WebAssembly, linked Rust) | FAIL, scoped style only | iPhone 15 Pro | `TAC_IOS_RUNTIME=17.0 TAC_IOS_SERVICE_LANGUAGES=go,rs node scripts/ios/page-services-test.mjs` | Both apps installed and launched. The click answered `2 in the basket` and `2 items`, and `/items/7/` opened (`go-root.png`, `go-item.png`, `rs-root.png`, `rs-item.png`). But the component style did not apply: the probe paragraph was `rgb(0, 0, 0)`, not `rgb(10, 20, 30)` |
| Async linked Rust (LR1) | not exercised | | | No check here calls an async linked Rust member |

Finding: Tac compiles component styles to CSS `@scope` (`crates/tachyon-core/src/compiler/component_styles.rs`). WebKit supports `@scope` only from iOS 17.4 and Safari 17.4. On iOS 17.0.x every component's `tac.css` is therefore dropped, while the iOS floor (`tac.application.minimumSystem`) is 17.0. Global `tac.styles` still apply (`native-test.mjs` checks `--ios-gate`). Either the floor or the style output needs to change.

## iOS 18: failed `simctl runtime add` attempts

`xcrun simctl runtime add` of `iOS_18_Simulator_Runtime.dmg` failed twice with `SimDiskImageError code=14: Cannot copy the image because the disk is almost full`:

- first with 6.4 GB free on the internal disk, while iOS 17.0.1 was installed;
- again after I removed iOS 17.0.1, with 9.0 GB free. The image is 8.6 GB.

Correction to my first report: the failed adds did leave data behind. About 38 GB of half-finished installs stayed in `/Library/Developer/CoreSimulator/Cryptex/Images` (I had checked only `Images`), and they pushed the internal disk down to 0.9 GB free. Samuel removed them with sudo. I made no further `runtime add` attempt.

## Runtimes kept on FORGE (no `simctl runtime add`)

`simctl runtime add` has no option for where the image goes: it always stages it into the root-owned area on the internal disk, and `--move` only deletes the source file afterwards. Instead, a hand-mounted image on FORGE works:

1. `hdiutil attach -readonly -nobrowse /Volumes/FORGE/iOS_17.0.1_Simulator_Runtime.dmg` mounts `/Volumes/iOS 17.0.1 21A342 Simulator`.
2. For iOS 18, `/Volumes/FORGE/iOS_18_Simulator_Runtime.dmg` is a bundle. Its inner `Restore/096-69245-692.dmg` (8.4 GB) is mounted the same way and gives `/Volumes/iOS 18.0 Simulator`.
3. A symlink to each `.simruntime` (`…/Library/Developer/CoreSimulator/Profiles/Runtimes/iOS 17.0.simruntime` and `iOS 18.0.simruntime`) goes in `~/Library/Developer/CoreSimulator/Profiles/Runtimes/` (a few bytes on the internal disk). `xcrun simctl list runtimes` then lists `iOS 17.0 (17.0.1 - 21A342)` and `iOS 18.0 (18.0 - 22A3351)`, with no service restart.
4. Devices and fixtures go in the APFS sparse bundle on FORGE: `hdiutil attach -nobrowse /Volumes/FORGE/offload/CoreSimulator-diag/SimDevices.sparsebundle` mounts `/Volumes/SimDevices`. The tests run with `TMPDIR=/Volumes/SimDevices/rm2-tmp/` and `TAC_PROBE_DIR=/Volumes/SimDevices/rm2-tmp`.

What still lands on the internal disk: CoreSimulator builds each runtime's dyld shared cache in `/Library/Developer/CoreSimulator/Caches/dyld/25G83/<runtime>/` (root-owned). That took 2.6 GB for iOS 17.0.1 and 3.4 GB for iOS 18.0 (beside 3.0 GB for iOS 26.5). It is built once, in the background, after the runtime first shows up. For iOS 18.0 the build took 52 minutes (17:12 to 18:04) at a load average of 50 to 120.

Sizes, measured before and after each step (`/Volumes/FORGE/Logs/TACHYON/rm2-ios/forge-mount-sizes.log`):

- `Images` stayed at 4 KB throughout.
- `Cryptex/Images` stayed at 0 KB throughout.
- `Caches` went from 3.18 GB to 9.44 GB: the two dyld caches above.

While the iOS 18.0 cache was still being built, a fresh iOS 18.0 device sat in "Waiting on Data Migration" for 23 minutes and ended with "Data Migration Failed", and `simctl launch` hung. Once the cache existed, a fresh device booted in 297 s at load average 69, so the scripts now wait up to 600 s for `bootstatus`.

## iOS 18.0 (22A3351), hand-mounted from FORGE

Base: develop `4889d1a58`, whose iOS floor is 17.4 (ADR 0108), plus this branch's script commit. `ty` is a debug build. Evidence is in `rm2-evidence/ios18/`, logs in `/Volumes/FORGE/Logs/TACHYON/rm2-ios/ios18-*.log`.

| Check | Result | Device | Command | Observed |
| --- | --- | --- | --- | --- |
| Install, launch and linked Rust (`ty build --target ios`) | PASS | iPhone SE (3rd generation) | `TAC_IOS_RUNTIME=18.0 node scripts/ios/native-test.mjs` | Render, input and focus, Swift methods and publish, and the clipboard bridge passed. The linked Rust service ran with origin `ios`, and its sync member `double()` took the tally to 42 (`ios:42`). Relaunch storage gave `local=kept,store=kept,cache=kept`. The first attempt stopped in `scripts/native/apple-protocol-test.mjs` with `cannot find 'IosFilePicker' in scope`, because the iOS bridge now calls the file picker; that test's Swift fixture now stubs `IosFilePicker`. The window-settings step that follows (iOS 26.5) failed again at load average 106 with "theme_color fills the status bar: 247,237,172"; it passed when run alone at lower load |
| Linked Rust with vendored crates | PASS | iPhone 16 Pro | `TAC_IOS_RUNTIME=18.0 node scripts/ios/rust-crate-test.mjs` | `hello from ios` |
| `ty bundle --target ios` apps at the 17.4 floor (Go WebAssembly, linked Rust) | PASS | iPhone 16 Pro | `TAC_IOS_RUNTIME=18.0 TAC_IOS_SERVICE_LANGUAGES=go,rs,kt node scripts/ios/page-services-test.mjs` | MinimumOSVersion 17.4. The click answered `2 in the basket` and `2 items`, `/items/7/` opened, and the component style applied (`rgb(10, 20, 30)` inside the component, `rgb(0, 0, 0)` outside) |
| 18.2-floor app (Kotlin) on 18.0 | refused at install, as expected | iPhone 16 Pro | same run | `App installation failed: "TallyKt" Requires a Newer Version of iOS … Have 18.0; need 18.2` (IXUserPresentableErrorDomain 16). A Kotlin, Dart or Java app needs an 18.2+ runtime, and none is available |
| Async linked Rust (LR1) | not exercised | | | |

iOS 17.0.1 through the FORGE mount (before the 17.4 floor change, on develop `4eac45df9`) gave the same results as the installed runtime above: `native-test.mjs` passed, `rust-crate-test.mjs` passed, and the Go and Rust apps failed only on the `@scope` style. Evidence is in `rm2-evidence/ios17-forge-mount/`.

## Runtimes now

- iOS 26.5: installed (system asset).
- iOS 17.0.1 and iOS 18.0: not installed. They are reachable only while their images are mounted from FORGE and the two symlinks are in `~/Library/Developer/CoreSimulator/Profiles/Runtimes/`. To remove them: delete the symlinks, `hdiutil detach` the volumes, and the dyld caches in `/Library/Developer/CoreSimulator/Caches/dyld/25G83/` (root-owned) need Samuel.
- iOS 17.0.1 was installed earlier with `simctl runtime add` and then removed with `simctl runtime delete`.
