use crate::yon::plan::RouteParameter;

pub(crate) struct PhpDoc;

impl PhpDoc {
    const ARRAY_TYPES: [&str; 3] = ["array", "?array", "iterable"];
    const LIST_NAMES: [&str; 3] = ["list", "array", "non-empty-list"];
    const NAME_SYMBOLS: &str = "_\\";
    const DECLARATION_STOPS: [char; 5] = [';', '{', '(', '=', '/'];

    pub(crate) fn read_types(
        documentation: &str,
        parameters: &mut [RouteParameter],
        returns: String,
    ) -> String {
        parameters
            .iter_mut()
            .filter(|parameter| Self::is_array(&parameter.type_text))
            .for_each(|parameter| Self::document_parameter(documentation, parameter));
        let documented =
            Self::tagged_type(documentation, "@return").filter(|_| Self::is_array(&returns));
        documented
            .map(|kind| Self::with_null(&returns, &kind))
            .unwrap_or(returns)
    }

    pub(crate) fn typed_arrays(source: &str) -> String {
        let mut output = String::with_capacity(source.len());
        let rest = Self::copy_typed_comments(source, &mut output);
        output.push_str(rest);
        output
    }

    pub(crate) fn list_item(type_text: &str) -> Option<&str> {
        let text = type_text.trim().trim_start_matches('?');
        let item = text
            .strip_suffix("[]")
            .or_else(|| Self::generic_list_item(text))?;
        Self::is_plain_name(item).then_some(item)
    }

    fn generic_list_item(text: &str) -> Option<&str> {
        let open = text.find('<')?;
        let list_name = &text[..open];
        let item = text[open + 1..].strip_suffix('>')?;
        Self::LIST_NAMES.contains(&list_name).then_some(item)
    }

    fn is_plain_name(item: &str) -> bool {
        !item.is_empty() && item.chars().all(Self::is_name_character)
    }

    fn is_name_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            Self::NAME_SYMBOLS.contains(character),
        ]
        .contains(&true)
    }

    fn is_array(type_text: &str) -> bool {
        Self::ARRAY_TYPES.contains(&type_text.trim())
    }

    fn document_parameter(documentation: &str, parameter: &mut RouteParameter) {
        let documented = Self::parameter_type(documentation, &parameter.code_name);
        if let Some(kind) = documented {
            parameter.type_text = Self::with_null(&parameter.type_text, &kind);
        }
    }

    fn with_null(declared: &str, kind: &str) -> String {
        let adds_null = [declared.trim().starts_with('?'), !kind.starts_with('?')] == [true, true];
        let nullable = adds_null.then(|| format!("?{kind}"));
        nullable.unwrap_or_else(|| kind.to_owned())
    }

    fn copy_typed_comments<'a>(source: &'a str, output: &mut String) -> &'a str {
        let mut rest = source;
        while let Some(next) = Self::copy_typed_comment(rest, output) {
            rest = next;
        }
        rest
    }

    fn copy_typed_comment<'a>(rest: &'a str, output: &mut String) -> Option<&'a str> {
        let start = rest.find("/**")?;
        let length = rest[start..].find("*/")?;
        let end = start + length + 2;
        let comment = &rest[start..end];
        output.push_str(&rest[..end]);
        let rest = &rest[end..];
        let typed = Self::typed_declaration(comment, rest);
        Some(typed.map_or(rest, |(declaration, dollar)| {
            output.push_str(&declaration);
            &rest[dollar..]
        }))
    }

    fn typed_declaration(comment: &str, rest: &str) -> Option<(String, usize)> {
        let kind = Self::tagged_type(comment, "@var")?;
        let dollar = rest.find('$')?;
        let declaration = &rest[..dollar];
        let is_plain = !declaration.contains(Self::DECLARATION_STOPS);
        is_plain.then(|| (Self::with_array_replaced(declaration, &kind), dollar))
    }

    fn with_array_replaced(declaration: &str, kind: &str) -> String {
        let words: Vec<String> = declaration
            .split(' ')
            .map(|word| Self::with_word_replaced(word, kind))
            .collect();
        words.join(" ")
    }

    fn with_word_replaced(word: &str, kind: &str) -> String {
        match word.trim() {
            "array" => word.replace("array", kind),
            "?array" => word.replace("?array", &format!("?{kind}")),
            _ => word.to_owned(),
        }
    }

    fn parameter_type(documentation: &str, name: &str) -> Option<String> {
        let variable = format!("${name}");
        Self::tag_lines(documentation, "@param")
            .into_iter()
            .find_map(|rest| Self::type_of_variable(rest, &variable))
    }

    fn type_of_variable(rest: &str, variable: &str) -> Option<String> {
        let kind = Self::type_at(rest);
        let after = rest[kind.len()..].split_whitespace().next();
        (after == Some(variable)).then(|| kind.to_owned())
    }

    fn tagged_type(documentation: &str, tag: &str) -> Option<String> {
        let rest = Self::tag_lines(documentation, tag).into_iter().next()?;
        let kind = Self::type_at(rest);
        (!kind.is_empty()).then(|| kind.to_owned())
    }

    fn tag_lines<'a>(documentation: &'a str, tag: &str) -> Vec<&'a str> {
        documentation
            .lines()
            .filter_map(|line| Self::tag_text(line, tag))
            .collect()
    }

    fn tag_text<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
        let line = line
            .trim()
            .trim_start_matches("/**")
            .trim_start_matches('*')
            .trim();
        let rest = line.strip_prefix(tag)?;
        let is_tag = rest.starts_with(char::is_whitespace);
        is_tag.then(|| rest.trim().trim_end_matches("*/").trim())
    }

    fn type_at(text: &str) -> &str {
        let mut depth = 0_i32;
        let end = text
            .char_indices()
            .find(|&(_, character)| Self::ends_type(character, &mut depth));
        end.map_or(text, |(at, _)| &text[..at])
    }

    fn ends_type(character: char, depth: &mut i32) -> bool {
        match character {
            '<' | '(' | '{' => *depth += 1,
            '>' | ')' | '}' => *depth -= 1,
            _ if character.is_whitespace() => return *depth <= 0,
            _ => {}
        }
        false
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
