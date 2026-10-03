use super::css_token::CssToken;

pub(super) struct FoundationPriority;

impl FoundationPriority {
    const DURATIONS: [&'static str; 4] = ["0", "0s", "0ms", "0.01ms"];
    const ITERATION_COUNT: &'static str = "animation-iteration-count";
    const SCROLL: &'static str = "scroll-behavior";

    pub(super) fn value<'a>(
        tokens: &'a [CssToken<'a>],
        property: &str,
        allowed: bool,
    ) -> Result<&'a [CssToken<'a>], &'static str> {
        let end = Self::value_end(tokens);
        let value = &tokens[..end];
        Self::check_priority(tokens.len() != end, allowed, property, value)?;
        Ok(value)
    }

    fn value_end(tokens: &[CssToken<'_>]) -> usize {
        let important = tokens
            .last()
            .is_some_and(|token| token.text.eq_ignore_ascii_case("important"));
        let marker = tokens
            .get(tokens.len().saturating_sub(2))
            .is_some_and(|token| token.text == "!");
        match important & marker {
            true => tokens.len() - 2,
            false => tokens.len(),
        }
    }

    fn check_priority(
        priority: bool,
        allowed: bool,
        property: &str,
        tokens: &[CssToken<'_>],
    ) -> Result<(), &'static str> {
        let value = Self::motion_value(property, tokens);
        let valid = !priority | (allowed & value);
        valid.then_some(()).ok_or("Foundation !important priority is limited to exact universal reduced-motion duration, iteration-count and scroll resets inside @media (prefers-reduced-motion: reduce).")
    }

    fn motion_value(property: &str, tokens: &[CssToken<'_>]) -> bool {
        let value: String = tokens.iter().map(|token| token.text).collect();
        match property {
            "animation-duration" | "transition-duration" => {
                Self::DURATIONS.contains(&value.as_str())
            }
            Self::ITERATION_COUNT => value == "1",
            Self::SCROLL => value == "auto",
            _ => false,
        }
    }
}
