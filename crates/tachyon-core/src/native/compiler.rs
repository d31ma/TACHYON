use super::android::AndroidHostGenerator;
use super::android_architecture::AndroidArchitecture;
use super::apple_icons::AppleIcons;
use super::config::NativeApplication;
use super::host::GeneratedHost;
use super::ios::IosHostGenerator;
use super::linux::LinuxHostGenerator;
use super::mac_os_architecture::MacOsArchitecture;
use super::macos::MacOsHostGenerator;
use super::native_web_stage::NativeWebStage;
use super::routes::NativeRouteIndex;
use super::windows::WindowsHostGenerator;
use super::windows_architecture::WindowsArchitecture;
use crate::compiler::{ArtifactProvenance, publish, resolve_output_path};
use crate::failure::diagnostic;
use crate::{BuildOptions, Failure, Project, ProjectDiscovery, WebCompiler};
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tachyon_contracts::{
    ArtifactContractVersions, ArtifactManifest, ArtifactOutput, ArtifactTarget, ArtifactToolchain,
    NativeTarget,
};

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBuildOptions {
    pub output_directory: PathBuf,
    pub target: NativeTarget,
    pub package: bool,
    pub release: bool,
    pub device: bool,
    pub windows_architecture: WindowsArchitecture,
    pub macos_architecture: MacOsArchitecture,
    pub android_architecture: AndroidArchitecture,
    pub incremental: bool,
    pub development_origin: Option<super::DevelopmentOrigin>,
}

impl Default for NativeBuildOptions {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::from("dist"),
            target: NativeTarget::Macos,
            package: true,
            release: false,
            device: false,
            windows_architecture: WindowsArchitecture::default(),
            macos_architecture: MacOsArchitecture::default(),
            android_architecture: AndroidArchitecture::default(),
            incremental: true,
            development_origin: None,
        }
    }
}

#[must_use]
pub const fn native_target_directory(target: NativeTarget) -> &'static str {
    match target {
        NativeTarget::Linux => "linux",
        NativeTarget::Macos => "macos",
        NativeTarget::Windows => "windows",
        NativeTarget::Android => "android",
        NativeTarget::Ios => "ios",
    }
}

pub(super) fn artifact_target(
    target: NativeTarget,
    windows: WindowsArchitecture,
    macos: MacOsArchitecture,
    android: AndroidArchitecture,
    ios: super::ios_destination::IosDestination,
) -> ArtifactTarget {
    let host_architecture = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    let (os, architecture, abi) = match target {
        NativeTarget::Linux => ("linux", host_architecture, "webkitgtk"),
        NativeTarget::Macos => ("macos", macos.artifact_architecture(), "wkwebview"),
        NativeTarget::Windows => ("windows", windows.artifact_architecture(), "webview2"),
        NativeTarget::Android => (
            "android",
            android.artifact_architecture(),
            "android-webview",
        ),
        NativeTarget::Ios => ("ios", ios.artifact_architecture(host_architecture), ios.abi),
    };
    ArtifactTarget {
        os: String::from(os),
        architecture: String::from(architecture),
        abi: String::from(abi),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBuildResult {
    output_directory: PathBuf,
    application_bundle: PathBuf,
    route_count: usize,
    compiled_routes: usize,
    reused_routes: usize,
    sha256: String,
    notes: Vec<String>,
    launch_identifier: String,
    executable_name: String,
}

impl NativeBuildResult {
    #[must_use]
    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    #[must_use]
    pub fn application_bundle(&self) -> &Path {
        &self.application_bundle
    }

    #[must_use]
    pub const fn route_count(&self) -> usize {
        self.route_count
    }

    #[must_use]
    pub const fn compiled_routes(&self) -> usize {
        self.compiled_routes
    }

    #[must_use]
    pub const fn reused_routes(&self) -> usize {
        self.reused_routes
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    #[must_use]
    pub fn launch_identifier(&self) -> &str {
        &self.launch_identifier
    }

    #[must_use]
    pub fn executable_name(&self) -> &str {
        &self.executable_name
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeCompiler;

impl NativeCompiler {
    pub fn emit_ui_descriptor(
        build: &NativeBuildResult,
        output: &cap_std::fs::Dir,
    ) -> Result<String, Failure> {
        Self::ui_generation(build)?.emit_descriptor(output)
    }

    pub fn ui_generation(build: &NativeBuildResult) -> Result<super::NativeUiGeneration, Failure> {
        super::native_ui_artifact::NativeUiArtifact::generation(build)
    }

    const MAX_NATIVE_SERVICES: usize = 128;
    const ARTIFACT_MANIFEST_NAME: &str = "artifact-manifest.json";

    pub fn build(
        project_root: impl AsRef<Path>,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure> {
        let project = ProjectDiscovery::discover(project_root)?;
        Self::build_project(&project, options)
    }

    pub fn build_project(
        project: &Project,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                Failure::one(diagnostic(
                    1605,
                    format!("Cannot start the native build runtime: {error}"),
                    None,
                    None,
                ))
            })?;
        runtime.block_on(Self::build_project_async(project, options))
    }

    pub async fn build_async(
        project_root: impl AsRef<Path>,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure> {
        let project = ProjectDiscovery::discover(project_root)?;
        Self::build_project_async(&project, options).await
    }

    pub async fn build_project_async(
        project: &Project,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure> {
        Self::build_project_in_mode(project, options, false).await
    }

    pub async fn build_ui_candidate_async(
        project_root: impl AsRef<Path>,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure> {
        super::native_ui_profile::NativeUiProfile::target(options.target)?;
        let project = ProjectDiscovery::discover(project_root)?;
        Self::build_project_in_mode(&project, options, true).await
    }

    async fn build_project_in_mode(
        project: &Project,
        options: &NativeBuildOptions,
        ui_candidate: bool,
    ) -> Result<NativeBuildResult, Failure> {
        let development_note = Self::declare_development_origin(project, options)?;
        let mut application =
            NativeApplication::discover_from_snapshot(project.snapshot_root(), project.root())?;
        let floor =
            super::system_floor::SystemFloor::for_project(project, &application, options.target)?;
        application.minimum_system = floor.minimum;
        let base_output = resolve_output_path(project.root(), &options.output_directory)?;
        ensure_entry_route_is_a_page(project, &application)?;
        let index =
            NativeRouteIndex::build(project.route_graph().routes(), &application.entry_route)?;
        application.menus.validate_for(options.target, &index)?;
        super::release_packaging::ReleasePackaging::check(options.target, options.release)?;
        let release_note = super::release_packaging::ReleasePackaging::signing_note(
            options.target,
            options.release,
        )?;
        super::ios_device_build::IosDeviceBuild::preflight(options)?;
        super::native_release_version::NativeReleaseVersion::of(&application)
            .validate_for(options.target)?;
        application.permissions.validate_for(options.target)?;
        super::linux_packages::LinuxPackages::check(&application, options.target, options.release)?;
        let notes = super::target_support::TargetSupport::native_notes(
            project.snapshot_root(),
            options.target,
        )?;
        let yon_note = super::yon_reach::YonReach::check(project, options.target)?;

        let destination = base_output.join(native_target_directory(options.target));
        let (temporary_web, web, companions) =
            build_web_bundle(project, options, &base_output, &destination).await?;
        super::native_menu_action_check::NativeMenuActionCheck::validate(
            &application.menus,
            &companions,
        )?;
        super::native_tray_icon::NativeTrayIcon::check(
            application.tray.as_ref(),
            temporary_web.path(),
        )?;
        super::native_ui_profile::NativeUiProfile::prepare(
            ui_candidate,
            temporary_web.path(),
            options.target,
        )?;
        let stage = create_stage(&destination, project.root())?;
        write_host_descriptor(
            stage.path(),
            &application,
            options.target,
            &index,
            &companions,
        )?;
        let gradle_note = super::android_gradle_cache::AndroidGradleCache::build_note(
            options.target,
            options.package,
            &companions,
        );
        let generated = generate_host(
            options,
            &application,
            &index,
            &companions,
            temporary_web.path(),
            stage.path(),
        )
        .await?;
        write_artifact_manifest(stage.path(), options, &generated)?;
        let digest = digest_tree(stage.path())?;
        publish(stage, &destination).map_err(|error| native_io_failure(&destination, &error))?;

        Ok(NativeBuildResult {
            output_directory: destination.clone(),
            application_bundle: destination.join(generated.application_bundle),
            route_count: index.routes.len(),
            compiled_routes: web.compiled_routes(),
            reused_routes: web.reused_routes(),
            sha256: digest,
            launch_identifier: launch_identifier(options.target, &application.application_id),
            executable_name: application.executable_name.clone(),
            notes: notes
                .into_iter()
                .chain(AppleIcons::skipped_note(&application, options.target))
                .chain(yon_note)
                .chain(release_note)
                .chain(gradle_note)
                .chain(floor.note)
                .chain(floor.style_note)
                .chain(development_note)
                .collect(),
        })
    }

    fn declare_development_origin(
        project: &Project,
        options: &NativeBuildOptions,
    ) -> Result<Option<String>, Failure> {
        let Some(development) = &options.development_origin else {
            return Ok(None);
        };
        if options.release {
            return Err(Failure::one(diagnostic(
                1601,
                "A release build cannot point the app at the development server.",
                Some(String::from(
                    "Build the release with `ty bundle --release`, without `ty preview`.",
                )),
                None,
            )));
        }
        development.declare_in(project.snapshot_root())
    }
}

fn launch_identifier(target: NativeTarget, application_id: &str) -> String {
    match target {
        NativeTarget::Android => super::android::java_package(application_id),
        NativeTarget::Macos | NativeTarget::Ios | NativeTarget::Linux | NativeTarget::Windows => {
            String::from(application_id)
        }
    }
}

fn create_stage(destination: &Path, project_root: &Path) -> Result<tempfile::TempDir, Failure> {
    let parent = destination.parent().unwrap_or(project_root);
    fs::create_dir_all(parent).map_err(|error| native_io_failure(parent, &error))?;
    tempfile::Builder::new()
        .prefix(".tachyon-native-build-")
        .tempdir_in(parent)
        .map_err(|error| native_io_failure(parent, &error))
}

fn ensure_entry_route_is_a_page(
    project: &Project,
    application: &NativeApplication,
) -> Result<(), Failure> {
    for route in project.route_graph().routes() {
        if route.route() == application.entry_route && route.source_path().is_some() {
            return Ok(());
        }
    }
    Err(Failure::one(diagnostic(
        1601,
        format!(
            "Native entry route '{}' does not identify a page.",
            application.entry_route
        ),
        Some(String::from(
            "Point start_url in client/manifest.json at a discovered page route.",
        )),
        None,
    )))
}

fn write_artifact_manifest(
    stage: &Path,
    options: &NativeBuildOptions,
    generated: &GeneratedHost,
) -> Result<(), Failure> {
    let outputs = collect_outputs(stage, NativeCompiler::ARTIFACT_MANIFEST_NAME)?;
    let manifest = artifact_manifest(options, generated, outputs);
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        Failure::one(diagnostic(
            1605,
            format!("Cannot serialize Artifact Manifest v1: {error}"),
            None,
            None,
        ))
    })?;
    manifest_bytes.push(b'\n');
    fs::write(
        stage.join(NativeCompiler::ARTIFACT_MANIFEST_NAME),
        manifest_bytes,
    )
    .map_err(|error| native_io_failure(stage, &error))
}

fn write_host_descriptor(
    stage: &Path,
    application: &NativeApplication,
    target: NativeTarget,
    index: &NativeRouteIndex,
    companions: &[super::registry::NativeCompanionInput],
) -> Result<(), Failure> {
    let mut companion_descriptions = Vec::with_capacity(companions.len());
    for companion in companions {
        companion_descriptions.push(serde_json::json!({
            "service": companion.service, "language": companion.language.label(),
        }));
    }
    let host = serde_json::json!({
        "schemaVersion": 3,
        "target": native_target_directory(target),
        "appName": application.name,
        "appId": application.application_id,
        "version": application.version,
        "renderMode": "bundle",
        "entryRoute": index.entry_route,
        "entryDocument": index.entry_document,
        "platformApiVersion": 1,
        "bridgeVersion": 3,
        "companions": companion_descriptions,
        "windowControls": application.window.controls,
        "hostCapabilities": ["companion.invoke"],
    });
    write_pretty_json(&stage.join("tachyon.host.json"), &host)
}

fn write_pretty_json(path: &Path, value: &serde_json::Value) -> Result<(), Failure> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        Failure::one(diagnostic(
            1605,
            format!("Cannot serialize native compatibility artifact: {error}"),
            None,
            None,
        ))
    })?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| native_io_failure(path, &error))
}

async fn generate_host(
    options: &NativeBuildOptions,
    application: &NativeApplication,
    index: &NativeRouteIndex,
    companions: &[super::registry::NativeCompanionInput],
    web_bundle: &Path,
    stage: &Path,
) -> Result<GeneratedHost, Failure> {
    match options.target {
        NativeTarget::Macos => {
            MacOsHostGenerator::generate(application, index, companions, web_bundle, stage, options)
                .await
        }
        NativeTarget::Ios => {
            IosHostGenerator::generate(application, index, companions, web_bundle, stage, options)
                .await
        }
        NativeTarget::Linux => {
            LinuxHostGenerator::generate(application, index, companions, web_bundle, stage, options)
                .await
        }
        NativeTarget::Windows => {
            WindowsHostGenerator::generate(
                application,
                index,
                companions,
                web_bundle,
                stage,
                options,
            )
            .await
        }
        NativeTarget::Android => {
            AndroidHostGenerator::generate(
                application,
                index,
                companions,
                web_bundle,
                stage,
                options,
            )
            .await
        }
    }
}

async fn build_web_bundle(
    project: &crate::Project,
    options: &NativeBuildOptions,
    output_root: &Path,
    previous_app: &Path,
) -> Result<
    (
        tempfile::TempDir,
        crate::BuildResult,
        Vec<super::registry::NativeCompanionInput>,
    ),
    Failure,
> {
    let temporary_web = NativeWebStage::create(output_root)?;
    NativeWebStage::seed(temporary_web.path(), previous_app, options.incremental);
    let web = WebCompiler::build_project_for_native(
        project,
        &BuildOptions {
            output_directory: temporary_web.path().to_path_buf(),
            incremental: options.incremental,
            ..BuildOptions::default()
        },
        options.target,
    )
    .await?;

    let companions = native_companions(project, options.target, web.native_service_roots())?;
    Ok((temporary_web, web, companions))
}

fn native_companions(
    project: &crate::Project,
    target: NativeTarget,
    roots: &std::collections::BTreeSet<String>,
) -> Result<Vec<super::registry::NativeCompanionInput>, Failure> {
    let registry = crate::template::ServiceRegistry::discover(project.snapshot_root())?
        .for_target(Some(target));
    let roots = super::native_service_roots::NativeServiceRoots::expand(&registry, roots);
    let mut compiled = Vec::new();
    for (instance, service) in registry.iter() {
        let exposed = roots.contains(instance);
        let application_provider =
            super::native_application_inputs::NativeApplicationInputs::provider(
                project.snapshot_root(),
                service,
                target,
                exposed,
            )?;
        if ![exposed, application_provider].contains(&true) {
            continue;
        }
        let Some((language, source)) = chosen_companion(service, target) else {
            continue;
        };
        if compiled.len() >= NativeCompiler::MAX_NATIVE_SERVICES {
            return Err(Failure::one(diagnostic(
                1010,
                "Native application exceeds the 128 native service limit.",
                None,
                None,
            )));
        }
        compiled.push(super::registry::NativeCompanionInput {
            snapshot_root: project.snapshot_root().to_path_buf(),
            language,
            source,
            service: instance.clone(),
            class: String::from(service.class()),
            web_assembly: false,
            application_provider,
        });
    }
    Ok(compiled)
}

fn chosen_companion(
    service: &crate::template::ServiceDefinition,
    target: NativeTarget,
) -> Option<(crate::project::NativeCompanion, PathBuf)> {
    let declared = service.native_companions();
    let languages: Vec<_> = declared.iter().map(|(language, _)| *language).collect();
    let chosen = crate::project::NativeCompanion::most_specific(&languages, target)?;
    declared
        .iter()
        .find(|(language, _)| *language == chosen)
        .map(|(language, source)| (*language, source.clone()))
}

fn artifact_manifest(
    options: &NativeBuildOptions,
    generated: &GeneratedHost,
    outputs: Vec<ArtifactOutput>,
) -> ArtifactManifest {
    ArtifactManifest {
        contract_version: 1,
        release_version: String::from(tachyon_contracts::PRODUCT_VERSION),
        commit: ArtifactProvenance::commit(),
        source_date_epoch: ArtifactProvenance::source_date_epoch(),
        target: artifact_target(
            options.target,
            options.windows_architecture,
            options.macos_architecture,
            options.android_architecture,
            super::ios_destination::IosDestination::of(options),
        ),
        toolchains: vec![
            ArtifactToolchain {
                name: String::from("rust"),
                version: String::from(ArtifactProvenance::RUST_VERSION),
            },
            ArtifactToolchain {
                name: generated.toolchain_name.clone(),
                version: generated.toolchain_version.clone(),
            },
        ],
        contracts: ArtifactContractVersions::default(),
        outputs,
    }
}

fn sorted_entries(directory: &Path) -> Result<Vec<fs::DirEntry>, Failure> {
    let mut entries = fs::read_dir(directory)
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(|error| native_io_failure(directory, &error))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

fn file_output(path: &Path, relative: String, size: u64) -> Result<ArtifactOutput, Failure> {
    let bytes = fs::read(path).map_err(|error| native_io_failure(path, &error))?;
    Ok(ArtifactOutput {
        path: relative,
        sha256: sha256_bytes(&bytes),
        size,
    })
}

fn collect_outputs(root: &Path, excluded: &str) -> Result<Vec<ArtifactOutput>, Failure> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in sorted_entries(&directory)? {
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| native_io_failure(&path, &error))?;
            if metadata.file_type().is_symlink() {
                return Err(Failure::one(diagnostic(
                    1605,
                    "Native output contains a symlink.",
                    None,
                    None,
                )));
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
            if relative != excluded {
                files.push(file_output(&path, relative, metadata.len())?);
            }
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn digest_tree(root: &Path) -> Result<String, Failure> {
    let outputs = collect_outputs(root, "\0")?;
    let mut hasher = Sha256::new();
    for output in outputs {
        hasher.update(output.path.as_bytes());
        hasher.update([0]);
        hasher.update(output.sha256.as_bytes());
        hasher.update([0]);
    }
    Ok(hex_digest(hasher.finalize()))
}

fn sha256_bytes(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex_digest(hasher.finalize())
}

fn hex_digest(value: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in value.as_ref() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn native_io_failure(path: &Path, error: &io::Error) -> Failure {
    Failure::one(diagnostic(
        1605,
        format!("Native build cannot access '{}': {error}", path.display()),
        Some(String::from(
            "Check permissions and keep generated output paths regular and project-contained.",
        )),
        None,
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::super::host::NativeHost;
    use super::super::{android, ios, linux, macos, rust, windows};
    use super::{NativeBuildOptions, NativeCompiler, chosen_companion};
    use crate::project::NativeCompanion;
    use crate::template::ServiceRegistry;
    use std::fs;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn every_host_implements_the_companion_channel_and_nothing_else() {
        assert!(NativeHost::SHIM.contains("__tachyonNativeHostCall"));
        assert!(!NativeHost::SHIM.contains("globalThis.tachyonHost"));
        assert!(!NativeHost::SHIM.contains("__tachyonNativeEmit"));
        for (platform, source) in [
            ("macos", macos::host_source()),
            ("windows", windows::host_source()),
            ("linux", linux::host_source()),
        ] {
            assert!(
                source.contains("companion.invoke"),
                "{platform} host cannot answer a companion call"
            );
            for gone in [
                "tray.set",
                "tray.remove",
                "window.show",
                "window.hide",
                "app.quit",
            ] {
                assert!(
                    !source.contains(&format!("\"{gone}\"")),
                    "{platform} host still implements '{gone}'"
                );
            }
        }
    }

    #[test]
    fn every_host_relays_what_its_companion_publishes() {
        assert_hosts_deliver_publishes();
        assert_companions_can_publish();
        assert_services_can_declare_publishers();
    }

    fn assert_hosts_deliver_publishes() {
        assert!(NativeHost::SHIM.contains("__tachyonCompanionPublish"));
        assert!(NativeHost::SHIM.contains("__tachyonCompanionQueue"));
        for (platform, source) in [
            ("macos", macos::host_source()),
            ("ios", ios::host_source()),
            ("windows", windows::host_source()),
            ("linux", linux::host_source()),
            ("android", android::host_source()),
        ] {
            assert!(
                source.contains("__tachyonCompanionPublish"),
                "{platform} host never delivers a publish to the page"
            );
        }
        for (platform, source) in [
            ("windows", windows::host_source()),
            ("linux", linux::host_source()),
        ] {
            assert!(
                source.contains("tac_native_set_emit"),
                "{platform} host never installs its sink"
            );
        }
    }

    fn assert_companions_can_publish() {
        for (language, prelude, publish) in [
            ("swift", macos::companion_prelude(), "func tacEventPublish"),
            (
                "kotlin",
                android::companion_prelude().as_str(),
                "fun tacEventPublish",
            ),
            ("csharp", windows::companion_prelude(), "void EventPublish"),
            ("rust", rust::companion_prelude(), "fn tac_event_publish"),
        ] {
            assert!(
                prelude.contains(publish),
                "a {language} companion cannot publish"
            );
        }
        for (language, prelude) in [
            ("csharp", windows::WindowsHostGenerator::native_tasks()),
            ("rust", rust::companion_prelude()),
        ] {
            assert!(
                prelude.contains("tac_native_set_emit"),
                "a {language} companion exports no sink installer"
            );
        }
    }

    fn assert_services_can_declare_publishers() {
        for (language, prelude, publisher) in [
            ("swift", macos::companion_prelude(), "func publishEvent"),
            (
                "kotlin",
                android::companion_prelude().as_str(),
                "fun <T : Any> publishEvent",
            ),
            ("csharp", windows::companion_prelude(), "void PublishEvent"),
            ("rust", rust::companion_prelude(), "fn publish_event"),
        ] {
            assert!(
                prelude.contains("ApplicationEventPublisher") && prelude.contains(publisher),
                "a {language} service cannot declare a publisher member"
            );
            assert!(
                prelude.contains("TacListener") || prelude.contains("TacMember Listener("),
                "a {language} listener carries no ADR 0031 options"
            );
            for option in ["order", "condition", "asynchronous"] {
                assert!(
                    prelude.to_lowercase().contains(option),
                    "a {language} listener has no {option}"
                );
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Reach {
        Linked(NativeCompanion),
        WebView,
    }

    fn service_files(extensions: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let services = root.path().join("client/services");
        fs::create_dir_all(&services).expect("services");
        for extension in extensions {
            let source = match *extension {
                "js" | "ts" => "@Service\nexport default class TallyService {}\n",
                "dart" => "@Service\nclass TallyService {}\n",
                "go" => "// @Service\ntype TallyService struct{ Count int }\n",
                "rs" => "#[Service]\n#[derive(Default)]\nstruct TallyService {}\n",
                "cs" => "[Service]\npublic class TallyService {}\n",
                "swift" => "@Service\nfinal class TallyService {}\n",
                _ => "@Service\nclass TallyService\n",
            };
            fs::write(services.join(format!("TallyService.{extension}")), source).expect("service");
        }
        root
    }

    fn reach(extensions: &[&str], target: NativeTarget) -> Reach {
        let root = service_files(extensions);
        let registry = ServiceRegistry::discover(root.path())
            .expect("registry")
            .for_target(Some(target));
        let service = registry.get("tally").expect("tally");
        chosen_companion(service, target)
            .map_or(Reach::WebView, |(language, _)| Reach::Linked(language))
    }

    fn assert_reach(extensions: &[&str], expected: [Reach; 5]) {
        let targets = [
            NativeTarget::Macos,
            NativeTarget::Ios,
            NativeTarget::Android,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ];
        for (target, wanted) in targets.into_iter().zip(expected) {
            assert_eq!(
                reach(extensions, target),
                wanted,
                "{extensions:?} on {target:?}"
            );
        }
    }

    #[test]
    fn rust_links_on_every_native_target() {
        let rust = Reach::Linked(NativeCompanion::Rust);
        assert_reach(&["rs"], [rust; 5]);
    }

    #[test]
    fn csharp_links_on_windows_and_is_webassembly_everywhere_else() {
        let csharp = Reach::Linked(NativeCompanion::CSharp);
        let web = Reach::WebView;
        assert_reach(&["cs"], [web, web, web, csharp, web]);
    }

    #[test]
    fn go_and_browser_languages_run_in_the_web_view_on_every_target() {
        for extensions in [&["go"][..], &["js"], &["ts"], &["dart"]] {
            assert_reach(extensions, [Reach::WebView; 5]);
        }
    }

    #[test]
    fn rust_with_go_is_webassembly_where_rust_does_not_link() {
        let rust = Reach::Linked(NativeCompanion::Rust);
        let web = Reach::WebView;
        assert_reach(&["rs", "go"], [rust; 5]);
        let csharp = Reach::Linked(NativeCompanion::CSharp);
        assert_reach(&["cs", "go"], [web, web, web, csharp, web]);
    }

    #[test]
    fn swift_links_on_apple_and_kotlin_links_on_android_and_is_webassembly_elsewhere() {
        let swift = Reach::Linked(NativeCompanion::Swift);
        let kotlin = Reach::Linked(NativeCompanion::Kotlin);
        let rust = Reach::Linked(NativeCompanion::Rust);
        let web = Reach::WebView;
        assert_reach(&["swift"], [swift, swift, web, web, web]);
        assert_reach(&["kt"], [web, web, kotlin, web, web]);
        assert_reach(&["swift", "rs"], [swift, swift, rust, rust, rust]);
        assert_reach(&["kt", "rs"], [rust, rust, kotlin, rust, rust]);
        assert_reach(
            &["kt", "cs"],
            [
                web,
                web,
                kotlin,
                Reach::Linked(NativeCompanion::CSharp),
                web,
            ],
        );
    }

    #[test]
    fn every_host_serves_webassembly_and_lets_the_page_compile_it() {
        for (platform, source) in [
            ("macos", macos::host_source()),
            ("ios", ios::host_source()),
            ("android", android::host_source()),
            ("windows", windows::host_source()),
            ("linux", linux::host_source()),
        ] {
            assert!(
                source.contains("application/wasm"),
                "{platform} host serves .wasm with the wrong type"
            );
            assert!(
                source.contains("Content-Security-Policy"),
                "{platform} host sends no Content-Security-Policy"
            );
            assert!(
                source.contains("__CONTENT_SECURITY_POLICY__"),
                "{platform} host does not send the generated policy"
            );
            let source = source.replace(
                "__CONTENT_SECURITY_POLICY__",
                &crate::compiler::ContentSecurityPolicy::native_host(""),
            );
            assert!(
                source.contains("script-src 'self' 'wasm-unsafe-eval'"),
                "{platform} host policy blocks WebAssembly"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn in_process_native_build_covers_packaging_and_public_evidence() {
        let root = tempfile::tempdir().expect("project");
        let pages = root.path().join("client/pages");
        fs::create_dir_all(&pages).expect("pages");
        fs::create_dir_all(root.path().join("client")).expect("client");
        fs::write(
            root.path().join("client/manifest.json"),
            r#"{"name":"Coverage","start_url":"/","tac":{"application":{"id":"dev.tachyon.coverage","version":"1.0.0"}}}"#,
        )
        .expect("manifest");
        fs::write(pages.join("tac.html"), "<page-view />").expect("view");
        let component = root.path().join("client/components/page/view");
        fs::create_dir_all(&component).expect("component");
        fs::write(component.join("tac.html"), r#"<main aria-label="Coverage"><h1>Coverage</h1><button aria-label="Add" data-tachyon-action="increment:count">Add</button><output aria-label="Count" data-tachyon-bind="count" data-tachyon-state="0">0</output><input aria-label="Name" data-tachyon-bind="name" data-tachyon-state=""><details aria-label="Details"><summary>Details</summary><p>Open</p></details><x-chart aria-label="Chart">Web</x-chart><iframe aria-label="Report" src="https://reports.example.test/path"></iframe></main>"#).expect("component view");

        let result =
            NativeCompiler::build(root.path(), &NativeBuildOptions::default()).expect("build");
        assert!(result.output_directory().ends_with("dist/macos"));
        assert!(result.application_bundle().is_dir());
        assert_eq!(result.route_count(), 1);
        assert_eq!(result.sha256().len(), 64);
    }

    #[test]
    fn invalid_entry_route_fails_before_native_tool_execution() {
        let root = tempfile::tempdir().expect("project");
        let pages = root.path().join("client/pages");
        fs::create_dir_all(&pages).expect("pages");
        fs::create_dir_all(root.path().join("client")).expect("client");
        fs::write(
            root.path().join("client/manifest.json"),
            r#"{"name":"Entry","start_url":"/missing","tac":{"application":{"id":"dev.tachyon.entry","version":"1"}}}"#,
        )
        .expect("manifest");
        fs::write(pages.join("tac.html"), "<main>Home</main>").expect("view");
        let error = NativeCompiler::build(root.path(), &NativeBuildOptions::default())
            .expect_err("missing entry");
        assert!(error.to_string().contains("TY1601"));
        assert!(error.to_string().contains("'/missing'"));
    }

    #[test]
    fn a_labelled_version_is_refused_before_native_tool_execution_where_stores_need_numbers() {
        let root = tempfile::tempdir().expect("project");
        let pages = root.path().join("client/pages");
        fs::create_dir_all(&pages).expect("pages");
        fs::write(
            root.path().join("client/manifest.json"),
            r#"{"name":"Labelled","start_url":"/","tac":{"application":{"id":"dev.tachyon.labelled","version":"1.0-beta","build":3}}}"#,
        )
        .expect("manifest");
        fs::write(pages.join("tac.html"), "<main>Home</main>").expect("view");
        for target in [
            tachyon_contracts::NativeTarget::Macos,
            tachyon_contracts::NativeTarget::Ios,
            tachyon_contracts::NativeTarget::Windows,
        ] {
            let options = NativeBuildOptions {
                target,
                ..Default::default()
            };
            let error = NativeCompiler::build(root.path(), &options).expect_err("labelled version");
            let text = error.to_string();
            assert!(text.contains("TY1601"), "{text}");
            assert!(text.contains("'1.0-beta'"), "{text}");
        }
    }
    #[cfg(unix)]
    #[test]
    fn public_native_build_from_project_reuses_the_owned_web_and_config_snapshot() {
        use std::os::unix::fs::symlink;

        if !crate::node_driver::NodeDriver::available() {
            return;
        }
        let workspace = tempfile::tempdir().expect("workspace");
        let authored = workspace.path().join("project");
        write_owned_project(&authored);
        let project = crate::ProjectDiscovery::discover(&authored).expect("owned snapshot");

        let retained = workspace.path().join("retained");
        fs::rename(&authored, &retained).expect("move project");
        let planted = tempfile::tempdir().expect("planted");
        write_planted_project(planted.path());
        symlink(planted.path(), &authored).expect("ambient replacement");

        let result = NativeCompiler::build_project(
            &project,
            &NativeBuildOptions {
                output_directory: workspace.path().join("native-output"),
                target: tachyon_contracts::NativeTarget::Macos,
                package: false,
                ..Default::default()
            },
        )
        .expect("snapshot native build");
        assert_native_snapshot_output(result.output_directory());
    }

    #[cfg(unix)]
    fn write(root: &std::path::Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    #[cfg(unix)]
    fn write_owned_project(authored: &std::path::Path) {
        write(
            authored,
            "client/pages/tac.html",
            r#"<owned-card hydrate="load" />"#,
        );
        write(
            authored,
            "client/components/owned/card/tac.html",
            "<section>owned native component {ownedCard.marker}</section>",
        );
        write(
            authored,
            "client/services/OwnedCardService.js",
            "@Service()\nexport default class OwnedCardService { static origin = 'owned-native-script'; marker = '' }\n",
        );
        write(authored, "client/shared/origin.txt", "owned-native-shared");
        write(
            authored,
            "client/services/OwnedCardService.swift",
            "@Service\nfinal class OwnedCardService {\n let marker: String = \"owned-native-companion\"\n}\n",
        );
        write(authored, "package.json", r#"{"type":"module"}"#);
        write(
            authored,
            "client/manifest.json",
            r#"{"name":"Owned Snapshot","start_url":"/","tac":{"origin":"https://owned.example","application":{"id":"dev.tachyon.owned","version":"1.2.3"}}}"#,
        );
    }

    #[cfg(unix)]
    fn write_planted_project(planted: &std::path::Path) {
        write(planted, "client/pages/tac.html", "<owned-card />");
        write(
            planted,
            "client/components/owned/card/tac.html",
            "<section>planted native component {ownedCard.marker}</section>",
        );
        write(
            planted,
            "client/services/OwnedCardService.js",
            "@Service()\nexport default class OwnedCardService { marker = '' }\n",
        );
        write(planted, "client/shared/origin.txt", "planted-native-shared");
        write(planted, "package.json", r#"{"type":"module"}"#);
        write(
            planted,
            "client/manifest.json",
            r#"{"name":"Planted Canary","start_url":"/","tac":{"origin":"https://planted.example","application":{"id":"dev.tachyon.planted","version":"9.9.9"}}}"#,
        );
    }

    #[cfg(unix)]
    fn assert_native_snapshot_output(output: &std::path::Path) {
        let web = output.join("web");
        let index = fs::read_to_string(web.join("index.html")).expect("web index");
        let component = fs::read_to_string(web.join(".tachyon/services/OwnedCardService.js"))
            .expect("service script");
        let plist = fs::read_to_string(output.join("OwnedSnapshot.app/Contents/Info.plist"))
            .expect("native config output");
        assert!(index.contains("owned native component"), "{index}");
        assert!(!index.contains("planted"), "{index}");
        assert!(component.contains("owned-native-script"), "{component}");
        assert_eq!(
            fs::read_to_string(web.join("shared/origin.txt")).expect("shared"),
            "owned-native-shared"
        );
        let robots = fs::read_to_string(web.join("robots.txt")).expect("robots");
        assert!(robots.contains("https://owned.example"), "{robots}");
        assert!(plist.contains("Owned Snapshot"), "{plist}");
        assert!(plist.contains("dev.tachyon.owned"), "{plist}");
        assert!(plist.contains("1.2.3"), "{plist}");
        assert!(!plist.contains("Planted Canary"), "{plist}");
        assert!(!plist.contains("dev.tachyon.planted"), "{plist}");
        let companion = fs::read_to_string(output.join("project/TachyonCompanion.swift"))
            .expect("captured companion");
        assert!(companion.contains("owned-native-companion"));
    }
}

#[cfg(test)]
#[path = "native_composition_tests.rs"]
mod composition_tests;

#[cfg(test)]
#[path = "native_external_output_tests.rs"]
mod external_output_tests;
