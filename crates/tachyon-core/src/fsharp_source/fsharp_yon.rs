use super::{FsharpMarkers, FsharpSource, FsharpTestUnit, FsharpView, LaterFeatures};
use crate::Failure;
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
        let lowered = crate::yon::lower_with_routes(view.path(), view.text(), models_root)
            .map_err(|failure| view.failure(&failure))?;
        Ok(lowered.map(|lowered| Self::program(view, contents, lowered)))
    }

    fn program(view: &FsharpView, contents: &str, lowered: Lowered) -> Lowered {
        let generated = lowered.source.get(view.text().len()..).unwrap_or_default();
        let csharp = format!("{}{generated}", view.usings());
        Lowered {
            source: FsharpSource::with_generated(&FsharpMarkers::strip(contents), &csharp),
            routes: lowered.routes,
            topics: lowered.topics,
        }
    }
}
