mod android;
mod browser_api_grants;
#[cfg(test)]
mod browser_task_tests;
mod compiler;
mod config;
mod csharp_web_apis;
mod dotnet_web_release;
mod events;
mod file_picker_sources;
mod fsharp_behaviors_project;
mod fsharp_service;
mod fsharp_service_events;
mod fsharp_web_project;
mod go;
mod go_compiled_with;
mod go_compiler;
mod go_compiler_choice;
mod go_events;
mod go_web_build;
mod host;
mod ios;
mod java_web_services;
mod kotlin_web_services;

#[cfg(test)]
mod kotlin_document_title_tests;

#[cfg(test)]
mod kotlin_document_title_scope_tests;
mod linux;
mod linux_window_controls;
mod mac_os_menu_source;
mod macos;
#[cfg(test)]
mod macos_menu_tests;
#[cfg(test)]
mod managed_capability_tests;
mod managed_native_capabilities;
mod manifest_setting;
#[cfg(test)]
mod model_tests;
mod native_capabilities;
#[cfg(test)]
mod native_capability_tests;
#[cfg(all(test, target_os = "macos"))]
mod native_file_picker_permission_tests;
#[cfg(test)]
mod native_helper_tests;
mod native_menu_config;
#[cfg(test)]
mod native_menu_config_tests;
mod native_menu_entry;
mod native_menu_shortcut;
#[cfg(test)]
mod native_packaged_utility_tests;
#[cfg(test)]
mod native_page_graph_tests;
#[cfg(test)]
mod native_rust_capability_tests;
mod native_service_roots;
mod native_type_program;
mod native_ui_artifact;
#[cfg(test)]
mod native_ui_artifact_tests;
mod native_ui_asset;
mod native_ui_bounds;
mod native_ui_capture;
mod native_ui_change;
mod native_ui_contract;
mod native_ui_descriptor;
mod native_ui_generation;
#[cfg(test)]
mod native_ui_generation_tests;
#[cfg(all(test, target_os = "macos"))]
mod native_ui_host_sources;
#[cfg(all(test, target_os = "macos"))]
mod native_ui_host_tests;
mod native_ui_problem;
mod native_ui_profile;
#[cfg(test)]
mod native_ui_profile_tests;
#[cfg(all(test, target_os = "macos"))]
mod native_ui_runtime_tests;
#[cfg(all(test, target_os = "macos"))]
mod native_ui_styles_sources;
#[cfg(all(test, target_os = "macos"))]
mod native_ui_styles_tests;
#[cfg(test)]
mod native_web_api_tests;
#[cfg(test)]
mod native_window_control_tests;
mod native_window_controls;
#[cfg(all(test, target_os = "macos"))]
mod native_window_example_tests;
mod registry;
mod routes;
mod rust;
#[cfg(test)]
mod rust_filename_tests;
mod rust_imports;
mod rust_local_paths;
mod rust_page_graph;
mod rust_service_plan;
mod rust_web_module;
mod site;
mod site_map;
#[cfg(test)]
mod site_tests;
#[cfg(test)]
mod storage_tests;
mod swift_web_services;
mod web_program_build;
mod windows;

pub(crate) use browser_api_grants::BrowserApiGrants;
pub use compiler::{
    NativeBuildOptions, NativeBuildResult, NativeCompiler, native_target_directory,
};
pub(crate) use config::{TacMember, WebAppManifest};
pub(crate) use go::{
    in_package as go_in_package, module_file as go_module_file,
    publisher_alias as go_publisher_alias, runtime_files as go_runtime_files, web_go,
};
pub(crate) use java_web_services::JavaWebServices;
pub(crate) use kotlin_web_services::KotlinWebServices;
pub use native_ui_change::NativeUiChange;
pub use native_ui_contract::NativeUiContract;
pub use native_ui_generation::NativeUiGeneration;
#[cfg(test)]
pub(crate) use rust::companion_prelude;
pub(crate) use rust::{RustCompanion, tac_value_source, web_assembly};
pub(crate) use site::{CacheRule, PageMetadata, SiteSettings};
pub(crate) use site_map::SiteMap;
pub(crate) use swift_web_services::SwiftWebServices;
pub(crate) use windows::web_dotnet;

#[cfg(feature = "fuzzing")]
pub(crate) fn plan_for_fuzzing(
    _target: tachyon_contracts::NativeTarget,
    html: &str,
) -> Result<(), crate::Failure> {
    crate::template::TemplateFrontend::compile(
        html,
        "fuzz/tac.html",
        &std::collections::BTreeSet::new(),
    )
    .map(|_| ())
}

mod csharp_browser_bindings;

#[cfg(test)]
mod csharp_browser_bindings_tests;
