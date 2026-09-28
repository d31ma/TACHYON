mod android;
mod browser_api_grants;
mod compiler;
mod config;
mod dotnet_web_release;
mod events;
mod fsharp_service;
mod fsharp_web_project;
mod go;
mod go_compiled_with;
mod go_compiler;
mod go_compiler_choice;
mod go_events;
mod go_web_build;
mod host;
mod ios;
mod kotlin_web_services;
mod linux;
mod macos;
mod manifest_setting;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod native_web_api_tests;
mod registry;
mod routes;
mod rust;
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
pub(crate) use kotlin_web_services::KotlinWebServices;
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
