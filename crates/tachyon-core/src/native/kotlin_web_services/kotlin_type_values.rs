use crate::route_types::{Shape, TypeText};
use std::collections::BTreeSet;

pub(super) struct KotlinTypeValues {
    names: BTreeSet<String>,
}

impl KotlinTypeValues {
    pub(super) fn new(names: BTreeSet<String>) -> Self {
        Self { names }
    }

    pub(super) fn typescript(&self, text: &str) -> Result<String, String> {
        Self::unit(text).map_or_else(|| self.value(text), |value| Ok(String::from(value)))
    }

    pub(super) fn shape(&self, text: &str) -> Result<Shape, String> {
        self.read(text.trim(), 0)
    }

    pub(super) fn rendered(shape: &Shape) -> String {
        Self::render_shape(shape)
    }

    fn unit(text: &str) -> Option<&'static str> {
        (text.trim() == "Unit").then_some("void")
    }

    fn value(&self, text: &str) -> Result<String, String> {
        self.shape(text).map(|shape| Self::render_shape(&shape))
    }

    fn read(&self, text: &str, depth: usize) -> Result<Shape, String> {
        Self::require_depth(depth)?;
        self.optional(text, depth)
    }

    fn require_depth(depth: usize) -> Result<(), String> {
        if depth > 16 {
            return Err(String::from(
                "The Kotlin browser type is nested too deeply.",
            ));
        }
        Ok(())
    }

    fn optional(&self, text: &str, depth: usize) -> Result<Shape, String> {
        text.strip_suffix('?').map_or_else(
            || self.nonnullable(text, depth),
            |inner| {
                self.read(inner.trim(), depth + 1)
                    .map(|value| Shape::Optional(Box::new(value)))
            },
        )
    }

    fn nonnullable(&self, text: &str, depth: usize) -> Result<Shape, String> {
        Self::list(text).map_or_else(
            || self.atom(text),
            |inner| {
                self.read(inner, depth + 1)
                    .map(|value| Shape::List(Box::new(value)))
            },
        )
    }

    fn list(text: &str) -> Option<&str> {
        let (name, rest) = text.split_once('<')?;
        ["List", "MutableList", "Collection"]
            .contains(&name.trim().trim_start_matches("kotlin.collections."))
            .then(|| rest.strip_suffix('>').map(str::trim))
            .flatten()
    }

    fn atom(&self, text: &str) -> Result<Shape, String> {
        Self::builtin(text).map_or_else(|| self.model(text), Ok)
    }

    fn builtin(text: &str) -> Option<Shape> {
        let name = text.trim_start_matches("kotlin.");
        match name {
            "String" | "Int" | "Long" | "Double" | "Float" | "Boolean" => {
                TypeText::shape("kt", name).ok()
            }
            "Any" => Some(Shape::Any),
            _ => None,
        }
    }

    fn model(&self, text: &str) -> Result<Shape, String> {
        let name = text.strip_prefix("models.").unwrap_or(text);
        if self.names.contains(name) {
            return Ok(Shape::Model(name.to_owned()));
        }
        Err(format!(
            "The Kotlin browser type '{text}' has no supported scalar, list, or client model codec. Maps, arrays, sets, inferred returns, and unregistered models need a separate adapter before composed typing."
        ))
    }

    fn render_shape(shape: &Shape) -> String {
        match shape {
            Shape::Any => String::from("unknown"),
            Shape::Text => String::from("string"),
            Shape::Integer | Shape::Number => String::from("number"),
            Shape::Boolean => String::from("boolean"),
            Shape::List(inner) => format!("Array<{}>", Self::render_shape(inner)),
            Shape::Optional(inner) => format!("{} | null", Self::render_shape(inner)),
            Shape::Model(name) => format!("__TachyonKotlinModels.{name}"),
        }
    }
}
