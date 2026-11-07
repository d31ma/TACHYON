use super::css_token::CssToken;
use std::collections::BTreeSet;

pub(super) struct FoundationProperties;

impl FoundationProperties {
    const MAX_NAME_BYTES: usize = 128;
    const NAME_PUNCTUATION: &'static [u8] = b"-_";
    const INHERITANCE: [&'static str; 2] = ["true", "false"];
    const SYNTAX: &'static [&'static str] = &[
        "*",
        "<color>",
        "<number>",
        "<integer>",
        "<length>",
        "<percentage>",
        "<length-percentage>",
        "<angle>",
        "<time>",
    ];

    pub(super) fn validate(
        header: &[CssToken<'_>],
        body: &[CssToken<'_>],
    ) -> Result<(), &'static str> {
        Self::validate_name(header)?;
        let mut descriptors = BTreeSet::new();
        body.split(|token| token.text == ";")
            .filter(|part| !part.is_empty())
            .try_for_each(|part| Self::descriptor(part, &mut descriptors))?;
        Self::require(
            descriptors.contains("syntax") & descriptors.contains("inherits"),
            "Custom property registrations require syntax and inherits descriptors.",
        )
    }

    fn validate_name(header: &[CssToken<'_>]) -> Result<(), &'static str> {
        let name = header.get(1).map_or("", |token| token.text);
        let valid = (header.len() == 2)
            & name.starts_with("--")
            & (name.len() <= Self::MAX_NAME_BYTES)
            & (name.len() > 2)
            & name.bytes().all(Self::name_character);
        Self::require(
            valid,
            "Foundation @property requires one plain bounded custom property name.",
        )
    }

    fn name_character(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() | Self::NAME_PUNCTUATION.contains(&byte)
    }

    fn descriptor<'a>(
        tokens: &'a [CssToken<'a>],
        names: &mut BTreeSet<&'a str>,
    ) -> Result<(), &'static str> {
        let property = tokens.first().map_or("", |token| token.text);
        Self::require(
            tokens.get(1).map(|token| token.text) == Some(":"),
            "Custom property descriptor needs a name and value.",
        )?;
        Self::require(
            names.insert(property),
            "Custom property descriptor is declared twice.",
        )?;
        let value: String = tokens[2..].iter().map(|token| token.text).collect();
        Self::check_value(property, &value)
    }

    fn check_value(property: &str, value: &str) -> Result<(), &'static str> {
        match property {
            "syntax" => Self::syntax(value),
            "inherits" => Self::require(
                Self::INHERITANCE.contains(&value),
                "Custom property inherits must be true or false.",
            ),
            "initial-value" => Self::require(
                Self::initial_value(value),
                "Custom property initial-value must be a bounded plain value without assets or nested rules.",
            ),
            _ => Err(
                "Foundation @property accepts only syntax, inherits and initial-value descriptors.",
            ),
        }
    }

    fn initial_value(value: &str) -> bool {
        !value.is_empty()
            & !value.contains(['{', '}', '@', '\\'])
            & !value.to_ascii_lowercase().contains("url(")
    }

    fn syntax(value: &str) -> Result<(), &'static str> {
        let syntax = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_default();
        Self::require(
            Self::SYNTAX.contains(&syntax),
            "Custom property syntax must use one supported simple CSS value type.",
        )
    }

    fn require(valid: bool, message: &'static str) -> Result<(), &'static str> {
        valid.then_some(()).ok_or(message)
    }
}
