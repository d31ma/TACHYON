pub(crate) struct MappingShape;

impl MappingShape {
    const NUMBERS: [&'static str; 16] = [
        "int",
        "int32",
        "int64",
        "int16",
        "uint",
        "uint32",
        "uint64",
        "uint16",
        "byte",
        "sbyte",
        "float",
        "double",
        "float32",
        "single",
        "decimal",
        "nativeint",
    ];

    pub(crate) const MAPPING_READER: &'static str = "read";
    pub(crate) const ANSWER_READER: &'static str = "answer";

    pub(crate) fn read(kind: &str, value: &str, reader: &str) -> Result<String, String> {
        Self::optional_read(kind, value, reader)
            .or_else(|| Self::list_read(kind, value, reader))
            .unwrap_or_else(|| Self::scalar_read(kind, value, reader))
    }

    fn optional_read(kind: &str, value: &str, reader: &str) -> Option<Result<String, String>> {
        kind.strip_suffix(" option").map(|inner| {
            let read = Self::read(inner, value, reader)?;
            Ok(format!("(if ({value}).IsNull then None else Some({read}))"))
        })
    }

    fn list_read(kind: &str, value: &str, reader: &str) -> Option<Result<String, String>> {
        Self::container(kind).map(|(inner, container)| {
            let read = Self::read(inner, "item", reader)?;
            Ok(format!(
                "(({value}).Items |> Array.map (fun item -> {read}){container})"
            ))
        })
    }

    fn scalar_read(kind: &str, value: &str, reader: &str) -> Result<String, String> {
        match kind {
            "string" => Ok(format!("({value}).Text")),
            "bool" => Ok(format!("({value}).Boolean")),
            "BehaviorJSON" => Ok(value.to_owned()),
            kind if Self::NUMBERS.contains(&kind) => {
                let property = Self::number_property(kind);
                let conversion = Self::number_conversion(kind);
                Ok(format!("({conversion} (({value}).{property}))"))
            }
            kind if Self::model_name(kind) => Ok(format!("{reader}{kind} ({value})")),
            _ => Err(format!("Unsupported F# typed mapping field: {kind}")),
        }
    }

    fn number_conversion(kind: &str) -> String {
        let integer = Self::number_property(kind) == "Integer";
        format!("{}{kind}", ["", "Checked."][usize::from(integer)])
    }

    fn number_property(kind: &str) -> &'static str {
        let floating = ["float", "double", "float32", "single", "decimal"].contains(&kind);
        ["Integer", "Number"][usize::from(floating)]
    }

    pub(super) fn write(kind: &str, value: &str) -> Result<String, String> {
        Self::optional_write(kind, value)
            .or_else(|| Self::list_write(kind, value))
            .unwrap_or_else(|| Self::scalar_write(kind, value))
    }

    fn optional_write(kind: &str, value: &str) -> Option<Result<String, String>> {
        kind.strip_suffix(" option").map(|inner| {
            let write = Self::write(inner, "item")?;
            Ok(format!(
                "(match {value} with None -> BehaviorJSON.From(null) | Some item -> {write})"
            ))
        })
    }

    fn list_write(kind: &str, value: &str) -> Option<Result<String, String>> {
        Self::container(kind).map(|(inner, _)| {
            let write = Self::write(inner, "item")?;
            Ok(format!(
                "BehaviorJSON.Array({value} |> Seq.map (fun item -> {write}) |> Seq.toArray)"
            ))
        })
    }

    fn scalar_write(kind: &str, value: &str) -> Result<String, String> {
        match kind {
            "string" | "bool" | "BehaviorJSON" => Ok(format!("BehaviorJSON.From({value})")),
            kind if Self::NUMBERS.contains(&kind) => Ok(format!("BehaviorJSON.From({value})")),
            kind if Self::model_name(kind) => Ok(format!("write{kind} {value}")),
            _ => Err(format!("Unsupported F# typed mapping field: {kind}")),
        }
    }

    fn container(kind: &str) -> Option<(&str, &'static str)> {
        [(" list", " |> Array.toList"), (" array", ""), ("[]", "")]
            .into_iter()
            .find_map(|(suffix, conversion)| {
                kind.strip_suffix(suffix)
                    .map(|inner| (inner.trim(), conversion))
            })
    }

    fn model_name(kind: &str) -> bool {
        let initial = kind.chars().next().is_some_and(char::is_uppercase);
        let simple = kind.chars().all(Self::name_character);
        [initial, simple] == [true, true]
    }

    fn name_character(character: char) -> bool {
        [character.is_alphanumeric(), character == '_'].contains(&true)
    }
}
