# ADR 0104: Release Packages Are Opt-In and Signed From the Environment

- Status: Accepted for Android and macOS (amended 2026-10-19); iOS, Windows and Linux are pending
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
- iOS: device builds and an `.ipa` for App Store Connect (decision D2).
- Windows: an MSIX package signed with a code-signing certificate (decision D3).
- Linux: AppImage and `.deb` packages (decision D4).
- Universal macOS binaries and Android ABI selection.

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
entry says `adapted` until a Developer ID release has been notarized on a
Mac; `crates/tachyon-cli/tests/it/macos_release_cli.rs` holds the Mac tests,
the Developer ID one ignored unless the team's certificate and profile are
present.
