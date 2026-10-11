use super::{Method, Module};
use crate::route_types::{Shape, TypeText};

pub(super) struct RubyTypedAnswer;

impl RubyTypedAnswer {
    const TAC: &'static str = "tac";
    const EXCHANGE: &'static str = "httpExchange";
    const EXTENSION: &'static str = "rb";
    const ANY: &'static str = "any";

    pub(super) fn wrapped(module: &Module, method: &Method, invoked: &str) -> String {
        Self::shape(module, method).map_or_else(
            || format!("; {invoked}; "),
            |shape| format!("; TacValues.answer({shape:?}, {invoked}); "),
        )
    }

    fn shape(module: &Module, method: &Method) -> Option<String> {
        let typed = [
            module.environment == Self::TAC,
            method.metadata.contains_key(Self::EXCHANGE),
        ] == [true, true];
        typed.then_some(())?;
        let shape = TypeText::shape(Self::EXTENSION, &method.return_type).ok()?;
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
    use super::RubyTypedAnswer;
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
            assert_eq!(RubyTypedAnswer::text(&shape), expected, "{written}");
        }
    }
}
