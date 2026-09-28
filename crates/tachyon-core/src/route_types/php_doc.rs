use crate::yon::plan::RouteParameter;

pub(crate) struct PhpDoc;

impl PhpDoc {
    const ARRAY_TYPES: [&str; 3] = ["array", "?array", "iterable"];
    const LIST_NAMES: [&str; 3] = ["list", "array", "non-empty-list"];

    pub(crate) fn read_types(
        documentation: &str,
        parameters: &mut [RouteParameter],
        returns: String,
    ) -> String {
        for parameter in parameters.iter_mut() {
            if !Self::is_array(&parameter.type_text) {
                continue;
            }
            if let Some(kind) = Self::parameter_type(documentation, &parameter.code_name) {
                parameter.type_text = Self::with_null(&parameter.type_text, &kind);
            }
        }
        match Self::tagged_type(documentation, "@return") {
            Some(kind) if Self::is_array(&returns) => Self::with_null(&returns, &kind),
            _ => returns,
        }
    }

    pub(crate) fn typed_arrays(source: &str) -> String {
        let mut output = String::with_capacity(source.len());
        let mut rest = source;
        while let Some(start) = rest.find("/**") {
            let Some(length) = rest[start..].find("*/") else {
                break;
            };
            let end = start + length + 2;
            let comment = &rest[start..end];
            output.push_str(&rest[..end]);
            rest = &rest[end..];
            let Some(kind) = Self::tagged_type(comment, "@var") else {
                continue;
            };
            let Some(dollar) = rest.find('$') else {
                continue;
            };
            let declaration = &rest[..dollar];
            if declaration.contains([';', '{', '(', '=', '/']) {
                continue;
            }
            output.push_str(&Self::with_array_replaced(declaration, &kind));
            rest = &rest[dollar..];
        }
        output.push_str(rest);
        output
    }

    pub(crate) fn list_item(type_text: &str) -> Option<&str> {
        let text = type_text.trim().trim_start_matches('?');
        let item = if let Some(item) = text.strip_suffix("[]") {
            item
        } else {
            let open = text.find('<')?;
            if !Self::LIST_NAMES.contains(&&text[..open]) {
                return None;
            }
            text[open + 1..].strip_suffix('>')?
        };
        let plain = !item.is_empty()
            && item
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_\\".contains(character));
        plain.then_some(item)
    }

    fn is_array(type_text: &str) -> bool {
        Self::ARRAY_TYPES.contains(&type_text.trim())
    }

    fn with_null(declared: &str, kind: &str) -> String {
        if declared.trim().starts_with('?') && !kind.starts_with('?') {
            return format!("?{kind}");
        }
        kind.to_owned()
    }

    fn with_array_replaced(declaration: &str, kind: &str) -> String {
        let mut words = Vec::new();
        for word in declaration.split(' ') {
            let replaced = match word.trim() {
                "array" => word.replace("array", kind),
                "?array" => word.replace("?array", &format!("?{kind}")),
                _ => word.to_owned(),
            };
            words.push(replaced);
        }
        words.join(" ")
    }

    fn parameter_type(documentation: &str, name: &str) -> Option<String> {
        let variable = format!("${name}");
        for rest in Self::tag_lines(documentation, "@param") {
            let kind = Self::type_at(rest);
            let after = rest[kind.len()..].split_whitespace().next();
            if after == Some(variable.as_str()) {
                return Some(kind.to_owned());
            }
        }
        None
    }

    fn tagged_type(documentation: &str, tag: &str) -> Option<String> {
        let rest = Self::tag_lines(documentation, tag).into_iter().next()?;
        let kind = Self::type_at(rest);
        (!kind.is_empty()).then(|| kind.to_owned())
    }

    fn tag_lines<'a>(documentation: &'a str, tag: &str) -> Vec<&'a str> {
        let mut found = Vec::new();
        for line in documentation.lines() {
            let line = line
                .trim()
                .trim_start_matches("/**")
                .trim_start_matches('*')
                .trim();
            let Some(rest) = line.strip_prefix(tag) else {
                continue;
            };
            if rest.starts_with(char::is_whitespace) {
                found.push(rest.trim().trim_end_matches("*/").trim());
            }
        }
        found
    }

    fn type_at(text: &str) -> &str {
        let mut depth = 0_i32;
        for (at, character) in text.char_indices() {
            match character {
                '<' | '(' | '{' => depth += 1,
                '>' | ')' | '}' => depth -= 1,
                _ if character.is_whitespace() && depth <= 0 => return &text[..at],
                _ => {}
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::PhpDoc;

    #[test]
    fn a_var_comment_types_an_array_property() {
        let source = "class Basket {\n    public function __construct(\n        /** @var list<Line> */\n        public array $lines,\n        public ?array $old = null,\n    ) {}\n}\n";
        let typed = PhpDoc::typed_arrays(source);
        assert!(typed.contains("public list<Line> $lines"), "{typed}");
        assert!(typed.contains("public ?array $old"), "{typed}");
    }

    #[test]
    fn a_list_item_is_a_plain_class_name() {
        assert_eq!(PhpDoc::list_item("list<Line>"), Some("Line"));
        assert_eq!(PhpDoc::list_item("?Line[]"), Some("Line"));
        assert_eq!(PhpDoc::list_item("array<int, Line>"), None);
        assert_eq!(PhpDoc::list_item("Line"), None);
    }
}
