use super::{CppFailure, CppFile, CppLowering, CppMarkerCheck, CppMemberKind};
use crate::Failure;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct CppModel {
    pub(crate) file: CppFile,
    pub(crate) path: PathBuf,
    source: String,
}

impl CppModel {
    pub(crate) fn read(relative: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(relative, source)?;
        let named = relative
            .file_stem()
            .is_some_and(|stem| stem == file.class.name.as_str());
        if !named {
            return Err(CppFailure::unreadable(
                relative,
                file.class.name_at,
                "A C++ model is named after its file",
            ));
        }
        CppMarkerCheck::check(relative, &file.class)?;
        Ok(Self {
            file,
            path: relative.to_path_buf(),
            source: source.to_owned(),
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.file.class.name
    }

    pub(crate) fn lowered(&self) -> String {
        CppLowering::model(&self.file, &self.source)
    }

    pub(crate) fn field_types(&self) -> Vec<String> {
        self.fields()
            .into_iter()
            .map(|(_, type_text)| type_text)
            .collect()
    }

    fn fields(&self) -> Vec<(String, String)> {
        self.file
            .class
            .members
            .iter()
            .filter(|member| {
                member.kind == CppMemberKind::Field && member.public && !member.is_static
            })
            .map(|member| (member.name.clone(), member.type_text.clone()))
            .collect()
    }

    pub(crate) fn codec(&self) -> String {
        if self.file.class.is_enum {
            return self.enum_codec();
        }
        let name = self.name();
        let mut writes = String::new();
        let mut reads = String::new();
        for (field, _) in self.fields() {
            let _ = writeln!(
                writes,
                "        JsonModelFields::write(object, \"{field}\", value.{field});"
            );
            let _ = writeln!(
                reads,
                "        JsonModelFields::read(node, \"{field}\", value.{field});"
            );
        }
        format!(
            "namespace tachyon::json {{\ntemplate <>\nstruct JsonCodec<::{name}> {{\n    static constexpr std::string_view NAME = \"{name}\";\n    static JsonNode toJson(const ::{name}& value) {{\n        JsonObject object;\n{writes}        return object;\n    }}\n    static ::{name} fromJson(const JsonNode& node) {{\n        JsonModelFields::requireObject(node, NAME);\n        ::{name} value{{}};\n{reads}        return value;\n    }}\n}};\n}}\n"
        )
    }

    fn enum_codec(&self) -> String {
        let name = self.name();
        let mut writes = String::new();
        let mut reads = String::new();
        for item in &self.file.class.enumerators {
            let _ = writeln!(
                writes,
                "        if (value == ::{name}::{item}) {{\n            return JsonNode(\"{item}\");\n        }}"
            );
            let _ = writeln!(
                reads,
                "        if (written == \"{item}\") {{\n            return ::{name}::{item};\n        }}"
            );
        }
        format!(
            "namespace tachyon::json {{\ntemplate <>\nstruct JsonCodec<::{name}> {{\n    static constexpr std::string_view NAME = \"{name}\";\n    static JsonNode toJson(const ::{name}& value) {{\n{writes}        JsonModelFields::unknownName(NAME);\n    }}\n    static ::{name} fromJson(const JsonNode& node) {{\n        std::string written = JsonModelFields::enumName(node, NAME);\n{reads}        JsonModelFields::unknownName(NAME);\n    }}\n}};\n}}\n"
        )
    }
}
