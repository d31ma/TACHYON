use super::expression::Expression;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ValuePart<'a> {
    Text(&'a str),
    Value(&'a str),
}

impl<'a> ValuePart<'a> {
    fn value(&self) -> Option<&'a str> {
        match self {
            Self::Value(value) => Some(value),
            Self::Text(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BracedAttribute<'a> {
    name: &'a str,
    parts: Vec<ValuePart<'a>>,
}

impl<'a> BracedAttribute<'a> {
    pub(crate) const CODE: u16 = 1313;
    const PATH_MARKS: &'static [u8] = b"_$.";
    const COUNT_MARKS: &'static [u8] = b", ";
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
        let has_value = parts.iter().any(|part| part.value().is_some());
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
        let only = self.parts.first().filter(|_| self.parts.len() == 1);
        only.and_then(ValuePart::value)
    }

    pub(crate) fn is_boolean(&self) -> bool {
        Self::BOOLEAN_NAMES.contains(&self.name.to_ascii_lowercase().as_str())
    }

    fn expression(&self) -> String {
        self.whole_value()
            .map_or_else(|| self.joined_expression(), String::from)
    }

    fn joined_expression(&self) -> String {
        let pieces: Vec<String> = self.parts.iter().map(Self::piece).collect();
        pieces.join(" + ")
    }

    fn piece(part: &ValuePart<'_>) -> String {
        match part {
            ValuePart::Text(text) => Self::quoted(text),
            ValuePart::Value(value) if Self::is_path(value) => String::from(*value),
            ValuePart::Value(value) => format!("({value})"),
        }
    }

    fn quoted(text: &str) -> String {
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn is_path(value: &str) -> bool {
        value.bytes().all(|byte| {
            [
                byte.is_ascii_alphanumeric(),
                Self::PATH_MARKS.contains(&byte),
            ]
            .contains(&true)
        })
    }

    fn split(value: &'a str) -> Vec<ValuePart<'a>> {
        let mut parts = Vec::new();
        let mut text_start = 0;
        while let Some((open, close, inner)) = Self::next_value(value, text_start) {
            let text = (text_start < open).then(|| ValuePart::Text(&value[text_start..open]));
            parts.extend(text);
            parts.push(ValuePart::Value(inner));
            text_start = close + 1;
        }
        let rest = (text_start < value.len()).then(|| ValuePart::Text(&value[text_start..]));
        parts.extend(rest);
        parts
    }

    fn next_value(value: &'a str, from: usize) -> Option<(usize, usize, &'a str)> {
        value[from..]
            .match_indices('{')
            .find_map(|(offset, _)| Self::value_at(value, from + offset))
    }

    fn value_at(value: &'a str, open: usize) -> Option<(usize, usize, &'a str)> {
        let close = open + 1 + value[open + 1..].find('}')?;
        let inner = value[open + 1..close].trim();
        let checks = [
            Self::is_escaped(value, open),
            Self::is_template_value(inner),
        ];
        (checks == [false, true]).then_some((open, close, inner))
    }

    fn is_escaped(value: &str, open: usize) -> bool {
        let before = open.checked_sub(1);
        before.is_some_and(|before| value.as_bytes()[before] == b'\\')
    }

    fn is_template_value(inner: &str) -> bool {
        let is_count = inner
            .bytes()
            .all(|byte| [byte.is_ascii_digit(), Self::COUNT_MARKS.contains(&byte)].contains(&true));
        [is_count, Expression::parse(inner, "", 0).is_ok()] == [false, true]
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
