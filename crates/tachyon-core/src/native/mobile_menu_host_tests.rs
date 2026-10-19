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
