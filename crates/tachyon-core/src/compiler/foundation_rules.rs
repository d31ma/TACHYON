use super::css_problem::CssProblem;
use super::css_rule::CssRule;
use super::css_rules::CssRules;
use super::css_token::CssToken;
use super::foundation_declarations::FoundationDeclarations;
use super::foundation_properties::FoundationProperties;
use super::foundation_selectors::FoundationSelectors;

pub(super) struct FoundationRules;

impl FoundationRules {
    const REDUCED_MOTION: &'static str = "@media(prefers-reduced-motion:reduce)";
    const CHARSETS: [&'static str; 2] = ["\"UTF-8\"", "\"utf-8\""];
    const LAYER_PUNCTUATION: &'static [u8] = b"_-.";
    const GROUPS: [&'static str; 3] = ["@layer", "@media", "@supports"];

    pub(super) fn validate(tokens: &[CssToken<'_>]) -> Result<(), CssProblem> {
        Self::validate_context(tokens, false)
    }

    fn validate_context(tokens: &[CssToken<'_>], reduced_motion: bool) -> Result<(), CssProblem> {
        let rules = CssRules::read(tokens).map_err(|message| CssProblem {
            position: 0,
            message,
        })?;
        rules
            .iter()
            .try_for_each(|rule| Self::validate_rule(rule, reduced_motion))
    }

    fn validate_rule(rule: &CssRule<'_>, reduced_motion: bool) -> Result<(), CssProblem> {
        let grouped = Self::group_body(rule);
        grouped.map_or_else(
            || {
                Self::ordinary_rule(rule, reduced_motion).map_err(|message| CssProblem {
                    position: rule.start,
                    message,
                })
            },
            |body| Self::validate_group(rule, body, reduced_motion),
        )
    }

    fn validate_group(
        rule: &CssRule<'_>,
        body: &[CssToken<'_>],
        inherited: bool,
    ) -> Result<(), CssProblem> {
        let header: String = rule.header.iter().map(|token| token.text).collect();
        let reduced_motion = inherited | (header.to_ascii_lowercase() == Self::REDUCED_MOTION);
        Self::validate_context(body, reduced_motion)
    }

    fn group_body<'a>(rule: &'a CssRule<'a>) -> Option<&'a [CssToken<'a>]> {
        let first = rule.header.first().map_or("", |token| token.text);
        rule.body.filter(|_| Self::GROUPS.contains(&first))
    }

    fn ordinary_rule(rule: &CssRule<'_>, reduced_motion: bool) -> Result<(), &'static str> {
        let first = rule.header.first().map_or("", |token| token.text);
        match (first, rule.body) {
            ("@import", None) => Ok(()),
            ("@charset", None) => Self::charset(rule.header),
            ("@layer", None) => Self::layer_names(rule.header),
            ("@property", Some(body)) => FoundationProperties::validate(rule.header, body),
            (name, _) if name.starts_with('@') => Err(
                "Unsupported global foundation at-rule; retain only audited import/layer/media/supports/property groups and move application animation/layout to a component.",
            ),
            (_, Some(body)) => Self::validate_style(rule.header, body, reduced_motion),
            _ => Err("Foundation stylesheet requires ordinary selector blocks."),
        }
    }

    fn validate_style(
        header: &[CssToken<'_>],
        body: &[CssToken<'_>],
        reduced_motion: bool,
    ) -> Result<(), &'static str> {
        FoundationSelectors::validate(header)?;
        let priority = reduced_motion & FoundationSelectors::universal_reset(header);
        FoundationDeclarations::validate(body, priority)
    }

    fn charset(header: &[CssToken<'_>]) -> Result<(), &'static str> {
        let value = header.get(1).map_or("", |token| token.text);
        let valid = (header.len() == 2) & Self::CHARSETS.contains(&value);
        valid
            .then_some(())
            .ok_or("Only UTF-8 charset declarations are supported.")
    }

    fn layer_names(header: &[CssToken<'_>]) -> Result<(), &'static str> {
        let valid = header[1..].iter().all(Self::layer_token);
        valid
            .then_some(())
            .ok_or("Layer declarations require plain local layer names.")
    }

    fn layer_token(token: &CssToken<'_>) -> bool {
        (token.text == ",") | token.text.bytes().all(Self::layer_character)
    }

    fn layer_character(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() | Self::LAYER_PUNCTUATION.contains(&byte)
    }
}
