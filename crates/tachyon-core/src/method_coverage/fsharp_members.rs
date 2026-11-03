use super::{PublicMethod, PublicMethods};
use crate::fsharp_source::FsharpView;

pub(crate) struct FsharpMembers;

impl FsharpMembers {
    const VIEW_LANGUAGE: &'static str = "cs";

    pub(crate) fn find(view: &FsharpView, masked_view: &str) -> Vec<PublicMethod> {
        PublicMethods::find(Self::VIEW_LANGUAGE, view.text(), masked_view)
            .into_iter()
            .filter_map(|method| Self::in_source(view, method))
            .collect()
    }

    fn in_source(view: &FsharpView, method: PublicMethod) -> Option<PublicMethod> {
        let line = view.source_line(method.line)?;
        Some(PublicMethod::new(method.name, line))
    }
}
