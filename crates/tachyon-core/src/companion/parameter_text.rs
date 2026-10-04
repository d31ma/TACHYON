pub(super) struct ParameterText;

impl ParameterText {
    pub(super) fn csharp_parameters(after_name: &str) -> Option<Vec<&str>> {
        let inside = after_name.strip_prefix('(')?;
        let inside = &inside[..Self::closing(inside)?];
        let parameters = Self::top_level_parts(inside)
            .into_iter()
            .map(Self::without_attributes)
            .filter(|parameter| !parameter.is_empty())
            .collect();
        Some(parameters)
    }

    pub(super) fn without_annotations(parameter: &str) -> &str {
        let trimmed = parameter.trim_start();
        trimmed.strip_prefix('@').map_or(trimmed, |annotation| {
            Self::without_annotations(Self::after_annotation(annotation))
        })
    }

    fn after_annotation(annotation: &str) -> &str {
        let arguments = annotation.trim_start_matches(Self::is_name_character);
        let end = arguments.strip_prefix('(').map_or(0, |inside| {
            Self::closing(inside).map_or(arguments.len(), |at| at + 2)
        });
        &arguments[end..]
    }

    fn closing(inside: &str) -> Option<usize> {
        Self::depths(inside)
            .into_iter()
            .find(|(_, _, depth)| *depth < 0)
            .map(|(at, _, _)| at)
    }

    fn top_level_parts(inside: &str) -> Vec<&str> {
        let commas: Vec<usize> = Self::depths(inside)
            .into_iter()
            .filter(|(_, character, depth)| (*character, *depth) == (',', 0))
            .map(|(at, _, _)| at)
            .collect();
        let starts = std::iter::once(0).chain(commas.iter().map(|at| at + 1));
        let ends = commas.iter().copied().chain(std::iter::once(inside.len()));
        starts
            .zip(ends)
            .map(|(start, end)| inside[start..end].trim())
            .collect()
    }

    fn without_attributes(parameter: &str) -> &str {
        let start = Self::depths(parameter)
            .into_iter()
            .find(|entry| Self::starts_declaration(*entry))
            .map_or(parameter.len(), |(at, _, _)| at);
        &parameter[start..]
    }

    fn starts_declaration((_, character, depth): (usize, char, i32)) -> bool {
        (depth, Self::is_attribute_edge(character)) == (0, false)
    }

    fn is_attribute_edge(character: char) -> bool {
        character == ']' || character.is_whitespace()
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn depths(text: &str) -> Vec<(usize, char, i32)> {
        text.char_indices()
            .scan(0, |depth, (at, character)| {
                *depth += Self::depth_change(character);
                Some((at, character, *depth))
            })
            .collect()
    }

    fn depth_change(character: char) -> i32 {
        match character {
            '(' | '[' | '{' | '<' => 1,
            ')' | ']' | '}' | '>' => -1,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParameterText;

    #[test]
    fn csharp_attributes_with_json_arguments_leave_one_typed_parameter() {
        let parameters = ParameterText::csharp_parameters(
            "([PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})] BehaviorJSON input) {",
        );
        assert_eq!(parameters, Some(vec!["BehaviorJSON input"]));
    }

    #[test]
    fn csharp_generic_types_and_several_parameters_split_at_top_level_commas() {
        let parameters = ParameterText::csharp_parameters(
            "(Dictionary<string, int> counts, [FromBody] [Named(\"a\")] Order order)",
        );
        assert_eq!(
            parameters,
            Some(vec!["Dictionary<string, int> counts", "Order order"])
        );
        assert_eq!(ParameterText::csharp_parameters("()"), Some(Vec::new()));
    }

    #[test]
    fn name_first_annotations_with_json_arguments_leave_the_declaration() {
        assert_eq!(
            ParameterText::without_annotations(
                "@PageDefaults({\"defaultSize\":20,\"sort\":[\"id\"]}) @Named(\"x\") input: Any?"
            ),
            "input: Any?"
        );
        assert_eq!(
            ParameterText::without_annotations(" @FromBody order: Order"),
            "order: Order"
        );
    }
}
