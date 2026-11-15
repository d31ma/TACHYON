#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;
use super::linux_crypto::LinuxCrypto;
use super::native_crypto_checks::NativeCryptoChecks;

struct LinuxCryptoTests;

impl LinuxCryptoTests {
    const LOCKED: &'static str = r"results.push('kept:' + word(await call('makeKeyPair', 'locked-key', 'ed25519', true), 'key'));
results.push('memory:' + word(await call('makeKeyPair', 'memory-key', 'ed25519', false), 'key'));
return results.join(',');";
    #[cfg(target_os = "linux")]
    const UNLOCKED_KEYRING: &'static str = "mkdir -p \"$HOME/.local/share/keyrings\" && export XDG_RUNTIME_DIR=\"$(mktemp -d)\" && printf %s test | setsid gnome-keyring-daemon --unlock --components=secrets > /dev/null && sleep 1 && printf %s setup | secret-tool store --label='desktop keyring' tachyon setup && echo open:$(timeout 5 secret-tool lookup tachyon setup)";
    #[cfg(target_os = "linux")]
    const LOCKED_KEYRING: &'static str = "export XDG_RUNTIME_DIR=\"$(mktemp -d)\" && setsid gnome-keyring-daemon --replace --components=secrets < /dev/null > /dev/null && sleep 1 && echo $(timeout 5 secret-tool lookup tachyon setup || echo locked)";
    #[cfg(target_os = "linux")]
    const STORE: &'static str = "os";
    #[cfg(target_os = "linux")]
    const LOCKED_EXPECTED: &'static str = "kept:locked,memory:ok";

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    fn host(granted: &[&str]) -> String {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        super::linux::c_source(
            &application,
            &super::linux_host_tests::LinuxHostTests::index(),
        )
    }

    fn fixture(script: &str, expected: &str) -> String {
        include_str!("linux_crypto_fixture.c")
            .replace(
                "__CRYPTO_SCRIPT__",
                &super::host::c_string_escape(&NativeCryptoChecks::script(script)),
            )
            .replace(
                "__CRYPTO_EXPECTED__",
                &super::host::c_string_escape(expected),
            )
    }

    #[cfg(target_os = "linux")]
    async fn compile(root: &std::path::Path, script: &str, expected: &str) -> std::path::PathBuf {
        let source = super::native_menu_action_tests::MenuActionFixture::stage(root);
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(&["crypto"]);
        super::linux_host_tests::LinuxHostTests::compile_application(
            root,
            &source,
            &application,
            &Self::fixture(script, expected),
        )
        .await
    }

    #[cfg(target_os = "linux")]
    async fn keyring(command: &str, sign: &str) {
        let output = super::host::run_tool("sh", &["-c", command])
            .await
            .expect("GNOME Keyring");
        assert!(output.contains(sign), "keyring setup: {output}");
        println!("keyring: {output}");
    }

    #[cfg(target_os = "linux")]
    async fn run(executable: &std::path::Path) {
        let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
            .await
            .expect("actual generated Linux host crypto");
        assert!(output.contains("linux crypto passed"), "{output}");
        println!("{output}");
    }
}

#[test]
fn the_linux_host_carries_native_crypto_only_for_the_crypto_grant() {
    let granted = LinuxCryptoTests::host(&["crypto"]);
    assert!(granted.contains("{\"browserApi.crypto.agree\", 7, tachyon_crypto_agree}"));
    assert!(
        granted.contains(
            "secret_password_store_sync(tachyon_crypto_schema(), collection, label, text"
        )
    );
    assert!(granted.contains("if (strcmp(api, \"crypto\") == 0) return TRUE;"));
    let refused = LinuxCryptoTests::host(&[]);
    assert!(refused.contains("tac.browserApis does not grant crypto."));
    assert!(!refused.contains("EVP_"));
    assert!(!refused.contains("libsecret/secret.h"));
    for source in [&granted, &refused] {
        assert!(source.contains("if (g_str_has_prefix(capability, \"browserApi.crypto.\")) {\n    tachyon_crypto_start(capability, payload, value, reply);"));
        assert!(!source.contains("__NATIVE_CRYPTO__"));
    }
}

#[test]
fn only_a_crypto_grant_links_libsecret_and_openssl() {
    let granted = LinuxCryptoTests::permissions(&["crypto"]);
    assert_eq!(
        LinuxCrypto::pkg_config_modules(&granted),
        ["libsecret-1", "libcrypto"]
    );
    assert_eq!(
        LinuxCrypto::package_depends(&granted),
        ", libsecret-1-0, libssl3t64 | libssl3"
    );
    let nothing = LinuxCryptoTests::permissions(&["share"]);
    assert!(LinuxCrypto::pkg_config_modules(&nothing).is_empty());
    assert_eq!(LinuxCrypto::package_depends(&nothing), "");
}

#[test]
fn the_linux_crypto_host_matches_the_page_format_and_limits() {
    let source = LinuxCrypto::SOURCE;
    for line in [
        "#define TACHYON_CRYPTO_ROUNDS 600000",
        "#define TACHYON_CRYPTO_VERSION 1",
        "#define TACHYON_CRYPTO_NONCE_BYTES 12",
        "#define TACHYON_CRYPTO_SEALED_EXTRA 29",
        "#define TACHYON_CRYPTO_MOST_BYTES 32768",
        "g_task_set_task_data(task, job, NULL);\n  g_task_run_in_thread(task, tachyon_crypto_job_run);",
        "return agreed && any != 0;",
        "gboolean open = collection != NULL && tachyon_crypto_unlocked(service, collection, unlock);",
        "return tachyon_crypto_lookup(name, &found, kept) == TACHYON_KEY_FOUND;",
        "#define TACHYON_CRYPTO_UNLOCK_SECONDS 60",
        "if (unlocked <= 0) g_crypto_unlock_refused_at = g_get_monotonic_time();",
        "return collection != NULL ? collection : tachyon_crypto_listed_collection(service);",
        "return !g_str_has_suffix(path, \"/session\") && (!unlocked || !secret_collection_get_locked(collection));",
    ] {
        assert!(source.contains(line), "{line}");
    }
}

#[test]
fn the_crypto_fixture_fills_every_placeholder() {
    let fixture = LinuxCryptoTests::fixture(LinuxCryptoTests::LOCKED, "kept:locked");
    assert!(!fixture.contains("__CRYPTO_"));
    assert!(fixture.contains("browserApi.crypto."));
    assert!(!LinuxCryptoTests::fixture(NativeCryptoChecks::FIRST, "x").contains("__CRYPTO_"));
    assert!(!LinuxCryptoTests::fixture(NativeCryptoChecks::AGAIN, "x").contains("__CRYPTO_"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0, libsecret and OpenSSL development files, gnome-keyring, a session bus and a display; run explicitly on the Linux lane"]
async fn linux_crypto_runs_natively_and_keeps_keys_in_the_secret_service() {
    let root = tempfile::tempdir().expect("fixture");
    LinuxCryptoTests::keyring(LinuxCryptoTests::UNLOCKED_KEYRING, "open:setup").await;
    for (folder, script, expected) in [
        (
            "first",
            NativeCryptoChecks::FIRST,
            NativeCryptoChecks::first_expected(LinuxCryptoTests::STORE),
        ),
        (
            "again",
            NativeCryptoChecks::AGAIN,
            NativeCryptoChecks::again_expected(LinuxCryptoTests::STORE),
        ),
    ] {
        let executable =
            LinuxCryptoTests::compile(&root.path().join(folder), script, &expected).await;
        LinuxCryptoTests::run(&executable).await;
    }
    LinuxCryptoTests::keyring(LinuxCryptoTests::LOCKED_KEYRING, "locked").await;
    let locked = LinuxCryptoTests::compile(
        &root.path().join("locked"),
        LinuxCryptoTests::LOCKED,
        LinuxCryptoTests::LOCKED_EXPECTED,
    )
    .await;
    LinuxCryptoTests::run(&locked).await;
}
