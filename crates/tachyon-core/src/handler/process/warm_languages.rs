use crate::handler::HandlerSource;
use crate::handler::readiness::YonLanguage;
use std::path::Path;

pub(super) struct WarmLanguages;

impl WarmLanguages {
    const ACCEPTED: [YonLanguage; 3] = [
        YonLanguage::JavaScript,
        YonLanguage::TypeScript,
        YonLanguage::Python,
    ];

    pub(super) fn accepts(source: &HandlerSource) -> bool {
        let language = YonLanguage::from_path(Path::new(source.relative_path()));
        language.is_some_and(|language| Self::ACCEPTED.contains(&language))
    }
}
