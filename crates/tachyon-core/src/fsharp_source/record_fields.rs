use super::FsharpType;

pub(crate) struct RecordFields;

impl RecordFields {
    const MUTABLE: &'static str = "mutable ";

    pub(crate) fn csharp(text: &str) -> String {
        let inside = text.trim().trim_start_matches('{').trim_end_matches('}');
        let fields: Vec<String> = FsharpType::split_top_level(inside, ";")
            .into_iter()
            .flat_map(|part| part.split('\n'))
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(Self::field)
            .collect();
        fields.join(" ")
    }

    fn field(text: &str) -> String {
        let writable = text.starts_with(Self::MUTABLE);
        let declaration = text.trim_start_matches(Self::MUTABLE);
        let (name, kind) = declaration.split_once(':').unwrap_or((declaration, "obj"));
        format!(
            "public {} {} {}",
            FsharpType::csharp(kind),
            name.trim(),
            Self::accessors(writable)
        )
    }

    fn accessors(writable: bool) -> &'static str {
        if writable {
            return "{ get; set; }";
        }
        "{ get; init; }"
    }
}

#[cfg(test)]
mod tests {
    use super::RecordFields;

    #[test]
    fn record_fields_become_properties() {
        assert_eq!(
            RecordFields::csharp("{ Id: int; Lines: Line list }"),
            "public int Id { get; init; } public FSharpList<Line> Lines { get; init; }"
        );
        assert_eq!(
            RecordFields::csharp("  Price: float }"),
            "public double Price { get; init; }"
        );
        assert_eq!(RecordFields::csharp("{"), "");
    }
}
