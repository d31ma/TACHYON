use super::FsharpView;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct FsharpSpans;

impl FsharpSpans {
    pub(crate) fn restored<'p>(
        diagnostics: &[Diagnostic],
        files: impl Iterator<Item = (&'p Path, &'p [u8])>,
    ) -> Vec<Diagnostic> {
        let views: Vec<FsharpView> = files.filter_map(Self::view).collect();
        diagnostics
            .iter()
            .map(|diagnostic| Self::through_views(diagnostic, &views))
            .collect()
    }

    fn view((path, bytes): (&Path, &[u8])) -> Option<FsharpView> {
        FsharpView::of(path, std::str::from_utf8(bytes).ok()?)
    }

    fn through_views(diagnostic: &Diagnostic, views: &[FsharpView]) -> Diagnostic {
        let mut restored = diagnostic.clone();
        for view in views {
            restored = view.diagnostic(&restored);
        }
        restored
    }
}
