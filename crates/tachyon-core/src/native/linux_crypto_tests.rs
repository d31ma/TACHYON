#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;
use super::linux_crypto::LinuxCrypto;

struct LinuxCryptoTests;

impl LinuxCryptoTests {
    const HELPERS: &'static str = r"let last = 'nothing';
const call = async (name, ...values) => {
  const raw = await window.webkit.messageHandlers.tachyon.postMessage({ capability: 'browserApi.crypto.' + name, payload: JSON.stringify(values) });
  last = name + ' ' + raw;
  const reply = JSON.parse(raw);
  return 'value' in reply ? reply.value : { reason: 'error' };
};
const word = (answer, field) => answer.reason ?? (field in answer ? 'ok' : 'missing');
const b64 = (bytes) => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
const unb64 = (text) => Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((text.length + 3) % 4)), (c) => c.charCodeAt(0));
const hex = (text) => Uint8Array.from(text.match(/../g), (pair) => parseInt(pair, 16));
const text = (value) => b64(new TextEncoder().encode(value));
const subtle = crypto.subtle;
const sealWith = async (aesKey, plain, aad) => {
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const cipher = new Uint8Array(await subtle.encrypt({ name: 'AES-GCM', iv: nonce, additionalData: aad }, aesKey, new TextEncoder().encode(plain)));
  return b64(new Uint8Array([1, ...nonce, ...cipher]));
};
const openWith = async (aesKey, sealed, aad) => {
  const bytes = unb64(sealed);
  const plain = await subtle.decrypt({ name: 'AES-GCM', iv: bytes.subarray(1, 13), additionalData: aad }, aesKey, bytes.subarray(13));
  return new TextDecoder().decode(plain);
};
const hkdfKey = async (raw, salt, info) => {
  const base = await subtle.importKey('raw', raw, 'HKDF', false, ['deriveBits']);
  const bits = await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt, info }, base, 256);
  return subtle.importKey('raw', bits, 'AES-GCM', false, ['encrypt', 'decrypt']);
};
const decrypted = async (key, sealed, aad) => {
  const answer = await call('decrypt', key, sealed, b64(aad));
  return answer.reason ?? new TextDecoder().decode(unb64(answer.data));
};
const results = [];
";
    const FIRST: &'static str = r"const check = await call('check');
results.push('store:' + check.keyStore);
const digest = await call('hash', text('abc'));
results.push(digest.digest === b64(hex('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')) ? 'hash' : 'hash-wrong');
const okm = await call('hkdf', b64(new Uint8Array(22).fill(11)), b64(hex('000102030405060708090a0b0c')), b64(hex('f0f1f2f3f4f5f6f7f8f9')), 42);
results.push(okm.bytes === b64(hex('3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865')) ? 'hkdf' : 'hkdf-wrong');
const rfcKey = b64(hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'));
const rfcSignature = hex('92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00');
results.push('rfc8032:' + (await call('verify', rfcKey, b64(hex('72')), b64(rfcSignature))).valid);
const emptyKey = b64(hex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'));
const emptySignature = b64(hex('e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b'));
results.push('rfc8032-empty:' + (await call('verify', emptyKey, '', emptySignature)).valid);
rfcSignature[0] ^= 1;
results.push('changed:' + (await call('verify', rfcKey, b64(hex('72')), b64(rfcSignature))).valid);
const device = await call('makeKeyPair', 'device', 'ed25519', true);
localStorage.setItem('device', device.publicKey);
const signature = await call('sign', 'device', text('hello'));
const webSigner = await subtle.importKey('raw', unb64(device.publicKey), { name: 'Ed25519' }, false, ['verify']);
results.push('webkit-verify:' + await subtle.verify({ name: 'Ed25519' }, webSigner, unb64(signature.signature), new TextEncoder().encode('hello')));
const emptySigned = await call('sign', 'device', '');
results.push('empty-round-trip:' + (await call('verify', device.publicKey, '', emptySigned.signature)).valid);
const secret = crypto.getRandomValues(new Uint8Array(32));
results.push('import:' + word(await call('importKey', 'shared-aes', b64(secret), 'encrypt', false), 'key'));
const webAes = await subtle.importKey('raw', secret, 'AES-GCM', false, ['encrypt', 'decrypt']);
const aad = new TextEncoder().encode('chat-1');
results.push('host-opens:' + await decrypted('shared-aes', await sealWith(webAes, 'from webkit', aad), aad));
const sealed = await call('encrypt', 'shared-aes', text('from host'), b64(aad));
results.push('webkit-opens:' + await openWith(webAes, sealed.sealed, aad));
results.push('wrong-aad:' + await decrypted('shared-aes', sealed.sealed, new TextEncoder().encode('chat-2')));
const mine = await call('makeKeyPair', 'agree-key', 'x25519', false);
const theirs = await subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
const theirPublic = new Uint8Array(await subtle.exportKey('raw', theirs.publicKey));
const salt = crypto.getRandomValues(new Uint8Array(16));
const info = new TextEncoder().encode('mesail');
results.push('agree:' + word(await call('agree', 'agree-key', b64(theirPublic), b64(salt), b64(info), 'agreed', 'encrypt', false), 'key'));
const myPublic = await subtle.importKey('raw', unb64(mine.publicKey), { name: 'X25519' }, false, []);
const sharedBits = await subtle.deriveBits({ name: 'X25519', public: myPublic }, theirs.privateKey, 256);
results.push('agreed-opens:' + await decrypted('agreed', await sealWith(await hkdfKey(sharedBits, salt, info), 'agreed text', aad), aad));
results.push('small-order:' + word(await call('agree', 'agree-key', b64(new Uint8Array(32)), '', '', 'never', 'encrypt', false), 'key'));
results.push('derive-base:' + word(await call('importKey', 'base', b64(secret), 'derive', false), 'key'));
results.push('derive:' + word(await call('deriveKey', 'base', b64(salt), b64(info), 'child', 'encrypt', false), 'key'));
results.push('child-opens:' + await decrypted('child', await sealWith(await hkdfKey(secret, salt, info), 'child text', aad), aad));
const codeSalt = crypto.getRandomValues(new Uint8Array(16));
results.push('code:' + word(await call('fromCode', 'backup', 'correct horse battery', b64(codeSalt), 'encrypt', true), 'key'));
const codeBase = await subtle.importKey('raw', new TextEncoder().encode('correct horse battery'), 'PBKDF2', false, ['deriveBits']);
const codeBits = await subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt: codeSalt, iterations: 600000 }, codeBase, 256);
const codeKey = await subtle.importKey('raw', codeBits, 'AES-GCM', false, ['encrypt', 'decrypt']);
results.push('code-opens:' + await decrypted('backup', await sealWith(codeKey, 'backup text', aad), aad));
localStorage.setItem('backup', (await call('encrypt', 'backup', text('kept text'), b64(aad))).sealed);
results.push('unknown:' + word(await call('publicKey', 'missing'), 'key'));
results.push('exists:' + word(await call('makeKeyPair', 'device', 'ed25519', false), 'key'));
results.push('wrong-kind:' + word(await call('sign', 'agree-key', text('x')), 'signature'));
results.push('forget-missing:' + word(await call('forget', 'missing'), 'nothing'));
return results.join(',');";
    const AGAIN: &'static str = r"const aad = new TextEncoder().encode('chat-1');
results.push('store:' + (await call('check')).keyStore);
results.push('public-kept:' + ((await call('publicKey', 'device')).publicKey === localStorage.getItem('device')));
results.push('backup-opens:' + await decrypted('backup', localStorage.getItem('backup'), aad));
results.push('memory-gone:' + word(await call('publicKey', 'agree-key'), 'key'));
results.push('exists:' + word(await call('makeKeyPair', 'device', 'ed25519', false), 'key'));
results.push('forget-all:' + word(await call('forgetAll'), 'nothing'));
results.push('after:' + word(await call('publicKey', 'device'), 'key'));
return results.join(',');";
    const LOCKED: &'static str = r"results.push('kept:' + word(await call('makeKeyPair', 'locked-key', 'ed25519', true), 'key'));
results.push('memory:' + word(await call('makeKeyPair', 'memory-key', 'ed25519', false), 'key'));
return results.join(',');";
    const STOPPED: &'static str = "\n} catch (error) {\n  return results.concat('stopped: ' + error.message + ' after ' + last).join(',');\n}";
    #[cfg(target_os = "linux")]
    const UNLOCKED_KEYRING: &'static str = "mkdir -p \"$HOME/.local/share/keyrings\" && export XDG_RUNTIME_DIR=\"$(mktemp -d)\" && printf %s test | setsid gnome-keyring-daemon --unlock --components=secrets > /dev/null && sleep 1 && printf %s setup | secret-tool store --label='desktop keyring' tachyon setup && echo open:$(timeout 5 secret-tool lookup tachyon setup)";
    #[cfg(target_os = "linux")]
    const LOCKED_KEYRING: &'static str = "export XDG_RUNTIME_DIR=\"$(mktemp -d)\" && setsid gnome-keyring-daemon --replace --components=secrets < /dev/null > /dev/null && sleep 1 && echo $(timeout 5 secret-tool lookup tachyon setup || echo locked)";
    #[cfg(target_os = "linux")]
    const FIRST_EXPECTED: &'static str = "store:os,hash,hkdf,rfc8032:true,rfc8032-empty:true,changed:false,webkit-verify:true,empty-round-trip:true,import:ok,host-opens:from webkit,webkit-opens:from host,wrong-aad:rejected,agree:ok,agreed-opens:agreed text,small-order:invalid,derive-base:ok,derive:ok,child-opens:child text,code:ok,code-opens:backup text,unknown:unknown,exists:exists,wrong-kind:invalid,forget-missing:missing";
    #[cfg(target_os = "linux")]
    const AGAIN_EXPECTED: &'static str = "store:os,public-kept:true,backup-opens:kept text,memory-gone:unknown,exists:exists,forget-all:missing,after:unknown";
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
                &super::host::c_string_escape(
                    &[Self::HELPERS, "try {\n", script, Self::STOPPED].concat(),
                ),
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
    assert!(!LinuxCryptoTests::fixture(LinuxCryptoTests::FIRST, "x").contains("__CRYPTO_"));
    assert!(!LinuxCryptoTests::fixture(LinuxCryptoTests::AGAIN, "x").contains("__CRYPTO_"));
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
            LinuxCryptoTests::FIRST,
            LinuxCryptoTests::FIRST_EXPECTED,
        ),
        (
            "again",
            LinuxCryptoTests::AGAIN,
            LinuxCryptoTests::AGAIN_EXPECTED,
        ),
    ] {
        let executable =
            LinuxCryptoTests::compile(&root.path().join(folder), script, expected).await;
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
