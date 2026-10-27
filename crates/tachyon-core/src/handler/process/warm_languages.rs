use crate::handler::HandlerSource;
use crate::handler::readiness::YonLanguage;
use std::path::Path;

pub(super) struct WarmLanguages;

impl WarmLanguages {
    const ACCEPTED: [YonLanguage; 15] = [
        YonLanguage::JavaScript,
        YonLanguage::TypeScript,
        YonLanguage::Python,
        YonLanguage::Ruby,
        YonLanguage::Go,
        YonLanguage::Lua,
        YonLanguage::Cpp,
        YonLanguage::Rust,
        YonLanguage::Kotlin,
        YonLanguage::Php,
        YonLanguage::CSharp,
        YonLanguage::FSharp,
        YonLanguage::Dart,
        YonLanguage::Java,
        YonLanguage::Groovy,
    ];

    pub(super) fn accepts(source: &HandlerSource) -> bool {
        let language = YonLanguage::from_path(Path::new(source.relative_path()));
        language.is_some_and(|language| Self::ACCEPTED.contains(&language))
    }
}
