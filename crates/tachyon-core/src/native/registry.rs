use crate::Failure;
use crate::project::NativeCompanion;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct NativeCompanionInput {
    pub(super) snapshot_root: PathBuf,
    pub(super) language: NativeCompanion,
    pub(super) source: PathBuf,
    pub(super) service: String,
    pub(super) class: String,
    pub(super) web_assembly: bool,
}

impl NativeCompanionInput {
    pub(super) fn web_services(
        snapshot_root: &Path,
        language: NativeCompanion,
        services: &[(String, String, PathBuf)],
    ) -> Vec<Self> {
        services
            .iter()
            .map(|(service, class, source)| Self {
                snapshot_root: snapshot_root.to_path_buf(),
                language,
                source: source.clone(),
                service: service.clone(),
                class: class.clone(),
                web_assembly: true,
            })
            .collect()
    }
}

pub(super) fn source(
    companions: &[NativeCompanionInput],
    language: NativeCompanion,
) -> Result<Option<String>, Failure> {
    let mut selected = Vec::new();
    for item in companions {
        if item.language == language {
            selected.push(item);
        }
    }
    if selected.is_empty() {
        return Ok(None);
    }
    let mut imports = BTreeSet::new();
    let mut namespaces = String::new();
    let mut services = Vec::new();
    let mut fsharp = String::new();
    let mut runtimes = String::new();
    for (index, item) in selected.iter().enumerate() {
        let body = service_body(language, item, &mut imports)?;
        let namespace = format!("TachyonService{index}");
        let (body, fsharp_body) = crate::fsharp_source::FsharpSource::web_split(&body);
        let (fsharp_body, runtime) = crate::fsharp_source::FsharpSource::runtime_split(fsharp_body);
        namespaces.push_str(&namespace_source(language, &namespace, body));
        fsharp.push_str(&crate::fsharp_source::FsharpSource::web_namespace(
            &namespace,
            fsharp_body,
        ));
        runtimes.push_str(&crate::fsharp_source::FsharpSource::runtime_namespace(
            &namespace, runtime,
        ));
        services.push((item.service.clone(), namespace));
    }
    let models = models_source(language, &selected[0].snapshot_root, &mut imports)?;
    let mut result = imports.into_iter().collect::<Vec<_>>().join("\n");
    result.push('\n');
    result.push_str(&namespaces);
    result.push_str(&models);
    result.push_str(&dispatch(language, &services));
    let result = super::fsharp_web_project::FsharpWebProject::with_model_context(
        &result,
        &selected[0].snapshot_root,
        &fsharp,
    )?;
    Ok(Some(crate::fsharp_source::FsharpSource::web_body(
        &result,
        &crate::fsharp_source::FsharpSource::with_runtime(&fsharp, &runtimes),
    )))
}

fn models_source(
    language: NativeCompanion,
    snapshot_root: &Path,
    imports: &mut BTreeSet<String>,
) -> Result<String, Failure> {
    let extension = language_extension(language);
    let models = crate::model_sources::ModelSources::read(snapshot_root, "client", extension)?;
    let conversions = crate::route_types::Converters::tac_text(extension, snapshot_root)
        .map_err(|message| super::host::native_tool_failure(1605, &message))?;
    let source = match language {
        NativeCompanion::Rust => models.rust_module(),
        NativeCompanion::CSharp => models.csharp_namespaces(),
        NativeCompanion::Swift => body_without_imports(language, &models.joined_sources(), imports),
        NativeCompanion::Kotlin => String::new(),
    };
    Ok(format!("{source}{conversions}"))
}

fn language_extension(language: NativeCompanion) -> &'static str {
    match language {
        NativeCompanion::Swift => "swift",
        NativeCompanion::Kotlin => "kt",
        NativeCompanion::CSharp => "cs",
        NativeCompanion::Rust => "rs",
    }
}

fn service_body(
    language: NativeCompanion,
    item: &NativeCompanionInput,
    imports: &mut BTreeSet<String>,
) -> Result<String, Failure> {
    let captured = read_captured_source(&item.source)?;
    let captured = crate::without_bom(&captured);
    super::fsharp_service::FsharpService::body_or(item, captured, || {
        compiled_service_body(language, item, captured, imports)
    })
}

fn compiled_service_body(
    language: NativeCompanion,
    item: &NativeCompanionInput,
    captured: &str,
    imports: &mut BTreeSet<String>,
) -> Result<String, Failure> {
    let identity = service_identity(language, &item.class);
    let relative = item
        .source
        .strip_prefix(&item.snapshot_root)
        .unwrap_or(&identity);
    let mut roles = crate::behavior::ServiceRoles::read(relative, captured)?;
