mod attribute_line;
mod behavior_body;
mod block_kind;
mod field_need;
mod field_needs;
mod fsharp_behaviors;
mod fsharp_body;
mod fsharp_constructor_props;
mod fsharp_events;
mod fsharp_lexer;
mod fsharp_lines;
mod fsharp_markers;
mod fsharp_parameters;
mod fsharp_project;
mod fsharp_shadow;
mod fsharp_spans;
mod fsharp_style;
mod fsharp_test_unit;
mod fsharp_type;
mod fsharp_value;
mod fsharp_view;
mod fsharp_yon;
mod later_features;
mod line_kind;
mod member_header;
mod member_line;
mod member_words;
mod open_block;
mod record_fields;
mod shadow_writer;
mod source_line;
mod type_form;
mod type_header;
mod type_name;

use attribute_line::AttributeLine;
use behavior_body::BehaviorBody;
use block_kind::BlockKind;
use field_need::FieldNeed;
pub(crate) use field_needs::FieldNeeds;
pub(crate) use fsharp_behaviors::FsharpBehaviors;
use fsharp_body::FsharpBody;
pub(crate) use fsharp_constructor_props::FsharpConstructorProps;
use fsharp_events::FsharpEvents;
pub(crate) use fsharp_lexer::FsharpLexer;
pub(crate) use fsharp_lines::FsharpLines;
pub(crate) use fsharp_markers::FsharpMarkers;
use fsharp_parameters::FsharpParameters;
pub(crate) use fsharp_project::FsharpProject;
pub(crate) use fsharp_shadow::FsharpShadow;
pub(crate) use fsharp_spans::FsharpSpans;
pub(crate) use fsharp_style::FsharpStyle;
pub(crate) use fsharp_test_unit::FsharpTestUnit;
use fsharp_type::FsharpType;
use fsharp_value::FsharpValue;
pub(crate) use fsharp_view::FsharpView;
pub(crate) use fsharp_yon::FsharpYon;
use later_features::LaterFeatures;
use line_kind::LineKind;
use member_header::MemberHeader;
use member_line::MemberLine;
use member_words::MemberWords;
use open_block::OpenBlock;
use record_fields::RecordFields;
use shadow_writer::ShadowWriter;
use source_line::SourceLine;
use type_form::TypeForm;
use type_header::TypeHeader;
use type_name::TypeName;

use std::path::{Path, PathBuf};

pub(crate) struct FsharpSource;

impl FsharpSource {
    pub(crate) const EXTENSION: &'static str = "fs";
    const SYNTAX: [(&'static str, &'static str); 1] = [(Self::EXTENSION, "cs")];
    const GENERATED_PART: &'static str = "\n//__YON_CSHARP__\n";
    const WEB_PART: &'static str = "\n//__TAC_FSHARP__\n";
    const RUNTIME_PART: &'static str = "\n//__FSHARP_BEHAVIOR_RUNTIME__\n";
    const MODELS_NAMESPACE: &'static str = "namespace rec Models\n";
    const DOTNET_MODELS: [&'static str; 2] = ["cs", Self::EXTENSION];

    pub(crate) fn is_fsharp(source: &Path) -> bool {
        source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(Self::EXTENSION))
    }

    pub(crate) fn model_languages(extension: &str) -> Vec<&str> {
        if extension == Self::DOTNET_MODELS[0] {
            return Self::DOTNET_MODELS.to_vec();
        }
        vec![extension]
    }

    pub(crate) fn syntax(extension: &str) -> &str {
        Self::SYNTAX
            .iter()
            .find(|(language, _)| *language == extension)
            .map_or(extension, |(_, syntax)| syntax)
    }

    pub(crate) fn refuse_unsupported(
        view: &FsharpView,
        source: &Path,
        contents: &str,
    ) -> Result<(), crate::Failure> {
        FieldNeeds::refuse(source, contents)?;
        LaterFeatures::refuse(view, source, contents)
    }

    pub(crate) fn with_generated(fsharp: &str, csharp: &str) -> String {
        format!("{fsharp}{}{csharp}", Self::GENERATED_PART)
    }

    pub(crate) fn web_body(csharp: &str, fsharp: &str) -> String {
        let joined = (!fsharp.is_empty()).then(|| format!("{csharp}{}{fsharp}", Self::WEB_PART));
        joined.unwrap_or_else(|| csharp.to_owned())
    }

    pub(crate) fn web_split(text: &str) -> (&str, &str) {
        text.split_once(Self::WEB_PART).unwrap_or((text, ""))
    }

    pub(crate) fn web_namespace(namespace: &str, fsharp: &str) -> String {
        let written =
            (!fsharp.is_empty()).then(|| format!("namespace rec {namespace}\n{fsharp}\n"));
        written.unwrap_or_default()
    }

    pub(crate) fn models_file(models: &[(PathBuf, String)]) -> String {
        Self::models_text(models)
    }

    fn models_text(models: &[(PathBuf, String)]) -> String {
        let mut file = String::from(Self::MODELS_NAMESPACE);
        for (path, source) in models {
            file.push_str(&FsharpLines::located(path, source));
            file.push('\n');
        }
        file
    }

    pub(crate) fn with_runtime(csharp: &str, runtime: &str) -> String {
        let joined =
            (!runtime.is_empty()).then(|| format!("{csharp}{}{runtime}", Self::RUNTIME_PART));
        joined.unwrap_or_else(|| csharp.to_owned())
    }

    pub(crate) fn runtime_namespace(namespace: &str, runtime: &str) -> String {
        let written =
            (!runtime.is_empty()).then(|| format!("namespace {namespace} {{\n{runtime}\n}}\n"));
        written.unwrap_or_default()
    }

    pub(crate) fn runtime_split(csharp: &str) -> (&str, &str) {
        csharp
            .split_once(Self::RUNTIME_PART)
            .unwrap_or((csharp, ""))
    }

    pub(crate) fn split(program: &str) -> (&str, &str) {
        program
            .split_once(Self::GENERATED_PART)
            .unwrap_or((program, ""))
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpSource;
    use std::path::Path;

    #[test]
    fn the_generated_csharp_follows_the_authored_fsharp() {
        let program =
            FsharpSource::with_generated("type A() = class end", "sealed class Handler {}");
        assert_eq!(
            FsharpSource::split(&program),
            ("type A() = class end", "sealed class Handler {}")
        );
        assert_eq!(FsharpSource::split("type A() = class end").1, "");
    }

    #[test]
    fn csharp_reads_the_fsharp_models_of_its_realm_too() {
        assert_eq!(FsharpSource::model_languages("cs"), ["cs", "fs"]);
        assert_eq!(FsharpSource::model_languages("kt"), ["kt"]);
    }

    #[test]
    fn only_fs_files_are_fsharp() {
        assert!(FsharpSource::is_fsharp(Path::new(
            "server/controllers/A.fs"
        )));
        assert!(!FsharpSource::is_fsharp(Path::new(
            "server/controllers/A.cs"
        )));
    }
}
