use super::FsharpType;

pub(crate) struct AttributeLine {
    pub(crate) name: String,
    pub(crate) csharp: String,
}

impl AttributeLine {
    const FSHARP_ONLY: [&'static str; 20] = [
        "AbstractClass",
        "Sealed",
        "Struct",
        "CLIMutable",
        "AllowNullLiteral",
        "RequireQualifiedAccess",
        "AutoOpen",
        "EntryPoint",
        "Literal",
        "DefaultValue",
        "CompiledName",
        "NoComparison",
        "NoEquality",
        "CustomEquality",
        "CustomComparison",
        "ReferenceEquality",
        "StructuralEquality",
        "StructuralComparison",
        "Measure",
        "Interface",
    ];

    pub(crate) fn read(trimmed: &str) -> Vec<Self> {
        Self::groups(trimmed)
    }

    fn groups(trimmed: &str) -> Vec<Self> {
        let mut attributes = Vec::new();
        let mut rest = trimmed;
        while let Some((inside, next)) = Self::next_group(rest) {
            attributes.extend(Self::group(inside));
            rest = next.trim_start();
        }
        attributes
    }

    fn next_group(text: &str) -> Option<(&str, &str)> {
        let inside = text.strip_prefix("[<")?;
        let closing = inside.find(">]")?;
        Some((&inside[..closing], &inside[closing + 2..]))
    }

    fn group(inside: &str) -> Vec<Self> {
        FsharpType::split_top_level(inside, ";")
            .into_iter()
            .filter(|written| !written.is_empty())
            .map(Self::attribute)
            .filter(|attribute| !Self::FSHARP_ONLY.contains(&attribute.name.as_str()))
            .collect()
    }

    fn attribute(written: &str) -> Self {
        let name_end = written.find(['(', ' ']).unwrap_or(written.len());
        let name = written[..name_end].trim_end_matches("Attribute");
        let arguments = Self::csharp_arguments(&written[name_end..]);
        Self {
            name: name.to_owned(),
            csharp: format!("[{name}{arguments}]"),
        }
    }

    fn csharp_arguments(arguments: &str) -> String {
        let mut written = arguments.trim().to_owned();
        while let Some(start) = written.find("typeof<") {
            written = Self::without_generic_typeof(&written, start);
        }
        written
    }

    fn without_generic_typeof(text: &str, start: usize) -> String {
        let after = &text[start + "typeof<".len()..];
        let closing = after.find('>').unwrap_or(after.len());
        let rest = after.get(closing + 1..).unwrap_or("");
        format!("{}typeof({}){rest}", &text[..start], &after[..closing])
    }
}

#[cfg(test)]
mod tests {
    use super::AttributeLine;

    fn written(line: &str) -> Vec<String> {
        AttributeLine::read(line)
            .into_iter()
            .map(|attribute| attribute.csharp)
            .collect()
    }

    #[test]
    fn fsharp_attributes_become_csharp_attributes() {
        assert_eq!(written("[<Service>]"), vec!["[Service]"]);
        assert_eq!(
            written("[<RequestRoute(\"/orders\"); Public>]"),
            vec!["[RequestRoute(\"/orders\")]", "[Public]"]
        );
        assert_eq!(
            written("[<Service>] [<Lifetime(\"request\")>]"),
            vec!["[Service]", "[Lifetime(\"request\")]"]
        );
        assert_eq!(
            written("[<Check(typeof<ProductInput>)>]"),
            vec!["[Check(typeof(ProductInput))]"]
        );
    }

    #[test]
    fn fsharp_only_attributes_are_left_out() {
        assert!(written("[<CLIMutable>]").is_empty());
        assert_eq!(AttributeLine::read("[<AbstractClass>]").len(), 0);
    }
}
