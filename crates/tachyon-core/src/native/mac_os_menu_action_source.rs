use super::native_menu_config::NativeMenuConfig;
use super::rust_page_graph::RustPageGraph;
use crate::Failure;
use std::fmt::Write as _;
use std::path::Path;

pub(super) struct MacOsMenuActionSource;

impl MacOsMenuActionSource {
    const SWIFT_CALLS: &'static str = r#"@_silgen_name("tac_native_menu_action")
private func tacRustNativeMenuAction(_ id: UnsafePointer<CChar>) -> Bool

@_silgen_name("tac_native_menu_ready")
private func tacRustNativeMenuReady(_ event: UnsafePointer<CChar>) -> Bool

@MainActor
private func tachyonMenuReady(_ event: String) -> Bool {
    event.withCString { (pointer: UnsafePointer<CChar>) -> Bool in tacRustNativeMenuReady(pointer) }
}

@MainActor
private func tachyonMenuPerform(_ id: String) -> Bool {
    id.withCString { (pointer: UnsafePointer<CChar>) -> Bool in tacRustNativeMenuAction(pointer) }
}"#;
    const SWIFT_REFUSALS: &'static str = r"@MainActor
private func tachyonMenuReady(_ event: String) -> Bool { false }

@MainActor
private func tachyonMenuPerform(_ id: String) -> Bool { false }";

    const RUST_EXPORTS: &'static str = "\n#[unsafe(no_mangle)]\npub extern \"C\" fn tac_native_menu_action(id: *const core::ffi::c_char) -> bool { NativeMenuActions::run(id, TachyonRustServices::tac_menu_action) }\n#[unsafe(no_mangle)]\npub extern \"C\" fn tac_native_menu_ready(event: *const core::ffi::c_char) -> bool { NativeMenuActions::ready(event) }\n";

    pub(super) fn has_actions(config: &NativeMenuConfig) -> bool {
        config.items().any(|item| item.action.is_some())
    }

    pub(super) fn swift_calls(config: &NativeMenuConfig, rust: bool) -> &'static str {
        if rust && Self::has_actions(config) {
            Self::SWIFT_CALLS
        } else {
            Self::SWIFT_REFUSALS
        }
    }

    pub(super) fn stage(rust: Option<&Path>, config: &NativeMenuConfig) -> Result<(), Failure> {
        let Some(path) = rust.filter(|_| Self::has_actions(config)) else {
            return Ok(());
        };
        let staged = super::host::native_io(std::fs::read_to_string(path), path)?;
        let start = RustPageGraph::MODULE_START;
        if staged.matches(start).count() != 1 {
            return Err(super::host::native_tool_failure(
                1605,
                "Native menu actions need the generated Rust service module.",
            ));
        }
        let services = format!("{start}{}", Self::rust_dispatch(config));
        let source = staged.replacen(start, &services, 1) + Self::RUST_EXPORTS;
        super::host::write(path, source.as_bytes())
    }

    pub(super) fn rust_dispatch(config: &NativeMenuConfig) -> String {
        let arms = config
            .items()
            .filter_map(|item| {
                item.action
                    .as_ref()
                    .map(|action| (item.id.as_str(), action))
            })
            .fold(String::new(), |mut arms, (id, action)| {
                let _ = writeln!(
                    arms,
                    "{id:?} => tac_with_application_{}(|service| {{ let _ = service.{}(); }}),",
                    action.service, action.method
                );
                arms
            });
        format!(
            "pub(super) fn tac_menu_action(id: &str) -> ::core::result::Result<(), ::std::string::String> {{ match id {{\n{arms}_ => Err(::std::string::String::from(\"Unknown native menu action.\")),\n}} }}\n"
        )
    }
}
