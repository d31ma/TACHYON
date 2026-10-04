use crate::Failure;
use crate::failure::{diagnostic, source_span};
use piece::Piece;
use std::path::{Component, Path};

pub(crate) use model_blocks::{ModelBlock, ModelBlocks};

mod csharp;
mod dart;
mod go;
mod groovy_model;
mod java;
mod java_model_schema;
pub(crate) use java_model_schema::JavaModelSchema;
mod java_script_model;
mod kotlin;
mod model_blocks;
mod php;
mod piece;
mod python;
mod ruby;
mod rust;
mod schema_constructor;
mod script_schema;
mod swift;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Language {
    TypeScript,
    JavaScript,
    Rust,
    CSharp,
    Dart,
    Go,
    Swift,
    Kotlin,
    Java,
    Groovy,
    Python,
    Php,
    Ruby,
}

impl Language {
    fn from_extension(extension: &str) -> Option<Self> {
        let language = match extension {
            "ts" => Self::TypeScript,
            "js" => Self::JavaScript,
            "rs" => Self::Rust,
            "cs" => Self::CSharp,
            "dart" => Self::Dart,
            "go" => Self::Go,
            "swift" => Self::Swift,
            "kt" => Self::Kotlin,
            "java" => Self::Java,
            "groovy" => Self::Groovy,
            "py" => Self::Python,
            "php" => Self::Php,
            "rb" => Self::Ruby,
            _ => return None,
        };
        Some(language)
    }

    fn name(self) -> &'static str {
        match self {
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Rust => "Rust",
            Self::CSharp => "C#",
            Self::Dart => "Dart",
            Self::Go => "Go",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::Java => "Java",
            Self::Groovy => "Groovy",
            Self::Python => "Python",
            Self::Php => "PHP",
            Self::Ruby => "Ruby",
        }
    }

    fn only_realm(self) -> Option<&'static str> {
        match self {
            Self::Swift => Some("client"),
            Self::Groovy | Self::Php => Some("server"),
            _ => None,
        }
    }

    fn wrong_realm_help(self) -> String {
        if self == Self::Swift {
            return String::from(
                "Swift is not a Yon language. Declare this type in a server language in 'server/models/', or move it to 'client/models/'.",
            );
        }
        let name = self.name();
        format!(
            "{name} runs only in the server realm. Declare this type in a client language in 'client/models/', or move it to 'server/models/'."
        )
    }

    fn declarations(self) -> &'static str {
        match self {
            Self::TypeScript => "interfaces and type aliases",
            Self::JavaScript => "JSDoc @typedef comments and `export {}`",
            Self::Rust => "structs, enums, type aliases and the use declarations they need",
            Self::CSharp => {
                "records, classes and structs with fields and properties, enums and the using directives they need"
            }
            Self::Dart => {
                "classes with fields and constructors, enums, typedefs and the imports they need"
            }
            Self::Go => "one struct, other defined type or type alias, and the imports it needs",
            Self::Swift => {
                "structs with stored properties, enums with cases, typealiases and the imports they need"
            }
            Self::Kotlin => {
                "classes with properties, enum classes with entries, typealiases and the imports they need"
            }
            Self::Java => {
                "records without a body, enums with constants, classes with fields and the imports they need"
            }
            Self::Groovy => {
                "records without a body, enums with constants, classes with typed fields and the imports they need"
            }
            Self::Python => {
                "dataclass, TypedDict, NamedTuple and Enum classes with fields, and the imports they need"
            }
            Self::Php => {
                "classes with typed properties, enums with cases and the use statements they need"
            }
            Self::Ruby => {
                "Data.define and Struct.new records, classes with attr_reader fields and an initialize method, and the require lines they need"
            }
        }
    }

    fn first_refused(self, code: Piece<'_>) -> Option<usize> {
        match self {
            Self::TypeScript => None,
            Self::JavaScript => java_script_model::JavaScriptModel::first_refused(code),
            Self::Rust => rust::first_refused(code),
            Self::CSharp => csharp::first_refused(code),
            Self::Dart => dart::first_refused(code),
            Self::Go => go::first_refused(code),
            Self::Swift => swift::first_refused(code),
            Self::Kotlin => kotlin::first_refused(code),
            Self::Java => java::first_refused(code),
            Self::Groovy => groovy_model::GroovyModel::first_refused(code),
            Self::Python => python::first_refused(code),
            Self::Php => php::first_refused(code),
            Self::Ruby => ruby::first_refused(code),
        }
    }
}

struct ModelFile<'a> {
    path: &'a Path,
    source: &'a str,
    realm: &'a str,
}

impl<'a> ModelFile<'a> {
    const DECLARATION_STARTS: [&'static str; 3] = ["interface ", "type ", "import type "];
    const NATIVE_PACKAGE_HELP: &'static str = " A native Go package with its own package name, such as a spec package, belongs in server/packages/ (ADR 0095).";

    fn find(path: &'a Path, source: &'a str) -> Option<Self> {
        let mut components = path.components();
        let Some(Component::Normal(realm)) = components.next() else {
            return None;
        };
        let realm = realm.to_str()?;
        if !matches!(realm, "client" | "server") {
            return None;
        }
        let Some(Component::Normal(folder)) = components.next() else {
            return None;
        };
        if folder != "models" || components.next().is_none() {
            return None;
        }
        Some(Self {
            path,
            source,
            realm,
        })
    }

    fn failure(&self, at: usize, message: String, help: String) -> Failure {
        let path = self.path.to_string_lossy().replace('\\', "/");
        let end = (at + 1).min(self.source.len());
        Failure::one(diagnostic(
            1310,
            message,
            Some(help),
            source_span(&path, at, end),
        ))
    }

    fn check_language(&self) -> Result<Language, Failure> {
        let extension = self.path.extension().and_then(|value| value.to_str());
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        let Some(language) = extension.and_then(Language::from_extension) else {
            return Err(self.failure(
                0,
                format!("'{name}' is in a language that a models folder does not hold"),
                self.unsupported_language_help(),
            ));
        };
        let Some(only_realm) = language.only_realm() else {
            return Ok(language);
        };
        if only_realm == self.realm {
            return Ok(language);
        }
        let language_name = language.name();
        Err(self.failure(
            0,
            format!("'{name}' is a {language_name} file, and only '{only_realm}/models/' holds {language_name} types"),
            language.wrong_realm_help(),
        ))
    }

    fn unsupported_language_help(&self) -> String {
        let realm = self.realm;
        let languages = match realm {
            "client" => {
                "TypeScript, JavaScript, Rust, C#, Dart, Go, Kotlin, Swift, Java, Python and Ruby"
            }
            _ => {
                "TypeScript, JavaScript, Rust, C#, Dart, Go, Kotlin, Java, Groovy, Python, PHP and Ruby"
            }
        };
        format!(
            "'{realm}/models/' holds data types in {languages}. Move code to its role folder, for example '{realm}/services/'."
        )
    }

    fn check_declarations(&self, language: Language) -> Result<(), Failure> {
        let projection = if language == Language::Java {
            None
        } else {
            crate::behavior::ModelSchema::plain(self.path, self.source)?
        };
        let source = projection.as_deref().unwrap_or(self.source);
        let code = crate::annotation_layout::masked_source(self.path, source)?;
        let script_schema =
            projection.is_some() && matches!(language, Language::JavaScript | Language::TypeScript);
        let code = if script_schema {
            model_blocks::StringFill::apply(
                self.path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or(""),
                source,
                &code,
            )
        } else {
            code
        };
        let start = source.len() - crate::without_bom(source).len();
        let piece = Piece::new(start, &code[start..]);
        let refused = match (language, projection.is_some()) {
            (Language::JavaScript | Language::TypeScript, true) => {
                script_schema::ScriptSchema::first_refused(piece, source)
            }
            (Language::Python, true) => python::PythonModel::schema_refused(piece),
            (Language::CSharp, true) => {
                schema_constructor::SchemaConstructor::first_refused(piece, source)
            }
            _ => language.first_refused(piece),
        };
        let Some(at) = refused else {
            return Ok(());
        };
        let realm = self.realm;
        let declarations = if script_schema {
            "data classes with fields and literal defaults"
        } else {
            language.declarations()
        };
        Err(self.failure(
            at,
            format!("A {} models file declares only {}", language.name(), declarations),
            format!("Move behavior to a service or role file, for example '{realm}/services/', and plain code to '{realm}/shared/'."),
        ))
    }

    fn check_go_package(&self) -> Result<(), Failure> {
        let code = crate::annotation_layout::masked_source(self.path, self.source)?;
        let start = self.source.len() - crate::without_bom(self.source).len();
        let Some(at) = go::other_package(Piece::new(start, &code[start..])) else {
            return Ok(());
        };
        Err(self.failure(
            at,
            String::from("A Go models file belongs to package main; found another package clause"),
            format!("Write `package main` as the first line, or leave the package clause out. Go code uses a model by its type name alone, without an import.{}", self.native_package_help()),
        ))
    }

    fn native_package_help(&self) -> &'static str {
        [("server", Self::NATIVE_PACKAGE_HELP)]
            .iter()
            .find(|(realm, _)| *realm == self.realm)
            .map_or("", |(_, help)| help)
    }

    fn check_statements(&self) -> Result<(), Failure> {
        let code = crate::annotation_layout::masked_source(self.path, self.source)?;
        let code = code.replace(';', "\n");
        let mut depth = 0_usize;
        let mut offset = 0;
        let mut previous_line = "";
        for line in code.split_inclusive('\n') {
            let line_depth = depth;
            depth = depth_after(line, depth);
            let text = line.trim();
            let at = offset + line.len() - line.trim_start().len();
            offset += line.len();
            if text.is_empty() {
                continue;
            }
            let continued =
                line_depth > 0 || ends_unfinished(previous_line) || starts_as_continuation(text);
            previous_line = text;
            if continued || Self::is_type_declaration(text) {
                continue;
            }
            return Err(self.failure(
                at,
                String::from("A models file declares only interfaces and type aliases"),
                format!("Move this code to its role folder, for example '{}/services/', or to '{}/shared/'. Other files use the types with `import type`.", self.realm, self.realm),
            ));
        }
        Ok(())
    }

    fn is_type_declaration(text: &str) -> bool {
        let declaration = text.strip_prefix("export ").unwrap_or(text).trim_start();
        Self::DECLARATION_STARTS
            .iter()
            .any(|start| declaration.starts_with(start))
    }
}

fn depth_after(line: &str, mut depth: usize) -> usize {
    for byte in line.bytes() {
        match byte {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn ends_unfinished(line: &str) -> bool {
    line.ends_with("=>") || line.ends_with(['=', '|', '&', ',', '<', ':', '?', '.'])
}

fn starts_as_continuation(line: &str) -> bool {
    line.starts_with(['|', '&', '=', '.', '?', ':', '>', ',']) || line.starts_with("extends ")
}

pub(crate) fn validate(path: &Path, source: &str) -> Result<(), Failure> {
    let Some(model) = ModelFile::find(path, source) else {
        return Ok(());
    };
    let language = model.check_language()?;
    if language == Language::TypeScript
        && crate::behavior::ModelSchema::plain(path, source)?.is_none()
    {
        return model.check_statements();
    }
    if language == Language::Go {
        model.check_go_package()?;
    }
    model.check_declarations(language)
}
