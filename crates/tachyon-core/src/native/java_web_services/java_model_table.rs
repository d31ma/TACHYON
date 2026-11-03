use super::java_model::JavaModel;
use super::java_model_kind::JavaModelKind;
use super::java_value_type::JavaValueType;
use std::fmt::Write as _;

pub(super) struct JavaModelTable<'a> {
    models: &'a [JavaModel],
    values: &'a JavaValueType,
}

impl<'a> JavaModelTable<'a> {
    const FILE: &'static str = "TacModels.java";
    const REQUIRED: [&'static str; 5] = ["int", "long", "double", "float", "boolean"];
    const VALUES: &'static str = "A model field may be int, long, double, float, boolean, their boxed types, String, Object, another model, or a List or Map with String keys of these.";

    pub(super) fn new(models: &'a [JavaModel], values: &'a JavaValueType) -> Self {
        Self { models, values }
    }

    pub(super) fn refusal(&self, model: &JavaModel) -> Option<String> {
        let (value_type, field) = model
            .fields
            .iter()
            .find(|(value_type, _)| self.values.reading(value_type, "value").is_none())?;
        Some(format!(
            "The model {} field {field} uses {value_type}. {}",
            model.name,
            Self::VALUES
        ))
    }

    pub(super) fn file(&self) -> (String, String) {
        let plain: String = self.models.iter().map(Self::plain_case).collect();
        let names: String = self.models.iter().map(Self::name_case).collect();
        let readers: String = self.models.iter().map(|model| self.reader(model)).collect();
        let source = format!(
            "final class TacModels {{\n    private TacModels() {{\n    }}\n\n    \
             static Object plain(Object value) {{\n{plain}        return value;\n    }}\n\n    \
             static String name(Object value) {{\n{names}        return \"\";\n    }}\n{readers}}}\n"
        );
        (String::from(Self::FILE), source)
    }

    fn name_case(model: &JavaModel) -> String {
        format!(
            "        if (value instanceof {name}) {{\n            return \"{name}\";\n        }}\n",
            name = model.name
        )
    }

    fn plain_case(model: &JavaModel) -> String {
        match model.kind {
            JavaModelKind::Enumeration { .. } => format!(
                "        if (value instanceof {} model) {{\n            return model.name();\n        }}\n",
                model.name
            ),
            _ => Self::fields_plain(model),
        }
    }

    fn fields_plain(model: &JavaModel) -> String {
        let puts = model
            .fields
            .iter()
            .fold(String::new(), |mut puts, (_, field)| {
                let read = Self::field_read(model, field);
                let _ = writeln!(puts, "            fields.put(\"{field}\", model.{read});");
                puts
            });
        format!(
            "        if (value instanceof {} model) {{\n            java.util.Map<String, Object> fields = new java.util.LinkedHashMap<>();\n{puts}            return fields;\n        }}\n",
            model.name
        )
    }

    fn field_read(model: &JavaModel, field: &str) -> String {
        match model.kind {
            JavaModelKind::Record => format!("{field}()"),
            _ => String::from(field),
        }
    }

    fn reader(&self, model: &JavaModel) -> String {
        let name = &model.name;
        let method = JavaValueType::model_reader(name);
        let body = match &model.kind {
            JavaModelKind::Record => self.record_body(model),
            JavaModelKind::Class => self.class_body(model),
            JavaModelKind::Enumeration { constants } => Self::enumeration_body(name, constants),
        };
        format!(
            "\n    static {name} {method}(Object value) {{\n        if (value == null) {{\n            return null;\n        }}\n{body}    }}\n"
        )
    }

    fn record_body(&self, model: &JavaModel) -> String {
        let mut arguments = Vec::new();
        for (value_type, field) in &model.fields {
            let required = Self::REQUIRED.contains(&value_type.trim());
            let value = format!("TacValues.field(fields, \"{field}\", {required})");
            arguments.push(self.values.reading(value_type, &value).unwrap_or_default());
        }
        format!(
            "        java.util.Map<?, ?> fields = TacValues.fields(value);\n        return new {}({});\n",
            model.name,
            arguments.join(", ")
        )
    }

    fn class_body(&self, model: &JavaModel) -> String {
        let mut assignments = String::new();
        for (value_type, field) in &model.fields {
            let value = format!("fields.get(\"{field}\")");
            let reading = self.values.reading(value_type, &value).unwrap_or_default();
            let _ = writeln!(
                assignments,
                "        if (fields.containsKey(\"{field}\")) {{\n            model.{field} = {reading};\n        }}"
            );
        }
        format!(
            "        java.util.Map<?, ?> fields = TacValues.fields(value);\n        {name} model = new {name}();\n{assignments}        return model;\n",
            name = model.name
        )
    }

    fn enumeration_body(name: &str, constants: &[String]) -> String {
        let mut cases = String::new();
        for constant in constants {
            let _ = writeln!(
                cases,
                "            case \"{constant}\" -> {name}.{constant};"
            );
        }
        format!(
            "        return switch (TacValues.text(value)) {{\n{cases}            default -> throw TacValues.wrongType();\n        }};\n"
        )
    }
}
