use super::CapturedFile;
use crate::Failure;
use crate::annotation_rules::AnnotationRules;
use crate::compiler::PortablePath;
use crate::failure::{diagnostic, source_span};
use std::ffi::OsStr;

pub(super) struct RoleSource;

impl RoleSource {
    const MODULE_MANIFESTS: [&str; 2] = ["tac.modules.json", "yon.modules.json"];
    const SERVER_ROOT: &str = "server";
    const YON_VIEW: &str = "yon.html";

    pub(super) fn is_module_manifest(name: &str) -> bool {
        Self::MODULE_MANIFESTS.contains(&name)
    }

    pub(super) fn validate(file: &CapturedFile) -> Result<(), Failure> {
        crate::annotation_layout::reject_reserved_folder(&file.relative)?;
        let name = file.relative.file_name().and_then(OsStr::to_str);
        Self::reject_server_view(file, name)?;
        Self::reject_module_manifest(file, name)?;
        Self::annotated_text(file).map_or(Ok(()), |source| {
            crate::annotation_layout::validate(&file.relative, source)
        })
    }

    fn reject_server_view(file: &CapturedFile, name: Option<&str>) -> Result<(), Failure> {
        if name == Some(Self::YON_VIEW) && file.relative.starts_with(Self::SERVER_ROOT) {
            return Err(Failure::one(diagnostic(
                1008,
                "Yon is REST-only and does not render HTML templates.",
                Some(String::from(
                    "Use client/pages/**/tac.html for views, or return HTML with Content-Type: text/html from a RequestRoute controller.",
                )),
                source_span(&PortablePath::of(&file.relative), 0, 0),
            )));
        }
        Ok(())
    }

    fn reject_module_manifest(file: &CapturedFile, name: Option<&str>) -> Result<(), Failure> {
        if name.is_some_and(Self::is_module_manifest) {
            return Err(Failure::one(diagnostic(
                2101,
                "Annotation module manifests are no longer supported.",
                Some("Remove the manifest and place annotated same-language role files in plural role folders beneath their Tac or Yon entry directory.".into()),
                source_span(&PortablePath::of(&file.relative), 0, 0),
            )));
        }
        Ok(())
    }

    fn is_annotated_language(extension: &str) -> bool {
        AnnotationRules::LANGUAGES.contains(&extension)
            || extension == crate::fsharp_source::FsharpSource::EXTENSION
    }

    fn annotated_text(file: &CapturedFile) -> Option<&str> {
        file.relative
            .extension()
            .and_then(OsStr::to_str)
            .filter(|extension| Self::is_annotated_language(extension))?;
        std::str::from_utf8(&file.bytes).ok()
    }
}
