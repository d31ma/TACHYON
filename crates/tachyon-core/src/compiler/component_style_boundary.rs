use super::css_rules::CssRules;
use super::css_token::CssToken;
use super::css_tokens::CssTokens;
use crate::Failure;
use crate::failure::{diagnostic, source_span};

pub(super) struct ComponentStyleBoundary;

impl ComponentStyleBoundary {
    const CODE: u16 = 1403;
    const HELP: &'static str = "Keep tac.css blocks, strings and comments closed within this file; the compiler wraps the complete component stylesheet in its owned @scope.";

    pub(super) fn validate(path: &str, source: &str) -> Result<(), Failure> {
        let tokens =
            CssTokens::read(source).map_err(|detail| Self::failure(path, source.len(), detail))?;
        Self::validate_rules(path, source.len(), &tokens)
    }

    fn validate_rules(path: &str, length: usize, tokens: &[CssToken<'_>]) -> Result<(), Failure> {
        CssRules::read(tokens)
            .map(|_| ())
            .map_err(|detail| Self::failure(path, length, detail))
    }

    fn failure(path: &str, length: usize, detail: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            format!("Cannot scope component stylesheet '{path}': {detail}"),
            Some(String::from(Self::HELP)),
            source_span(path, 0, length),
        ))
    }
}
