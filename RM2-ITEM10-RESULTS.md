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

## iOS 18 (`iOS_18_Simulator_Runtime.dmg`)

Not installed. `xcrun simctl runtime add` failed twice with `SimDiskImageError code=14: Cannot copy the image because the disk is almost full`:

- first with 6.4 GB free on the internal disk, while iOS 17.0.1 was installed;
- again after I removed iOS 17.0.1, with 9.0 GB free. The image is 8.6 GB.

Within minutes the internal disk fell to about 0.9 GB free. Other processes on the Mac used that space; none of my files were left behind. `/System/Library/AssetsV2/com_apple_MobileAsset_iOSSimulatorRuntime` holds 17 GB, including the stray iOS 26.0 asset that the disk cleanup thread owns.

## Runtimes now

- Removed: iOS 17.0.1 (`xcrun simctl runtime delete C45B4530-0EB8-4F44-9C49-2EA018C17A6D`), to make room for iOS 18. Re-add it with `xcrun simctl runtime add /Volumes/FORGE/iOS_17.0.1_Simulator_Runtime.dmg`.
- Installed: iOS 26.5 only.
- iOS 18 needs about 9 GB free on the internal disk, plus headroom.
