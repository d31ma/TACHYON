use super::css_token::CssToken;

pub(super) struct FoundationSelectors;

impl FoundationSelectors {
    const MOTION: [&'static str; 5] = ["*", "::before", "::after", "*::before", "*::after"];
    const ROOTS: [&'static str; 3] = [":root", "html", "body"];
    const RESET: [&'static str; 8] = [
        "*",
        "::before",
        "::after",
        "*::before",
        "*::after",
        "*::backdrop",
        "::backdrop",
        "[hidden]",
    ];
    const ELEMENTS: &'static [&'static str] = &[
        "html",
        "body",
        "a",
        "abbr",
        "address",
        "article",
        "aside",
        "audio",
        "b",
        "blockquote",
        "button",
        "canvas",
        "caption",
        "code",
        "dd",
        "details",
        "dialog",
        "div",
        "dl",
        "dt",
        "em",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "header",
        "hr",
        "i",
        "img",
        "input",
        "label",
        "legend",
        "li",
        "main",
        "menu",
        "nav",
        "ol",
        "optgroup",
        "option",
        "p",
        "pre",
        "progress",
        "q",
        "s",
        "section",
        "select",
        "small",
        "span",
        "strong",
        "sub",
        "summary",
        "sup",
        "svg",
        "table",
        "tbody",
        "td",
        "textarea",
        "th",
        "thead",
        "time",
        "tr",
        "u",
        "ul",
        "video",
    ];

    pub(super) fn validate(tokens: &[CssToken<'_>]) -> Result<(), &'static str> {
        tokens
            .split(|token| token.text == ",")
            .try_for_each(Self::validate_one)
    }

    pub(super) fn universal_reset(tokens: &[CssToken<'_>]) -> bool {
        tokens
            .split(|token| token.text == ",")
            .all(Self::motion_selector)
    }

    fn motion_selector(tokens: &[CssToken<'_>]) -> bool {
        let selector: String = tokens.iter().map(|token| token.text).collect();
        Self::MOTION.contains(&selector.as_str()) & Self::valid_spacing(tokens)
    }

    fn validate_one(tokens: &[CssToken<'_>]) -> Result<(), &'static str> {
        let compact: String = tokens.iter().map(|token| token.text).collect();
        let allowed = Self::ROOTS.contains(&compact.as_str())
            | Self::RESET.contains(&compact.as_str())
            | Self::ELEMENTS.contains(&compact.as_str())
            | Self::theme_attribute(&compact);
        Self::require(allowed & Self::valid_spacing(tokens))
    }

    fn valid_spacing(tokens: &[CssToken<'_>]) -> bool {
        let outside = tokens
            .split(|token| token.text == "[")
            .next()
            .unwrap_or_default();
        outside.windows(2).all(|pair| pair[0].end == pair[1].start)
    }

    fn theme_attribute(selector: &str) -> bool {
        selector.split_once('[').is_some_and(Self::root_attribute)
    }

    fn root_attribute((root, attribute): (&str, &str)) -> bool {
        Self::ROOTS.contains(&root) & Self::valid_attribute(attribute)
    }

    fn valid_attribute(attribute: &str) -> bool {
        let Some(value) = attribute.strip_suffix(']') else {
            return false;
        };
        !value.contains(['[', ']', '\\']) & value.contains('=')
    }

    fn require(allowed: bool) -> Result<(), &'static str> {
        if !allowed {
            return Err(
                "Global foundation selectors must be root/html/body theme attributes, universal resets or plain HTML element defaults; move application selectors beside their component HTML.",
            );
        }
        Ok(())
    }
}
