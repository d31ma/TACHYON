# ADR 0066: ty init Is a Setup Wizard

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0044](0044-an-annotated-file-holds-one-class.md) (one
  class per file), [ADR 0062](0062-yon-uses-native-http-types.md) (PHP runs
  with `php-cgi`), [ADR 0063](0063-services-and-controllers-are-cohesive.md)
  and [ADR 0064](0064-code-is-not-duplicated.md) (the guards stay on in a new
  app)

## Context

`ty init` wrote one JavaScript project with both realms. A developer who
wanted Python on the server or Rust in the page deleted files by hand, copied
examples from the website, and then learned from `ty test` or `ty preview`
which toolchains were missing. The website examples follow every guard, but a
hand-made mix of them often did not: a class without a test failed the
coverage check, and a Swift service without a web copy did not run in the
browser.

## Decision

`ty init` follows Spring Initializr: the developer chooses, and the generator
writes only what was chosen.

1. **Questions and options.** In a terminal, `ty init` asks five questions as
   numbered lists (plain lines, so a screen reader reads them): the realms
   (client, server or both), the Tac languages, the Yon languages, the targets
   and the examples. A question that does not fit an earlier answer is
   skipped. Every question is also an option: `--realm`, `--tac`, `--yon`,
   `--target` and `--features`. Without a terminal and without options,
   `ty init` keeps the defaults (both realms, JavaScript, the web), so scripts
   and the existing guides keep working. `--interactive` asks even without a
   terminal. An unknown or conflicting answer stops with `TY1408` before any
   file is written.
2. **Generated files.** Each chosen language gets one greeting class, a test
   unit that calls each public method, and a separate test setup. JavaScript
   keeps the old `ShellService` and `HomeController`. A Swift or Kotlin Tac
   service also gets a JavaScript copy for the web. Examples are JavaScript,
   or TypeScript when TypeScript is the only JavaScript-family language of
   that realm. The templates are real files under
   `crates/tachyon-core/src/scaffold/templates/`, compiled into `ty`. Every
   combination passes `ty test --coverage` and `ty build` with the cohesion,
   one-class, duplication, `NeedsAll`, template and native HTTP type guards on.
3. **Tool check.** After it writes the app, `ty init` runs the `ty doctor`
   checks on it and adds the target tools (Xcode, the Android SDK and Gradle,
   the Linux desktop libraries, .NET for Windows). For each missing tool it
   shows the exact commands and asks. `--yes` answers yes for every tool;
   without a terminal and without `--yes` it only prints the commands.
4. **Installers.** Tools are installed with the vendor's official installer
   into the home folder, at a version that Tachyon is tested with: the Bun,
   Go and Dart archives, `rustup-init`, the .NET install script, SDKMAN for
   Java, Kotlin and Gradle, and emsdk for Emscripten. Each archive is checked
   against the SHA-256 that its vendor publishes; the emsdk hash is pinned in
   `runtimes/ruby-web/sources.txt`. A mismatch deletes the download and stops
   with `TY1409`. Tachyon never runs a command with administrator rights: it
   prints it (for example `sudo apt-get install -y php-cgi`). Xcode and the
   Android SDK get a link and plain steps.
5. **Growing an app.** `ty add language client|server <language>`,
   `ty add target <target>` and `ty add feature <example>` write the same
   files and run the same tool check. They read the existing app from its
   folders, never overwrite a file (`TY1402`), and add the new component to
   the home page.

## Consequences

- A new app of any combination starts green, and a developer sees the next
  language's shape by adding it rather than by copying a website example.
- The installer table in `crates/tachyon-core/src/tool_setup/` pins versions.
  Updating a toolchain means updating that table and testing it again.
- Tests never install anything: the installer runs its steps through a
  `ToolRunner`, and the tests use a fake runner.
- The .NET install script and the SDKMAN install script have no published
  checksum. Tachyon says so before it runs them; both check the files that
  they download themselves.

## Amendment (2026-10-18): target tools match the hosts, and `ty doctor` reports them

- The target tools are the ones each host actually uses. Linux: a C
  compiler, `pkg-config`, GTK 4 and WebKitGTK 6.0 (`libgtk-4-dev` and
  `libwebkitgtk-6.0-dev` on Debian and Ubuntu). Windows: the Microsoft C++
  build tools (`cl` and the Windows SDK's `rc.exe`, from a Visual Studio
  developer prompt) and the WebView2 SDK in `TAC_WEBVIEW2_SDK`. .NET is no
  longer a Windows target tool; it is still checked when the app has C#
  services. Android: `ANDROID_HOME` (or `ANDROID_SDK_ROOT`) with an SDK
  platform, Gradle and a JDK. macOS: `xcrun`. iOS: Xcode (`xcodebuild`) and
  the iOS simulator SDK.
- `ty doctor` reports these tools for each native target that the project
  declares (a `bundle:<target>` script in `package.json`), as `info` rows
  named `<target> apps`. They never change the exit status. `ty init`,
  `ty add target` and `ty doctor --install` take the same rows from the same
  table (`doctor/target_toolchains.rs`), so all three name the same tools and
  offer the same recipes. MSVC and the WebView2 SDK get plain steps, like
  Xcode and the Android SDK.

## Amendment (2026-10-19): `ty doctor` reports the release tools, and `--release` needs them

- For each declared native target, `ty doctor` also reports the tools that
  build and sign its release (`ty bundle --release`), as rows named
  `<target> releases`. macOS: `xcrun notarytool` and a Developer ID
  Application certificate in the keychain. iOS: the iPhoneOS SDK
  (`xcrun --sdk iphoneos`) and an Apple Distribution (or iPhone
  Distribution) certificate in the keychain. Android: `apksigner` and
  `zipalign` in a `build-tools` folder of the SDK in `ANDROID_HOME` or
  `ANDROID_SDK_ROOT`. Windows: `makeappx.exe` and `signtool.exe` on PATH.
  Linux: `dpkg-deb` and `mksquashfs`.
- Certificates are counted from `security find-identity -v -p
  codesigning`. The report shows only how many valid ones there are, never
  their names, teams or hashes.
- Like the `<target> apps` rows, these rows are `info` and never change the
  exit status. `ty doctor --release` makes the `<target> apps` and
  `<target> releases` rows needed for each declared target that this
  machine can build (macOS and iOS on a Mac, Windows on Windows, Linux on
  Linux, Android anywhere), so a missing tool exits with 1. Rows of targets
  that build elsewhere stay `info`, so one machine can pass for a project
  that declares every target.
- Evidence (develop 6d74c93f, 2026-10-19). Mac (macOS 26, Xcode 27.1):
  notarytool 1.1.3, one Developer ID Application certificate, the iPhoneOS
  27.1 SDK and, with the Android SDK set, build-tools 35.0.0 were ready; no
  Apple Distribution certificate is installed. Surface (Windows 11 ARM64,
  over SSH from an arm64 developer prompt): `makeappx.exe` and
  `signtool.exe` were ready. Linux cloud container: `dpkg-deb` 1.22.6 and
  `mksquashfs` 4.6.1 were ready. Those runs made every declared target's
  rows needed, so `--release` exited with 1 on every machine for a project
  that declares all five targets; that led to the per-machine rule above.
- The rows live in `doctor/release_toolchains.rs`. `ty init`, `ty add target`
  and `ty doctor --install` do not install them: signing certificates and
  the platform SDKs are installed by hand, and the install text says how.
- `TAC_MAKEMSIX` and `osslsigncode` are not probed. Only the Windows package
  tests use them; `ty bundle --target windows --release` runs `makeappx.exe`
  and `signtool.exe`.

## Amendment (2026-10-24): the Android NDK, `TAC_MAKEMSIX` and `osslsigncode`

PKG7 adds three rows to `ty doctor`.

- A project that declares android and has a native Rust service gets an
  `android Rust services` row for the Android NDK, found the way the build
  finds it: `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, or the newest folder
  under the SDK's `ndk` folder that holds the LLVM toolchain. It is needed
  under `--release`, like the `apps` rows. A project without a native Rust
  service does not get the row.
- Away from Windows, a project that declares windows gets two
  `windows package checks` rows, `TAC_MAKEMSIX` (a file) and `osslsigncode`.
  They are never needed: `ty bundle --target windows --release` runs only on
  Windows, with `makeappx.exe` and `signtool.exe`, and these two tools serve
  Tachyon's MSIX package test on Linux and macOS. On Windows the rows are
  not listed.
- `ty doctor` now reads the project folder by its canonical path, so
  `ty doctor .` (or a path ending in `..`) no longer stops with `TY2101` in a
  project with services.
- Evidence (2026-10-24, x86_64 Linux cloud container): on the PKG7 probe app
  with android and windows declared, the NDK row was ready from the SDK's
  `ndk/30.0.16248370` (also with `ANDROID_NDK_HOME` set to a missing folder),
  and both package check rows were `info`. Unit tests cover the rows and the
  file probe.
