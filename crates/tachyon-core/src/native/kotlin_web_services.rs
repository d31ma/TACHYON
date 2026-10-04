use super::file_picker_sources::FilePickerSources;
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::project::NativeCompanion;
use kotlin_model_readers::KotlinModelReaders;
use kotlin_wasm_program::KotlinWasmProgram;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod kotlin_browser_http_tests;
mod kotlin_composed_types;
mod kotlin_model_readers;
mod kotlin_type_members;
mod kotlin_type_models;
mod kotlin_type_values;
mod kotlin_wasm_program;

pub(crate) struct KotlinWebServices;

impl KotlinWebServices {
    const PRELUDE: &'static str = concat!(
        include_str!("prelude.kt"),
        include_str!("prelude_reads.kt"),
        include_str!("prelude_web.kt"),
        include_str!("prelude_web_tasks.kt")
    );

    const WEB_APIS: &'static str = concat!(
        include_str!("web_apis/Clock.kt"),
        include_str!("web_apis/SecureRandom.kt"),
        include_str!("web_apis/Connectivity.kt"),
        include_str!("web_apis/PageVisibility.kt"),
        include_str!("web_apis/Preferences.kt"),
        include_str!("web_apis/Clipboard.kt"),
        include_str!("web_apis/Share.kt"),
        include_str!("web_apis/Notifications.kt"),
        include_str!("web_apis/Geolocation.kt"),
        include_str!("web_apis/WindowControls.kt")
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
    const BROWSER_BUILTINS: &'static str = concat!(
        include_str!("kotlin_web_services/kotlin_storage.kt"),
        include_str!("kotlin_web_services/kotlin_browser_http.kt")
    );
    const PROGRAM_SOURCE: &'static str = "services.kt";
    const TASK_IMPORT: &'static str = "import kotlin.coroutines.startCoroutine\n";
    const MODEL_REALM: &'static str = "client";
    const MODEL_EXTENSION: &'static str = "kt";

    pub(crate) fn composed_types(
        services: &[(PathBuf, String)],
        files: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        kotlin_composed_types::KotlinComposedTypes::render(services, files)
    }

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
        let schemas = models
            .project_files()
            .iter()
            .map(|(path, source)| crate::annotation_layout::SchemaMarkers::contains(path, source))
            .collect::<Result<Vec<bool>, Failure>>()?;
        let texts: Vec<(String, String)> = model_files
            .iter()
            .zip(schemas)
            .filter(|(_, schema)| !schema)
            .map(|((path, text), _)| (path.to_string_lossy().into_owned(), text.clone()))
            .collect();
        let authored =
            crate::behavior::KotlinBuiltins::in_browser(authored, Self::BROWSER_BUILTINS);
        let program = format!(
            "{}{authored}\n{}{}{}{}{}",
            Self::TASK_IMPORT,
            crate::route_types::WebEventSources::kotlin(),
            KotlinModelReaders::text(&texts),
            Self::PRELUDE,
            Self::WEB_APIS,
            FilePickerSources::kotlin()
        );
        let mut sources = vec![(PathBuf::from(Self::PROGRAM_SOURCE), program)];
        sources.extend(model_files);
        sources
            .extend(Self::JVM_SHIMS.map(|(path, text)| (PathBuf::from(path), String::from(text))));
        Ok(sources)
    }
}
