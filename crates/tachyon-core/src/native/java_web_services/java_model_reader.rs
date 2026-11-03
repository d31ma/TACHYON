use super::java_code::JavaCode;
use super::java_declaration::JavaDeclaration;
use super::java_model::JavaModel;
use super::java_model_kind::JavaModelKind;
use super::java_type_declaration::JavaTypeDeclaration;
use std::ops::Range;

pub(super) struct JavaModelReader {
    code: JavaCode,
}

impl JavaModelReader {
    const RECORD: &'static str = "record";
    const CLASS: &'static str = "class";
    const ENUMERATION: &'static str = "enum";
    const OPEN_COMPONENTS: u8 = b'(';
    const CONSTANTS_END: char = ';';

    pub(super) fn models(source: &str) -> Vec<JavaModel> {
        let reader = Self {
            code: JavaCode::new(source),
        };
        reader
            .code
            .top_level_types()
            .iter()
            .filter_map(|declaration| reader.model(declaration))
            .collect()
    }

    fn model(&self, declaration: &JavaTypeDeclaration) -> Option<JavaModel> {
        let (kind, fields) = match declaration.keyword {
            Self::RECORD => (JavaModelKind::Record, self.components(declaration)?),
            Self::CLASS => (JavaModelKind::Class, self.fields(declaration)?),
            Self::ENUMERATION => (self.enumeration(declaration)?, Vec::new()),
            _ => return None,
        };
        Some(JavaModel {
            name: declaration.name.clone(),
            kind,
            fields,
        })
    }

    fn components(&self, declaration: &JavaTypeDeclaration) -> Option<Vec<(String, String)>> {
        let open = self
            .code
            .next(declaration.name_end, Self::OPEN_COMPONENTS)?;
        let close = self.code.closing_parenthesis(open)?;
        let list = self.code.text(&(open + 1..close));
        Some(JavaDeclaration::named_parameters(&list))
    }

    fn fields(&self, declaration: &JavaTypeDeclaration) -> Option<Vec<(String, String)>> {
        let body = self.code.body_after(declaration.name_end)?;
        let fields = self
            .code
            .member_headers(&body)
            .iter()
            .filter_map(|header| JavaDeclaration::parse(&self.code.text(header)).model_field())
            .collect();
        Some(fields)
    }

    fn enumeration(&self, declaration: &JavaTypeDeclaration) -> Option<JavaModelKind> {
        let body = self.code.body_after(declaration.name_end)?;
        Some(JavaModelKind::Enumeration {
            constants: self.constants(&body),
        })
    }

    fn constants(&self, body: &Range<usize>) -> Vec<String> {
        let text = self.code.text(body);
        let listed = text.split(Self::CONSTANTS_END).next().unwrap_or_default();
        listed
            .split(',')
            .map(Self::constant_name)
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn constant_name(piece: &str) -> String {
        piece
            .trim()
            .chars()
            .take_while(|character| {
                [character.is_alphanumeric(), *character == '_'].contains(&true)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::JavaModelReader;
    use crate::native::java_web_services::java_model_kind::JavaModelKind;

    #[test]
    fn records_classes_and_enums_become_models() {
        let source = "import java.util.List;\n\n@Topic\nrecord Guide(String title, int pages, List<String> tags, @Deprecated Size size) {\n    Guide {\n    }\n}\n\nenum Size { SHORT, LONG; int weight() { return 1; } }\n\nclass Author {\n    String name = \"\";\n    private int secret;\n    static int shared;\n    String greeting() { return name; }\n}\n";
        let models = JavaModelReader::models(source);
        let found: Vec<(&str, &JavaModelKind, usize)> = models
            .iter()
            .map(|model| (model.name.as_str(), &model.kind, model.fields.len()))
            .collect();
        assert_eq!(
            found,
            [
                ("Guide", &JavaModelKind::Record, 4),
                (
                    "Size",
                    &JavaModelKind::Enumeration {
                        constants: vec![String::from("SHORT"), String::from("LONG")]
                    },
                    0
                ),
                ("Author", &JavaModelKind::Class, 1),
            ]
        );
        assert_eq!(
            models[0].fields[3],
            (String::from("Size"), String::from("size"))
        );
        assert_eq!(
            models[2].fields[0],
            (String::from("String"), String::from("name"))
        );
    }
}
