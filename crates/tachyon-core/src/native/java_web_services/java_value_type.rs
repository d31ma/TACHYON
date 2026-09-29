pub(super) struct JavaValueType {
    models: Vec<String>,
}

impl JavaValueType {
    const LISTS: [&'static str; 2] = ["List", "java.util.List"];
    const MAPS: [&'static str; 2] = ["Map", "java.util.Map"];
    const MAP_KEY: &'static str = "String,";
    const NOTHING: &'static str = "void";
    const MODEL_READERS: &'static str = "TacModels";
    const MODEL_READER_PREFIX: &'static str = "read";

    pub(super) fn new(models: Vec<String>) -> Self {
        Self { models }
    }

    pub(super) fn reading(&self, kind: &str, value: &str) -> Option<String> {
        let kind = Self::compact(kind);
        self.single_reading(&kind, value)
            .or_else(|| self.collection_reading(&kind, value, 0))
    }

    pub(super) fn model_reader(name: &str) -> String {
        format!("{}{name}", Self::MODEL_READER_PREFIX)
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

    fn single(&self, kind: &str) -> Option<(&'static str, String)> {
        let scalar = Self::scalar(kind).map(|reading| ("TacValues", String::from(reading)));
        let model = self.models.iter().any(|model| model == kind);
        scalar.or_else(|| model.then(|| (Self::MODEL_READERS, Self::model_reader(kind))))
    }

    fn single_reading(&self, kind: &str, value: &str) -> Option<String> {
        self.single(kind)
            .map(|(owner, reading)| format!("{owner}.{reading}({value})"))
    }

    fn function(&self, kind: &str, depth: usize) -> Option<String> {
        let item = format!("item{depth}");
        let collection = self.collection_reading(kind, &item, depth + 1);
        self.single(kind)
            .map(|(owner, reading)| format!("{owner}::{reading}"))
            .or_else(|| collection.map(|reading| format!("{item} -> {reading}")))
    }

    fn collection_reading(&self, kind: &str, value: &str, depth: usize) -> Option<String> {
        self.list_reading(kind, value, depth)
            .or_else(|| self.map_reading(kind, value, depth))
    }

    fn list_reading(&self, kind: &str, value: &str, depth: usize) -> Option<String> {
        let item = Self::generic_argument(kind, &Self::LISTS)?;
        let function = self.function(item, depth)?;
        Some(format!("TacValues.list({value}, {function})"))
    }

    fn map_reading(&self, kind: &str, value: &str, depth: usize) -> Option<String> {
        let arguments = Self::generic_argument(kind, &Self::MAPS)?;
        let item = arguments.strip_prefix(Self::MAP_KEY)?;
        let function = self.function(item, depth)?;
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

    fn plain() -> JavaValueType {
        JavaValueType::new(Vec::new())
    }

    #[test]
    fn plain_values_read_through_the_prelude() {
        assert_eq!(
            plain().reading("int", "value").as_deref(),
            Some("TacValues.integer(value)")
        );
        assert_eq!(
            plain().reading("String", "value").as_deref(),
            Some("TacValues.text(value)")
        );
    }

    #[test]
    fn models_read_through_the_generated_readers() {
        let values = JavaValueType::new(vec![String::from("Guide")]);
        assert_eq!(
            values.reading("Guide", "value").as_deref(),
            Some("TacModels.readGuide(value)")
        );
        assert_eq!(
            values.reading("List<Guide>", "value").as_deref(),
            Some("TacValues.list(value, TacModels::readGuide)")
        );
        assert_eq!(plain().reading("Guide", "value"), None);
    }

    #[test]
    fn collections_read_each_item() {
        assert_eq!(
            plain()
                .reading("List<Map<String, List<Long>>>", "value")
                .as_deref(),
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
            assert_eq!(plain().reading(kind, "value"), None, "{kind}");
        }
    }
}
