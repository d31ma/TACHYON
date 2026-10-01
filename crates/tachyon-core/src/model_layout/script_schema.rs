use super::piece::Piece;

pub(super) struct ScriptSchema;

impl ScriptSchema {
    const FIELD_MODIFIERS: [&'static str; 4] = ["public", "private", "protected", "readonly"];

    pub(super) fn first_refused(code: Piece<'_>, source: &str) -> Option<usize> {
        code.items()
            .into_iter()
            .find_map(|item| Self::refused_class(item, source))
    }

    fn refused_class(item: Piece<'_>, source: &str) -> Option<usize> {
        let declaration = item.after_words(&["export", "default", "declare"]);
        declaration
            .after_word("class")
            .and_then(Piece::body)
            .filter(Self::data_class)
            .map_or(Some(item.at), |body| {
                body.inner
                    .statements()
                    .into_iter()
                    .find_map(|member| Self::refused_field(member, source))
            })
    }

    fn data_class(body: &super::piece::Body<'_>) -> bool {
        ![
            body.head.text == body.head.first_word(),
            body.has_plain_end(),
        ]
        .contains(&false)
    }

    fn refused_field(member: Piece<'_>, source: &str) -> Option<usize> {
        let field = member.after_words(&Self::FIELD_MODIFIERS);
        let (head, value) = field.split_first_top(b'=');
        [
            Self::field_head(head),
            Self::default_value(field, value, source),
        ]
        .contains(&false)
        .then_some(member.at)
    }

    fn default_value(field: Piece<'_>, value: Piece<'_>, source: &str) -> bool {
        field.find_top(b"=").map_or(true, |_| {
            let literal = source[value.at..value.at + value.text.len()].trim();
            Self::literal(value, literal)
        })
    }

    fn field_head(head: Piece<'_>) -> bool {
        let name = head.first_word();
        let named = name
            .as_bytes()
            .first()
            .is_some_and(|byte| !byte.is_ascii_digit());
        let suffix = head.from(name.len()).trim();
        let suffix = suffix
            .after("?")
            .or_else(|| suffix.after("!"))
            .unwrap_or(suffix);
        ![named, Self::field_type(suffix)].contains(&false)
    }

    fn field_type(suffix: Piece<'_>) -> bool {
        [suffix.text.is_empty(), Self::typed_field(suffix)].contains(&true)
    }

    fn typed_field(suffix: Piece<'_>) -> bool {
        suffix.after(":").is_some_and(|kind| {
            ![kind.text.is_empty(), kind.find_top(b"(={").is_some()].contains(&true)
        })
    }

    fn literal(code: Piece<'_>, literal: &str) -> bool {
        if Self::filled_literal(code) {
            return Self::quoted(literal);
        }
        serde_json::from_str::<serde_json::Value>(literal).is_ok()
    }

    fn filled_literal(code: Piece<'_>) -> bool {
        code.text
            .bytes()
            .all(|byte| [byte == b'_', byte.is_ascii_whitespace()].contains(&true))
    }

    fn quoted(literal: &str) -> bool {
        literal
            .as_bytes()
            .first()
            .filter(|quote| b"\"'".contains(quote))
            .is_some_and(|quote| {
                ![literal.len() >= 2, literal.as_bytes().last() == Some(quote)].contains(&false)
            })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::ScriptSchema;
    use crate::model_layout::piece::Piece;
    use std::path::Path;

    #[test]
    fn script_schema_data_fields_preserve_literal_defaults() {
        let source = "export class ProductInput { name: string; enabled = false; limit = 20; label = 'Ready'; }";
        let masked = crate::annotation_layout::masked_source(Path::new("ProductInput.ts"), source)
            .expect("mask");
        let code = crate::model_layout::model_blocks::StringFill::apply("ts", source, &masked);
        assert_eq!(
            ScriptSchema::first_refused(Piece::new(0, &code), source),
            None
        );
    }

    #[test]
    fn script_schema_refuses_constructors_methods_and_executable_defaults() {
        for source in [
            "class ProductInput { constructor() { throw new Error('schema'); } }",
            "class ProductInput { read() { return 1; } }",
            "class ProductInput { get name() { return 'Ready'; } }",
            "class ProductInput { value = load(); }",
            "class ProductInput { value = () => 1; }",
            "class ProductInput extends Service { name; }",
            "const input = {};",
        ] {
            let masked =
                crate::annotation_layout::masked_source(Path::new("ProductInput.js"), source)
                    .expect("mask");
            let code = crate::model_layout::model_blocks::StringFill::apply("js", source, &masked);
            assert!(
                ScriptSchema::first_refused(Piece::new(0, &code), source).is_some(),
                "{source}"
            );
        }
    }

    #[test]
    fn script_schema_layout_requires_real_metadata_and_data_only_classes() {
        for extension in ["js", "ts"] {
            for realm in ["client", "server"] {
                let path = format!("{realm}/models/nested/ProductInput.{extension}");
                let source =
                    "@Checked\nexport class ProductInput {\n@Required\nname;\nlabel = 'Ready';\n}";
                crate::model_layout::validate(Path::new(&path), source).expect("schema data");
                for body in [
                    "read() { return 1; }",
                    "value = load();",
                    "constructor() {}",
                ] {
                    let source = format!("@Checked\nclass ProductInput {{ {body} }}");
                    assert!(
                        crate::model_layout::validate(Path::new(&path), &source).is_err(),
                        "{source}"
                    );
                }
                assert!(
                    crate::model_layout::validate(
                        Path::new(&path),
                        "class ProductInput {\n label = '@Checked';\n}"
                    )
                    .is_err()
                );
            }
        }
    }
}
