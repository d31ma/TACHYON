use super::expression::Expression;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ValuePart<'a> {
    Text(&'a str),
    Value(&'a str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BracedAttribute<'a> {
    name: &'a str,
    parts: Vec<ValuePart<'a>>,
}

impl<'a> BracedAttribute<'a> {
    pub(crate) const CODE: u16 = 1313;
    const BOOLEAN_NAMES: &'static [&'static str] = &[
        "allowfullscreen",
        "async",
        "autofocus",
        "autoplay",
        "checked",
        "controls",
        "default",
        "defer",
        "disabled",
        "formnovalidate",
        "hidden",
        "inert",
        "ismap",
        "itemscope",
        "loop",
        "multiple",
        "muted",
        "nomodule",
        "novalidate",
        "open",
        "playsinline",
        "readonly",
        "required",
        "reversed",
        "selected",
    ];

    pub(crate) fn find(name: &'a str, value: &'a str) -> Option<Self> {
        let parts = Self::split(value);
        let has_value = parts.iter().any(|part| matches!(part, ValuePart::Value(_)));
        has_value.then_some(Self { name, parts })
    }

    pub(crate) fn message(&self) -> String {
        format!(
            "The attribute {} uses braces, but braces work only in text.",
            self.name
        )
    }

    pub(crate) fn help(&self) -> String {
        let name = self.name;
        if self.is_boolean() {
            return format!(
                "Write :{name}=\"condition\" with a condition that is true or false. When the condition is false, Tachyon leaves the attribute out."
            );
        }
        let expression = self.expression();
        if self.whole_value().is_some() {
            return format!("Write :{name}=\"{expression}\" to set it from a value.");
        }
        format!("Write :{name}=\"{expression}\" to join the text and the values.")
    }

    pub(crate) fn whole_value(&self) -> Option<&'a str> {
        match self.parts.as_slice() {
            [ValuePart::Value(value)] => Some(value),
            _ => None,
        }
    }

    pub(crate) fn is_boolean(&self) -> bool {
        Self::BOOLEAN_NAMES.contains(&self.name.to_ascii_lowercase().as_str())
    }

    fn expression(&self) -> String {
        if let Some(value) = self.whole_value() {
            return String::from(value);
        }
        let pieces: Vec<String> = self
            .parts
            .iter()
            .map(|part| match part {
                ValuePart::Text(text) => Self::quoted(text),
                ValuePart::Value(value) if Self::is_path(value) => String::from(*value),
                ValuePart::Value(value) => format!("({value})"),
            })
            .collect();
        pieces.join(" + ")
    }

    fn quoted(text: &str) -> String {
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn is_path(value: &str) -> bool {
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.'))
    }

    fn split(value: &'a str) -> Vec<ValuePart<'a>> {
        let mut parts = Vec::new();
        let mut text_start = 0;
        let mut search = 0;
        while let Some(offset) = value[search..].find('{') {
            let open = search + offset;
            let Some(length) = value[open + 1..].find('}') else {
                break;
            };
            let close = open + 1 + length;
            let inner = value[open + 1..close].trim();
            search = open + 1;
            if Self::is_escaped(value, open) || !Self::is_template_value(inner) {
                continue;
            }
            if text_start < open {
                parts.push(ValuePart::Text(&value[text_start..open]));
            }
            parts.push(ValuePart::Value(inner));
            search = close + 1;
            text_start = search;
        }
        if text_start < value.len() {
            parts.push(ValuePart::Text(&value[text_start..]));
        }
        parts
    }

    fn is_escaped(value: &str, open: usize) -> bool {
        open > 0 && value.as_bytes()[open - 1] == b'\\'
    }

    fn is_template_value(inner: &str) -> bool {
        let is_count = inner
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b',' | b' '));
        !is_count && Expression::parse(inner, "", 0).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::BracedAttribute;

    fn help(name: &str, value: &str) -> Option<String> {
        BracedAttribute::find(name, value).map(|braced| braced.help())
    }

    #[test]
    fn whole_values_and_mixed_text_suggest_a_dynamic_attribute() {
        assert_eq!(
            help("title", "{name}").as_deref(),
            Some("Write :title=\"name\" to set it from a value.")
        );
        assert_eq!(
            help("class", "card {kind}").as_deref(),
            Some("Write :class=\"'card ' + kind\" to join the text and the values.")
        );
        assert_eq!(
            help("aria-label", "It's {count + 1} of {total}").as_deref(),
            Some(
                "Write :aria-label=\"'It\\'s ' + (count + 1) + ' of ' + total\" to join the text and the values."
            )
        );
    }

    #[test]
    fn boolean_attributes_explain_that_false_leaves_the_attribute_out() {
        let help = help("hidden", "{drawer.open ? '' : 'hidden'}").unwrap_or_default();
        assert!(help.starts_with("Write :hidden=\"condition\""), "{help}");
        assert!(
            help.contains("false, Tachyon leaves the attribute out"),
            "{help}"
        );
    }

    #[test]
    fn literal_braces_are_not_template_values() {
        for (name, value) in [
            ("pattern", "[0-9]{3}"),
            ("pattern", "\\d{2,4}"),
            ("data-config", "{\"theme\": \"dark\"}"),
            ("title", "\\{name}"),
            ("title", "{}"),
            ("title", "open { only"),
            ("style", "color: red"),
        ] {
            assert_eq!(BracedAttribute::find(name, value), None, "{name}={value}");
        }
    }
}
