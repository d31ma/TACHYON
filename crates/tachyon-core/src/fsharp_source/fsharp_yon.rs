use super::{
    FsharpBehaviors, FsharpMarkers, FsharpSource, FsharpTestUnit, FsharpView, LaterFeatures,
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
        LaterFeatures::refuse(view, source, contents, "yon")?;
        let detached = crate::behavior::detached_csharp(view.path(), view.text(), "yon")
            .map_err(|failure| view.failure(&failure))?;
        let lowered = crate::yon::lower_with_routes(view.path(), view.text(), models_root)
            .map_err(|failure| view.failure(&failure))?;
        let Some(lowered) = lowered else {
            return Ok(None);
        };
        Self::program(view, source, contents, lowered, detached.as_ref()).map(Some)
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
        let (fsharp, setups, runtime) = match detached {
            Some(detached) => (
                FsharpBehaviors::program(view, &stripped, detached)
                    .map_err(|message| Self::failure(source, &message))?,
                detached.setups.as_str(),
                detached.runtime.as_str(),
            ),
            None => (stripped, "", ""),
        };
        let csharp = format!("{}{generated}\n{setups}", view.usings());
        Ok(Lowered {
            source: FsharpSource::with_generated(
                &fsharp,
                &FsharpSource::with_runtime(&csharp, runtime),
            ),
            routes: lowered.routes,
            topics: lowered.topics,
        })
    }

    fn failure(source: &Path, message: &str) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            1308,
            message,
            None,
            source_span(&portable, 0, 0),
        ))
    }
}
