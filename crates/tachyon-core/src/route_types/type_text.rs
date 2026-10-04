use super::Shape;

pub(crate) struct TypeText;

impl TypeText {
    const MAX_DEPTH: usize = 16;
    const GO_REQUEST_BOUND: [&'static str; 5] = [
        "http.Request",
        "context.Context",
        "multipart.FileHeader",
        "multipart.File",
        "multipart.Form",
    ];

    pub(crate) fn shape(extension: &str, text: &str) -> Result<Shape, String> {
        Self::read(extension, text.trim(), 0)
    }

    fn read(extension: &str, text: &str, depth: usize) -> Result<Shape, String> {
        if depth > Self::MAX_DEPTH {
            return Err(format!("The type '{text}' is nested too deeply"));
        }
        let text = text.trim();
        if text.is_empty() {
            return Ok(Shape::Any);
        }
        if let Some(shape) = Self::optional(extension, text, depth)? {
            return Ok(shape);
        }
        if let Some(shape) = Self::python_tuple(extension, text, depth)? {
            return Ok(shape);
        }
        if let Some(shape) = Self::list(extension, text, depth)? {
            return Ok(shape);
        }
        if let Some(inner) = Self::wrapper(extension, text) {
            return Self::read(extension, &inner, depth + 1);
        }
        if let Some(shape) = Self::scalar(extension, text) {
            return Ok(shape);
        }
        if Self::is_any(extension, text) {
            return Ok(Shape::Any);
        }
        Self::model_name(text)
    }

    fn optional(extension: &str, text: &str, depth: usize) -> Result<Option<Shape>, String> {
        if let Some(members) = Self::union_members(extension, text) {
            return Self::union(extension, &members, depth).map(Some);
        }
        let inner = match extension {
            "kt" | "cs" | "dart" | "ts" | "swift" => text.strip_suffix('?'),
            "php" | "js" => text.strip_prefix('?'),
            "go" => text.strip_prefix('*'),
            _ => None,
        };
        let inner = inner.or_else(|| Self::generic(text, &["Optional", "Option", "FSharpOption"]));
        let inner = if extension == "js" {
            inner.or_else(|| text.strip_suffix('='))
        } else {
            inner
        };
        match inner {
            Some(inner) => Ok(Some(Shape::Optional(Box::new(Self::read(
                extension,
                inner,
                depth + 1,
            )?)))),
            None => Ok(None),
        }
    }

    fn union_members(extension: &str, text: &str) -> Option<Vec<String>> {
        let separator = match extension {
            "ts" | "js" | "py" | "php" => '|',
            "rb" => ',',
            _ => return None,
        };
        let members = Self::split_top_level(text, separator);
        if members.len() < 2 {
            return None;
        }
        Some(members)
    }

    fn union(extension: &str, members: &[String], depth: usize) -> Result<Shape, String> {
        let mut kept = Vec::new();
        let mut nullable = false;
        for member in members {
            let member = member.trim();
            if matches!(member, "null" | "undefined" | "None" | "void" | "nil") {
                nullable = true;
            } else {
                kept.push(member);
            }
        }
        let shape = if kept.len() == 1 {
            Self::read(extension, kept[0], depth + 1)?
        } else if kept == ["true", "false"] || kept == ["false", "true"] {
            Shape::Boolean
        } else if kept.iter().all(|member| Self::is_quoted(member)) {
            Shape::Text
        } else {
            Shape::Any
        };
        if nullable {
            return Ok(Shape::Optional(Box::new(shape)));
        }
        Ok(shape)
    }

    fn python_tuple(extension: &str, text: &str, depth: usize) -> Result<Option<Shape>, String> {
        let Some(inner) = Self::generic(text, &["tuple", "Tuple"]).filter(|_| extension == "py")
        else {
            return Ok(None);
        };
        let item = match Self::split_top_level(inner, ',').as_slice() {
            [item] => item.clone(),
            [item, rest] if rest == "..." => item.clone(),
            _ => return Ok(Some(Shape::Any)),
        };
        let item = Self::read(extension, &item, depth + 1)?;
        Ok(Some(Shape::List(Box::new(item))))
    }

    fn list(extension: &str, text: &str, depth: usize) -> Result<Option<Shape>, String> {
        let inner = match extension {
            "go" => text.strip_prefix("[]"),
            "ts" | "js" | "java" | "cs" | "php" => text.strip_suffix("[]"),
            "swift" if !text.contains(':') => text
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']')),
            _ => None,
        };
        let names: &[&str] = match extension {
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
            "dart" => &["List", "Set", "Iterable"],
            "rb" => &["Array", "Set", "Enumerable"],
            "php" => &["list", "array", "non-empty-list"],
            _ => &[],
        };
        if extension == "rb" && text == "Array" {
            return Ok(Some(Shape::List(Box::new(Shape::Any))));
        }
        let inner = inner.or_else(|| Self::generic(text, names));
        let Some(inner) = inner else {
            return Ok(None);
        };
        let item = Self::read(extension, inner, depth + 1)?;
        Ok(Some(Shape::List(Box::new(item))))
    }

    fn wrapper(extension: &str, text: &str) -> Option<String> {
        let names: &[&str] = match extension {
            "ts" | "js" => &["Promise"],
            "java" => &["CompletableFuture", "CompletionStage"],
            "cs" => &["Task", "ValueTask"],
            "dart" => &["Future", "FutureOr"],
            "py" => &["Awaitable", "Coroutine"],
            "rs" => &["Result", "Box"],
            _ => &[],
        };
        if let Some(inner) = Self::generic(text, names) {
            let first = Self::split_top_level(inner, ',').into_iter().next();
            return first.or_else(|| Some(inner.to_owned()));
        }
        if extension != "go" {
            return None;
        }
        let inner = text.strip_prefix('(')?.strip_suffix(')')?;
        let parts = Self::split_top_level(inner, ',');
        if parts.len() == 2 && parts[1] == "error" {
            return Some(parts[0].clone());
        }
        None
    }

    fn scalar(extension: &str, text: &str) -> Option<Shape> {
        let name = Self::last_segment(text);
        let (texts, integers, numbers, booleans): (&[&str], &[&str], &[&str], &[&str]) =
            match extension {
                "ts" | "js" => (
                    &["string", "String"],
                    &["integer", "bigint"],
                    &["number", "Number"],
                    &["boolean", "Boolean"],
                ),
                "py" => (&["str"], &["int"], &["float"], &["bool"]),
                "php" => (&["string"], &["int"], &["float"], &["bool"]),
                "java" => (
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
                ),
                "kt" => (
                    &["String"],
                    &["Int", "Long", "Short", "Byte"],
                    &["Double", "Float"],
                    &["Boolean"],
                ),
                "cs" => (
                    &["string", "String"],
                    &[
                        "int", "long", "short", "byte", "uint", "ulong", "ushort", "sbyte",
                        "Int32", "Int64", "Int16",
                    ],
                    &["double", "float", "decimal", "Double", "Single", "Decimal"],
                    &["bool", "Boolean"],
                ),
                "rs" => (
                    &["String", "&str", "&'static str", "str"],
                    &[
                        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
                        "isize", "usize",
                    ],
                    &["f32", "f64"],
                    &["bool"],
                ),
                "dart" => (&["String"], &["int"], &["double", "num"], &["bool"]),
                "swift" => (
                    &["String"],
                    &["Int", "Int64", "Int32"],
                    &["Double", "Float"],
                    &["Bool"],
                ),
                "rb" => (
                    &["String", "Symbol"],
                    &["Integer"],
                    &["Float", "Numeric", "BigDecimal", "Rational"],
                    &["Boolean", "TrueClass", "FalseClass"],
                ),
                "go" => (
                    &["string"],
                    &[
                        "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16",
                        "uint32", "uint64",
                    ],
                    &["float32", "float64"],
                    &["bool"],
                ),
                _ => return None,
            };
        if texts.contains(&name) || texts.contains(&text) {
            return Some(Shape::Text);
        }
        if integers.contains(&name) {
            return Some(Shape::Integer);
        }
        if numbers.contains(&name) {
            return Some(Shape::Number);
        }
        if booleans.contains(&name) {
            return Some(Shape::Boolean);
        }
        None
    }

    fn is_any(extension: &str, text: &str) -> bool {
        let name = Self::generic_name(text);
        let name = Self::last_segment(name);
        if text.starts_with('{') || text.starts_with('(') || Self::is_quoted(text) {
            return true;
        }
        let names = Self::any_names(extension);
        if extension == "rb" && text.starts_with("Hash{") {
            return true;
        }
        if extension == "swift" && text.starts_with('[') && text.contains(':') {
            return true;
        }
        names.contains(&name)
            || names.contains(&text)
            || text.starts_with("map[")
            || text.starts_with("impl ")
            || (extension == "go" && Self::is_open_go_type(text))
    }

    fn is_open_go_type(text: &str) -> bool {
        let name = text.trim_start_matches('*');
        let qualified = name.contains('.');
        let request_bound = Self::GO_REQUEST_BOUND.contains(&name);
        [qualified, request_bound] == [true, false]
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
        let name = Self::last_segment(text.trim_matches(['\'', '"']));
        let valid = name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if valid {
            return Ok(Shape::Model(name.to_owned()));
        }
        Err(format!(
            "The type '{text}' is not supported here. Use text, a number, true or false, a list, or a model from server/models/"
        ))
    }

    fn generic<'a>(text: &'a str, names: &[&str]) -> Option<&'a str> {
        let open = text.find(['<', '['])?;
        let close = text.rfind(['>', ']'])?;
        if close != text.len() - 1 || close <= open {
            return None;
        }
        let name = text[..open].trim();
        if !names.contains(&name) && !names.contains(&Self::last_segment(name)) {
            return None;
        }
        Some(text[open + 1..close].trim())
    }

    fn generic_name(text: &str) -> &str {
        match text.find(['<', '[']) {
            Some(open) if open > 0 => text[..open].trim(),
            _ => text,
        }
    }

    fn last_segment(text: &str) -> &str {
        let text = text.trim_start_matches('\\');
        let after_colons = text.rsplit("::").next().unwrap_or(text);
        let after_dots = after_colons.rsplit('.').next().unwrap_or(after_colons);
        after_dots.rsplit('\\').next().unwrap_or(after_dots)
    }

    fn is_quoted(text: &str) -> bool {
        text.len() >= 2
            && ((text.starts_with('\'') && text.ends_with('\''))
                || (text.starts_with('"') && text.ends_with('"')))
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
