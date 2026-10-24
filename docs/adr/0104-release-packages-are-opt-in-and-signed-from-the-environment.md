# ADR 0104: Release Packages Are Opt-In and Signed From the Environment

- Status: Accepted for Android, macOS, Linux, Windows and iOS (amended 2026-10-19)
- Date: 2026-10-18
- Related: [ADR 0052](0052-the-manifest-is-the-tac-configuration.md)
  (the version and build number of a native app),
  [ADR 0102](0102-every-manifest-member-states-its-reach-per-target.md)
  (the "release packaging" row of the support table)

## Context

`ty bundle --target <native>` builds an app that a developer can run on the
build machine or in a simulator or emulator. It is not a package that a store
or a user can install from a trusted source. Before this decision, the
Android build was always a debug APK: debuggable, signed with the Android
SDK's debug key, and without an Android App Bundle (`.aab`), which Google
Play requires.

A store package must be signed with the developer's own key. That key and its
passwords are secrets. ADR 0034 makes `client/manifest.json` public
configuration, so the key can never be named there.

## Decision

1. A release package is opt-in: `ty bundle --target <target> --release`.
   Without `--release`, builds stay what they were.
2. Signing material comes only from environment variables or the platform
   keychain, never from `client/manifest.json`. `ty` never writes a password
   into a generated file or prints it.
3. `--release` is refused when it cannot do what it says:
   - a target that has no release packaging yet stops the build with
     `TY1601` before anything is built;
   - `--native`, which only stages the host project, refuses `--release`
     (`TY1001`);
   - a web-only build refuses `--release` (`TY1001`), because the web bundle
     is always a release build.
4. **Android.** `ty bundle --target android --release` runs Gradle's
   `assembleRelease` and `bundleRelease` and publishes
   `<Name>/<Name>.apk` and `<Name>/<Name>.aab`, both signed with the upload
   key from these variables:

   | Variable | Value |
   | --- | --- |
   | `TAC_ANDROID_KEYSTORE` | The keystore file (`.jks` or `.p12`). A relative path is read from the directory where `ty` runs. |
   | `TAC_ANDROID_KEYSTORE_PASSWORD` | The keystore password. |
   | `TAC_ANDROID_KEYSTORE_ALIAS` | The alias of the key in the keystore. |
   | `TAC_ANDROID_KEYSTORE_KEY_PASSWORD` | Optional. The key's own password; the keystore password when it is not set. |

   - `ty` checks the first three before Gradle runs and stops with `TY1605`,
     naming every missing variable, or when the keystore is not a file.
   - The generated `app/build.gradle.kts` reads the same variables in its
     `release` signing config, so the staged project also builds a signed
     release in Android Studio or on a CI machine.
   - A release build is not debuggable, so the host does not turn on web
     contents debugging in it. Code shrinking stays off.
   - The version name and code are `tac.application.version` and
     `tac.application.build` (ADR 0052).
   - Uploading to Google Play is out of scope.

## Pending

Each item below is decided by its own ADR amendment when its packaging lands:

- macOS: decided in the 2026-10-19 amendment below.
- iOS: decided in the 2026-10-19 amendment below.
- Linux: AppImage and `.deb` packages (decision D4). Decided in the
  2026-10-19 amendment below.
- Universal macOS binaries: no longer pending. `ty bundle --target macos
  --arch universal` builds an arm64 and an x86_64 app and joins the app and
  each Rust helper with `lipo` (ADR 0097 amendment of 2026-10-18), and a
  `--release` build signs that app (below). Only the staging and the
  artifact manifest are tested. Still owed: the Mac test that builds the
  universal app and checks it with `lipo -archs`, and a run on an Intel Mac.
- Android ABI selection.

## Consequences

- A developer can build a Play-ready `.aab` and a signed APK from the same
  manifest, with no Android Studio step.
- CI machines keep the keystore as a secret file and the passwords as secret
  variables. `ty` inherits them like any other build tool.
- The emulator gate (`scripts/android/native-test.mjs`) builds a release with
  a throwaway keystore, checks both signatures and runs the release APK.

## Amendment (2026-10-19): macOS ships a Developer ID–signed, notarized `.dmg`

Decision D2 (Samuel, 2026-10-19): macOS apps are distributed outside the Mac
App Store as a Developer ID–signed, notarized disk image, on the project's
Apple Developer team. A Mac App Store variant comes later and is not built.

1. `ty bundle --target macos --release` builds the app as before, for the
   `--arch` choice (a universal app included), then:
   - signs each Rust helper in `Contents/MacOS`, then the app, with
     `codesign --force --options runtime --timestamp`, so every binary has
     the hardened runtime and a secure timestamp;
   - gives the app entitlements derived from the manifest grants, written to
     `project/TachyonHost.entitlements`: a `geolocation` grant in
     `tac.browserApis` adds `com.apple.security.personal-information.location`,
     which CoreLocation needs under the hardened runtime. The app is not
     sandboxed, so `filePicker`, `tac.yon.origin` and `tac.outgoing.hosts`
     need no entitlement. The sandbox entitlements (user-selected read-only
     files, network client) belong to the later Mac App Store variant;
   - verifies the app with `codesign --verify --strict --deep`;
   - writes `dist/macos/<Name>.dmg` with `hdiutil` (UDZO), holding the app and
     an `Applications` shortcut;
   - signs the `.dmg`, submits it with `xcrun notarytool submit --wait`,
     staples the ticket with `xcrun stapler staple`, then checks it with
     `xcrun stapler validate` and `spctl --assess --type open`.
   The build prints the `.dmg` as its result; the `.app` stays next to it.
2. Signing material never comes from `client/manifest.json`:

   | Variable | Value |
   | --- | --- |
   | `TAC_APPLE_TEAM_ID` | The team ID (10 capital letters and digits). `ty` picks the keychain's one valid "Developer ID Application: … (team ID)" certificate from `security find-identity -v -p codesigning`. |
   | `TAC_APPLE_NOTARY_PROFILE` | A `notarytool` keychain profile (`xcrun notarytool store-credentials`), so no Apple ID password or API key reaches `ty`. |
   | `TAC_MACOS_SIGNING_IDENTITY` | Optional. A certificate name or SHA-1 hash, for a keychain with several certificates for the team. `-` signs ad hoc. |

   `TAC_APPLE_TEAM_ID` and `TAC_APPLE_NOTARY_PROFILE` are shared with the iOS
   release (PKG2); the identity variable names the platform because macOS and
   iOS sign with different certificates.
3. A missing variable or a malformed team ID stops the build with `TY1605`
   before the web build starts. No certificate for the team, or several,
   stops it with `TY1605` before signing. A notarization answer other than
   `Accepted` stops it with `TY1605`, naming the submission and the
   `xcrun notarytool log` command that shows Apple's reasons.
4. `TAC_MACOS_SIGNING_IDENTITY=-` is the test path for a Mac without the
   team's certificate: the same hardened runtime and entitlements, an ad hoc
   signature without a timestamp, no notarization, and a build note saying
   the `.dmg` opens only on that Mac.
5. A release `.dmg` is not byte-identical between builds: the secure
   timestamp, `hdiutil` and the stapled ticket differ each time (ADR 0005).

Consequences: the macOS row of the support table's "release packaging"
entry says `honored`, since a Developer ID release was notarized, stapled and
accepted by Gatekeeper on a Mac on 2026-10-19;
`crates/tachyon-cli/tests/it/macos_release_cli.rs` holds the Mac tests, the
Developer ID one ignored unless the team's certificate and profile are
present.

## Amendment (2026-10-19): Linux AppImage and `.deb`

Samuel chose AppImage and `.deb` for Linux (decision D4); Flatpak comes later.

1. `ty bundle --target linux --release` builds the app folder as before, then
   writes two packages next to it in `dist/linux/`:
   - `<Name>-<version>-<build>-<machine>.AppImage`, for example
     `Catalog-1.4.0-12-x86_64.AppImage`;
   - `<package>_<version>-<build>_<architecture>.deb`, for example
     `catalog_1.4.0-12_amd64.deb`.

   The package name is the app's executable name in lowercase. The build
   prints the AppImage path; the `.deb` sits beside it.
2. Both packages install the same tree:
   - the app folder in `/usr/lib/<package>/`, so the host still finds
     `resources/` and its Rust library next to its own `bin/`;
   - the command `/usr/bin/<package>`, a link to the app;
   - `/usr/share/applications/<id>.desktop`, with `Exec=<package>` and the
     themed icon;
   - AppStream metainfo in `/usr/share/metainfo/<id>.metainfo.xml`, with
     the manifest's `description` as the summary and
     `tac.application.version` as the release;
   - each raster icon in the hicolor theme at its declared size.

   `<id>` is the GTK application id: `tac.application.id` with each hyphen
   turned into an underscore, so the desktop file matches the window.
3. The `.deb` version is `<version>-<build>`. A release stops with `TY1601`
   before anything is built when `tac.application.version` does not start
   with a digit, when the lowercase executable name is shorter than two
   characters, or when the build machine is not x86_64 or aarch64. The
   package depends on `libc6`, `libgtk-4-1` and on `libwebkitgtk-6.0-4` at
   `tac.application.minimumSystem.linux` or newer. `dpkg-deb` builds it with
   root as the owner of every file.
4. The AppImage is the same tree in an AppDir, with `AppRun` linked to the
   command, compressed by `mksquashfs` (zstd) and joined to the AppImage
   type 2 runtime. Tachyon pins the runtime of release `20251108` by SHA-256
   for x86_64 and aarch64, downloads it once into the runtime cache
   (`ty cache clean` removes it), and checks the pin each time.
   `TAC_APPIMAGE_RUNTIME` names a local runtime file instead, for offline
   build machines.
5. Both packages use the system's GTK 4 and WebKitGTK 6. The AppImage does
   not bundle the web engine: WebKitGTK starts helper processes from paths
   fixed when it was built, so a bundled copy needs a patched build. A
   self-contained Linux package comes with Flatpak, whose GNOME runtime
   holds WebKitGTK.
6. Signing: `ty` does not sign Linux packages. A `.deb` is trusted through
   the signed `Release` file of the apt repository that serves it, and an
   AppImage signature is optional in its format.
   `TAC_LINUX_MAINTAINER` (for example `Ada Lovelace <ada@example.com>`)
   fills the `.deb` `Maintainer` field; without it the field is the app's
   name, because dpkg warns about a package with no maintainer each time it
   reads its database.
7. `SOURCE_DATE_EPOCH` reaches `dpkg-deb` and `mksquashfs`, and the AppStream
   release date is the UTC day of that value, so two release builds give the
   same packages byte for byte. The release is a `date`, not a `timestamp`,
   because `appstreamcli` refuses a timestamp of 0, which is the value when
   `SOURCE_DATE_EPOCH` is unset; the date is then 1970-01-01.
8. The deep-link scheme handler that LK2 adds to the desktop file goes into
   both packages without a change here.

Checked on 2026-10-19 in an x86_64 Ubuntu 24.04 cloud container (GTK 4.14.5,
WebKitGTK 2.52.6, dpkg 1.22.6, squashfs-tools 4.6.1), recorded in
`docs/SUPPORT_TIERS.md`. Owed: the same run on WSL Ubuntu 24.04 (aarch64),
and an install on a Linux desktop. (The WSL run of develop `38eaed29` passed
the eleven Linux host tests, but not these package tests: the three
`linux_package_tests` came in that same commit and were not part of it.)

## Amendment (2026-10-19): Windows builds an MSIX package

Decision D3 chose MSIX over a classic installer, because a package gives the
app an identity, which toast notifications and protocol activation need.

1. `ty bundle --target windows --release` builds the app folder as before,
   adds `AppxManifest.xml` and `Assets\` to it, packs it with
   `makeappx pack /h SHA256` and signs the package with
   `signtool sign /fd SHA256`. It publishes `<Name>/<Name>.msix` inside the app
   folder. `makeappx.exe` and `signtool.exe` come from the Windows SDK, which
   the Visual Studio developer prompt already puts on `PATH` for `rc.exe`.
2. Signing material comes from these variables, never the manifest:

   | Variable | Value |
   | --- | --- |
   | `TAC_WINDOWS_CERTIFICATE` | A `.pfx` file. A relative path is read from the directory where `ty` runs. |
   | `TAC_WINDOWS_CERTIFICATE_PASSWORD` | Optional. The `.pfx` password. |
   | `TAC_WINDOWS_CERTIFICATE_THUMBPRINT` | Instead of a file: the SHA-1 thumbprint of a certificate in the user's store (`signtool /sha1`), which is how hardware-token certificates are used. |
   | `TAC_WINDOWS_CERTIFICATE_PUBLISHER` | The certificate's subject. MSIX requires the package publisher to equal it. |
   | `TAC_WINDOWS_CERTIFICATE_TIMESTAMP_URL` | Optional. A timestamp server (`signtool /tr ... /td SHA256`). |

   `ty` checks them before the host is compiled and stops with `TY1605` when
   the publisher has no `CN=` part, when neither or both of the file and the
   thumbprint are set, when the file is missing, when the thumbprint is not 40
   hexadecimal digits, or when the timestamp URL is not `http(s)`. The
   password reaches `signtool` as an argument and is never printed.
3. The package identity comes from the manifest. The name is
   `tac.application.id` with every character other than a letter, digit, dot
   or hyphen changed to a hyphen, and must be 3 to 50 characters (`TY1601`).
   The version is the executable's four-part version (ADR 0052), so the
   fourth number is `tac.application.build`; the Microsoft Store needs that
   number to be 0, so a Store submission sets `build` to 0. The architecture
   is `--arch`. The publisher display name is the certificate subject's `CN`.
4. The package declares one full-trust desktop application (`runFullTrust`)
   on Windows 10 1809 (10.0.17763.0) or later. Its tile and store images are
   copies of the smallest PNG manifest icon that covers each size (44, 50 and
   150 pixels) or of the largest PNG icon. `ty` does not resize images; with
   no PNG icon the tiles are plain squares in `theme_color`, or gray.
5. A packaged app's install folder is read-only, so WebView2 cannot keep its
   data beside the executable there. When the host has package identity
   (`GetCurrentPackageFamilyName` succeeds), it passes
   `%LOCALAPPDATA%\Packages\<family>\LocalState\WebView2` as WebView2's
   user data folder; Windows removes it with the package. The unpackaged app
   keeps the default folder.
6. Not part of this amendment: toast activation (BA3) and protocol
   activation (LK2) add their package extensions to the same manifest, and a
   Microsoft Store upload stays out of scope. Distribution needs a trusted
   code-signing certificate; tests use a self-signed one.

### Evidence (2026-10-19)

- Cloud (Linux): the unit tests check the package manifest, tiles, the
  signing checks and the `makeappx` and `signtool` arguments.
  `windows_package_tests::the_staged_layout_packs_and_signs_with_the_msix_sdk`
  (ignored; needs Microsoft's MSIX SDK `makemsix`, `osslsigncode` and
  `openssl`) packed a staged layout with `makemsix pack`, which validates
  `AppxManifest.xml` against Microsoft's schemas, signed it with a
  self-signed `CN=Tachyon Test` certificate, verified the signature and
  unpacked it again. It passed.
  `windows_package_tests::the_package_folder_helper_runs_unpackaged_under_wine`
  (ignored; MinGW and Wine) passed: the helper builds the package folder path
  and returns no folder when the process has no package identity.
- Windows (Surface, ARM64, Windows SDK 10.0.26100, MSVC 14.44, develop
  51abbc50): `scripts/windows/native-test.ps1 -Msix` built the gate app with
  `--release` and a self-signed certificate. `makeappx` packed an arm64
  `NativeGate.msix`, `signtool` signed it, and the script found the gate
  certificate's thumbprint on it. The AppX deployment service also accepted
  the signature. Over SSH, `Add-AppxPackage` stops with 0x80070005 because
  a network logon has no desktop session, so the script skips the install
  there. From an elevated prompt on the signed-in desktop (same commit, with
  the developer prompt's `Platform` variable cleared), the script trusted the
  certificate, installed the package, launched it from its package identity,
  checked the rendered pages and the native Rust, OS and publish calls,
  found the WebView2 data in the package's LocalState folder, and removed
  the package again. It passed.

## Amendment (2026-10-19): iOS device and App Store Connect builds

Decision D2 ships iOS through App Store Connect (TestFlight, then the App
Store) on the developer's own Apple Developer team.

1. **Where the app runs.** `ty bundle --target ios` still builds a simulator
   app. `--device` builds an arm64 `iphoneos` app for a registered iPhone or
   iPad, and `--release` builds one for App Store Connect. Both publish
   `<Name>.app` and `<Name>.ipa` in `dist/ios/`, and the artifact manifest
   names the `.ipa` (`ios`, `aarch64`, `wkwebview`). `--device` without an iOS
   target, and `--device` with `--native`, stop with `TY1001`.
2. **Signing material** comes only from the build machine:

   | Variable | Value |
   | --- | --- |
   | `TAC_APPLE_TEAM_ID` | The Apple Developer team ID. |
   | `TAC_IOS_PROVISIONING_PROFILE` | The `.mobileprovision` file. A relative path is read from the directory where `ty` runs. |
   | `TAC_IOS_SIGNING_IDENTITY` | Optional. A keychain certificate name or SHA-1. |

   - Before the web build, `ty` stops with `TY1605` when a variable is missing,
     the team ID is not 10 capital letters and digits, or the profile is not a
     file.
   - Before compiling, it decodes the profile (`security cms -D`) and stops
     with `TY1605` when the profile's team is not `TAC_APPLE_TEAM_ID`, its app
     ID does not cover `<team>.<tac.application.id>`, or its kind does not fit
     the build: `--release` needs an App Store profile (no device list, not
     debuggable) and `--device` a development, ad hoc or enterprise one.
   - The certificate is the keychain identity (`security find-identity -v -p
     codesigning`) whose SHA-1 is one of the profile's certificates. None, or
     more than one, stops the build with `TY1605`, unless
     `TAC_IOS_SIGNING_IDENTITY` names it.
3. **The bundle.**
   - `Info.plist` names `iPhoneOS`, `LSRequiresIPhoneOS`, `arm64` and
     `UIRequiresFullScreen` (the app is portrait only, so iPad multitasking is
     off), plus the Xcode and SDK build keys (`DTXcode`, `DTSDKName` and
     others) that App Store Connect checks, read from `xcodebuild -version`,
     `xcrun --show-sdk-version` and `sw_vers`.
   - The largest raster icon in `icons` becomes a 1024x1024 asset catalog icon
     compiled by `actool`. `--release` stops with `TY1601` when there is no
     icon or it has an alpha channel, which App Store Connect refuses.
   - `PrivacyInfo.xcprivacy` declares no tracking and the two required-reason
     APIs the host and its Swift prelude use: file attributes (`C617.1`, sizes
     of bundled files) and `UserDefaults` (`CA92.1`, the app's own values).
     APIs that the app's own Swift services use are the developer's to add.
   - The profile is embedded as `embedded.mobileprovision`. Entitlements name
     `<team>.<id>`, the team, `get-task-allow` from the profile and, for App
     Store builds, `beta-reports-active`. When `tac.application.links` lists
     `hosts` (ADR 0106), they are added as `applinks:` associated domains, and
     a profile without the Associated Domains capability stops the build with
     `TY1605` before compiling.
   - `codesign --force --generate-entitlement-der --timestamp=none` signs the
     app, `codesign --verify --strict` checks it, and `ditto` zips
     `Payload/<Name>.app` into the `.ipa`.
4. **Out of scope.** Uploading to App Store Connect (Transporter or `xcrun
   altool`), installing on a device (`xcrun devicectl device install app`),
   and capabilities that need other entitlements. A signed `.ipa` is not byte-reproducible, because the signature is
   not.

### Evidence (2026-10-19)

- Cloud (Linux): unit tests for the profile checks, the certificate choice,
  the entitlements, the Xcode build keys and the device `Info.plist`; CLI
  tests for the `TY1001` and `TY1605` refusals.
- Mac (macOS 26.6, Xcode 27.1 27A9275, iphoneos SDK 27.1, develop fd0384ad):
  `scripts/ios/native-test.mjs` passed without signing variables and again
  with team JAU9V4WH99 and a wildcard development profile. The `--device` app
  was signed with the Apple Development certificate the profile names and
  passed `codesign --verify --strict`; the bundle holds `iPhoneOS`,
  `embedded.mobileprovision` and `PrivacyInfo.xcprivacy`, and the `.ipa` holds
  `Payload/<Name>.app`.
- Owed: `--release` with an App Store profile, an install and launch on an
  iOS 17 or later device, and an App Store Connect upload.

## Amendment (2026-10-19): a Windows build without `--release` says it is unsigned

- A Windows build without `--release` is a folder with an unsigned `.exe`.
  It now prints one note that says so, and that `--release` builds a signed
  MSIX package. The note is the Windows text of the "development code
  signing" entry of the support table, which states the reach of
  development signing on every target: macOS and iOS simulator apps are
  signed ad hoc, `--device` iOS builds with the profile's Apple Development
  certificate, Android debug APKs with the machine's debug keystore; web
  builds and Linux apps are not code signed.
- Signing the debug `.exe` with the development certificate was not chosen:
  a debug build would then need `signtool.exe` and a timestamp server on
  every run, and the release certificate variables would sign builds that
  were never meant to leave the machine.

## Amendment (2026-10-19): the Linux packages on WSL and through FUSE

RL1 ran `scripts/linux/native-test.sh` on WSL Ubuntu 24.04.5 (aarch64, GTK
4.14.5, WebKitGTK 2.52.6, libfuse2t64 2.9.9). The release packages built, the
extracted `.deb` started its command, and the AppImage started through a real
FUSE mount. The script now starts the AppImage through FUSE when `/dev/fuse`
can be opened and `fusermount` exists, and falls back to
`APPIMAGE_EXTRACT_AND_RUN=1` with a message otherwise. RL1 also installed the
`.deb`'s desktop file in a private `XDG_DATA_HOME` and opened scheme links
through it with `gio open` and `xdg-open`. Still owed: `dpkg -i` or
`apt install` on WSL, which needs administrator rights, and an install on a
Linux desktop.

## Amendment (2026-10-24): the `.deb` installed as root (LX1)

`scripts/linux/deb-install-test.sh` builds the release packages, installs the
`.deb` with `apt-get install` as root, checks the installed command, Rust
library and desktop file, starts the app from `PATH` and from its desktop file
with `gio launch` until it shows its first page, and removes it with
`apt-get purge`. It passed in an x86_64 Ubuntu 24.04 cloud container. apt
showed 0 B installed, because the control file has no `Installed-Size` field.
Still owed: `dpkg -i` or `apt install` on WSL, and an install on a Linux
desktop.

## Amendment (2026-10-24): Android releases are shrunk by R8, take `--arch`, and build offline

PKG7 (decision D11 of the iteration 2 plan).

1. `--release` turns on R8 and resource shrinking
   (`proguard-android-optimize.txt` plus a generated `proguard-rules.pro`).
   The rules keep every class outside the AndroidX, Kotlin, JetBrains and
   Google libraries under its own name, with all its members: the activity,
   `TachyonRust` (whose `publish`, `dispatch`, `capability` and `http`
   methods Rust finds by name through JNI), the Kotlin services and models
   (which the prelude reads by reflection) and the prelude itself. Native
   methods and `@JavascriptInterface` methods are kept anywhere. Debug builds
   are not shrunk.
2. `--arch arm64`, `x64` or `universal` choose the Android ABIs: the Rust
   library is built for `arm64-v8a`, `x86_64` or both, and Gradle's
   `abiFilters` name the same ABIs. `universal` is the default, so a build
   without `--arch` is unchanged; `native` is the build machine's ABI. The
   artifact manifest names `aarch64`, `x86_64` or `universal`. Gradle's
   per-ABI APK splits were not chosen: Google Play splits an App Bundle by
   ABI itself, and a single-ABI APK is what a test device or emulator needs.
3. When Gradle's cache (`GRADLE_USER_HOME`, or `.gradle` in the home folder)
   holds the pinned Android Gradle plugin, its plugin marker, AndroidX WebKit
   and, for a Kotlin service, the Kotlin Gradle plugin and its marker, Gradle
   runs with `--offline`. If an offline run still misses a module (a
   dependency of those, such as `aapt2` or the lint jars of a first
   `--release`), the build runs Gradle once more online. Before that cache is
   filled, the build prints a note naming the modules Gradle downloads.
4. Gradle runs under the 15-minute limit of other long native tools instead
   of the 2-minute one: a first build that downloads the plugin, and an R8
   release, took longer than 2 minutes in the cloud.

### Evidence (2026-10-24)

In an x86_64 Linux cloud container (SDK platform 35, build-tools 35.0.0, NDK
r30 30.0.16248370, Gradle 8.14.3, AGP 8.7.3, OpenJDK 21) the Rust and Kotlin
probe app of `scripts/android/rust-services-test.mjs` was built with `ty`
(SUPPORT_TIERS.md, "R8, ABIs and offline Gradle"). `--arch arm64` packaged
one library; a signed `--release` was verified with `apksigner` and its
`classes.dex` kept the bridge, Rust and Kotlin classes with their names; a
`--release --arch x64` built offline with the network cut. Maven Central
answered 429 to Gradle's bursts from the cloud, so the container's own Gradle
init script used Google's Maven Central mirror; Tachyon's generated build
files are unchanged. Owed (RM3): the shrunk APK installed and run on the
emulator, and WebView debugging off in a release on a Google Play image.
