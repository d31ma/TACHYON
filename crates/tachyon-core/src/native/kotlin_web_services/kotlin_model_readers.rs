use crate::route_types::{Model, ModelField, ModelTable, WebEventModels};
use std::fmt::Write as _;

pub(super) struct KotlinModelReaders {
    reads: String,
    writes: String,
}

impl KotlinModelReaders {
    const MODEL_PACKAGE: &'static str = "models";
    const NOT_A_MODEL: &'static str =
        "else -> throw IllegalArgumentException(\"The event payload is not a model.\")";

    pub(super) fn text(model_files: &[(String, String)]) -> String {
        let mut readers = Self {
            reads: String::new(),
            writes: String::new(),
        };
        readers.add_web_events();
        readers.add_model_files(model_files);
        readers.finished()
    }

    fn add_web_events(&mut self) {
        for (name, model) in WebEventModels::models() {
            self.add_model(&name, &model);
        }
    }

    fn add_model_files(&mut self, model_files: &[(String, String)]) {
        for (_, source) in model_files {
            let enums = Self::enum_names(source);
            self.add_enums(&enums);
            self.add_records(source, &enums);
        }
    }

    fn add_enums(&mut self, enums: &[String]) {
        for name in enums {
            let qualified = Self::qualified(name);
            let _ = writeln!(
                self.reads,
                "        {qualified}::class -> enumValueOf<{qualified}>(tacString(value))"
            );
        }
    }

    fn add_records(&mut self, source: &str, enums: &[String]) {
        let models = ModelTable::declared_models("kt", source);
        for (name, model) in models.iter().filter(|(name, _)| !enums.contains(name)) {
            self.add_model(&Self::qualified(name), model);
        }
    }

    fn qualified(name: &str) -> String {
        format!("{}.{name}", Self::MODEL_PACKAGE)
    }

    fn enum_names(source: &str) -> Vec<String> {
        source
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("enum class "))
            .map(Self::leading_name)
            .collect()
    }

    fn leading_name(text: &str) -> String {
        text.chars()
            .take_while(|character| Self::is_name_character(*character))
            .collect()
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn add_model(&mut self, name: &str, model: &Model) {
        let Model::Record(record) = model else {
            return;
        };
        self.add_record(name, &record.fields);
    }

    fn add_record(&mut self, name: &str, fields: &[ModelField]) {
        let mut arguments = Vec::new();
        let mut entries = Vec::new();
        for field in fields {
            let field_name = &field.name;
            arguments.push(format!("{field_name} = tacRead(it[\"{field_name}\"])"));
            entries.push(format!("\"{field_name}\" to tacPlain(value.{field_name})"));
        }
        let _ = writeln!(
            self.reads,
            "        {name}::class -> tacModelEntries(value).let {{ {name}({}) }}",
            arguments.join(", ")
        );
        let _ = writeln!(
            self.writes,
            "        is {name} -> linkedMapOf<String, Any?>({})",
            entries.join(", ")
        );
    }

    fn finished(self) -> String {
        let not_a_model = Self::NOT_A_MODEL;
        format!(
            "\nfun tacModel(kind: kotlin.reflect.KClass<*>, value: Any?): Any? =\n    when (kind) {{\n{}        {not_a_model}\n    }}\n\n\
             private fun tacPlainModel(value: Any): Any? =\n    when (value) {{\n{}        else -> value.toString()\n    }}\n",
            self.reads, self.writes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::KotlinModelReaders;

    #[test]
    fn records_and_enums_get_readers_and_records_get_writers() {
        let files = [
            (
                String::from("models/Guide.kt"),
                String::from(
                    "package models\n\ndata class Guide(val title: String, val pages: Int)\n",
                ),
            ),
            (
                String::from("models/Kind.kt"),
                String::from("package models\n\nenum class Kind { SHORT, LONG }\n"),
            ),
        ];
        let text = KotlinModelReaders::text(&files);
        assert!(text.contains(
            "models.Guide::class -> tacModelEntries(value).let { models.Guide(title = tacRead(it[\"title\"]), pages = tacRead(it[\"pages\"])) }"
        ), "{text}");
        assert!(text.contains("is models.Guide -> linkedMapOf<String, Any?>(\"title\" to tacPlain(value.title), \"pages\" to tacPlain(value.pages))"), "{text}");
        assert!(
            text.contains("models.Kind::class -> enumValueOf<models.Kind>(tacString(value))"),
            "{text}"
        );
        assert!(!text.contains("is models.Kind"), "{text}");
        assert!(
            text.contains("PositionFound::class -> tacModelEntries(value)"),
            "{text}"
        );
        assert!(
            text.contains("is ShareFinished -> linkedMapOf<String, Any?>()"),
            "{text}"
        );
    }
}
