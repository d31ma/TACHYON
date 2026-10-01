use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
mod model_metadata;
use model_metadata::ModelMetadata;

struct ModelFile {
    inside: PathBuf,
    portable: String,
    source: String,
    plain: String,
    schema: bool,
}

impl ModelFile {
    fn go_source(&self) -> String {
        let position = format!("//line {}:1\n", self.portable);
        let code = crate::annotation_layout::masked_source(Path::new(&self.portable), &self.source)
            .unwrap_or_default();
        if code.split_whitespace().next() == Some("package") {
            return format!("{position}{}", self.plain);
        }
        format!("{position}package main; {}", self.plain)
    }
}

pub(crate) struct ModelSources {
    extension: String,
    files: Vec<ModelFile>,
}

impl ModelSources {
    const DATA_LANGUAGES: [&str; 15] = [
        "rs", "cs", "fs", "dart", "go", "kt", "swift", "java", "groovy", "py", "php", "lua", "rb",
        "js", "ts",
    ];
    const MAX_FILES: usize = 256;
    const MAX_DEPTH: usize = 8;
    const MAX_TOTAL_BYTES: usize = 1024 * 1024;

    pub(crate) fn none() -> Self {
        Self {
            extension: String::new(),
            files: Vec::new(),
        }
    }

    pub(crate) fn read(root: &Path, realm: &str, extension: &str) -> Result<Self, Failure> {
        if !Self::DATA_LANGUAGES.contains(&extension) {
            return Ok(Self::none());
        }
        let mut sources = Self {
            extension: extension.to_owned(),
            files: Vec::new(),
        };
        let folder = PathBuf::from(realm).join("models");
        let mut reader = FolderReader {
            root,
            folder: &folder,
            extension,
            files: &mut sources.files,
            total_bytes: 0,
        };
        reader.read_directory(&folder, 0)?;
        if extension == "py" {
            sources.files = python_dependency_order(sources.files);
        }
        Ok(sources)
    }

    pub(crate) fn metadata(&self) -> ModelMetadata {
        ModelMetadata::new(
            self.files
                .iter()
                .filter(|file| file.schema)
                .map(|file| (PathBuf::from(&file.portable), file.source.clone()))
                .collect(),
        )
    }

    pub(crate) fn changes_program_text(&self) -> bool {
        let inline = matches!(
            self.extension.as_str(),
            "rs" | "cs" | "py" | "php" | "java" | "groovy" | "lua" | "rb" | "js" | "ts"
        );
        inline && !self.files.is_empty()
    }

    pub(crate) fn add_to_program(&self, program: &str) -> String {
        if self.files.is_empty() {
            return program.to_owned();
        }
        match self.extension.as_str() {
            "rs" => format!("{program}\n{}", self.rust_module()),
            "cs" => format!("{program}\n{}", self.csharp_namespaces()),
            "py" => self.python_program(program),
            "php" => self.php_program(program),
            "java" => self.java_program(program),
            "groovy" => self.groovy_program(program),
            "lua" | "rb" | "js" | "ts" => format!("{}\n{program}", self.joined_sources()),
            _ => program.to_owned(),
        }
    }

    pub(crate) fn separate_files(&self) -> Vec<(PathBuf, String)> {
        let mut staged = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            let path = Path::new("models").join(&file.inside);
            match self.extension.as_str() {
                "kt" => staged.push((path, format!("package models\n\n{}", file.plain))),
                "dart" => staged.push((path, file.plain.clone())),
                "go" => staged.push((PathBuf::from(format!("model_{index}.go")), file.go_source())),
                "fs" => staged.push((PathBuf::from(&file.portable), file.plain.clone())),
                _ => {}
            }
        }
        staged
    }

    pub(crate) fn project_files(&self) -> Vec<(PathBuf, String)> {
        self.files
            .iter()
            .map(|file| (PathBuf::from(&file.portable), file.source.clone()))
            .collect()
    }

    pub(crate) fn rust_module(&self) -> String {
        if self.files.is_empty() {
            return String::new();
        }
        let mut exports = String::new();
        let mut modules = String::new();
        for (index, file) in self.files.iter().enumerate() {
            let _ = writeln!(exports, "pub use self::model_{index}::*;");
            let _ = writeln!(
                modules,
                "#[doc = {:?}]\nmod model_{index} {{\n{}\n}}",
                file.portable, file.plain
            );
        }
        format!("#[allow(dead_code, unused_imports)]\npub mod models {{\n{exports}{modules}}}\n")
    }

    pub(crate) fn csharp_namespaces(&self) -> String {
        let mut namespaces = String::new();
        for file in &self.files {
            let _ = writeln!(namespaces, "namespace Models {{\n{}\n}}", file.plain);
        }
        namespaces
    }

    pub(crate) fn joined_sources(&self) -> String {
        let mut joined = String::new();
        for file in &self.files {
            joined.push_str(&file.plain);
            joined.push('\n');
        }
        joined
    }

    fn python_program(&self, program: &str) -> String {
        let mut entries = String::new();
        for file in &self.files {
            let path = serde_json::to_string(&file.portable).unwrap_or_default();
            let source = serde_json::to_string(&file.plain).unwrap_or_default();
            let _ = writeln!(entries, "        ({path}, {source}),");
        }
        let loader = format!(
            "def __tachyon_load_models():\n    import sys\n    import types\n    \
             module = types.ModuleType(\"models\")\n    sys.modules[\"models\"] = module\n    \
             for path, source in [\n{entries}    ]:\n        \
             exec(compile(source, path, \"exec\"), module.__dict__)\n\n\n\
             __tachyon_load_models()\ndel __tachyon_load_models\n"
        );
        let split_at = python_future_imports_end(program);
        let (future_imports, rest) = program.split_at(split_at);
        format!("{future_imports}{loader}\n{rest}")
    }

    fn php_program(&self, program: &str) -> String {
        let (declare, body) = leading_php_declare(php_body(program));
        let mut output = format!("<?php\n{declare}");
        for file in &self.files {
            let _ = writeln!(output, "namespace Models {{\n{}\n}}", php_body(&file.plain));
        }
        let _ = writeln!(output, "namespace {{\n{body}\n}}");
        output
    }

    fn java_program(&self, program: &str) -> String {
        let (header, program, bodies) = self.java_parts(program);
        format!("{header}{program}\n{bodies}")
    }

    fn groovy_program(&self, program: &str) -> String {
        let (header, program, bodies) = self.java_parts(program);
        let program = crate::groovy_source::GroovySource::before_java_part(&program, &bodies);
        format!("{header}{program}")
    }

    pub(crate) fn java_behavior_program(&self, program: &str) -> String {
        let (header, program, bodies) = self.java_parts_written(program, true);
        format!("{header}{program}\n{bodies}")
    }

    fn java_parts(&self, program: &str) -> (String, String, String) {
        self.java_parts_written(program, false)
    }

    fn java_parts_written(&self, program: &str, authored: bool) -> (String, String, String) {
        let mut imports = BTreeSet::new();
        let mut bodies = String::new();
        for file in &self.files {
            let source = if authored { &file.source } else { &file.plain };
            let body = java_body_moving_imports(source, &mut imports);
            bodies.push_str(&body);
            bodies.push('\n');
        }
        let program = java_without_model_imports(program);
        let mut header = String::new();
        for import in imports {
            header.push_str(&import);
            header.push('\n');
        }
        (header, program, bodies)
    }
}

struct FolderReader<'a> {
    root: &'a Path,
    folder: &'a Path,
    extension: &'a str,
    files: &'a mut Vec<ModelFile>,
    total_bytes: usize,
}

impl FolderReader<'_> {
    fn read_directory(&mut self, relative: &Path, depth: usize) -> Result<(), Failure> {
        if depth > ModelSources::MAX_DEPTH {
            return Err(read_failure(relative, "models folders nest too deeply"));
        }
        let Ok(entries) = std::fs::read_dir(self.root.join(relative)) else {
            return Ok(());
        };
        let mut names = Vec::new();
        for entry in entries.flatten() {
            names.push(entry.file_name());
        }
        names.sort();
        for name in names {
            self.read_entry(&relative.join(name), depth)?;
        }
        Ok(())
    }

    fn read_entry(&mut self, relative: &Path, depth: usize) -> Result<(), Failure> {
        let Ok(metadata) = std::fs::symlink_metadata(self.root.join(relative)) else {
            return Ok(());
        };
        if metadata.is_dir() {
            return self.read_directory(relative, depth + 1);
        }
        let matches = relative
            .extension()
            .is_some_and(|value| value == self.extension);
        if !metadata.is_file() || !matches {
            return Ok(());
        }
        self.read_file(relative)
    }

    fn read_file(&mut self, relative: &Path) -> Result<(), Failure> {
        if self.files.len() == ModelSources::MAX_FILES {
            return Err(read_failure(
                relative,
                "a realm holds at most 256 models files per language",
            ));
        }
        let bytes = std::fs::read(self.root.join(relative))
            .map_err(|error| read_failure(relative, &format!("cannot read the file: {error}")))?;
        self.total_bytes += bytes.len();
        if self.total_bytes > ModelSources::MAX_TOTAL_BYTES {
            return Err(read_failure(
                relative,
                "the models files of one language exceed 1 MiB",
            ));
        }
        let source = String::from_utf8(bytes)
            .map_err(|_| read_failure(relative, "a models file must be UTF-8"))?;
        let source = crate::without_bom(&source).to_owned();
        crate::annotation_layout::validate(relative, &source)?;
        let source = crate::topics::Topics::blanked(&source);
        let schema = crate::behavior::ModelSchema::plain(relative, &source)?;
        crate::behavior::ModelSchema::require_realm(relative, &source, schema.is_some())?;
        let marked = schema.is_some();
        let plain = schema.unwrap_or_else(|| source.clone());
        let script = ["js", "ts"].contains(&self.extension);
        let plain = script
            .then(|| crate::annotation_modules::Module::plain_model(relative, &plain))
            .unwrap_or_else(|| Ok(plain.clone()))?;
        let inside = relative
            .strip_prefix(self.folder)
            .unwrap_or(relative)
            .to_path_buf();
        self.files.push(ModelFile {
            inside,
            portable: relative.to_string_lossy().replace('\\', "/"),
            source,
            plain,
            schema: marked,
        });
        Ok(())
    }
}

fn read_failure(relative: &Path, detail: &str) -> Failure {
    let portable = relative.to_string_lossy().replace('\\', "/");
    Failure::one(diagnostic(
        1310,
        format!("Cannot add the models file '{portable}': {detail}"),
        Some(String::from(
            "A models folder holds small data-type files; see docs/adr/0043-models-hold-data-types.md.",
        )),
        source_span(&portable, 0, 0),
    ))
}

fn python_dependency_order(files: Vec<ModelFile>) -> Vec<ModelFile> {
    let mut declared_by = Vec::new();
    for file in &files {
        declared_by.push(python_class_names(&file.source));
    }
    let mut pending: Vec<Option<ModelFile>> = files.into_iter().map(Some).collect();
    let mut ordered = Vec::new();
    let mut defined = BTreeSet::new();
    while ordered.len() < pending.len() {
        let next = next_ready_file(&pending, &declared_by, &defined);
        let Some(file) = pending[next].take() else {
            break;
        };
        defined.extend(declared_by[next].iter().cloned());
        ordered.push(file);
    }
    ordered
}

fn next_ready_file(
    pending: &[Option<ModelFile>],
    declared_by: &[BTreeSet<String>],
    defined: &BTreeSet<String>,
) -> usize {
    let mut first_waiting = None;
    for (index, file) in pending.iter().enumerate() {
        let Some(file) = file else {
            continue;
        };
        first_waiting.get_or_insert(index);
        if python_needs_are_met(&file.source, index, declared_by, defined) {
            return index;
        }
    }
    first_waiting.unwrap_or(0)
}

fn python_needs_are_met(
    source: &str,
    own_index: usize,
    declared_by: &[BTreeSet<String>],
    defined: &BTreeSet<String>,
) -> bool {
    for name in python_model_imports(source) {
        let declared_elsewhere = declared_by
            .iter()
            .enumerate()
            .any(|(index, names)| index != own_index && names.contains(&name));
        if declared_elsewhere && !defined.contains(&name) {
            return false;
        }
    }
    true
}

fn python_class_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("class ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        names.insert(name);
    }
    names
}

fn python_model_imports(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("from models import ") else {
            continue;
        };
        for part in rest.split(',') {
            let part = part.trim().trim_matches(['(', ')']).trim();
            let name = part.split_whitespace().next().unwrap_or_default();
            if !name.is_empty() {
                names.push(name.to_owned());
            }
        }
    }
    names
}

fn python_future_imports_end(program: &str) -> usize {
    let mut end = 0;
    let mut offset = 0;
    for line in program.split_inclusive('\n') {
        offset += line.len();
        if line.starts_with("from __future__ import ") {
            end = offset;
        }
    }
    end
}

fn php_body(source: &str) -> &str {
    let trimmed = source.trim();
    let body = trimmed.strip_prefix("<?php").unwrap_or(trimmed).trim();
    body.strip_suffix("?>").unwrap_or(body)
}

fn leading_php_declare(body: &str) -> (String, &str) {
    if !body.starts_with("declare(") {
        return (String::new(), body);
    }
    let Some(end) = body.find(';') else {
        return (String::new(), body);
    };
    let declare = format!("{}\n", &body[..=end]);
    (declare, body[end + 1..].trim_start())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JavaLine {
    Code,
    ModelImport,
    OtherImport,
}

fn java_lines(source: &str) -> Vec<(&str, JavaLine)> {
    let masked = crate::annotation_layout::masked_source(Path::new("model.java"), source)
        .unwrap_or_else(|_| source.to_owned());
    let mut lines = Vec::new();
    let mut depth = 0_usize;
    for (line, code) in source
        .split_inclusive('\n')
        .zip(masked.split_inclusive('\n'))
    {
        let code_text = code.trim_start();
        let mut kind = JavaLine::Code;
        if depth == 0 && code_text.starts_with("import models.") {
            kind = JavaLine::ModelImport;
        } else if depth == 0 && code_text.starts_with("import ") {
            kind = JavaLine::OtherImport;
        }
        depth = crate::annotation_layout::brace_depth_after(code, depth);
        lines.push((line, kind));
    }
    lines
}

fn java_without_model_imports(program: &str) -> String {
    let mut output = String::new();
    for (line, kind) in java_lines(program) {
        if kind == JavaLine::ModelImport {
            output.push('\n');
        } else {
            output.push_str(line);
        }
    }
    output
}

fn java_body_moving_imports(source: &str, imports: &mut BTreeSet<String>) -> String {
    let mut body = String::new();
    for (line, kind) in java_lines(source) {
        match kind {
            JavaLine::Code => body.push_str(line),
            JavaLine::OtherImport => {
                imports.insert(line.trim().to_owned());
                body.push('\n');
            }
            JavaLine::ModelImport => body.push('\n'),
        }
    }
    body
}

#[cfg(test)]
mod tests;
