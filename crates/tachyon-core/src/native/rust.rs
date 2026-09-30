use super::host::{native_tool_failure, run_tool, write};
use super::rust_web_module::RustWebModule;
use crate::Failure;
use crate::wasm_release_step::WasmReleaseStep;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct RustCompanion;

impl RustCompanion {
    const PRELUDE: &str = concat!(
        include_str!("prelude.rs"),
        include_str!("web_apis/tac_web_host.rs"),
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
    const FAILURE_CODE: u16 = 1605;
    pub(crate) const WEB_TARGET: &str = "wasm32-unknown-unknown";
}

#[cfg(test)]
pub(super) const fn companion_prelude() -> &'static str {
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
    run_tool("rustc", &arguments).await?;
    Ok(version)
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
