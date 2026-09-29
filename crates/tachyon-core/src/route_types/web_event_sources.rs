use super::language_kinds::LanguageKinds;
use super::{Model, ModelField, Shape, WebEventModels};
use std::fmt::Write as _;

pub(crate) struct WebEventSources;

impl WebEventSources {
    const RUST: LanguageKinds = LanguageKinds {
        text: "String",
        number: "f64",
        flag: "bool",
    };
    const GO: LanguageKinds = LanguageKinds {
        text: "string",
        number: "float64",
        flag: "bool",
    };
    const CSHARP: LanguageKinds = LanguageKinds {
        text: "string",
        number: "double",
        flag: "bool",
    };
    const DART: LanguageKinds = LanguageKinds {
        text: "String",
        number: "double",
        flag: "bool",
    };
    const KOTLIN: LanguageKinds = LanguageKinds {
        text: "String",
        number: "Double",
        flag: "Boolean",
    };
    const SWIFT: LanguageKinds = LanguageKinds {
        text: "String",
        number: "Double",
        flag: "Bool",
    };
    const JAVA: LanguageKinds = LanguageKinds {
        text: "String",
        number: "double",
        flag: "boolean",
    };
    const GO_PACKAGE: &'static str = "package tac\n";

    pub(crate) fn rust() -> String {
        Self::each(Self::rust_model)
    }

    pub(crate) fn go() -> String {
        format!("{}{}", Self::GO_PACKAGE, Self::each(Self::go_model))
    }

    pub(crate) fn csharp() -> String {
        Self::each(Self::csharp_model)
    }

    pub(crate) fn dart() -> String {
        Self::each(Self::dart_model)
    }

    pub(crate) fn kotlin() -> String {
        Self::each(Self::kotlin_model)
    }

    pub(crate) fn swift_used_by(source: &str) -> String {
        WebEventModels::models()
            .into_iter()
            .filter(|(name, _)| source.contains(name.as_str()))
            .map(|(name, model)| Self::swift_model(&name, &Self::fields(model)))
            .collect()
    }

    pub(crate) fn java_used_by(source: &str) -> String {
        WebEventModels::models()
            .into_iter()
            .filter(|(name, _)| source.contains(name.as_str()))
            .map(|(name, model)| Self::java_model(&name, &Self::fields(model)))
            .collect()
    }

    pub(crate) fn ruby() -> String {
        Self::each(Self::ruby_model)
    }

    fn each(write_model: fn(&str, &[ModelField]) -> String) -> String {
        let mut source = String::new();
        for (name, model) in WebEventModels::models() {
            source.push_str(&write_model(&name, &Self::fields(model)));
        }
        source
    }

    fn fields(model: Model) -> Vec<ModelField> {
        match model {
            Model::Record(record) => record.fields,
            Model::Alias(_) => Vec::new(),
        }
    }

    fn kind(kinds: &LanguageKinds, shape: &Shape) -> &'static str {
        match shape {
            Shape::Number => kinds.number,
            Shape::Boolean => kinds.flag,
            _ => kinds.text,
        }
    }

    fn capitalized(name: &str) -> String {
        let mut characters = name.chars();
        let first = characters.next().map(|first| first.to_ascii_uppercase());
        first.into_iter().chain(characters).collect()
    }

    fn rust_model(name: &str, fields: &[ModelField]) -> String {
        let mut declared = String::new();
        let mut reads = String::new();
        let mut writes = String::new();
        for field in fields {
            let field_name = &field.name;
            let kind = Self::kind(&Self::RUST, &field.shape);
            let _ = writeln!(declared, "    pub {field_name}: {kind},");
            let _ = write!(
                reads,
                "{field_name}: tac_read_field(value, \"{field_name}\"), "
            );
            let _ = write!(
                writes,
                "(String::from(\"{field_name}\"), TacModel::write(self.{field_name})), "
            );
        }
        format!(
            "\n#[derive(Clone, Debug, Default, PartialEq)]\npub struct {name} {{\n{declared}}}\n\n\
             impl TacModel for {name} {{\n    fn read(value: &TacValue) -> Self {{\n        let _ = value;\n        Self {{ {reads}}}\n    }}\n\n    \
             fn write(self) -> TacValue {{\n        TacValue::Object(vec![{writes}])\n    }}\n}}\n"
        )
    }

    fn go_model(name: &str, fields: &[ModelField]) -> String {
        let mut declared = String::new();
        for field in fields {
            let kind = Self::kind(&Self::GO, &field.shape);
            let exported = Self::capitalized(&field.name);
            let _ = writeln!(declared, "\t{exported} {kind} `json:\"{}\"`", field.name);
        }
        format!("\ntype {name} struct {{\n{declared}}}\n")
    }

    fn csharp_model(name: &str, fields: &[ModelField]) -> String {
        let mut parameters = Vec::new();
        for field in fields {
            let kind = Self::kind(&Self::CSHARP, &field.shape);
            parameters.push(format!("{kind} {}", Self::capitalized(&field.name)));
        }
        format!("public sealed record {name}({});\n", parameters.join(", "))
    }

    fn dart_model(name: &str, fields: &[ModelField]) -> String {
        let mut named = Vec::new();
        let mut declared = String::new();
        for field in fields {
            let kind = Self::kind(&Self::DART, &field.shape);
            named.push(format!("required this.{}", field.name));
            let _ = writeln!(declared, "  final {kind} {};", field.name);
        }
        let parameters = Self::dart_parameters(&named);
        format!("\nfinal class {name} {{\n  const {name}({parameters});\n\n{declared}}}\n")
    }

    fn dart_parameters(named: &[String]) -> String {
        if named.is_empty() {
            return String::new();
        }
        format!("{{{}}}", named.join(", "))
    }

    fn kotlin_model(name: &str, fields: &[ModelField]) -> String {
        let mut parameters = Vec::new();
        for field in fields {
            let kind = Self::kind(&Self::KOTLIN, &field.shape);
            parameters.push(format!("val {}: {kind}", field.name));
        }
        format!("\nclass {name}({})\n", parameters.join(", "))
    }

    fn swift_model(name: &str, fields: &[ModelField]) -> String {
        let mut declared = String::new();
        for field in fields {
            let kind = Self::kind(&Self::SWIFT, &field.shape);
            let _ = writeln!(declared, "    let {}: {kind}", field.name);
        }
        format!("\nstruct {name}: Codable {{\n{declared}}}\n")
    }

    fn java_model(name: &str, fields: &[ModelField]) -> String {
        let mut components = Vec::new();
        for field in fields {
            let kind = Self::kind(&Self::JAVA, &field.shape);
            components.push(format!("{kind} {}", field.name));
        }
        format!("\nrecord {name}({}) {{\n}}\n", components.join(", "))
    }

    fn ruby_model(name: &str, fields: &[ModelField]) -> String {
        let mut members = Vec::new();
        for field in fields {
            members.push(format!(":{}", field.name));
        }
        format!("\n{name} = Data.define({})\n", members.join(", "))
    }
}
