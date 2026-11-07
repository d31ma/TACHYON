use super::{Method, Module};
use crate::route_types::{Shape, TypeText};
use serde_json::{Map, Value};

pub(super) struct TypedAnswer;

impl TypedAnswer {
    const TAC: &'static str = "tac";
    const EXCHANGE: &'static str = "httpExchange";
    const RUBY: &'static str = "rb";
    const PYTHON: &'static str = "py";
    const ANSWER: &'static str = "answer";
    const ANY: &'static str = "any";

    pub(super) fn ruby_wrapped(module: &Module, method: &Method, invoked: &str) -> String {
        Self::shape(module, method, Self::RUBY).map_or_else(
            || format!("; {invoked}; "),
            |shape| format!("; TacValues.answer({shape:?}, {invoked}); "),
        )
    }

    pub(super) fn python_metadata(module: &Module, method: &Method) -> Map<String, Value> {
        let mut metadata = method.metadata.clone();
        metadata.extend(
            Self::shape(module, method, Self::PYTHON)
                .map(|shape| (String::from(Self::ANSWER), Value::String(shape))),
        );
        metadata
    }

    fn shape(module: &Module, method: &Method, extension: &str) -> Option<String> {
        let typed = [
            module.environment == Self::TAC,
            method.metadata.contains_key(Self::EXCHANGE),
        ] == [true, true];
        typed.then_some(())?;
        let shape = TypeText::shape(extension, &method.return_type).ok()?;
        let text = Self::text(&shape);
        (text != Self::ANY).then_some(text)
    }

    fn text(shape: &Shape) -> String {
        match shape {
            Shape::Any => String::from(Self::ANY),
            Shape::Text => String::from("string"),
            Shape::Integer => String::from("integer"),
            Shape::Number => String::from("number"),
            Shape::Boolean => String::from("boolean"),
            Shape::Optional(inner) => format!("?{}", Self::text(inner)),
            Shape::List(item) => format!("[]{}", Self::text(item)),
            Shape::Model(name) => name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypedAnswer;
    use crate::route_types::TypeText;

    #[test]
    fn ruby_return_types_become_answer_shapes() {
        let cases = [
            ("Greeting", "Greeting"),
            ("Array<Greeting>", "[]Greeting"),
            ("String", "string"),
            ("Integer", "integer"),
            ("Greeting, nil", "?Greeting"),
            ("Hash", "any"),
            ("void", "any"),
        ];
        for (written, expected) in cases {
            let shape = TypeText::shape("rb", written).unwrap_or_default();
            assert_eq!(TypedAnswer::text(&shape), expected, "{written}");
        }
    }

    #[test]
    fn python_return_types_become_answer_shapes() {
        let cases = [
            ("Greeting", "Greeting"),
            ("list[Greeting]", "[]Greeting"),
            ("str", "string"),
            ("int", "integer"),
            ("Greeting | None", "?Greeting"),
            ("dict[str, object]", "any"),
            ("None", "any"),
        ];
        for (written, expected) in cases {
            let shape = TypeText::shape("py", written).unwrap_or_default();
            assert_eq!(TypedAnswer::text(&shape), expected, "{written}");
        }
    }
}
