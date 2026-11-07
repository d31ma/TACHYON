#![allow(clippy::expect_used)]

use super::native_menu_config::NativeMenuConfig;

fn linux_host() -> String {
    super::linux::c_source(
        &super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default()),
        &super::linux_host_tests::LinuxHostTests::index(),
    )
}

#[test]
fn linux_host_goes_back_and_forward_on_alt_arrows_the_back_keys_and_the_mouse_buttons() {
    let source = linux_host();
    assert!(source.contains("tachyon_system_back_install(GTK_WINDOW(window));"));
    for shortcut in [
        "GDK_KEY_Left, GDK_ALT_MASK, FALSE",
        "GDK_KEY_Right, GDK_ALT_MASK, TRUE",
        "GDK_KEY_Back, 0, FALSE",
        "GDK_KEY_Forward, 0, TRUE",
    ] {
        assert!(source.contains(shortcut), "{shortcut}");
    }
    assert!(source.contains("if (button != 8 && button != 9) return FALSE;"));
    assert!(
        source.contains("gtk_event_controller_set_propagation_phase(keys, GTK_PHASE_CAPTURE);")
    );
    assert!(source.contains("if (!tachyon_trusted_view(g_view)) return;"));
    assert!(!source.contains("__NATIVE_SYSTEM_BACK__"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, xdotool and an X11 display with a window manager (xfwm4 under Xvfb); run explicitly on the Linux lane"]
async fn linux_generated_host_goes_back_and_forward_through_page_history() {
    let root = tempfile::tempdir().expect("fixture");
    let source =
        super::linux_window_appearance_tests::LinuxWindowAppearanceTests::stage(root.path());
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default()),
        include_str!("linux_system_back_fixture.c"),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host system back");
    assert!(output.contains("linux system back:"), "{output}");
}

#[test]
fn android_host_goes_back_in_page_history_and_leaves_the_app_from_the_first_route() {
    let source = super::android::java_source(
        &super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default()),
        "org.tachyon.system_back",
        false,
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    assert!(source.contains("public void doUpdateVisitedHistory(WebView web, String url, boolean reload) {\n            holdSystemBack();"));
    assert!(source.contains("boolean wanted = web.canGoBack();"));
    assert!(source.contains("getOnBackInvokedDispatcher().registerOnBackInvokedCallback("));
    assert!(
        source.contains(
            "else getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(callback);"
        )
    );
    assert!(source.contains("if (!goBackInPage()) super.onBackPressed();"));
    assert!(source.contains("if (android.os.Build.VERSION.SDK_INT < 33 || web == null) return;"));
    assert!(!source.contains("__NATIVE_SYSTEM_BACK__"));
}

#[test]
fn apple_hosts_turn_on_the_back_and_forward_swipe_and_the_mac_view_menu_has_back_and_forward() {
    let swipe = "view.allowsBackForwardNavigationGestures = true";
    assert!(super::ios::host_source().contains(swipe));
    assert!(super::macos::host_source().contains(swipe));
    let menus = include_str!("menus/MacOsMenus.swift");
    assert!(menus.contains("let menu = NSMenu(title: \"View\")\n        history?.install(menu)"));
    let history = include_str!("menus/MacOsMenuHistory.swift");
    assert!(history.contains("menu.addItem(item(\"Back\", #selector(back(_:)), \"[\"))"));
    assert!(history.contains("menu.addItem(item(\"Forward\", #selector(forward(_:)), \"]\"))"));
    assert!(history.contains(
        "return item.action == #selector(back(_:)) ? view.canGoBack : view.canGoForward"
    ));
    assert!(history.contains("guard let view, let window, window.owns(view) else { return nil }"));
}
