#![allow(clippy::expect_used)]

use super::native_menu_config::NativeMenuConfig;
use serde_json::json;

pub(super) struct LinuxHostTests;

impl LinuxHostTests {
    pub(super) fn menus() -> NativeMenuConfig {
        serde_json::from_value(json!({
            "file": [
                {"id":"refresh","label":"Refresh","action":{"service":"SectionMenuService","method":"refresh"}}
            ],
            "view": [
                {"id":"show-running","label":"What's Running","action":{"service":"SectionMenuService","method":"show_running","event":"SectionChosen"},"shortcut":{"key":"1","modifiers":["command"]}}
            ],
            "help": [
                {"id":"guide","label":"Guide","route":"/help","shortcut":{"key":"g","modifiers":["command","option"]}}
            ]
        }))
        .expect("menu declarations")
    }

    pub(super) fn application(menus: NativeMenuConfig) -> super::config::NativeApplication {
        super::config::NativeApplication {
            name: String::from("Linux Host Check"),
            executable_name: String::from("LinuxHostCheck"),
            application_id: String::from("org.tachyon.linux-host-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus,
            rust: crate::native::native_rust_project::NativeRustProject::default(),
        }
    }

    pub(super) fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: [("/", "index.html"), ("/help", "help/index.html")]
                .into_iter()
                .map(|(route, document)| super::routes::NativeRouteIndexEntry {
                    route: route.to_owned(),
                    document: document.to_owned(),
                })
                .collect(),
        }
    }

    fn host(menus: NativeMenuConfig) -> String {
        super::linux::c_source(&Self::application(menus), &Self::index())
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    #[cfg(target_os = "linux")]
    fn write(path: &std::path::Path, source: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("fixture");
    }

    #[cfg(target_os = "linux")]
    async fn compile(root: &std::path::Path) -> std::path::PathBuf {
        let source = super::native_menu_action_tests::MenuActionFixture::stage(root);
        Self::compile_with(
            root,
            &source,
            Self::menus(),
            include_str!("linux_host_check_fixture.c"),
        )
        .await
    }

    #[cfg(target_os = "linux")]
    pub(super) async fn compile_with(
        root: &std::path::Path,
        source: &std::path::Path,
        menus: NativeMenuConfig,
        fixture: &str,
    ) -> std::path::PathBuf {
        Self::compile_application(root, source, &Self::application(menus), fixture).await
    }

    #[cfg(target_os = "linux")]
    pub(super) async fn compile_application(
        root: &std::path::Path,
        source: &std::path::Path,
        application: &super::config::NativeApplication,
        fixture: &str,
    ) -> std::path::PathBuf {
        let bundle = root.join("LinuxHostCheck");
        super::rust::compile(
            source,
            super::rust::Linkage::Shared,
            None,
            &bundle.join("bin/libtachyoncompanion.so"),
        )
        .await
        .expect("actual Rust SDK");
        for (document, title) in [("index.html", "Home"), ("help/index.html", "Help")] {
            Self::write(
                &bundle.join("resources/WebBundle").join(document),
                &format!(
                    "<!doctype html><html><head><title>{title}</title></head><body>{title}</body></html>"
                ),
            );
        }
        let host = super::linux::c_source(application, &Self::index()).replacen(
            "int main(int argc, char **argv) {",
            "static int tachyon_host_main(int argc, char **argv) {",
            1,
        );
        let check = root.join("linux_host_check.c");
        Self::write(&check, &format!("{host}\n{fixture}"));
        let executable = bundle.join("bin/LinuxHostCheck");
        super::linux::compile_c(&check, &executable)
            .await
            .expect("actual GTK 4 and WebKitGTK 6.0 toolchain");
        executable
    }
}

#[test]
fn linux_host_installs_declared_menus_with_gtk_actions_and_accelerators() {
    let source = LinuxHostTests::host(LinuxHostTests::menus());
    assert!(source.contains("static const int TACHYON_MENU_ITEM_COUNT = 3;"));
    assert!(source.contains(
        "{\"show-running\", \"What's Running\", TACHYON_MENU_VIEW, NULL, \"SectionChosen\", '1', 1}"
    ));
    assert!(source.contains("{\"guide\", \"Guide\", TACHYON_MENU_HELP, \"/help\", NULL, 'g', 3}"));
    assert!(source.contains("gtk_application_set_menubar(application, G_MENU_MODEL(bar));"));
    assert!(source.contains("gtk_application_set_accels_for_action(g_menu_application"));
    assert!(source.contains("\"close-window\", \"<Primary>w\""));
    assert!(source.contains("g_module_symbol(module, \"tac_native_menu_action\""));
    assert!(source.contains("webkit_web_view_load_uri(g_view, uri);"));
    let enabled = LinuxHostTests::section(&source, "static gboolean tachyon_menu_enabled(");
    assert!(enabled.contains("tachyon_menu_page_shown() && g_menu_ready(item->event)"));
    let host_call = LinuxHostTests::section(&source, "static gboolean tachyon_on_host_call(");
    assert!(host_call.contains("tachyon_menus_refresh();"));
    assert!(!source.contains("__NATIVE_MENU"));
}

#[test]
fn linux_host_starts_the_application_once_before_its_window_and_stops_it_at_shutdown() {
    let source = LinuxHostTests::host(NativeMenuConfig::default());
    let startup = LinuxHostTests::section(&source, "static void tachyon_startup(");
    assert!(
        startup.find("tachyon_companion_load();").expect("load")
            < startup.find("g_application_start()").expect("start")
    );
    assert!(startup.contains("tachyon_record(\"application.start.failed\");"));
    let activate = LinuxHostTests::section(&source, "static void tachyon_activate(");
    assert!(
        activate.contains(
            "if (g_application_failed) { g_application_quit(G_APPLICATION(app)); return; }"
        )
    );
    assert!(!activate.contains("tachyon_companion_load"));
    let load = LinuxHostTests::section(&source, "static void tachyon_on_load(");
    assert!(!load.contains("tachyon_application_stop"));
    let shutdown = LinuxHostTests::section(&source, "static void tachyon_shutdown(");
    assert!(
        shutdown
            .find("tachyon_companion_dispose();")
            .expect("dispose")
            < shutdown.find("tachyon_application_stop();").expect("stop")
    );
    let stop = LinuxHostTests::section(&source, "static void tachyon_application_stop(");
    assert!(stop.contains("if (g_application_stopped) return;"));
    assert!(source.contains("g_unix_signal_add(SIGTERM, tachyon_terminate, app);"));
    assert!(
        source.contains("g_signal_connect(app, \"startup\", G_CALLBACK(tachyon_startup), NULL);")
    );
    assert!(source.contains("static const int TACHYON_MENU_ITEM_COUNT = 0;"));
}

#[test]
fn linux_host_always_shows_file_edit_and_window_menus_sent_to_the_web_view() {
    let source = LinuxHostTests::host(NativeMenuConfig::default());
    let install = LinuxHostTests::section(&source, "static void tachyon_menus_install(");
    assert!(!install.contains("TACHYON_MENU_ITEM_COUNT == 0"));
    assert!(install.contains("g_menu_append_submenu(bar, \"_Edit\", G_MENU_MODEL(edit));"));
    assert!(install.contains("g_menu_append_submenu(bar, \"_Window\", G_MENU_MODEL(window));"));
    for row in [
        "{\"edit-undo\", \"_Undo\", \"Undo\", \"<Primary>z\"}",
        "{\"edit-redo\", \"_Redo\", \"Redo\", \"<Primary><Shift>z\"}",
        "{\"edit-copy\", \"_Copy\", \"Copy\", \"<Primary>c\"}",
        "{\"edit-select-all\", \"Select _All\", \"SelectAll\", \"<Primary>a\"}",
    ] {
        assert!(source.contains(row), "{row}");
    }
    let edit = LinuxHostTests::section(&source, "static gboolean tachyon_menu_edit(");
    assert!(edit.contains("webkit_web_view_execute_editing_command(g_view, command);"));
    let maximize = LinuxHostTests::section(&source, "static void tachyon_menu_maximize(");
    assert!(
        maximize.contains("if (gtk_window_is_maximized(window)) gtk_window_unmaximize(window);")
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a display and a working WebKit sandbox; run explicitly on the Linux lane"]
async fn linux_generated_host_runs_menu_actions_and_one_application_lifetime() {
    let root = tempfile::tempdir().expect("fixture");
    let executable = LinuxHostTests::compile(root.path()).await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host lifecycle");
    assert!(
        output.contains("Close Window and one stop passed"),
        "{output}"
    );
}
