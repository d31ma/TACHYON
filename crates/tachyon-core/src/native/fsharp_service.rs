use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::companion::CompanionLanguage;
use crate::fsharp_source::{FsharpMarkers, FsharpSource, FsharpView};

pub(super) struct FsharpService;

impl FsharpService {
    const NO_REQUIRED: &'static str = "internal static class TacServiceInfo { internal static readonly string[] Required = { }; }\n";

    pub(super) fn body_or(
        item: &NativeCompanionInput,
        authored: &str,
        other: impl FnOnce() -> Result<String, Failure>,
    ) -> Result<String, Failure> {
        Some(item)
            .filter(|item| FsharpSource::is_fsharp(&item.source))
            .map_or_else(other, |item| Self::body(item, authored))
    }

    fn body(item: &NativeCompanionInput, authored: &str) -> Result<String, Failure> {
        let relative = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&item.source);
        let view = FsharpView::new(relative, authored);
        FsharpSource::refuse_later_features(&view, relative, authored)?;
        let shadow_path = view.path().to_string_lossy().replace('\\', "/");
        let table = crate::companion::owner_table(
            CompanionLanguage::CSharp,
            view.text(),
            &shadow_path,
            &[],
            "",
            false,
        )
        .map_err(|failure| view.failure(&failure))?;
        let csharp = format!("{table}{}", Self::NO_REQUIRED);
        Ok(FsharpSource::web_body(
            &csharp,
            &FsharpMarkers::strip(authored),
        ))
    }
}
