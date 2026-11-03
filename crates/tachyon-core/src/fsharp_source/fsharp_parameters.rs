use super::{AttributeLine, FsharpType};

pub(crate) struct FsharpParameters;

impl FsharpParameters {
    const UNTYPED: &'static str = "object";

    pub(crate) fn read(after_name: &str) -> (String, &str) {
        let (parameters, rest) = Self::groups(after_name);
        (parameters.join(", "), rest)
    }

    fn groups(after_name: &str) -> (Vec<String>, &str) {
        let mut parameters = Vec::new();
        let mut rest = after_name.trim_start();
        while let Some((group, next)) = Self::next_group(rest) {
            parameters.extend(group);
            rest = next.trim_start();
        }
        (parameters, rest)
    }

    fn next_group(text: &str) -> Option<(Vec<String>, &str)> {
        if text.starts_with('(') {
            Self::parenthesized_group(text)
        } else if text.starts_with(char::is_alphabetic) {
            Some(Self::bare_group(text))
        } else {
            None
        }
    }

    fn parenthesized_group(text: &str) -> Option<(Vec<String>, &str)> {
        let closing = Self::closing_parenthesis(text)?;
        let inside = &text[1..closing];
        let parameters = FsharpType::split_top_level(inside, ",")
            .into_iter()
            .filter(|parameter| !parameter.is_empty())
            .map(Self::parameter)
            .collect();
        Some((parameters, &text[closing + 1..]))
    }

    pub(crate) fn closing(text: &str) -> Option<usize> {
        Self::closing_parenthesis(text)
    }

    fn closing_parenthesis(text: &str) -> Option<usize> {
        let mut depth = 0_i32;
        let mut closing = None;
        for (at, character) in text.char_indices() {
            depth += Self::depth_change(character);
            closing = closing.or(Self::closes_at(at, depth));
        }
        closing
    }

    fn closes_at(at: usize, depth: i32) -> Option<usize> {
        (depth == 0).then_some(at)
    }

    fn depth_change(character: char) -> i32 {
        match character {
            '(' => 1,
            ')' => -1,
            _ => 0,
        }
    }

    fn bare_group(text: &str) -> (Vec<String>, &str) {
        let length = text
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(text.len());
        let name = &text[..length];
        (vec![format!("{} {name}", Self::UNTYPED)], &text[length..])
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    pub(crate) fn parameter(text: &str) -> String {
        let (attributes, declaration) = Self::attributes(text.trim());
        let declaration = declaration.trim_start_matches('?');
        let declaration = Self::without_parentheses(declaration);
        let (name, kind) = declaration
            .split_once(':')
            .unwrap_or((declaration, Self::UNTYPED));
        format!("{attributes}{} {}", FsharpType::csharp(kind), name.trim())
    }

    fn without_parentheses(text: &str) -> &str {
        text.strip_prefix('(')
            .and_then(|inner| inner.strip_suffix(')'))
            .unwrap_or(text)
            .trim()
    }

    fn attributes(text: &str) -> (String, &str) {
        let mut attributes = String::new();
        let mut rest = text;
        while let Some((inside, next)) = Self::next_attribute(rest) {
            for attribute in AttributeLine::read(&format!("[<{inside}>]")) {
                attributes.push_str(&attribute.csharp);
                attributes.push(' ');
            }
            rest = next.trim_start();
        }
        (attributes, rest)
    }

    fn next_attribute(text: &str) -> Option<(&str, &str)> {
        let rest = text.strip_prefix("[<")?;
        let (inside, next) = rest.split_once(">]")?;
        Some((inside.trim(), next))
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpParameters;

    #[test]
    fn tupled_parameters_become_csharp_parameters() {
        let (parameters, rest) =
            FsharpParameters::read("(id: int, name: string) : Order = find id");
        assert_eq!(parameters, "int id, string name");
        assert_eq!(rest, ": Order = find id");
    }

    #[test]
    fn unit_curried_and_untyped_parameters() {
        assert_eq!(
            FsharpParameters::read("() : int = 1"),
            (String::new(), ": int = 1")
        );
        let (parameters, rest) = FsharpParameters::read("(a: int) (b: float) x = a");
        assert_eq!(parameters, "int a, double b, object x");
        assert_eq!(rest, "= a");
    }

    #[test]
    fn parameter_attributes_keep_their_arguments() {
        let (parameters, _) = FsharpParameters::read(
            "([<FromBody>] order: Order, [<PathVariable(\"id\")>] id: int, [<Check(typeof<OrderInput>)>] input: BehaviorJSON) =",
        );
        assert_eq!(
            parameters,
            "[FromBody] Order order, [PathVariable(\"id\")] int id, [Check(typeof(OrderInput))] BehaviorJSON input"
        );
    }
}
