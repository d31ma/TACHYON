pub(super) struct CsharpParameterList;

impl CsharpParameterList {
    pub(super) fn parameters(after_name: &str) -> Option<Vec<&str>> {
        let inside = after_name.strip_prefix('(')?;
        let inside = &inside[..Self::closing(inside)?];
        let parameters = Self::top_level_parts(inside)
            .into_iter()
            .map(Self::without_attributes)
            .filter(|parameter| !parameter.is_empty())
            .collect();
        Some(parameters)
    }

    fn closing(inside: &str) -> Option<usize> {
        let mut depth = 0_i32;
        for (at, character) in inside.char_indices() {
            depth += Self::depth_change(character);
            if depth < 0 {
                return Some(at);
            }
        }
        None
    }

    fn top_level_parts(inside: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0_i32;
        let mut start = 0;
        for (at, character) in inside.char_indices() {
            depth += Self::depth_change(character);
            if character == ',' && depth == 0 {
                parts.push(&inside[start..at]);
                start = at + 1;
            }
        }
        parts.push(&inside[start..]);
        parts
    }

    fn without_attributes(parameter: &str) -> &str {
        let mut rest = parameter.trim();
        while rest.starts_with('[') {
            let Some(end) = Self::attribute_end(rest) else {
                return rest;
            };
            rest = rest[end + 1..].trim_start();
        }
        rest
    }

    fn attribute_end(attribute: &str) -> Option<usize> {
        let mut depth = 0_i32;
        for (at, character) in attribute.char_indices() {
            depth += Self::depth_change(character);
            if depth == 0 {
                return Some(at);
            }
        }
        None
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
    use super::CsharpParameterList;

    #[test]
    fn attributes_with_json_arguments_leave_one_typed_parameter() {
        let parameters = CsharpParameterList::parameters(
            "([PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})] BehaviorJSON input) {",
        );
        assert_eq!(parameters, Some(vec!["BehaviorJSON input"]));
    }

    #[test]
    fn generic_types_and_several_parameters_split_at_top_level_commas() {
        let parameters = CsharpParameterList::parameters(
            "(Dictionary<string, int> counts, [FromBody] Order order)",
        );
        assert_eq!(
            parameters,
            Some(vec!["Dictionary<string, int> counts", "Order order"])
        );
        assert_eq!(CsharpParameterList::parameters("()"), Some(Vec::new()));
    }
}
