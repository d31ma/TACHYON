use super::{
    FieldNeeds, FsharpBehaviors, FsharpEvents, FsharpMarkers, FsharpSource, FsharpTestUnit,
    FsharpView,
};
use crate::Failure;
use crate::behavior::DetachedCsharp;
use crate::failure::{diagnostic, source_span};
use crate::yon::Lowered;
use std::path::Path;

pub(crate) struct FsharpYon;

impl FsharpYon {
    const BUILT_IN_ERRORS: &'static str = include_str!("../yon/built_in_errors/built_in_errors.cs");

    pub(crate) fn lower(
        source: &Path,
        contents: &str,
        models_root: Option<&Path>,
    ) -> Result<Option<Lowered>, Failure> {
        let view = FsharpView::new(source, contents);
        let unit = FsharpTestUnit::lower(source, contents, &view, "yon")?;
        unit.map_or_else(
            || Self::roles(source, contents, models_root, &view),
            |unit| Ok(Some(unit)),
        )
    }

    fn roles(
        source: &Path,
        contents: &str,
        models_root: Option<&Path>,
        view: &FsharpView,
    ) -> Result<Option<Lowered>, Failure> {
        FsharpSource::refuse_unsupported(view, source, contents)?;
        let detached = crate::behavior::detached_csharp(view.path(), view.text(), "yon")
            .map_err(|failure| view.failure(&failure))?;
        let lowered = crate::yon::lower_with_routes(view.path(), view.text(), models_root)
            .map_err(|failure| view.failure(&failure))?;
        lowered
            .map(|lowered| Self::program(view, source, contents, lowered, detached.as_ref()))
            .transpose()
    }

    fn program(
        view: &FsharpView,
        source: &Path,
        contents: &str,
        lowered: Lowered,
        detached: Option<&DetachedCsharp>,
    ) -> Result<Lowered, Failure> {
        let authored = detached.map_or(view.text().len(), |detached| detached.lowered.len());
        let (lowered_part, built_ins) =
            Self::without_built_in_errors(lowered.source.get(authored..).unwrap_or_default());
        let generated = FieldNeeds::constructed(contents, lowered_part);
        let stripped = FsharpMarkers::strip(contents);
        let behaviors = detached
            .map(|detached| Self::behaviors(view, source, &stripped, detached))
            .transpose()?;
        let (fsharp, setups, runtime) = behaviors.unwrap_or((stripped, "", ""));
        let events = FsharpEvents::detach(&generated);
        let csharp = format!("{}{}\n{setups}", view.usings(), events.glue);
        let runtime = Self::with_built_in_errors(&events.joined_runtime(runtime), built_ins);
        Ok(Lowered {
            source: FsharpSource::with_generated(
                &fsharp,
                &FsharpSource::with_runtime(&csharp, &runtime),
            ),
            routes: lowered.routes,
            topics: lowered.topics,
        })
    }

    fn without_built_in_errors(generated: &str) -> (&str, &str) {
        let Some(rest) = generated.strip_suffix(Self::BUILT_IN_ERRORS) else {
            return (generated, "");
        };
        (rest, Self::BUILT_IN_ERRORS)
    }

    fn with_built_in_errors(runtime: &str, built_ins: &str) -> String {
        let parts = [runtime, built_ins];
        let written: Vec<&str> = parts.into_iter().filter(|part| !part.is_empty()).collect();
        written.join("\n")
    }

    fn behaviors<'d>(
        view: &FsharpView,
        source: &Path,
        stripped: &str,
        detached: &'d DetachedCsharp,
    ) -> Result<(String, &'d str, &'d str), Failure> {
        let fsharp = FsharpBehaviors::program(view, stripped, detached)
            .map_err(|message| Self::failure(source, &message))?;
        Ok((fsharp, &detached.setups, &detached.runtime))
    }

    pub(super) fn failure(source: &Path, message: &str) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            1308,
            message,
            None,
            source_span(&portable, 0, 0),
        ))
    }
}
