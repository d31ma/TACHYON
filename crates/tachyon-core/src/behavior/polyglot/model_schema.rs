use super::{Input, Prepared};
use crate::Failure;
use std::path::Path;

pub(crate) struct ModelSchema;

impl ModelSchema {
    const NAMES: [&'static str; 9] = [
        "Settings",
        "Setting",
        "Checked",
        "Required",
        "NotEmpty",
        "Length",
        "Min",
        "Max",
        "Description",
    ];

    pub(crate) fn plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        crate::lua_source::LuaView::of(path, source)
            .map(|view| Self::lua_plain(&view, path, source))
            .unwrap_or_else(|| Self::other_plain(path, source))
    }

    fn other_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        crate::fsharp_source::FsharpView::of(path, source)
            .map(|view| Self::fsharp_plain(&view, source))
            .unwrap_or_else(|| Self::native_plain(path, source))
    }

    pub(crate) fn require_realm(path: &Path, source: &str, schema: bool) -> Result<(), Failure> {
        let unsupported = ![
            schema,
            path.starts_with("client/models"),
            !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| ["java", "js", "ts"].contains(&extension)),
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
            Some("Plain client models remain supported. Java Checked model metadata is qualified; other client schema adapters remain pending under ADR0092.".to_owned()),
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
        plain
            .map(|_| Err(Self::lua_failure(path, source)))
            .unwrap_or(Ok(None))
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
        let java = path
            .extension()
            .is_some_and(|extension| extension == "java");
        java.then(|| Self::java_plain(path, source))
            .unwrap_or_else(|| Self::non_java_plain(path, source))
    }

    fn non_java_plain(path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let script = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| ["js", "ts"].contains(&value));
        script
            .then(|| Self::script_plain(path, source))
            .unwrap_or_else(|| Self::polyglot_plain(path, source))
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
        let markers = super::super::markers(&input)?;
        let schema = markers
            .iter()
            .any(|marker| Self::NAMES.contains(&marker.name.as_str()));
        schema
            .then(|| Self::script_projection(&input, &markers))
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
        let markers = super::annotations(&input, language, environment)?;
        let schema = markers
            .iter()
            .any(|marker| Self::NAMES.contains(&marker.name.as_str()));
        schema
            .then(|| Self::project(&input, &markers, environment))
            .transpose()
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
