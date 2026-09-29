use super::{
    FsharpBehaviors, FsharpEvents, FsharpMarkers, FsharpSource, FsharpTestUnit, FsharpView,
    LaterFeatures,
};
use crate::Failure;
use crate::behavior::DetachedCsharp;
use crate::failure::{diagnostic, source_span};
use crate::yon::Lowered;
use std::path::Path;

pub(crate) struct FsharpYon;

impl FsharpYon {
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
        LaterFeatures::refuse(view, source, contents)?;
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
        let generated = lowered.source.get(authored..).unwrap_or_default();
        let stripped = FsharpMarkers::strip(contents);
        let behaviors = detached
            .map(|detached| Self::behaviors(view, source, &stripped, detached))
            .transpose()?;
        let (fsharp, setups, runtime) = behaviors.unwrap_or((stripped, "", ""));
        let events = FsharpEvents::detach(generated);
        let csharp = format!("{}{}\n{setups}", view.usings(), events.glue);
        Ok(Lowered {
            source: FsharpSource::with_generated(
                &fsharp,
                &FsharpSource::with_runtime(&csharp, &events.joined_runtime(runtime)),
            ),
            routes: lowered.routes,
            topics: lowered.topics,
        })
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
