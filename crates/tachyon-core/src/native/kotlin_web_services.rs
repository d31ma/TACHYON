use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::project::NativeCompanion;
use kotlin_model_readers::KotlinModelReaders;
use kotlin_wasm_program::KotlinWasmProgram;
use std::path::{Path, PathBuf};

mod kotlin_model_readers;
mod kotlin_wasm_program;

pub(crate) struct KotlinWebServices;

impl KotlinWebServices {
    const PRELUDE: &'static str = concat!(
        include_str!("prelude.kt"),
        include_str!("prelude_web.kt"),
        include_str!("web_apis/Clock.kt"),
        include_str!("web_apis/SecureRandom.kt"),
        include_str!("web_apis/Connectivity.kt"),
        include_str!("web_apis/PageVisibility.kt"),
        include_str!("web_apis/Preferences.kt"),
        include_str!("web_apis/Clipboard.kt"),
        include_str!("web_apis/Share.kt"),
        include_str!("web_apis/Notifications.kt"),
        include_str!("web_apis/Geolocation.kt")
    );
    const JVM_SHIMS: [(&'static str, &'static str); 3] = [
        (
            "jvm_shims/java_util.kt",
            include_str!("kotlin_web_services/jvm_shims/java_util.kt"),
        ),
        (
            "jvm_shims/java_util_concurrent.kt",
            include_str!("kotlin_web_services/jvm_shims/java_util_concurrent.kt"),
        ),
        (
            "jvm_shims/jvm_names.kt",
            include_str!("kotlin_web_services/jvm_shims/jvm_names.kt"),
        ),
    ];
    const STORAGE: &'static str = include_str!("kotlin_web_services/kotlin_storage.kt");
    const PROGRAM_SOURCE: &'static str = "services.kt";
    const MODEL_REALM: &'static str = "client";
    const MODEL_EXTENSION: &'static str = "kt";

    pub(crate) async fn build(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let sources = Self::sources(snapshot_root, services)?;
        Self::compiled(&sources, profile).await
    }

    async fn compiled(
        sources: &[(PathBuf, String)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        if sources.is_empty() {
            return Ok((Vec::new(), None));
        }
        let program = KotlinWasmProgram::compile(sources, profile).await?;
        let note = program.note(profile);
        Ok((program.published(), note))
    }

    fn sources(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let companions =
            NativeCompanionInput::web_services(snapshot_root, NativeCompanion::Kotlin, services);
        let Some(authored) = super::registry::source(&companions, NativeCompanion::Kotlin)? else {
            return Ok(Vec::new());
        };
        Self::program_sources(snapshot_root, &authored)
    }

    fn program_sources(
        snapshot_root: &Path,
        authored: &str,
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let models = crate::model_sources::ModelSources::read(
            snapshot_root,
            Self::MODEL_REALM,
            Self::MODEL_EXTENSION,
        )?;
        let model_files: Vec<(PathBuf, String)> = models.separate_files();
        let texts: Vec<(String, String)> = model_files
            .iter()
            .map(|(path, text)| (path.to_string_lossy().into_owned(), text.clone()))
            .collect();
        let authored = crate::behavior::KotlinBuiltins::in_browser(authored, Self::STORAGE);
        let program = format!(
            "{authored}\n{}{}{}",
            crate::route_types::WebEventSources::kotlin(),
            KotlinModelReaders::text(&texts),
            Self::PRELUDE
        );
        let mut sources = vec![(PathBuf::from(Self::PROGRAM_SOURCE), program)];
        sources.extend(model_files);
        sources
            .extend(Self::JVM_SHIMS.map(|(path, text)| (PathBuf::from(path), String::from(text))));
        Ok(sources)
    }
}
