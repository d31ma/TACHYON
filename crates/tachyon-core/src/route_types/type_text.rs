use super::Shape;

pub(crate) struct TypeText;

impl TypeText {
    const MAX_DEPTH: usize = 16;
    const CPP_INTEGERS: [&'static str; 18] = [
        "int",
        "long",
        "short",
        "long long",
        "unsigned",
        "unsigned int",
        "unsigned long",
        "unsigned long long",
        "unsigned short",
        "int8_t",
        "int16_t",
        "int32_t",
        "int64_t",
        "uint8_t",
        "uint16_t",
        "uint32_t",
        "uint64_t",
        "size_t",
    ];
    const GO_REQUEST_BOUND: [&'static str; 5] = [
        "http.Request",
        "context.Context",
        "multipart.FileHeader",
        "multipart.File",
        "multipart.Form",
    ];
    const NULL_WORDS: [&'static str; 5] = ["null", "undefined", "None", "void", "nil"];
    const OPTIONAL_NAMES: [&'static str; 3] = ["Optional", "Option", "FSharpOption"];
    const TUPLE_NAMES: [&'static str; 2] = ["tuple", "Tuple"];
    const QUOTES: [char; 2] = ['\'', '"'];

    pub(crate) fn shape(extension: &str, text: &str) -> Result<Shape, String> {
        Self::read(extension, text.trim(), 0)
    }

    fn read(extension: &str, text: &str, depth: usize) -> Result<Shape, String> {
        Self::require_shallow(text, depth)?;
        let text = text.trim();
        let shape = Self::read_empty(text)
            .or_else(|| Self::read_union(extension, text, depth))
            .or_else(|| Self::read_optional(extension, text, depth))
            .or_else(|| Self::read_python_tuple(extension, text, depth))
            .or_else(|| Self::read_ruby_array(extension, text))
            .or_else(|| Self::read_list(extension, text, depth))
            .or_else(|| Self::read_wrapped(extension, text, depth))
            .or_else(|| Self::read_scalar(extension, text))
            .or_else(|| Self::read_any(extension, text));
        shape.unwrap_or_else(|| Self::model_name(text))
    }

    fn require_shallow(text: &str, depth: usize) -> Result<(), String> {
        (depth <= Self::MAX_DEPTH)
            .then_some(())
            .ok_or_else(|| format!("The type '{text}' is nested too deeply"))
    }

    fn read_empty(text: &str) -> Option<Result<Shape, String>> {
        text.is_empty().then_some(Ok(Shape::Any))
    }

    fn read_union(extension: &str, text: &str, depth: usize) -> Option<Result<Shape, String>> {
        let members = Self::union_members(extension, text)?;
        Some(Self::union(extension, &members, depth))
    }

    fn read_optional(extension: &str, text: &str, depth: usize) -> Option<Result<Shape, String>> {
        let inner = Self::optional_marker(extension, text)
            .or_else(|| Self::generic(text, &Self::OPTIONAL_NAMES))
            .or_else(|| text.strip_suffix('=').filter(|_| extension == "js"))?;
        let inner = Self::read(extension, inner, depth + 1);
        Some(inner.map(|inner| Shape::Optional(Box::new(inner))))
    }

    fn read_python_tuple(
        extension: &str,
        text: &str,
        depth: usize,
    ) -> Option<Result<Shape, String>> {
        let inner = Self::generic(text, &Self::TUPLE_NAMES).filter(|_| extension == "py")?;
        let item = Self::tuple_item(inner);
        Some(item.map_or(Ok(Shape::Any), |item| {
            Self::list_of(extension, &item, depth)
        }))
    }

    fn read_ruby_array(extension: &str, text: &str) -> Option<Result<Shape, String>> {
        let is_ruby_array = [extension, text] == ["rb", "Array"];
        is_ruby_array.then(|| Ok(Shape::List(Box::new(Shape::Any))))
    }

    fn read_list(extension: &str, text: &str, depth: usize) -> Option<Result<Shape, String>> {
        let inner = Self::list_marker(extension, text)
            .or_else(|| Self::generic(text, Self::list_names(extension)))?;
        Some(Self::list_of(extension, inner, depth))
    }

    fn read_wrapped(extension: &str, text: &str, depth: usize) -> Option<Result<Shape, String>> {
        let inner = Self::wrapper(extension, text)?;
        Some(Self::read(extension, &inner, depth + 1))
    }

    fn read_scalar(extension: &str, text: &str) -> Option<Result<Shape, String>> {
        let names = Self::scalar_names(extension)?;
        Self::scalar_shape(names, text).map(Ok)
    }

    fn read_any(extension: &str, text: &str) -> Option<Result<Shape, String>> {
        Self::is_any(extension, text).then_some(Ok(Shape::Any))
    }

    fn optional_marker<'a>(extension: &str, text: &'a str) -> Option<&'a str> {
        match extension {
            "kt" | "cs" | "dart" | "ts" | "swift" => text.strip_suffix('?'),
            "php" | "js" => text.strip_prefix('?'),
            "go" => text.strip_prefix('*'),
            "hpp" => Self::generic(text, &["optional"]),
            _ => None,
        }
    }

    fn union_members(extension: &str, text: &str) -> Option<Vec<String>> {
        let separator = Self::union_separator(extension)?;
        let members = Self::split_top_level(text, separator);
        (members.len() >= 2).then_some(members)
    }

    fn union_separator(extension: &str) -> Option<char> {
        match extension {
            "ts" | "js" | "py" | "php" => Some('|'),
            "rb" => Some(','),
            _ => None,
        }
    }

    fn union(extension: &str, members: &[String], depth: usize) -> Result<Shape, String> {
        let (nulls, kept): (Vec<&str>, Vec<&str>) = members
            .iter()
            .map(|member| member.trim())
            .partition(|member| Self::NULL_WORDS.contains(member));
        let shape = Self::union_shape(extension, &kept, depth)?;
        Ok(Self::nullable_if(!nulls.is_empty(), shape))
    }

    fn union_shape(extension: &str, kept: &[&str], depth: usize) -> Result<Shape, String> {
        if kept.len() == 1 {
            Self::read(extension, kept[0], depth + 1)
        } else if kept == ["true", "false"] || kept == ["false", "true"] {
            Ok(Shape::Boolean)
        } else if kept.iter().all(|member| Self::is_quoted(member)) {
            Ok(Shape::Text)
        } else {
            Ok(Shape::Any)
        }
    }

    fn nullable_if(nullable: bool, shape: Shape) -> Shape {
        if nullable {
            return Shape::Optional(Box::new(shape));
        }
        shape
    }

    fn tuple_item(inner: &str) -> Option<String> {
        match Self::split_top_level(inner, ',').as_slice() {
            [item] => Some(item.clone()),
            [item, rest] if rest == "..." => Some(item.clone()),
            _ => None,
        }
    }

    fn list_of(extension: &str, item: &str, depth: usize) -> Result<Shape, String> {
        let item = Self::read(extension, item, depth + 1)?;
        Ok(Shape::List(Box::new(item)))
    }

    fn list_marker<'a>(extension: &str, text: &'a str) -> Option<&'a str> {
        match extension {
            "go" => text.strip_prefix("[]"),
            "ts" | "js" | "java" | "cs" | "php" => text.strip_suffix("[]"),
            "swift" if !text.contains(':') => text
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']')),
            _ => None,
        }
    }

    fn list_names(extension: &str) -> &'static [&'static str] {
        match extension {
            "ts" => &["Array", "ReadonlyArray"],
            "js" => &["Array", "Array."],
            "py" => &["list", "List", "Sequence"],
            "java" => &[
                "List",
                "java.util.List",
                "Collection",
                "Set",
                "java.util.Set",
            ],
            "kt" => &["List", "MutableList", "Set", "Array", "Collection"],
            "cs" => &[
                "List",
                "IList",
                "IEnumerable",
                "IReadOnlyList",
                "ICollection",
                "HashSet",
                "FSharpList",
                "FSharpSet",
            ],
            "rs" => &["Vec", "VecDeque", "HashSet", "BTreeSet"],
            "hpp" => &["vector", "deque", "list", "set"],
            "dart" => &["List", "Set", "Iterable"],
            "rb" => &["Array", "Set", "Enumerable"],
            "php" => &["list", "array", "non-empty-list"],
            _ => &[],
        }
    }

    fn wrapper(extension: &str, text: &str) -> Option<String> {
        Self::generic(text, Self::wrapper_names(extension))
            .map(Self::first_type_argument)
            .or_else(|| Self::go_result(extension, text))
    }

    fn wrapper_names(extension: &str) -> &'static [&'static str] {
        match extension {
            "ts" | "js" => &["Promise"],
            "java" => &["CompletableFuture", "CompletionStage"],
            "cs" => &["Task", "ValueTask"],
            "dart" => &["Future", "FutureOr"],
            "py" => &["Awaitable", "Coroutine"],
            "rs" => &["Result", "Box"],
            _ => &[],
        }
    }

    fn first_type_argument(inner: &str) -> String {
        let first = Self::split_top_level(inner, ',').into_iter().next();
        first.unwrap_or_else(|| inner.to_owned())
    }

    fn go_result(extension: &str, text: &str) -> Option<String> {
        let inner = text
            .strip_prefix('(')
            .filter(|_| extension == "go")?
            .strip_suffix(')')?;
        match Self::split_top_level(inner, ',').as_slice() {
            [value, error] if error == "error" => Some(value.clone()),
            _ => None,
        }
    }

    fn scalar_names(extension: &str) -> Option<[&'static [&'static str]; 4]> {
        let names: [&'static [&'static str]; 4] = match extension {
            "ts" | "js" => [
                &["string", "String"],
                &["integer", "bigint"],
                &["number", "Number"],
                &["boolean", "Boolean"],
            ],
            "py" => [&["str"], &["int"], &["float"], &["bool"]],
            "php" => [&["string"], &["int"], &["float"], &["bool"]],
            "java" => [
                &["String", "CharSequence"],
                &[
                    "int",
                    "long",
                    "short",
                    "byte",
                    "Integer",
                    "Long",
                    "Short",
                    "Byte",
                    "BigInteger",
                ],
                &["double", "float", "Double", "Float", "BigDecimal"],
                &["boolean", "Boolean"],
            ],
            "kt" => [
                &["String"],
                &["Int", "Long", "Short", "Byte"],
                &["Double", "Float"],
                &["Boolean"],
            ],
            "cs" => [
                &["string", "String"],
                &[
                    "int", "long", "short", "byte", "uint", "ulong", "ushort", "sbyte", "Int32",
                    "Int64", "Int16",
                ],
                &["double", "float", "decimal", "Double", "Single", "Decimal"],
                &["bool", "Boolean"],
            ],
            "rs" => [
                &["String", "&str", "&'static str", "str"],
                &[
                    "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "isize",
                    "usize",
                ],
                &["f32", "f64"],
                &["bool"],
            ],
            "dart" => [&["String"], &["int"], &["double", "num"], &["bool"]],
            "swift" => [
                &["String"],
                &["Int", "Int64", "Int32"],
                &["Double", "Float"],
                &["Bool"],
            ],
            "rb" => [
                &["String", "Symbol"],
                &["Integer"],
                &["Float", "Numeric", "BigDecimal", "Rational"],
                &["Boolean", "TrueClass", "FalseClass"],
            ],
            "hpp" => [
                &["string", "string_view"],
                &Self::CPP_INTEGERS,
                &["double", "float"],
                &["bool"],
            ],
            "go" => [
                &["string"],
                &[
                    "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32",
                    "uint64",
                ],
                &["float32", "float64"],
                &["bool"],
            ],
            _ => return None,
        };
        Some(names)
    }

    fn scalar_shape(names: [&[&str]; 4], text: &str) -> Option<Shape> {
        let [texts, integers, numbers, booleans] = names;
        let name = Self::last_segment(text);
        if texts.contains(&name) || texts.contains(&text) {
            Some(Shape::Text)
        } else if integers.contains(&name) {
            Some(Shape::Integer)
        } else if numbers.contains(&name) {
            Some(Shape::Number)
        } else if booleans.contains(&name) {
            Some(Shape::Boolean)
        } else {
            None
        }
    }

    fn is_any(extension: &str, text: &str) -> bool {
        let names = Self::any_names(extension);
        let name = Self::last_segment(Self::generic_name(text));
        [
            Self::is_inline_type(text),
            Self::is_dictionary(extension, text),
            Self::is_open_name(extension, names, name, text),
        ]
        .contains(&true)
    }

    fn is_inline_type(text: &str) -> bool {
        [
            text.starts_with('{'),
            text.starts_with('('),
            Self::is_quoted(text),
        ]
        .contains(&true)
    }

    fn is_dictionary(extension: &str, text: &str) -> bool {
        match extension {
            "rb" => text.starts_with("Hash{"),
            "swift" => [text.starts_with('['), text.contains(':')] == [true, true],
            _ => false,
        }
    }

    fn is_open_name(extension: &str, names: &[&str], name: &str, text: &str) -> bool {
        names.contains(&name)
            || names.contains(&text)
            || text.starts_with("map[")
            || text.starts_with("impl ")
            || Self::is_open_go_type(extension, text)
    }

    fn is_open_go_type(extension: &str, text: &str) -> bool {
        let name = text.trim_start_matches('*');
        let qualified = name.contains('.');
        let request_bound = Self::GO_REQUEST_BOUND.contains(&name);
        [extension == "go", qualified, request_bound] == [true, true, false]
    }

    fn any_names(extension: &str) -> &'static [&'static str] {
        match extension {
            "ts" | "js" => &[
                "any",
                "unknown",
                "object",
                "Object",
                "void",
                "Record",
                "Map",
                "*",
                "AsyncGenerator",
                "Generator",
                "AsyncIterable",
                "Iterable",
                "Response",
            ],
            "py" => &[
                "Any",
                "dict",
                "Dict",
                "object",
                "None",
                "Mapping",
                "Iterator",
                "Generator",
                "Iterable",
                "AsyncIterator",
            ],
            "php" => &[
                "array",
                "mixed",
                "object",
                "iterable",
                "Generator",
                "void",
                "Traversable",
            ],
            "java" => &[
                "Object", "Map", "HashMap", "void", "Void", "Iterator", "Stream",
            ],
            "kt" => &[
                "Any",
                "Map",
                "MutableMap",
                "Unit",
                "Json",
                "Sequence",
                "Iterator",
                "Flow",
            ],
            "cs" => &[
                "object",
                "Object",
                "Dictionary",
                "IDictionary",
                "JsonElement",
                "JsonNode",
                "BehaviorJSON",
                "HttpResponseMessage",
                "void",
                "Task",
                "ValueTask",
                "IAsyncEnumerable",
                "IEnumerator",
            ],
            "rs" => &["Json", "Response", "()", "HashMap", "BTreeMap", "Value"],
            "dart" => &["Object", "dynamic", "Map", "void", "Response", "Stream"],
            "go" => &["any", "interface{}", "error"],
            "hpp" => &[
                "JsonNode",
                "JsonObject",
                "JsonArray",
                "JsonValue",
                "map",
                "unordered_map",
                "void",
            ],
            "swift" => &["Any", "AnyObject", "Void", "()"],
            "rb" => &[
                "Object",
                "BasicObject",
                "Hash",
                "void",
                "nil",
                "Enumerator",
                "YonResponse",
            ],
            _ => &[],
        }
    }

    fn model_name(text: &str) -> Result<Shape, String> {
        let name = Self::last_segment(text.trim_matches(Self::QUOTES));
        Self::is_identifier(name)
            .then(|| Shape::Model(name.to_owned()))
            .ok_or_else(|| {
                format!(
                    "The type '{text}' is not supported here. Use text, a number, true or false, a list, or a model from server/models/"
                )
            })
    }

    fn is_identifier(name: &str) -> bool {
        let starts_well = name.bytes().next().is_some_and(Self::is_identifier_start);
        starts_well && name.bytes().all(Self::is_identifier_byte)
    }

    fn is_identifier_start(byte: u8) -> bool {
        [byte.is_ascii_alphabetic(), byte == b'_'].contains(&true)
    }

    fn is_identifier_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn generic<'a>(text: &'a str, names: &[&str]) -> Option<&'a str> {
        let open = text.find(['<', '['])?;
        let close = text.rfind(['>', ']'])?;
        let ends_with_close = [close == text.len() - 1, close > open] == [true, true];
        let name = text[..open].trim();
        let is_named = [
            names.contains(&name),
            names.contains(&Self::last_segment(name)),
        ]
        .contains(&true);
        let is_generic = [ends_with_close, is_named] == [true, true];
        is_generic.then(|| text[open + 1..close].trim())
    }

    fn generic_name(text: &str) -> &str {
        let open = text.find(['<', '[']).filter(|&open| open > 0);
        open.map_or(text, |open| text[..open].trim())
    }

    fn last_segment(text: &str) -> &str {
        let text = text.trim_start_matches('\\');
        let after_colons = text.rsplit("::").next().unwrap_or(text);
        let after_dots = after_colons.rsplit('.').next().unwrap_or(after_colons);
        after_dots.rsplit('\\').next().unwrap_or(after_dots)
    }

    fn is_quoted(text: &str) -> bool {
        Self::QUOTES
            .iter()
            .any(|&quote| Self::is_wrapped_in(text, quote))
    }

    fn is_wrapped_in(text: &str, quote: char) -> bool {
        text.len() >= 2 && text.starts_with(quote) && text.ends_with(quote)
    }

    pub(super) fn split_top_level(text: &str, separator: char) -> Vec<String> {
        let mut parts = Vec::new();
        let mut depth = 0_i32;
        let mut current = String::new();
        let mut previous = ' ';
        for character in text.chars() {
            match character {
                '<' | '[' | '(' | '{' => depth += 1,
                '>' if matches!(previous, '=' | '-') => {}
                '>' | ']' | ')' | '}' => depth -= 1,
                _ => {}
            }
            previous = character;
            if character == separator && depth == 0 {
                parts.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            current.push(character);
        }
        parts.push(current.trim().to_owned());
        parts
    }
}
