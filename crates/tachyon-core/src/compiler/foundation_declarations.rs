use super::css_token::CssToken;
use super::foundation_colors::FoundationColors;
use super::foundation_priority::FoundationPriority;
use super::foundation_values::FoundationValues;

pub(super) struct FoundationDeclarations;

impl FoundationDeclarations {
    const THEME: &'static [&'static str] = &[
        "color",
        "background-color",
        "font",
        "font-family",
        "font-size",
        "font-style",
        "font-weight",
        "font-stretch",
        "font-feature-settings",
        "font-variation-settings",
        "line-height",
        "letter-spacing",
        "word-spacing",
        "color-scheme",
        "accent-color",
        "caret-color",
        "text-rendering",
        "text-size-adjust",
        "-webkit-text-size-adjust",
        "-webkit-font-smoothing",
        "-moz-osx-font-smoothing",
    ];
    const PROPERTY_PUNCTUATION: &'static [u8] = b"-_";
    const BORDER_STYLES: [&'static str; 2] = ["solid", "none"];
    const BORDER_COLORS: [&'static str; 2] = ["currentColor", "transparent"];
    const ZERO: [&'static str; 6] = ["0", "0px", "0rem", "0em", "0s", "0ms"];
    const RESET: &'static [&'static str] = &[
        "margin",
        "margin-block",
        "margin-inline",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "padding",
        "padding-block",
        "padding-inline",
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "border-width",
        "border",
        "outline",
    ];

    pub(super) fn validate(
        tokens: &[CssToken<'_>],
        motion_priority: bool,
    ) -> Result<(), &'static str> {
        tokens
            .split(|token| token.text == ";")
            .filter(|part| !part.is_empty())
            .try_for_each(|part| Self::validate_one(part, motion_priority))
    }

    fn validate_one(tokens: &[CssToken<'_>], motion_priority: bool) -> Result<(), &'static str> {
        let colon = tokens
            .iter()
            .position(|token| token.text == ":")
            .ok_or("Foundation declaration needs a property and value.")?;
        let property: String = tokens[..colon].iter().map(|token| token.text).collect();
        let tokens = FoundationPriority::value(&tokens[colon + 1..], &property, motion_priority)?;
        let value: String = tokens.iter().map(|token| token.text).collect();
        Self::validate_value(&property, &value, tokens)
    }

    fn validate_value(
        property: &str,
        value: &str,
        tokens: &[CssToken<'_>],
    ) -> Result<(), &'static str> {
        Self::require(
            Self::plain_property(property),
            "Escaped or malformed foundation property names are unsupported.",
        )?;
        Self::require(
            !value.is_empty(),
            "Foundation declaration needs a nonempty value.",
        )?;
        FoundationValues::validate(tokens)?;
        let allowed = property.starts_with("--")
            | Self::THEME.contains(&property)
            | Self::reset_value(property, value, tokens);
        Self::require(
            allowed,
            "Global foundation declarations permit colors, inherited theme variables, font defaults and narrow resets; move layout/position/grid/flex rules beside component HTML.",
        )
    }

    fn plain_property(property: &str) -> bool {
        !property.is_empty() & property.bytes().all(Self::property_character)
    }

    fn property_character(byte: u8) -> bool {
        byte.is_ascii_lowercase()
            | byte.is_ascii_digit()
            | Self::PROPERTY_PUNCTUATION.contains(&byte)
    }

    fn reset_value(property: &str, value: &str, tokens: &[CssToken<'_>]) -> bool {
        let duration = Self::duration_reset(value);
        Self::reset_rule(property, value, tokens, duration)
    }

    fn duration_reset(value: &str) -> bool {
        Self::ZERO.contains(&value) || value == "0.01ms"
    }

    fn reset_rule(property: &str, value: &str, tokens: &[CssToken<'_>], duration: bool) -> bool {
        match property {
            "box-sizing" => value == "border-box",
            "background" => FoundationColors::background(tokens),
            "border-style" => Self::BORDER_STYLES.contains(&value),
            "border-color" => Self::BORDER_COLORS.contains(&value),
            "list-style" | "appearance" | "-webkit-appearance" => value == "none",
            "vertical-align" => value == "baseline",
            "animation-duration" | "transition-duration" => duration,
            "animation-iteration-count" => value == "1",
            "scroll-behavior" => value == "auto",
            _ => Self::RESET.contains(&property) & Self::ZERO.contains(&value),
        }
    }

    fn require(allowed: bool, message: &'static str) -> Result<(), &'static str> {
        allowed.then_some(()).ok_or(message)
    }
}
