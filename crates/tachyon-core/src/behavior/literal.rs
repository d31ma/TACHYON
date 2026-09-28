use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

struct Literal(Value);

impl<'de> Deserialize<'de> for Literal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LiteralVisitor).map(Self)
    }
}

struct LiteralVisitor;

impl<'de> Visitor<'de> for LiteralVisitor {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON literal with unique object keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite literal"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(Literal(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some((key, Literal(value))) = object.next_entry::<String, Literal>()? {
            if values.insert(key, value).is_some() {
                return Err(de::Error::custom("duplicate policy key"));
            }
        }
        Ok(Value::Object(values))
    }
}

pub(crate) fn parse(source: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str::<Literal>(source).map(|literal| literal.0)
}

pub(crate) fn class_reference<'a>(extension: &str, text: &'a str) -> Option<&'a str> {
    let text = text.trim();
    let name = match extension {
        "java" => text.strip_suffix(".class").unwrap_or(text),
        "php" | "kt" => text.strip_suffix("::class").unwrap_or(text),
        "swift" => text.strip_suffix(".self").unwrap_or(text),
        "cs" => text
            .strip_prefix("typeof(")
            .and_then(|value| value.strip_suffix(')'))
            .map_or(text, str::trim),
        _ => text,
    };
    super::identifier(name).then_some(name)
}

pub(crate) fn class_reference_example(extension: &str, name: &str) -> String {
    match extension {
        "java" => format!("{name}.class"),
        "php" | "kt" => format!("{name}::class"),
        "cs" => format!("typeof({name})"),
        _ => name.to_owned(),
    }
}

pub(crate) fn reference_and_options(arguments: &str) -> (&str, Option<&str>) {
    let Some(comma) = top_level_comma(arguments) else {
        return (arguments.trim(), None);
    };
    let options = arguments[comma + 1..].trim();
    (arguments[..comma].trim(), Some(options))
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    for (at, character) in text.char_indices() {
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == open {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            ',' if depth == 0 => return Some(at),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_nested_duplicate_policy_keys() {
        assert!(super::parse(r#"{"fields":{"port":{"type":"integer","type":"string"}}}"#).is_err());
        assert!(super::parse(r#"{"values":[null,true,1,-1,1.5,"text"]}"#).is_ok());
    }

    #[test]
    fn class_references_use_each_language_spelling() {
        let written = [
            ("java", "Order.class"),
            ("kt", "Order::class"),
            ("php", "Order::class"),
            ("cs", "typeof(Order)"),
            ("swift", "Order.self"),
            ("swift", "Order"),
            ("ts", "Order"),
            ("go", " Order "),
        ];
        for (extension, text) in written {
            assert_eq!(
                super::class_reference(extension, text),
                Some("Order"),
                "{extension} {text}"
            );
        }
        for (extension, text) in [("js", "\"Order\""), ("ts", "Order.class"), ("py", "{}")] {
            assert_eq!(
                super::class_reference(extension, text),
                None,
                "{extension} {text}"
            );
        }
    }

    #[test]
    fn options_start_after_the_first_top_level_comma() {
        assert_eq!(super::reference_and_options(" Order "), ("Order", None));
        assert_eq!(
            super::reference_and_options("typeof(Order), {\"a\":[1,2],\"b\":\"x,y\"}"),
            ("typeof(Order)", Some("{\"a\":[1,2],\"b\":\"x,y\"}"))
        );
        assert_eq!(super::reference_and_options("Order,"), ("Order", Some("")));
    }
}
