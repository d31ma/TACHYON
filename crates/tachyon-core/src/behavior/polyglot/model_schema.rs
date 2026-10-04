use super::{Input, Prepared};
use crate::Failure;
use std::path::Path;

pub(crate) struct ModelSchema;

impl ModelSchema {
    const NAMES: [&'static str; 9] = crate::annotation_layout::SchemaMarkers::NAMES;
    const CLIENT_LANGUAGES: [&'static str; 12] = [
        "java", "js", "ts", "py", "cs", "fs", "rs", "swift", "kt", "go", "dart", "rb",
    ];

    pub(crate) fn plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        crate::lua_source::LuaView::of(path, source).map_or_else(
            || Self::other_plain(path, source),
            |view| Self::lua_plain(&view, path, source),
        )
    }

    fn other_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        crate::fsharp_source::FsharpView::of(path, source).map_or_else(
            || Self::native_plain(path, source),
            |view| Self::fsharp_plain(&view, source),
        )
    }

    pub(crate) fn require_realm(path: &Path, source: &str, schema: bool) -> Result<(), Failure> {
        let unsupported = ![
            schema,
            path.starts_with("client/models"),
            !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| Self::CLIENT_LANGUAGES.contains(&extension)),
        ]
        .contains(&false);
        unsupported
            .then(|| Self::client_failure(path, source))
            .map_or(Ok(()), Err)
    }

    fn client_failure(path: &Path, source: &str) -> Failure {
        let at = Self::marker_offset(path, source).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(
            1310, "This client model schema needs its Tac metadata adapter",
            Some("Plain client models remain supported. Tac reads schema models in JavaScript, TypeScript, Python, Java, C#, F#, Rust, Swift, Kotlin, Go, Dart and Ruby (ADR0092).".to_owned()),
            crate::failure::source_span(&path.to_string_lossy(), at, at + 1),
        ))
    }

    fn marker_offset(path: &Path, source: &str) -> Option<usize> {
        let code = crate::annotation_layout::masked_source(path, source).ok()?;
        Self::NAMES
            .iter()
            .flat_map(|name| {
                [
                    format!("@{name}"),
                    format!("[{name}"),
                    format!("[<{name}"),
                    format!("@{}", name.to_lowercase()),
                    format!("@yon::{name}"),
                    format!("@tac::{name}"),
                    format!("[yon::{name}"),
                    format!("[tac::{name}"),
                ]
            })
            .filter_map(|marker| code.find(&marker))
            .min()
    }

    fn lua_plain(
        view: &crate::lua_source::LuaView,
        path: &Path,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        let plain =
            Self::native_plain(view.path(), view.text()).map_err(|error| view.failure(&error))?;
        plain.map_or(Ok(None), |_| Err(Self::lua_failure(path, source)))
    }

    fn lua_failure(path: &Path, source: &str) -> Failure {
        let at = Self::marker_offset(path, source).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(1310,
            "Lua model schemas need the model metadata adapter",
            Some("Plain Lua models and Converter roles remain supported. Lua schema models are outside the affected Yon adapter qualification in ADR0092.".to_owned()),
            crate::failure::source_span(&path.to_string_lossy(), at, at + 1)))
    }

    fn fsharp_plain(
        view: &crate::fsharp_source::FsharpView,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        let plain =
            Self::native_plain(view.path(), view.text()).map_err(|error| view.failure(&error))?;
        Ok(plain.map(|_| crate::fsharp_source::FsharpMarkers::strip(source)))
    }

    fn native_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let project = Self::native_projector(extension);
        project(path, source)
    }

    fn native_projector(extension: &str) -> fn(&Path, &str) -> Result<Option<String>, Failure> {
        match extension {
            "java" => Self::java_plain,
            "js" | "ts" => Self::script_plain,
            _ => Self::polyglot_plain,
        }
    }

    fn script_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let mask = crate::lexical::code_mask(extension, source);
        let input = Input {
            path,
            source,
            code: super::super::visible_code(source, &mask.code),
        };
        crate::annotation_layout::SchemaMarkers::contains(path, source)?
            .then(|| Self::script_marked(&input))
            .transpose()?
            .map_or(Ok(None), Ok)
    }

    fn script_marked(input: &Input<'_>) -> Result<Option<String>, Failure> {
        let markers = super::super::markers(input)?;
        let schema = markers
            .iter()
            .any(|marker| Self::NAMES.contains(&marker.name.as_str()));
        schema
            .then(|| Self::script_projection(input, &markers))
            .transpose()
    }

    fn script_projection(input: &Input<'_>, markers: &[super::Marker]) -> Result<String, Failure> {
        Self::require_schema(input, markers)?;
        let environment = ["yon", "tac"][usize::from(input.path.starts_with("client"))];
        super::super::lower(input.path, input.source, environment)?;
        Ok(super::erase(input.source, markers))
    }

    fn polyglot_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let language = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let mask = crate::lexical::code_mask(language, source);
        let language = crate::groovy_source::GroovySource::syntax(language);
        let input = Input {
            path,
            source,
            code: super::visible_code(source, mask.code),
        };
        let environment = ["yon", "tac"][usize::from(path.starts_with("client"))];
        crate::annotation_layout::SchemaMarkers::contains(path, source)?
            .then(|| Self::polyglot_projection(&input, language, environment))
            .transpose()
    }

    fn polyglot_projection(
        input: &Input<'_>,
        language: &str,
        environment: &str,
    ) -> Result<String, Failure> {
        let markers = super::annotations(input, language, environment)?;
        Self::project(input, &markers, environment)
    }

    fn project(
        input: &Input<'_>,
        markers: &[super::Marker],
        environment: &str,
    ) -> Result<String, Failure> {
        Self::require_schema(input, markers)?;
        let prepared = super::prepare(input.path, input.source, environment)?;
        Self::projection(prepared).map_err(|message| input.failure(0, message))
    }

    fn java_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let plain = crate::model_layout::JavaModelSchema::blanked(path, source)?;
        Ok((plain != source).then_some(plain))
    }

    fn require_schema(input: &Input<'_>, markers: &[super::Marker]) -> Result<(), Failure> {
        let refused = markers
            .iter()
            .find(|marker| !Self::NAMES.contains(&marker.name.as_str()));
        refused.map_or(Ok(()), |marker| {
            Err(Failure::one(crate::failure::diagnostic(
                1310,
                "A model carries only declarative settings and validation metadata",
                Some(
                    "Move behavior and role annotations into the appropriate role folder."
                        .to_owned(),
                ),
                crate::failure::source_span(
                    &input.path.to_string_lossy(),
                    marker.span.start,
                    marker.span.end,
                ),
            )))
        })
    }

    fn projection(prepared: Option<Prepared>) -> Result<String, String> {
        match prepared {
            Some(Prepared::Annotated(module)) => super::render(&module, Vec::new(), ""),
            Some(Prepared::Erased(source)) => Ok(source),
            None => Err("Model schema metadata was not consumed".to_owned()),
        }
    }
}
