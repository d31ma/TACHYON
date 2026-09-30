pub(super) struct TypeScriptFieldType<'a> {
    text: &'a str,
}

impl<'a> TypeScriptFieldType<'a> {
    const REQUIRED: char = '!';
    const OPTIONAL: char = '?';
    const LIST: &'static str = "[]";
    const PLAIN_TYPES: [(&'static str, &'static str); 5] = [
        ("string", "string"),
        ("integer", "number"),
        ("number", "number"),
        ("boolean", "boolean"),
        ("any", "unknown"),
    ];

    pub(super) const fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub(super) fn is_required(&self) -> bool {
        self.text.starts_with(Self::REQUIRED)
    }

    pub(super) fn type_text(&self) -> String {
        let shape = self.text.trim_start_matches(Self::REQUIRED);
        Self::shape_text(shape)
    }

    fn shape_text(shape: &str) -> String {
        if let Some(inner) = shape.strip_prefix(Self::OPTIONAL) {
            return format!("{} | null", Self::shape_text(inner));
        }
        if let Some(item) = shape.strip_prefix(Self::LIST) {
            return Self::list_text(item);
        }
        Self::PLAIN_TYPES
            .iter()
            .find(|(written, _)| *written == shape)
            .map_or_else(|| String::from(shape), |(_, typed)| String::from(*typed))
    }

    fn list_text(item: &str) -> String {
        let item_text = Self::shape_text(item);
        if item.starts_with(Self::OPTIONAL) {
            return format!("({item_text})[]");
        }
        format!("{item_text}[]")
    }
}

#[cfg(test)]
#[path = "typescript_field_type_tests.rs"]
mod tests;
