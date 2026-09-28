pub(crate) struct FsharpType;

impl FsharpType {
    const NAMES: [(&'static str, &'static str); 30] = [
        ("int", "int"),
        ("int32", "int"),
        ("int64", "long"),
        ("int16", "short"),
        ("uint", "uint"),
        ("uint32", "uint"),
        ("uint64", "ulong"),
        ("uint16", "ushort"),
        ("byte", "byte"),
        ("sbyte", "sbyte"),
        ("float", "double"),
        ("double", "double"),
        ("float32", "float"),
        ("single", "float"),
        ("decimal", "decimal"),
        ("bool", "bool"),
        ("string", "string"),
        ("char", "char"),
        ("unit", "void"),
        ("obj", "object"),
        ("list", "FSharpList"),
        ("option", "FSharpOption"),
        ("voption", "FSharpValueOption"),
        ("seq", "IEnumerable"),
        ("Map", "FSharpMap"),
        ("Set", "FSharpSet"),
        ("ResizeArray", "List"),
        ("Async", "FSharpAsync"),
        ("exn", "Exception"),
        ("nativeint", "nint"),
    ];
    const POSTFIX_NAMES: [&'static str; 6] =
        ["list", "option", "voption", "seq", "ResizeArray", "Set"];
    const UNKNOWN: &'static str = "object";

    pub(crate) fn csharp(text: &str) -> String {
        let text = text.trim().trim_start_matches('\'');
        Self::from_shape(text)
    }

    fn from_shape(text: &str) -> String {
        if Self::top_level_position(text, "->").is_some() {
            String::from(Self::UNKNOWN)
        } else if let Some(parts) = Self::tuple_parts(text) {
            Self::tuple(&parts)
        } else if let Some(inner) = text.strip_suffix(" array") {
            format!("{}[]", Self::csharp(inner))
        } else if let Some(inner) = Self::array_argument(text) {
            format!("{}[]", Self::csharp(inner))
        } else if let Some((inner, postfix)) = Self::postfix_application(text) {
            Self::postfix(inner, postfix)
        } else if let Some(inner) = text.strip_suffix("[]") {
            format!("{}[]", Self::csharp(inner))
        } else if let Some((name, arguments)) = Self::generic_application(text) {
            Self::generic(name, arguments)
        } else {
            Self::named(text)
        }
    }

    fn tuple_parts(text: &str) -> Option<Vec<&str>> {
        let parts = Self::split_top_level(text, "*");
        (parts.len() > 1).then_some(parts)
    }

    fn tuple(parts: &[&str]) -> String {
        let items: Vec<String> = parts.iter().map(|part| Self::csharp(part)).collect();
        format!("System.Tuple<{}>", items.join(", "))
    }

    fn postfix_application(text: &str) -> Option<(&str, &str)> {
        let (inner, last) = text.rsplit_once(' ')?;
        Self::POSTFIX_NAMES
            .contains(&last)
            .then_some((inner.trim(), last))
    }

    fn postfix(inner: &str, postfix: &str) -> String {
        format!("{}<{}>", Self::named(postfix), Self::csharp(inner))
    }

    fn array_argument(text: &str) -> Option<&str> {
        text.strip_prefix("array<")?.strip_suffix('>')
    }

    fn generic_application(text: &str) -> Option<(&str, &str)> {
        let (name, rest) = text.split_once('<')?;
        let arguments = rest.strip_suffix('>')?;
        Some((name.trim(), arguments))
    }

    fn generic(name: &str, arguments: &str) -> String {
        let items: Vec<String> = Self::split_top_level(arguments, ",")
            .into_iter()
            .map(Self::csharp)
            .collect();
        format!("{}<{}>", Self::named(name), items.join(", "))
    }

    fn named(name: &str) -> String {
        let found = Self::NAMES
            .iter()
            .find(|(fsharp, _)| *fsharp == name)
            .map(|(_, csharp)| *csharp);
        String::from(found.unwrap_or(name))
    }

    pub(crate) fn split_top_level<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
        Self::parts(text, separator)
    }

    fn parts<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
        let mut parts = Vec::new();
        let mut rest = text;
        while let Some(at) = Self::top_level_position(rest, separator) {
            parts.push(rest[..at].trim());
            rest = &rest[at + separator.len()..];
        }
        parts.push(rest.trim());
        parts
    }

    fn top_level_position(text: &str, separator: &str) -> Option<usize> {
        let mut depth = 0_i32;
        let mut found = None;
        for (at, character) in text.char_indices() {
            depth += Self::depth_change(character);
            found = found.or(Self::separator_at(text, at, depth, separator));
        }
        found
    }

    fn separator_at(text: &str, at: usize, depth: i32, separator: &str) -> Option<usize> {
        (depth == 0 && text[at..].starts_with(separator)).then_some(at)
    }

    fn depth_change(character: char) -> i32 {
        match character {
            '<' | '(' | '[' | '{' => 1,
            '>' | ')' | ']' | '}' => -1,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpType;

    #[test]
    fn primitive_types_use_their_csharp_names() {
        assert_eq!(FsharpType::csharp("int"), "int");
        assert_eq!(FsharpType::csharp("float"), "double");
        assert_eq!(FsharpType::csharp("int64"), "long");
        assert_eq!(FsharpType::csharp("unit"), "void");
        assert_eq!(FsharpType::csharp("Order"), "Order");
    }

    #[test]
    fn postfix_and_generic_types_use_the_dotnet_type() {
        assert_eq!(FsharpType::csharp("Order list"), "FSharpList<Order>");
        assert_eq!(FsharpType::csharp("list<Order>"), "FSharpList<Order>");
        assert_eq!(FsharpType::csharp("string option"), "FSharpOption<string>");
        assert_eq!(
            FsharpType::csharp("int list option"),
            "FSharpOption<FSharpList<int>>"
        );
        assert_eq!(FsharpType::csharp("Order[]"), "Order[]");
        assert_eq!(FsharpType::csharp("float array"), "double[]");
        assert_eq!(FsharpType::csharp("ResizeArray<Order>"), "List<Order>");
        assert_eq!(
            FsharpType::csharp("Dictionary<string, float>"),
            "Dictionary<string, double>"
        );
        assert_eq!(FsharpType::csharp("seq<Tick>"), "IEnumerable<Tick>");
        assert_eq!(FsharpType::csharp("Task<Order>"), "Task<Order>");
    }

    #[test]
    fn tuples_functions_and_type_variables() {
        assert_eq!(
            FsharpType::csharp("int * string"),
            "System.Tuple<int, string>"
        );
        assert_eq!(FsharpType::csharp("int -> string"), "object");
        assert_eq!(FsharpType::csharp("'T"), "T");
    }
}
