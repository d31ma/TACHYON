pub(super) struct JavaValueType;

impl JavaValueType {
    const LISTS: [&'static str; 2] = ["List", "java.util.List"];
    const MAPS: [&'static str; 2] = ["Map", "java.util.Map"];
    const MAP_KEY: &'static str = "String,";
    const NOTHING: &'static str = "void";

    pub(super) fn reading(kind: &str, value: &str) -> Option<String> {
        let kind = Self::compact(kind);
        Self::scalar_reading(&kind, value).or_else(|| Self::collection_reading(&kind, value, 0))
    }

    pub(super) fn is_nothing(kind: &str) -> bool {
        kind.trim() == Self::NOTHING
    }

    fn compact(kind: &str) -> String {
        kind.split_whitespace().collect()
    }

    fn scalar(kind: &str) -> Option<&'static str> {
        let reading = match kind {
            "int" => "integer",
            "long" => "whole",
            "double" => "decimal",
            "float" => "single",
            "boolean" => "truth",
            "Integer" => "optionalInteger",
            "Long" => "optionalWhole",
            "Double" => "optionalDecimal",
            "Float" => "optionalSingle",
            "Boolean" => "optionalTruth",
            "String" => "text",
            "Object" => "same",
            _ => return None,
        };
        Some(reading)
    }

    fn scalar_reading(kind: &str, value: &str) -> Option<String> {
        Self::scalar(kind).map(|reading| format!("TacValues.{reading}({value})"))
    }

    fn function(kind: &str, depth: usize) -> Option<String> {
        let item = format!("item{depth}");
        let collection = Self::collection_reading(kind, &item, depth + 1);
        Self::scalar(kind)
            .map(|reading| format!("TacValues::{reading}"))
            .or_else(|| collection.map(|reading| format!("{item} -> {reading}")))
    }

    fn collection_reading(kind: &str, value: &str, depth: usize) -> Option<String> {
        Self::list_reading(kind, value, depth).or_else(|| Self::map_reading(kind, value, depth))
    }

    fn list_reading(kind: &str, value: &str, depth: usize) -> Option<String> {
        let item = Self::generic_argument(kind, &Self::LISTS)?;
        let function = Self::function(item, depth)?;
        Some(format!("TacValues.list({value}, {function})"))
    }

    fn map_reading(kind: &str, value: &str, depth: usize) -> Option<String> {
        let arguments = Self::generic_argument(kind, &Self::MAPS)?;
        let item = arguments.strip_prefix(Self::MAP_KEY)?;
        let function = Self::function(item, depth)?;
        Some(format!("TacValues.map({value}, {function})"))
    }

    fn generic_argument<'a>(kind: &'a str, names: &[&str]) -> Option<&'a str> {
        names.iter().find_map(|name| {
            kind.strip_prefix(name)?
                .strip_prefix('<')?
                .strip_suffix('>')
        })
    }
}

#[cfg(test)]
mod tests {
    use super::JavaValueType;

    #[test]
    fn plain_values_read_through_the_prelude() {
        assert_eq!(
            JavaValueType::reading("int", "value").as_deref(),
            Some("TacValues.integer(value)")
        );
        assert_eq!(
            JavaValueType::reading("String", "value").as_deref(),
            Some("TacValues.text(value)")
        );
    }

    #[test]
    fn collections_read_each_item() {
        assert_eq!(
            JavaValueType::reading("List<Map<String, List<Long>>>", "value").as_deref(),
            Some(
                "TacValues.list(value, item0 -> TacValues.map(item0, item1 -> TacValues.list(item1, TacValues::optionalWhole)))"
            )
        );
    }

    #[test]
    fn other_types_are_not_read() {
        for kind in [
            "char",
            "int[]",
            "Map<Integer, String>",
            "Basket",
            "List<? extends Number>",
        ] {
            assert_eq!(JavaValueType::reading(kind, "value"), None, "{kind}");
        }
    }
}
