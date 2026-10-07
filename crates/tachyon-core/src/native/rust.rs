use super::host::{native_tool_failure, run_tool, run_tool_output, write};
use super::rust_native_libraries::RustNativeLibraries;
use super::rust_web_module::RustWebModule;
use crate::Failure;
use crate::wasm_release_step::WasmReleaseStep;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct RustCompanion;

pub(super) struct RustCompanionOutput {
    pub(super) library: PathBuf,
    pub(super) executables: Vec<PathBuf>,
    pub(super) native_libraries: RustNativeLibraries,
}

impl RustCompanion {
    const PRELUDE: &str = concat!(
        concat!(
            include_str!("prelude.rs"),
            include_str!("capabilities/rust/native_capability_adapter.rs"),
            include_str!("capabilities/rust/native_capability_scope.rs"),
            include_str!("capabilities/rust/native_capabilities.rs"),
            include_str!("page_graph/native_service_definition.rs"),
            include_str!("page_graph/native_service_node.rs"),
            include_str!("page_graph/native_service_page.rs"),
            concat!(
                include_str!("page_graph/native_application_phase.rs"),
                include_str!("page_graph/native_application_state.rs"),
                include_str!("page_graph/native_application_services.rs")
            ),
            include_str!("page_graph/native_service_scope.rs"),
            include_str!("page_graph/native_service_activation.rs"),
            include_str!("page_graph/native_page_services.rs"),
            include_str!("page_graph/native_page_events.rs"),
            include_str!("page_graph/native_menu_actions.rs"),
            include_str!("capabilities/rust/file_picker_status.rs"),
            include_str!("capabilities/rust/file_picker_result.rs"),
            include_str!("capabilities/rust/file_picker_options.rs"),
            include_str!("capabilities/rust/file_picker.rs"),
            include_str!("capabilities/rust/window_controls.rs"),
            include_str!("capabilities/rust/current_page_events.rs"),
            include_str!("web_apis/tac_web_host.rs")
        ),
        concat!(
            include_str!("tac_task_error.rs"),
            include_str!("tac_task_waker.rs"),
            include_str!("tac_tasks.rs"),
            include_str!("tac_http_ticket.rs")
        ),
        include_str!("web_apis/clock.rs"),
        include_str!("web_apis/secure_random.rs"),
        include_str!("web_apis/connectivity.rs"),
        include_str!("web_apis/page_visibility.rs"),
        include_str!("web_apis/preferences.rs"),
        include_str!("web_apis/clipboard.rs"),
        include_str!("web_apis/share.rs"),
        include_str!("web_apis/notifications.rs"),
        include_str!("web_apis/geolocation.rs"),
        include_str!("web_apis/window_controls.rs")
    );
    const VALUE_TYPES_START: &str = "#[derive(Clone, Default)]\npub enum TacValue";
    const LISTENER_TYPE_START: &str = "pub struct TacListener";
    const MODEL_TYPES_START: &str = "pub trait TacModel";
    const MEMBER_TYPE_START: &str = "pub enum TacMember";
    const FAILURE_CODE: u16 = 1605;
    pub(crate) const WEB_TARGET: &str = "wasm32-unknown-unknown";

    pub(crate) fn model_source() -> &'static str {
        let start = Self::PRELUDE.find(Self::MODEL_TYPES_START).unwrap_or(0);
        let from_model_types = &Self::PRELUDE[start..];
        let length = from_model_types
            .find(Self::MEMBER_TYPE_START)
            .unwrap_or(from_model_types.len());
        &from_model_types[..length]
    }
}

#[cfg(test)]
pub(crate) const fn companion_prelude() -> &'static str {
    RustCompanion::PRELUDE
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Linkage {
    Static,
    Shared,
    Web(crate::BuildProfile),
}

impl Linkage {
    const fn crate_type(self) -> &'static str {
        match self {
            Self::Static => "staticlib",
            Self::Shared | Self::Web(_) => "cdylib",
        }
    }

    const fn profile_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Static | Self::Shared => &[],
            Self::Web(crate::BuildProfile::Release) => &RustWebModule::RELEASE_ARGUMENTS,
            Self::Web(crate::BuildProfile::Development) => &RustWebModule::DEVELOPMENT_ARGUMENTS,
        }
    }
}

pub(super) fn stage(
    companions: &[super::registry::NativeCompanionInput],
    stage: &Path,
    application_id: &str,
) -> Result<Option<PathBuf>, Failure> {
    let Some(authored) =
        super::registry::source(companions, crate::project::NativeCompanion::Rust)?
    else {
        return Ok(None);
    };
    let staged = stage.join("project").join("companion.rs");
    write(
        &staged,
        format!(
            "{authored}\nconst TAC_APPLICATION_ID: &str = {application_id:?};\n{}{}",
            RustCompanion::PRELUDE,
            crate::route_types::WebEventSources::rust()
        )
        .as_bytes(),
    )?;
    Ok(Some(staged))
}

impl RustCompanion {
    pub(super) fn stage_for_host(
        companions: &[super::registry::NativeCompanionInput],
        stage_folder: &Path,
        application: &super::config::NativeApplication,
        linkage: Linkage,
    ) -> Result<Option<PathBuf>, Failure> {
        let source = stage(companions, stage_folder, &application.application_id)?;
        super::mac_os_menu_action_source::MacOsMenuActionSource::stage(
            source.as_deref(),
            &application.menus,
        )?;
        let staged: Option<Result<(), Failure>> = application
            .rust
            .declaration
            .uses_cargo()
            .then(|| Self::stage_crate(application, source.as_deref(), linkage));
        staged.unwrap_or(Ok(()))?;
        Ok(source)
    }

    pub(super) async fn compile_companion(
        application: &super::config::NativeApplication,
        source: &Path,
        linkage: Linkage,
        target_triple: Option<&str>,
        library: &Path,
        helpers: &Path,
    ) -> Result<RustCompanionOutput, Failure> {
        let build = application
            .rust
            .crate_build(source, &application.application_id);
        Self::compile_chosen(&build, linkage, target_triple, library, helpers).await
    }

    async fn compile_chosen(
        build: &super::rust_crate_build::RustCrateBuild<'_>,
        linkage: Linkage,
        target_triple: Option<&str>,
        library: &Path,
        helpers: &Path,
    ) -> Result<RustCompanionOutput, Failure> {
        if build.declaration.uses_cargo() {
            return build.build(linkage, target_triple, library, helpers).await;
        }
        let (_, native_libraries) = compile_output(
            build.source,
            linkage,
            target_triple,
            library,
            linkage == Linkage::Static,
        )
        .await?;
        Ok(RustCompanionOutput {
            library: library.to_path_buf(),
            executables: Vec::new(),
            native_libraries,
        })
    }

    fn stage_crate(
        application: &super::config::NativeApplication,
        source: Option<&Path>,
        linkage: Linkage,
    ) -> Result<(), Failure> {
        let source = source.ok_or_else(|| {
            native_tool_failure(
                Self::FAILURE_CODE,
                "tac.application.rust declares crates or executables, but the application has no \
                 Rust service to build them with. Add a Rust service in client/services.",
            )
        })?;
        Self::require_distinct_executables(application)?;
        application
            .rust
            .crate_build(source, &application.application_id)
            .stage(linkage)
            .map(|_| ())
    }

    fn require_distinct_executables(
        application: &super::config::NativeApplication,
    ) -> Result<(), Failure> {
        let main = &application.executable_name;
        application
            .rust
            .declaration
            .executables
            .keys()
            .find(|name| name.eq_ignore_ascii_case(main))
            .map_or(Ok(()), |name| {
                Err(native_tool_failure(
                    Self::FAILURE_CODE,
                    &format!(
                        "The executable {name} has the name of the app's own executable ({main}). \
                         Choose another name in tac.application.rust.executables."
                    ),
                ))
            })
    }
}

fn unicode_path<'path>(path: &'path Path, message: &str) -> Result<&'path str, Failure> {
    path.to_str()
        .ok_or_else(|| native_tool_failure(RustCompanion::FAILURE_CODE, message))
}

pub(super) async fn compile(
    source: &Path,
    linkage: Linkage,
    target_triple: Option<&str>,
    output: &Path,
) -> Result<String, Failure> {
    compile_output(source, linkage, target_triple, output, false)
        .await
        .map(|(version, _)| version)
}

async fn compile_output(
    source: &Path,
    linkage: Linkage,
    target_triple: Option<&str>,
    output: &Path,
    report_native_libraries: bool,
) -> Result<(String, RustNativeLibraries), Failure> {
    if let Some(parent) = output.parent() {
        super::host::native_io(std::fs::create_dir_all(parent), parent)?;
    }
    let version_output = run_tool("rustc", &["--version"]).await?;
    let version = super::host::first_line(&version_output, "rustc unknown");
    let source = unicode_path(source, "Companion source path is not valid Unicode.")?;
    let output = unicode_path(output, "Companion output path is not valid Unicode.")?;
    let mut arguments = vec![
        "--edition",
        "2024",
        "--crate-type",
        linkage.crate_type(),
        "-O",
        source,
        "-o",
        output,
    ];
    if let Some(triple) = target_triple {
        arguments.extend(["--target", triple]);
    }
    arguments.extend(linkage.profile_arguments());
    if report_native_libraries {
        arguments.push("--print=native-static-libs");
    }
    let compiled = run_tool_output("rustc", &arguments).await?;
    let native_libraries = if report_native_libraries {
        RustNativeLibraries::read(&compiled)?
    } else {
        RustNativeLibraries::default()
    };
    Ok((version, native_libraries))
}

pub(crate) fn tac_value_source() -> &'static str {
    let prelude = RustCompanion::PRELUDE;
    let start = prelude.find(RustCompanion::VALUE_TYPES_START).unwrap_or(0);
    let from_value_types = &prelude[start..];
    let length = from_value_types
        .find(RustCompanion::LISTENER_TYPE_START)
        .unwrap_or(from_value_types.len());
    &from_value_types[..length]
}

fn web_companion_inputs(
    snapshot_root: &Path,
    services: &[(String, String, PathBuf)],
) -> Vec<super::registry::NativeCompanionInput> {
    let mut companions = Vec::with_capacity(services.len());
    for (service, class, source) in services {
        companions.push(super::registry::NativeCompanionInput {
            snapshot_root: snapshot_root.to_path_buf(),
            language: crate::project::NativeCompanion::Rust,
            source: source.clone(),
            service: service.clone(),
            class: class.clone(),
            web_assembly: true,
            application_provider: false,
        });
    }
    companions
}

pub(super) fn cache_key(compiler_version: &str, source: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(compiler_version.as_bytes());
    hasher.update([0]);
    hasher.update(source);
    let mut key = String::new();
    for byte in hasher.finalize() {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

pub(super) fn store_in_cache(cached: &Path, bytes: &[u8]) {
    let Some(parent) = cached.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_ok() {
        let _ = std::fs::write(cached, bytes);
    }
}

pub(crate) async fn web_assembly(
    snapshot_root: &Path,
    services: &[(String, String, PathBuf)],
    profile: crate::BuildProfile,
) -> Result<RustWebModule, Failure> {
    let companions = web_companion_inputs(snapshot_root, services);
    let staged = tempfile::Builder::new()
        .prefix(".tachyon-rust-wasm-")
        .tempdir()
        .map_err(|error| {
            native_tool_failure(
                RustCompanion::FAILURE_CODE,
                &format!("Cannot stage Rust services: {error}"),
            )
        })?;
    let Some(source) = stage(&companions, staged.path(), "web")? else {
        return Ok(RustWebModule::default());
    };
    let release = WasmReleaseStep::prepare(profile).await;
    let version = run_tool("rustc", &["-vV"]).await? + &release.identity();
    let source_bytes = super::host::native_io(std::fs::read(&source), &source)?;
    let key = cache_key(&version, &source_bytes);
    let cached = crate::cache::root()
        .join("rust-wasm")
        .join(format!("{key}.wasm"));
    if let Ok(bytes) = std::fs::read(&cached) {
        return Ok(RustWebModule::built(bytes, &release, release.will_shrink()));
    }
    let output = staged.path().join("services.wasm");
    compile(
        &source,
        Linkage::Web(profile),
        Some(RustCompanion::WEB_TARGET),
        &output,
    )
    .await?;
    let shrunk = release
        .shrink(&output, &RustWebModule::OPTIMIZER_FLAGS)
        .await;
    let bytes = super::host::native_io(std::fs::read(&output), &output)?;
    if shrunk == release.will_shrink() {
        store_in_cache(&cached, &bytes);
    }
    Ok(RustWebModule::built(bytes, &release, shrunk))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_value_source_is_only_the_value_types() {
        let source = super::tac_value_source();
        assert!(source.starts_with("#[derive(Clone, Default)]\npub enum TacValue"));
        assert!(!source.contains("TacListener"));
        assert!(!source.contains("TAC_APPLICATION_ID"));
    }
}
