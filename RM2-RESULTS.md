# RM2 results: iOS simulator runs (Mac leg)

Date: 2026-10-24. Host: the Mac (macOS 26.6.2, Xcode 27.1 27A9275).
Base: develop `4c052177e`. Branch: `claude/project-thread-rngd5i`.
Simulator: iOS 26.5 (23F77), the newest installed runtime. Xcode 27.1 offers no newer iOS runtime.
`ty`: a debug build of the branch (`cargo build --locked --bin ty`), passed to each script as `TAC_BIN`.

Script commits on the branch (no host code changed):

- `eb3329061`: `scripts/ios/page-checks-test.mjs` and `scripts/ios/simulator-tap.m` (items 2 to 6)
- `f0dc8224e`: `scripts/ios/page-services-test.mjs` (item 7)
- `2322ce2ab`: `scripts/ios/rust-crate-test.mjs`, `scripts/ios/browser-prompts-test.mjs` and `scripts/ios/screenshot-text.swift` (items 8 and 11)

Evidence (screenshots, reports and logs) is not committed, because the repository keeps no evidence folder.
It is in the worktree at `/Volumes/FORGE/Worktrees/rm2-ios-sim/rm2-evidence/`. Raw logs are in `/Volumes/FORGE/Logs/TACHYON/rm2-ios/`.

| # | Item | Result | Runtime and device | Commit | Command | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Cache API across relaunch (Tac-format entry) | PASS, 2 of 2 runs | iOS 26.5, iPhone SE (3rd generation) | `4c052177e` | `TAC_BIN=… node scripts/ios/native-test.mjs` | `rm2-evidence/item1/` (report.json, native-test.log, webkit-files.log, run2/) |
| 2 | Offline Yon reads, network-first and cache-first | PASS | iOS 26.5, iPhone 17 Pro | `eb3329061` | `TAC_BIN=… node scripts/ios/page-checks-test.mjs` | `rm2-evidence/page-checks/report.json`, first-launch.png, relaunch.png |
| 3 | preferenceStrings across terminate and relaunch | PASS | iOS 26.5, iPhone 17 Pro | `eb3329061` | same | same report (`first.themeAfter`, `relaunch.second.themeBefore`) |
| 4 | Background and foreground (ApplicationStateChanged) | PASS. Disposal on termination: not observable | iOS 26.5, iPhone 17 Pro | `eb3329061` | same | report `state`, `relaunch.terminate`; state-background.png, state-foreground.png |
| 5 | External links and the user-name rule | PASS (mailto: handed to the system, which has no handler on the simulator) | iOS 26.5, iPhone 17 Pro | `eb3329061` | same | report `links`; link-*.png |
| 6 | CSP / tac.outgoing: undeclared host refused | PASS | iOS 26.5, iPhone 17 Pro | `eb3329061` | same | report `connectSrc`, `first.undeclared` |
| 7 | WebAssembly page services, 18.2 floor, linked Rust, dynamic route, scoped style | PASS: py, cs (AOT), kt, dart, java, go, rb, lua, hpp, fs, rs | iOS 26.5, iPhone 17 Pro | `f0dc8224e` | `TAC_BIN=… node scripts/ios/page-services-test.mjs` (plus `TAC_IOS_SERVICE_LANGUAGES=cs TAC_IOS_DOTNET_AOT=false` for the interpreter comparison) | `rm2-evidence/page-services/report.json` and `<language>-root.png` / `<language>-item.png`; `rm2-evidence/page-services-cs*/` |
| 8 | Rust service with a vendored crate | PASS | iOS 26.5, iPhone 17 Pro | `2322ce2ab` | `TAC_BIN=… node scripts/ios/rust-crate-test.mjs` | `rm2-evidence/rust-crate/report.json`, greeted.png |
| 9 | Byte-identical iOS rebuild | PASS | (build only) | `4c052177e` | `cargo test --locked -p tachyon-core --lib native_reproducible_build_tests::a_packaged_ios_app_is_byte_identical_when_built_twice -- --ignored` | `rm2-evidence/item9/cargo-test.log` |
| 10 | Oldest iOS 17.x and 18.x runtimes | BLOCKED | n/a | n/a | `xcodebuild -downloadPlatform iOS -buildVersion <v>` | `/Volumes/FORGE/Logs/TACHYON/rm2-ios/dl-probe-*.log` |
| 11 | Browser-API prompts from screenshots | PASS | iOS 26.5, iPhone 17 Pro | `2322ce2ab` | `TAC_BIN=… node scripts/ios/browser-prompts-test.mjs` | `rm2-evidence/browser-prompts/` (report.json, location-prompt.png, notification-prompt.png, notification-banner.png, share-sheet.png) |

## What was observed

### 1. Cache API across relaunch

`scripts/native/relaunch-storage-probe.mjs` already writes the entry in Tac's format: cache `tachyon-cache:relaunch`, a JSON body, and an `x-tachyon-expires-at` one hour ahead. The run did not change it.

- Run 1: launch pid 43666 wrote `written`. After `simctl terminate`, relaunch pid 44132 read `local=kept,store=kept,cache=kept`.
- Run 2: pid 24406 wrote, and pid 24500 read `local=kept,store=kept,cache=kept`.

A file watcher on the app's data container recorded what WebKit kept (`rm2-evidence/item1/webkit-files.log`):

- On the first launch, WebKit wrote a 714-byte record at `Library/WebKit/dev.tachyon.ios-gate/WebsiteData/Default/<origin hash>/<origin hash>/CacheStorage/Version 16/Records/<cache id>/<uuid>/<record>`. It also wrote `cacheslist` (76 bytes), `estimatedsize` and `salt` beside it.
- The same record file was still there, unchanged, after the terminate and the relaunch, and the new process read the value from it.
- IndexedDB (`IndexedDB.sqlite3`) and localStorage (`localstorage.sqlite3`) were in the same origin folder.

So the old "cache name kept, entries not" result came from Tac's start-up sweep deleting the raw entry, not from WebKit.

Item 2 shows the same for `tachyon-yon-reads`, whose keys are `http://127.0.0.1:<port>` Yon URLs: both kept reads answered after a terminate and relaunch.

Note: `native-test.mjs` `keptStorage` still accepts `cache=missing` with the old `cacheLimit` note. Tightening it is left to the cloud thread together with the support cells.

### 2. Offline Yon reads (tac.cache)

The manifest has `tac.cache: [{"/api/notes", "network-first"}, {"/api/kept", "cache-first"}]` and `tac.yon.origin = http://127.0.0.1:<port>`. The `@Cached` client has `ttlMs: 1`, so each call goes past the `@Cached` layer as a public read.

- Yon running:
  - Network-first gave two different answers, 200 ms apart (`Note 1792845188524`, then `Note 1792845188869`).
  - Cache-first gave the same answer twice (`Kept 1792845188986`).
  - Cache Storage `tachyon-yon-reads` held `/api/notes` and `/api/kept`.
- Yon stopped (the airplane-mode stand-in):
  - Network-first answered `Note 1792845188869`, the last kept copy.
  - Cache-first answered `Kept 1792845188986`.
  - A read with no cache rule (`/api/fresh`) failed (`failed adapter`).
- After `simctl terminate` and a relaunch, with Yon still stopped, the note read still answered `Note 1792845188869` and the kept read `Kept 1792845188986`. Both entries were still listed.

### 3. preferenceStrings

- First launch: `read('theme')` gave `null`. After `write('theme', 'dark')`, `read('theme')` gave `dark`.
- After `simctl terminate` and a new process (a new pid, checked): `read('theme')` gave `dark`.

### 4. Background and foreground

- At start the page heard `foreground` (`@OnEvent({"replay":true})`).
- `simctl launch … com.apple.Preferences` sent the app to the background. The linked Swift service's `@OnEvent ApplicationStateChanged` logged `native:background`, and the page service logged `page:background` through a Swift call.
- `simctl launch <app>` brought it back: `native:foreground`, then `page:foreground`.
- The host telemetry (`Library/Logs/Tachyon/<id>.jsonl`) recorded `application.background` and `application.foreground` for every switch, including the link runs.

Termination disposal is not observable with `simctl terminate`. The app is killed: no `controller.destroyed` telemetry, and the Swift `@OnStop` wrote nothing. Seeing `applicationWillTerminate` would need a swipe-up in the app switcher, which these tools cannot drive.

### 5. External links

Each link was clicked from the page. The test checked whether the app left the foreground (the Swift `native:background` line) and read the simulator log (`SpringBoard … Handling OpenURL from PageChecks`, `lsd … requests to open URL`).

| Link | Result |
| --- | --- |
| `https://example.com/tachyon-rm2-https` (script click) | Safari opened (log: `Handling OpenURL from PageChecks:<pid>: https://example.com/tachyon-rm2-https`); the screenshot shows Safari at example.com with "◀ PageChecks" |
| `http://example.com/tachyon-rm2-http` | Safari opened |
| `https://…-tapped` (a real tap with `simulator-tap.m`) | Safari opened |
| `target="_blank"` by a script click | nothing opened and no open request: WebKit blocks a script-opened new window without a user gesture |
| `target="_blank"` by a real tap | Safari opened (`Handling OpenURL … tachyon-rm2-blank-tapped`) |
| `mailto:rm2@example.com?subject=tachyon-rm2` | handed to the system (`lsd: pid … requests to open URL with scheme mailto`), but the simulator has no Mail app: `ERROR: There is no registered handler for URL scheme mailto`, `LSApplicationWorkspaceErrorDomain Code=115`; the app stayed in front |
| `https://someone@example.com/…` (user name) | refused: no open request, the app stayed in front |
| `https://someone:secret@example.com/…` | refused |
| `ftp://example.com/…` | refused |
| `mailto:` (empty) | refused |

### 6. CSP / tac.outgoing

- The generated host's policy is `connect-src 'self' http://127.0.0.1:<yon> ws://127.0.0.1:<yon>`.
- A page fetch to an undeclared local host was refused, with a `securitypolicyviolation` whose effective directive was `connect-src`. That host's server logged no request.
- The Yon origin answered 200.

### 7. WebAssembly page services

Each language got its own app, made from `website/content/features/web-assembly-services` plus a probe page service (JavaScript, with a linked Swift recorder). In one clean run, every app:

- clicked "Add two" and read `2 in the basket` and `2 items` (a method with an argument);
- found the component style `rgb(10, 20, 30)` inside the component and not on the tally paragraph;
- opened the dynamic route `/items/7/`.

| Language | MinimumOSVersion | WebAssembly in the bundle | Ready after page start |
| --- | --- | --- | --- |
| py (Pyodide) | 17.0 | 18747 KB | 3580 ms |
| cs (`tac.dotnet.aot: true`) | 17.0 | 14385 KB | 305 ms |
| kt | 18.2 | 186 KB | 93 ms |
| dart | 18.2 | 92 KB | 79 ms |
| java | 18.2 | 151 KB | 82 ms |
| go | 17.0 | 629 KB | 86 ms |
| rb | 17.0 | 2150 KB | 98 ms |
| lua | 17.0 | 340 KB | 87 ms |
| hpp (C++) | 17.0 | 152 KB | 78 ms |
| fs | 17.0 | 18283 KB | 246 ms |
| rs | 17.0 | none (linked) | 76 ms |

- The 18.2 floor: the Kotlin, Dart and Java apps were built with MinimumOSVersion 18.2 and opened on the iOS 26.5 simulator. No iOS 18.x runtime was available (item 10).
- C# ahead of time: `dotnet.native.wasm` was 5,778,393 bytes with `tac.dotnet.aot: true` and 2,978,728 bytes without it (`TAC_IOS_DOTNET_AOT=false`). Both builds passed.
- `ty bundle --target ios` prints no `tac.dotnet` release note for native builds; the note is printed only for web bundles.

Observation: every WebAssembly file is in the iOS bundle twice, under `WebBundle/.tachyon/services/…` and under `WebBundle/tachyon-runtime/services/…`. For example, `pyodide.asm.wasm` and each .NET `_framework` file appear in both places, so the sizes above count each file twice.

### 8. Vendored Rust crate

- The manifest declares `tac.application.rust.dependencies {"tiny-greeting": "=1.0.0"}` and `platformDependencies.ios {"tiny-platform": "=1.0.0"}`, both vendored under `vendor/rust/`.
- The generated `rust-companion/Cargo.toml` lists `tiny-greeting` under `[dependencies]` and `tiny-platform` under `[target.'cfg(target_os = "ios")'.dependencies]`.
- On the simulator the page clicked Greet, and the linked Rust service answered `hello from ios`.

### 9. Byte-identical rebuild

`native::native_reproducible_build_tests::a_packaged_ios_app_is_byte_identical_when_built_twice` passed in 8.95 s.

### 10. Device stand-ins (iOS 17.x and 18.x)

Blocked. Xcode 27.1 offers neither runtime: `xcodebuild -downloadPlatform iOS -buildVersion` answered "iOS <v> is not available for download" for 17.0, 21A328, 17.0.1, 17.5, 18.0, 18.2 (also with `-architectureVariant universal`) and 18.6. Only the 26.x runtimes download; a 26.0 download started and was cancelled.

Apple's index (`devimages-cdn.apple.com/downloads/xcode/simulators/index2.dvtdownloadableindex`) lists:

- the oldest 17.x, iOS 17.0 (21A328, 7.56 GB), only at `download.developer.apple.com` with `authentication: virtual`, which needs an Apple developer sign-in;
- the oldest 18.x, iOS 18.0 (22A3351, 8.39 GB), only as a mobileAsset, which Xcode 27.1 does not offer.

The Mac has about 19 GB free on the internal disk, where simulator runtimes are installed, so only one of the two fits at a time.

To unblock: Samuel signs in at developer.apple.com, downloads "iOS 17.0 Simulator Runtime", and runs `xcrun simctl runtime add <dmg>`. For 18.x, an Xcode 16.x installation (or Apple's download page) is needed. Nothing was run on 17.x or 18.x, including the linked Rust set; the LR1 async `HTTP_MISSING` gap was not re-recorded here.

### 11. Browser-API prompts

The screenshot text below was read with Vision OCR (`scripts/ios/screenshot-text.swift`). Taps went to the button found by its text (`scripts/ios/simulator-tap.m`, through SimulatorKit). No new permissions were needed. The simulated location was set with `simctl location … set 51.5007,-0.1246`.

- Location:
  - Info.plist `NSLocationWhenInUseUsageDescription` = `Shows stores near you in the RM2 check`.
  - A tap on "Find my place" showed the prompt: "Allow "PromptCheck" to use your location? / Shows stores near you in the RM2 check / Allow Once / Allow While Using App / Don't Allow".
  - After a tap on "Allow While Using App", the page got `PositionFound` `51.5007, -0.1246`.
- Notification permission:
  - A tap on "Ask for notifications" showed: ""PromptCheck" Would Like to Send You Notifications / Notifications may include alerts, sounds, and icon badges. These can be configured in Settings. / Don't Allow / Allow".
  - iOS shows no app reason here, so the manifest's notifications reason does not appear.
  - After a tap on "Allow", the page heard `NotificationPermissionChanged` `granted`.
- Banner: a tap on "Show a notification" showed a foreground banner "Order 42 / Your order is ready / now".
- Share:
  - A tap on "Share a link" opened the share sheet, showing `example.com`, Reminders, More, Copy and Add to Reading List.
  - A tap outside the sheet closed it, and the page heard `BrowserCallFailed` "Share was cancelled.".
- `requestPermission` and `share.open` throw "needs a user action" when a script calls them outside a click. That is by design (`tac-web.js`), so the test taps real buttons.

## Environment changes on this Mac

- I installed the .NET workload `wasm-tools-net9` into FORGE's dotnet (`dotnet workload install wasm-tools-net9`). Without it, every C# and F# WebAssembly build failed with NETSDK1147, because `ty` targets net9.0. This also moved the SDK's workload set to 10.0.402.1; `wasm-tools` stayed at 10.0.113.
- No Apple account, keychain, profile or sudo change.
- No simulator runtime was installed.
- Each script deletes the simulator device sets it created under `$TMPDIR`.
