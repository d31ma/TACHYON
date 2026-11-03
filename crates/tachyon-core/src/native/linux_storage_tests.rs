#![allow(clippy::expect_used)]

struct LinuxStorageTests;

impl LinuxStorageTests {
    const SCRIPT: &'static str = r"const key = 'tachyon-relaunch:LocalState';
const loading = document.createElement('script');
loading.src = '/tac-storage.js';
await new Promise((resolve, reject) => { loading.onload = resolve; loading.onerror = () => reject(new Error('tac-storage.js did not load')); document.head.append(loading); });
await TacStorage.loaded;
const storage = globalThis[Symbol.for('tachyon.tac.storage')];
const cache = storage.cache('relaunch');
const request = (operation) => new Promise((resolve, reject) => { operation.onsuccess = () => resolve(operation.result); operation.onerror = () => reject(operation.error); });
const opening = indexedDB.open('tachyon-relaunch', 1);
opening.onupgradeneeded = () => opening.result.createObjectStore('rows', {keyPath: 'id'});
const database = await request(opening);
const local = localStorage.getItem(key);
const row = await request(database.transaction('rows').objectStore('rows').get('row'));
const cached = cache.get('row');
if (local === null && !row && !cached.hit) {
  localStorage.setItem(key, 'kept');
  const writing = database.transaction('rows', 'readwrite');
  writing.objectStore('rows').put({id: 'row', value: 'kept'});
  await new Promise((resolve, reject) => { writing.oncomplete = resolve; writing.onerror = () => reject(writing.error); });
  cache.set('row', 'kept', {expiresAt: Date.now() + 3600000, maxEntries: 10});
  await storage.settled();
  database.close();
  return 'written secure=' + isSecureContext;
}
database.close();
return ['local=' + (local ?? 'missing'), 'store=' + (row ? row.value : 'missing'), 'cache=' + (cached.hit ? cached.value : 'missing')].join(',');";
    #[cfg(target_os = "linux")]
    const KEPT: &'static str = "local=kept,store=kept,cache=kept";

    fn fixture() -> String {
        include_str!("linux_storage_fixture.c").replace(
            "__STORAGE_SCRIPT__",
            &super::host::c_string_escape(Self::SCRIPT),
        )
    }
}

#[test]
fn the_linux_web_view_keeps_its_data_in_the_user_data_and_cache_folders() {
    let source = super::linux::c_source(
        &super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        ),
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    assert!(!source.contains("webkit_network_session_new_ephemeral"));
    assert!(
        source.contains(
            "g_build_filename(g_get_user_data_dir(), TACHYON_BUNDLE_ID, \"webkit\", NULL)"
        )
    );
    assert!(
        source.contains(
            "g_build_filename(g_get_user_cache_dir(), TACHYON_BUNDLE_ID, \"webkit\", NULL)"
        )
    );
    assert!(source.contains("WebKitNetworkSession *session = tachyon_network_session();"));
    let shutdown = source
        .split("static void tachyon_shutdown(")
        .nth(1)
        .expect("shutdown handler");
    assert!(
        shutdown
            .find("tachyon_settle_web_storage();")
            .expect("settle")
            < shutdown
                .find("tachyon_companion_dispose();")
                .expect("dispose")
    );
    assert!(!LinuxStorageTests::fixture().contains("__STORAGE_SCRIPT__"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and a display; run explicitly on the Linux lane"]
async fn linux_web_storage_is_kept_after_the_application_quits_and_starts_again() {
    let root = tempfile::tempdir().expect("fixture");
    let source = super::native_menu_action_tests::MenuActionFixture::stage(root.path());
    let executable = super::linux_host_tests::LinuxHostTests::compile_with(
        root.path(),
        &source,
        super::native_menu_config::NativeMenuConfig::default(),
        &LinuxStorageTests::fixture(),
    )
    .await;
    std::fs::write(
        root.path()
            .join("LinuxHostCheck/resources/WebBundle/tac-storage.js"),
        include_str!("../template/tac-storage.js"),
    )
    .expect("storage runtime");
    let program = executable.to_str().expect("binary");
    let home = root.path().join("home");
    let data = home.join("data");
    let cache = home.join("cache");
    let state = home.join("state");
    let environment = [
        ("XDG_DATA_HOME", data.to_str().expect("data folder")),
        ("XDG_CACHE_HOME", cache.to_str().expect("cache folder")),
        ("XDG_STATE_HOME", state.to_str().expect("state folder")),
    ];
    let first = super::host::run_tool_with(program, &[], None, &environment)
        .await
        .expect("first launch of the generated Linux host");
    assert!(
        first.contains("linux storage: written secure=true"),
        "{first}"
    );
    assert!(data.join("org.tachyon.linux-host-check/webkit").is_dir());
    let second = super::host::run_tool_with(program, &[], None, &environment)
        .await
        .expect("second launch of the generated Linux host");
    assert!(
        second.contains(&format!("linux storage: {}", LinuxStorageTests::KEPT)),
        "{second}"
    );
}
