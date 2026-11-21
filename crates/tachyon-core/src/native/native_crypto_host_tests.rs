#![allow(clippy::expect_used)]

use super::native_browser_api_host_tests::BrowserApiHostTests;
use super::native_crypto_checks::NativeCryptoChecks;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

type Expected = fn(&str) -> String;

struct CryptoHostTests;

impl CryptoHostTests {
    const MACOS_FIXTURE: &str = include_str!("NativeCryptoCheck.fixture.swift");
    const IOS_FIXTURE: &str = include_str!("IosCryptoCheck.fixture.swift");
    const PLACEHOLDER: &str = "__CRYPTO_CHECKS__";
    const IOS_MAIN: &str = "@UIApplicationMain\n";
    const KEEP: &str = r"results.push('kept:' + word(await call('makeKeyPair', 'survivor', 'ed25519', true), 'key'));
return results.join(',');";
    const WIPED: &str = r"results.push('wiped:' + word(await call('publicKey', 'survivor'), 'key'));
return results.join(',');";
    const MACOS_PHASES: [(&str, &[(&str, Expected)]); 1] = [(
        "first",
        &[
            (
                NativeCryptoChecks::FIRST,
                NativeCryptoChecks::first_expected,
            ),
            (
                NativeCryptoChecks::FORGET,
                NativeCryptoChecks::forget_expected,
            ),
        ],
    )];
    const IOS_PHASES: [(&str, &[(&str, Expected)]); 3] = [
        (
            "first",
            &[(
                NativeCryptoChecks::FIRST,
                NativeCryptoChecks::first_expected,
            )],
        ),
        (
            "again",
            &[
                (
                    NativeCryptoChecks::AGAIN,
                    NativeCryptoChecks::again_expected,
                ),
                (Self::KEEP, Self::kept),
            ],
        ),
        (
            "wiped",
            &[
                (Self::WIPED, Self::wiped),
                (
                    NativeCryptoChecks::FORGET,
                    NativeCryptoChecks::forget_expected,
                ),
            ],
        ),
    ];
    #[cfg(target_os = "macos")]
    const IOS_APPLICATION: &str = "CryptoCheck.app";
    #[cfg(target_os = "macos")]
    const IOS_DEVICE: &str = "TAC_IOS_DEVICE";

    fn kept(_store: &str) -> String {
        String::from("kept:ok")
    }

    fn wiped(_store: &str) -> String {
        String::from("wiped:unknown")
    }

    fn swift_text(text: &str) -> String {
        format!("#\"\"\"\n{text}\n\"\"\"#")
    }

    fn checks(phases: &[(&str, &[(&str, Expected)])]) -> String {
        let entries: Vec<String> = phases
            .iter()
            .map(|(phase, checks)| {
                let scripts: Vec<String> = checks
                    .iter()
                    .map(|(script, _)| Self::swift_text(&NativeCryptoChecks::script(script)))
                    .collect();
                format!("\"{phase}\": [{}]", scripts.join(",\n"))
            })
            .collect();
        format!("[{}]", entries.join(",\n"))
    }

    fn macos_host(granted: &[&str]) -> String {
        let fixture =
            Self::MACOS_FIXTURE.replace(Self::PLACEHOLDER, &Self::checks(&Self::MACOS_PHASES));
        BrowserApiHostTests::host(granted, &fixture)
    }

    fn ios_host(granted: &[&str]) -> String {
        let application = BrowserApiHostTests::application(granted);
        let call = super::macos::companion_call(None, None);
        let host = super::ios::swift_source(&application, call, &BrowserApiHostTests::index());
        let fixture =
            Self::IOS_FIXTURE.replace(Self::PLACEHOLDER, &Self::checks(&Self::IOS_PHASES));
        format!("{}\n{fixture}", host.replacen(Self::IOS_MAIN, "", 1))
    }

    fn answers(output: &str, phase: &str) -> Vec<String> {
        let prefix = format!("crypto check {phase}: ");
        output
            .lines()
            .filter_map(|line| line.strip_prefix(&prefix))
            .map(String::from)
            .collect()
    }

    fn store(answers: &[String]) -> String {
        let first = answers.first().map_or("", String::as_str);
        let store = first.strip_prefix("store:").unwrap_or_default();
        String::from(store.split(',').next().unwrap_or_default())
    }

    #[cfg(target_os = "macos")]
    fn expect(output: &str, phase: &str, checks: &[(&str, Expected)], store: &str) {
        let wanted: Vec<String> = checks.iter().map(|(_, expected)| expected(store)).collect();
        assert_eq!(Self::answers(output, phase), wanted, "{output}");
        assert!(output.contains("crypto checks done"), "{output}");
    }

    #[cfg(target_os = "macos")]
    async fn run(program: &str, arguments: &[&str]) -> String {
        let output = super::host::run_tool(program, arguments)
            .await
            .expect(program);
        println!("{output}");
        output
    }

    #[cfg(target_os = "macos")]
    fn ios_triple() -> String {
        let deployment = BrowserApiHostTests::application(&["crypto"])
            .minimum_system
            .floor(tachyon_contracts::NativeTarget::Ios);
        super::ios_destination::IosDestination::SIMULATOR.triple(deployment)
    }

    #[cfg(target_os = "macos")]
    async fn ios_typecheck(root: &Path) {
        let source =
            BrowserApiHostTests::write(root, "CryptoCheck.swift", &Self::ios_host(&["crypto"]));
        let triple = Self::ios_triple();
        Self::run(
            "/usr/bin/xcrun",
            &[
                "--sdk",
                super::ios_destination::IosDestination::SIMULATOR.sdk,
                "swiftc",
                "-parse-as-library",
                "-typecheck",
                "-target",
                &triple,
                source.to_str().expect("source"),
            ],
        )
        .await;
    }

    #[cfg(target_os = "macos")]
    async fn ios_compile(root: &Path) -> PathBuf {
        let application = BrowserApiHostTests::application(&["crypto"]);
        let bundle = root.join(Self::IOS_APPLICATION);
        let destination = super::ios_destination::IosDestination::SIMULATOR;
        let plist = super::ios::info_plist(&application, &[], &destination.plist_keys(""));
        BrowserApiHostTests::write(&bundle, "Info.plist", &plist);
        BrowserApiHostTests::write(
            &bundle,
            "WebBundle/index.html",
            "<!doctype html><html><head><title>Home</title></head><body>Home</body></html>",
        );
        let source =
            BrowserApiHostTests::write(root, "CryptoCheck.swift", &Self::ios_host(&["crypto"]));
        let executable = bundle.join(&application.executable_name);
        let triple = Self::ios_triple();
        let mut arguments = vec![
            "--sdk",
            super::ios_destination::IosDestination::SIMULATOR.sdk,
            "swiftc",
            "-parse-as-library",
            "-target",
            &triple,
        ];
        arguments.extend(["-framework", "UIKit", "-framework", "WebKit"]);
        arguments.extend([
            source.to_str().expect("source"),
            "-o",
            executable.to_str().expect("binary"),
        ]);
        Self::run("/usr/bin/xcrun", &arguments).await;
        Self::run(
            "/usr/bin/codesign",
            &["--force", "--sign", "-", bundle.to_str().expect("bundle")],
        )
        .await;
        bundle
    }

    #[cfg(target_os = "macos")]
    async fn ios_launch(device: &str, phase: &str) -> String {
        let identifier = BrowserApiHostTests::application(&[]).application_id;
        Self::run(
            "/usr/bin/xcrun",
            &[
                "simctl",
                "launch",
                "--console-pty",
                "--terminate-running-process",
                device,
                &identifier,
                phase,
            ],
        )
        .await
    }

    #[cfg(target_os = "macos")]
    async fn ios_install(device: &str, bundle: &Path) {
        let identifier = BrowserApiHostTests::application(&[]).application_id;
        let _ = super::host::run_tool(
            "/usr/bin/xcrun",
            &["simctl", "uninstall", device, &identifier],
        )
        .await;
        Self::run(
            "/usr/bin/xcrun",
            &[
                "simctl",
                "install",
                device,
                bundle.to_str().expect("bundle"),
            ],
        )
        .await;
    }
}

#[test]
fn the_crypto_checks_fill_their_scripts_into_both_apple_hosts() {
    for host in [
        CryptoHostTests::macos_host(&["crypto"]),
        CryptoHostTests::ios_host(&["crypto"]),
    ] {
        assert!(!host.contains(CryptoHostTests::PLACEHOLDER));
        assert!(host.contains("AppleBrowserApis.shared.granted = [\"crypto\"]"));
        assert!(host.contains("final class AppleCrypto"));
        assert!(host.contains(
            "if capability.hasPrefix(AppleBrowserApis.cryptoPrefix) {\n            AppleBrowserApis.shared.crypto(capability, payload) { replyHandler($0, nil) }"
        ));
        assert!(host.contains("\"first\": [#\"\"\"\nlet last = 'nothing';"));
        assert_eq!(host.matches("\n@main\n").count(), 1);
    }
    let ios = CryptoHostTests::ios_host(&["crypto"]);
    assert!(!ios.contains(CryptoHostTests::IOS_MAIN));
    assert!(ios.contains("\"wiped\": ["));
}

#[test]
fn the_answers_are_read_by_phase_and_follow_the_key_store() {
    let output = "noise\ncrypto check first: store:os,keep:os,hash\ncrypto check again: x\ncrypto checks done\n";
    let first = CryptoHostTests::answers(output, "first");
    assert_eq!(first, ["store:os,keep:os,hash"]);
    assert_eq!(CryptoHostTests::store(&first), "os");
    assert_eq!(CryptoHostTests::store(&[]), "");
    assert!(
        NativeCryptoChecks::first_expected("none").starts_with("store:none,keep:unsupported,hash,")
    );
    assert!(NativeCryptoChecks::again_expected("os").starts_with("store:os,public-kept:true,"));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires an actual AppKit/WebKit window; run explicitly in the coordinator SDK lane"]
async fn macos_crypto_runs_natively_through_the_generated_host() {
    let root = tempfile::tempdir().expect("fixture");
    let executable =
        BrowserApiHostTests::compile(root.path(), &CryptoHostTests::macos_host(&["crypto"])).await;
    let output = CryptoHostTests::run(executable.to_str().expect("binary"), &[]).await;
    let (phase, checks) = CryptoHostTests::MACOS_PHASES[0];
    let store = CryptoHostTests::store(&CryptoHostTests::answers(&output, phase));
    CryptoHostTests::expect(&output, phase, checks, &store);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn the_ios_crypto_fixture_typechecks_against_the_simulator_sdk_without_a_simulator() {
    let root = tempfile::tempdir().expect("fixture");
    CryptoHostTests::ios_typecheck(root.path()).await;
}

#[test]
fn the_ios_crypto_fixture_finds_the_web_view_through_the_window_scene() {
    let ios = CryptoHostTests::ios_host(&["crypto"]);
    assert!(ios.contains("UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.windows.first }.first"));
    assert!(!ios.contains("as? AppDelegate)?.window"));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires a booted iOS simulator named by TAC_IOS_DEVICE; run only inside a mac-heavy --vm=ios job"]
async fn ios_crypto_keeps_keys_in_the_simulator_keychain_across_a_relaunch() {
    let device = std::env::var(CryptoHostTests::IOS_DEVICE)
        .expect("TAC_IOS_DEVICE names a booted simulator");
    let root = tempfile::tempdir().expect("fixture");
    let bundle = CryptoHostTests::ios_compile(root.path()).await;
    CryptoHostTests::ios_install(&device, &bundle).await;
    let [first, again, wiped] = CryptoHostTests::IOS_PHASES;
    let output = CryptoHostTests::ios_launch(&device, first.0).await;
    let store = CryptoHostTests::store(&CryptoHostTests::answers(&output, first.0));
    CryptoHostTests::expect(&output, first.0, first.1, &store);
    if store == "os" {
        let output = CryptoHostTests::ios_launch(&device, again.0).await;
        CryptoHostTests::expect(&output, again.0, again.1, &store);
        CryptoHostTests::ios_install(&device, &bundle).await;
        let output = CryptoHostTests::ios_launch(&device, wiped.0).await;
        CryptoHostTests::expect(&output, wiped.0, wiped.1, &store);
    }
    let identifier = BrowserApiHostTests::application(&[]).application_id;
    let _ = super::host::run_tool(
        "/usr/bin/xcrun",
        &["simctl", "uninstall", &device, &identifier],
    )
    .await;
    println!("ios crypto store: {store}");
}
