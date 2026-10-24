#![allow(clippy::expect_used)]

use super::config::NativeApplication;
use serde_json::json;

struct MobileMenuHostTests;

impl MobileMenuHostTests {
    fn application() -> NativeApplication {
        let root = tempfile::tempdir().expect("menu project");
        std::fs::create_dir_all(root.path().join("client")).expect("client directory");
        let menus = json!({
            "file": [{"id":"status","label":"Status \"now\"","route":"/status","shortcut":{"key":"k","modifiers":["command","shift"]}}],
            "help": [{"id":"guide","label":"Guide","route":"/"}]
        });
        let manifest = json!({"name":"Menu Check","tac":{"application":{"menus":menus}}});
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("real manifest");
        NativeApplication::discover(root.path()).expect("menu declarations")
    }

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: [("/", "index.html"), ("/status", "status/index.html")]
                .into_iter()
                .map(|(route, document)| super::routes::NativeRouteIndexEntry {
                    route: route.to_owned(),
                    document: document.to_owned(),
                })
                .collect(),
        }
    }
}

#[test]
fn ios_route_items_go_into_the_main_menu_and_into_iphone_shortcuts() {
    let source = super::ios::swift_source(
        &MobileMenuHostTests::application(),
        "tacNativeInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains(
        "IosMenuRoute(id: \"status\", label: \"Status \\\"now\\\"\", group: \"file\", route: \"/status\", key: \"k\", modifiers: [.command, .shift])"
    ), "{source}");
    assert!(source.contains(
        "IosMenuRoute(id: \"guide\", label: \"Guide\", group: \"help\", route: \"/\", key: \"\", modifiers: [])"
    ));
    assert!(source.contains("override func buildMenu(with builder: UIMenuBuilder)"));
    assert!(source.contains("builder.insertChild(UIMenu(title: \"\", identifier: identifier, options: .displayInline, children: commands), atStartOfMenu: parent)"));
    assert!(
        source.contains("guard UIDevice.current.userInterfaceIdiom == .phone else { return [] }")
    );
    assert!(source.contains("override var keyCommands: [UIKeyCommand]?"));
    assert!(source.contains("tachyonRouteDocuments[entry.route] != nil"));
    assert!(!source.contains("__NATIVE_MENU"));
}

#[test]
fn ios_hosts_without_menus_install_no_items() {
    let mut application = MobileMenuHostTests::application();
    application.menus = super::native_menu_config::NativeMenuConfig::default();
    let source = super::ios::swift_source(
        &application,
        "tacNativeInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains("static let routes: [IosMenuRoute] = []"));
}

#[test]
fn android_route_items_go_into_the_app_bar_overflow_menu_with_ctrl_shortcuts() {
    let source = super::android::java_source(
        &MobileMenuHostTests::application(),
        "dev.tachyon.menu_check",
        false,
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains(
        "new MenuRoute(\"status\", \"Status \\\"now\\\"\", \"file\", \"/status\", \"k\", KeyEvent.META_CTRL_ON | KeyEvent.META_SHIFT_ON)"
    ), "{source}");
    assert!(source.contains("new MenuRoute(\"guide\", \"Guide\", \"help\", \"/\", \"\", 0)"));
    assert!(source.contains("root.addView(withMenuBar(view));"));
    assert!(source.contains("item.setShowAsAction(android.view.MenuItem.SHOW_AS_ACTION_NEVER);"));
    assert!(source.contains("public void onProvideKeyboardShortcuts("));
    assert!(source.contains("if (view == null || canonicalRoute(entry.route) == null) return;"));
    assert!(!source.contains("__NATIVE_MENU"));
}

#[test]
fn android_control_and_command_both_become_one_ctrl() {
    let mut application = MobileMenuHostTests::application();
    application.menus = serde_json::from_value(json!({"view":[
        {"id":"zoom","label":"Zoom","route":"/","shortcut":{"key":"2","modifiers":["control","command"]}}
    ]}))
    .expect("menu declarations");
    let source = super::android::java_source(
        &application,
        "dev.tachyon.menu_check",
        false,
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains(
        "new MenuRoute(\"zoom\", \"Zoom\", \"view\", \"/\", \"2\", KeyEvent.META_CTRL_ON)"
    ));
}

impl MobileMenuHostTests {
    fn with_actions() -> NativeApplication {
        let mut application = Self::application();
        application.menus = serde_json::from_value(json!({
            "view": [
                {"id":"show-running","label":"What's Running","action":{"service":"SectionMenuService","method":"show_running","event":"SectionChosen"},"shortcut":{"key":"1","modifiers":["command"]}},
                {"id":"refresh","label":"Refresh","action":{"service":"SectionMenuService","method":"refresh"}}
            ],
            "help": [{"id":"guide","label":"Guide","route":"/"}]
        }))
        .expect("menu declarations");
        application
    }
}

#[test]
fn ios_action_items_call_the_application_service_while_a_page_listens() {
    let source = super::ios::swift_source(
        &MobileMenuHostTests::with_actions(),
        "tacRustInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains(
        "IosMenuRoute(id: \"show-running\", label: \"What's Running\", group: \"view\", route: \"\", key: \"1\", modifiers: [.command], event: \"SectionChosen\", action: true)"
    ), "{source}");
    assert!(source.contains(
        "IosMenuRoute(id: \"refresh\", label: \"Refresh\", group: \"view\", route: \"\", key: \"\", modifiers: [], event: \"\", action: true)"
    ));
    assert!(source.contains("@_silgen_name(\"tac_native_menu_action\")"));
    assert!(source.contains(
        "guard let action = IosMenus.action(command, webView), tachyonMenuPerform(action.id) else {"
    ));
    assert!(source.contains(
        "return entry.event.isEmpty || (view != nil && tachyonMenuReady(entry.event)) ? entry : nil"
    ));
    assert!(source.contains(
        "override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {"
    ));
    assert!(source.contains("command.attributes.insert(.disabled)"));
    assert!(source.contains("route.action ? #selector(RootViewController.runMenuAction(_:)) : #selector(RootViewController.openMenuRoute(_:))"));
    let routes = super::ios::swift_source(
        &MobileMenuHostTests::application(),
        "tacRustInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    assert!(routes.contains("private func tachyonMenuPerform(_ id: String) -> Bool { false }"));
    assert!(!source.contains("__NATIVE_MENU"));
}

#[test]
fn ios_hosts_start_the_application_before_the_window_and_stop_it_on_termination() {
    let source = super::ios::swift_source(
        &MobileMenuHostTests::application(),
        "tacRustInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    let start = source
        .find("guard tacRustNativeApplicationStart() else { Telemetry.shared.record(\"application.start.failed\"); exit(EXIT_FAILURE) }")
        .expect("the host starts the application services");
    let window = source
        .find("let window = UIWindow(frame: UIScreen.main.bounds)")
        .expect("window");
    assert!(
        start < window,
        "application services start before the window"
    );
    let terminate = source
        .find("func applicationWillTerminate(")
        .expect("termination");
    let dispose = source[terminate..]
        .find("tachyonDisposeCompanions()")
        .expect("page disposal")
        + terminate;
    let stop = source[terminate..]
        .find("tacRustNativeApplicationShutdown()")
        .expect("stop")
        + terminate;
    assert!(
        dispose < stop,
        "page services stop before the application services"
    );
    let swift = super::ios::swift_source(
        &MobileMenuHostTests::application(),
        "tacNativeInvoke(payload)",
        &MobileMenuHostTests::index(),
    );
    assert!(!swift.contains("tacRustNativeApplicationStart()"));
    assert!(!swift.contains("__NATIVE_APPLICATION_"));
}

#[test]
fn android_action_items_call_the_application_service_on_the_rust_thread() {
    let source = super::android::linked_java_source(
        &MobileMenuHostTests::with_actions(),
        "dev.tachyon.menu_check",
        (false, &[String::from("section")]),
        &MobileMenuHostTests::index(),
    );
    assert!(source.contains(
        "MenuRoute.action(\"show-running\", \"What's Running\", \"view\", \"SectionChosen\", \"1\", KeyEvent.META_CTRL_ON)"
    ), "{source}");
    assert!(source.contains("MenuRoute.action(\"refresh\", \"Refresh\", \"view\", \"\", \"\", 0)"));
    assert!(source.contains("public boolean onPrepareOptionsMenu(android.view.Menu menu) {"));
    assert!(source.contains("return dev.tachyon.host.TachyonRust.menuReady(event);"));
    assert!(source.contains("dev.tachyon.host.TachyonRust.menuAction(entry.id,"));
    assert!(source.contains("if (!menuActionEnabled(entry)) {"));
    assert!(
        source.contains("dev.tachyon.host.TachyonRust.applicationStart(this::applicationStarted);")
    );
    assert!(source.contains("if (isFinishing() && !isChangingConfigurations()) dev.tachyon.host.TachyonRust.applicationStop();"));
    assert!(
        source.contains(
            "companionInvoke(\"{\\\"op\\\":\\\"host-dispose\\\",\\\"retiring\\\":true}\")"
        )
    );
    assert!(!source.contains("__NATIVE_"));
    let routes = super::android::java_source(
        &MobileMenuHostTests::application(),
        "dev.tachyon.menu_check",
        false,
        &MobileMenuHostTests::index(),
    );
    assert!(routes.contains("private boolean menuReady(String event) {\n    return false;"));
    assert!(!routes.contains("TachyonRust"));
    assert!(!routes.contains("__NATIVE_"));
}

#[test]
fn android_rust_calls_share_one_process_thread_and_the_menu_exports_reach_it() {
    let java = super::android::AndroidHostGenerator::RUST_LIBRARY_SOURCE;
    assert!(java.contains("Executors.newSingleThreadExecutor(work -> {"));
    assert!(java.contains(
        "byte[] answer = onRust(() -> call(request.getBytes(StandardCharsets.UTF_8)), null, 0);"
    ));
    assert!(java.contains("private static native boolean start();"));
    assert!(java.contains("private static native boolean runMenuAction(byte[] id);"));
    assert!(
        java.contains("host = value;\n    if (loaded) return true;"),
        "a new activity receives the published signals"
    );
    let dispatch = super::mac_os_menu_action_source::MacOsMenuActionSource::rust_dispatch(
        &MobileMenuHostTests::with_actions().menus,
    );
    assert!(dispatch.contains("\"show-running\" => tac_with_application_SectionMenuService"));
}
