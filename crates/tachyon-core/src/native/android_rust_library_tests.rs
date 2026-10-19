use super::android::{AndroidHostGenerator, java_source, linked_java_source};
use super::android_rust_libraries::AndroidRustLibraries;
use super::config::NativeApplication;
use std::path::Path;

struct AndroidRustLibraryCheck;

impl AndroidRustLibraryCheck {
    const PACKAGE: &'static str = "dev.tachyon.rust_probe";
    const JAVA_DECLARATIONS: [&'static str; 7] = [
        "package dev.tachyon.host;",
        "private static native void install(byte[] storeFolder);",
        "private static native byte[] call(byte[] request);",
        "private static void publish(byte[] payload)",
        "private static void dispatch()",
        "private static byte[] capability(byte[] capability, byte[] payload)",
        "private static byte[] http(byte[] method, byte[] url, byte[] headers, byte[] body, long timeoutMs, long limit)",
    ];
    const RUST_BINDINGS: [&'static str; 6] = [
        "Java_dev_tachyon_host_TachyonRust_install",
        "Java_dev_tachyon_host_TachyonRust_call",
        "c\"publish\", c\"([B)V\"",
        "c\"dispatch\", c\"()V\"",
        "c\"capability\", c\"([B[B)[B\"",
        "c\"http\", c\"([B[B[B[BJJ)[B\"",
    ];

    fn application() -> NativeApplication {
        NativeApplication {
            name: String::from("Rust Probe"),
            executable_name: String::from("RustProbe"),
            application_id: String::from("dev.tachyon.rust-probe"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: super::native_rust_project::NativeRustProject::default(),
            outgoing: super::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: super::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: super::minimum_system::MinimumSystem::default(),
            description: String::new(),
            links: super::native_links::NativeLinks::default(),
        }
    }

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: Vec::new(),
        }
    }

    fn source(has_kotlin: bool, rust_services: &[String]) -> String {
        linked_java_source(
            &Self::application(),
            Self::PACKAGE,
            (has_kotlin, rust_services),
            &Self::index(),
        )
    }
}

#[test]
fn rust_services_reach_the_linked_library_and_kotlin_keeps_the_other_services() {
    let source = AndroidRustLibraryCheck::source(true, &[String::from("rustCounter")]);
    assert!(source.contains("return rustOrCompanion(request);"));
    assert!(
        source
            .contains("return rust != null ? rust : TachyonCompanionKt.tacNativeInvoke(request);")
    );
    assert!(source.contains("java.util.Arrays.asList(\"rustCounter\")"));
    assert!(source.contains("TacBridge.setEmit(this::relayPublish);"));
    assert!(source.contains("installRustCompanion();"));
    assert!(source.contains("\"host-resume\".equals(operation)"));
    for placeholder in [
        "__RUST_SERVICES__",
        "__OTHER_COMPANION_CALL__",
        "__NATIVE_RUST_SERVICES__",
    ] {
        assert!(
            !source.contains(placeholder),
            "{placeholder} is left in the host"
        );
    }
}

#[test]
fn rust_without_kotlin_answers_other_services_with_the_empty_companion() {
    let source = AndroidRustLibraryCheck::source(false, &[String::from("rustCounter")]);
    assert!(source.contains("return rust != null ? rust : emptyCompanion(request);"));
    assert!(!source.contains("TachyonCompanionKt"));
}

#[test]
fn a_host_without_rust_services_names_no_rust_symbol() {
    let source = java_source(
        &AndroidRustLibraryCheck::application(),
        AndroidRustLibraryCheck::PACKAGE,
        true,
        &AndroidRustLibraryCheck::index(),
    );
    assert!(!source.contains("TachyonRust"));
    assert!(!source.contains("rustOrCompanion"));
    assert!(!source.contains("__NATIVE_RUST_SERVICES__"));
}

#[test]
fn the_java_library_and_the_rust_bridge_agree_on_every_jni_name() {
    let java = AndroidHostGenerator::RUST_LIBRARY_SOURCE;
    for declaration in AndroidRustLibraryCheck::JAVA_DECLARATIONS {
        assert!(
            java.contains(declaration),
            "TachyonRust.java has no {declaration}"
        );
    }
    assert!(java.contains("System.loadLibrary(\"__LIBRARY__\");"));
    let rust = super::rust::companion_prelude();
    for binding in AndroidRustLibraryCheck::RUST_BINDINGS {
        assert!(rust.contains(binding), "the Rust prelude has no {binding}");
    }
}

#[test]
fn every_abi_library_goes_where_gradle_packages_it() {
    let project = Path::new("project");
    let placed: Vec<_> = AndroidRustLibraries::ABIS
        .iter()
        .map(|(abi, _)| AndroidRustLibraries::library_path(project, abi))
        .collect();
    assert_eq!(
        placed,
        [
            Path::new("project/app/src/main/jniLibs/arm64-v8a/libtachyoncompanion.so"),
            Path::new("project/app/src/main/jniLibs/x86_64/libtachyoncompanion.so"),
        ]
    );
    assert_eq!(
        AndroidRustLibraries::ABIS.map(|(_, triple)| triple),
        ["aarch64-linux-android", "x86_64-linux-android"]
    );
}
