use super::edits::TextEdit;
use super::model_split::ModelSplit;
use super::model_text::ModelText;
use super::type_import::TypeImport;
use std::path::Path;

pub(super) struct ModelImporters<'s, 'a> {
    split: &'s ModelSplit<'a>,
    moved: Vec<&'s str>,
    realm: String,
    models_folder: String,
}

impl<'s, 'a> ModelImporters<'s, 'a> {
    const SCRIPT_LANGUAGES: [&'static str; 2] = ["ts", "js"];
    const DART: &'static str = "dart";
    const DOC_IMPORT: &'static str = "import(";
    const DART_IMPORT: &'static str = "import ";
    const QUOTES: [char; 2] = ['\'', '"'];
    const PARENT_FOLDERS: [&'static str; 2] = ["", "."];

    pub(super) fn new(split: &'s ModelSplit<'a>) -> Self {
        let folder = split.folder.trim_end_matches('/');
        let (realm, models_folder) = folder.split_once('/').unwrap_or((folder, ""));
        Self {
            split,
            moved: split.moved_names(),
            realm: format!("{realm}/"),
            models_folder: models_folder.to_owned(),
        }
    }

    pub(super) fn edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        self.language_edits(relative, source)
    }

    pub(super) fn reads(&self, relative: &str) -> bool {
        let checks = [
            relative.starts_with(&self.realm),
            relative != self.split.relative,
            self.imports_language(Self::extension(relative)),
        ];
        !checks.contains(&false)
    }

    fn imports_language(&self, extension: &str) -> bool {
        match self.split.extension {
            "ts" => Self::SCRIPT_LANGUAGES.contains(&extension),
            "dart" => extension == Self::DART,
            _ => false,
        }
    }

    fn extension(relative: &str) -> &str {
        relative
            .rsplit_once('.')
            .map_or("", |(_, extension)| extension)
    }

    fn language_edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        if self.split.extension == Self::DART {
            return self.dart_edits(relative, source);
        }
        [
            self.script_edits(relative, source),
            self.doc_edits(relative, source),
        ]
        .concat()
    }

    fn script_edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        TypeImport::all(source)
            .into_iter()
            .filter(|import| self.names_split_file(relative, &import.spec))
            .filter(|import| self.moved.iter().any(|name| import.imports(name)))
            .map(|import| self.script_edit(&import))
            .collect()
    }

    fn script_edit(&self, import: &TypeImport) -> TextEdit {
        let rename = |name: &str| Self::renamed_spec(&import.spec, name);
        TextEdit::replace(
            import.start,
            import.end,
            &import.split(&self.moved, &rename),
        )
    }

    fn doc_edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        source
            .match_indices(Self::DOC_IMPORT)
            .filter_map(|(at, _)| self.doc_edit(relative, source, at + Self::DOC_IMPORT.len()))
            .collect()
    }

    fn doc_edit(&self, relative: &str, source: &str, open: usize) -> Option<TextEdit> {
        let spec = Self::quoted(&source[open..])?;
        let after = &source[open + spec.len() + 2..];
        let name = Self::word(after.strip_prefix(").")?);
        let wanted = [
            self.moved.contains(&name),
            self.names_split_file(relative, spec),
        ];
        let start = open + 1;
        (!wanted.contains(&false))
            .then(|| TextEdit::replace(start, start + spec.len(), &Self::renamed_spec(spec, name)))
    }

    fn dart_edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        let code = crate::annotation_layout::masked_source(Path::new(relative), source)
            .unwrap_or_default();
        let used = ModelText::mentioned(&code, &self.moved);
        source
            .match_indices(Self::DART_IMPORT)
            .filter_map(|(at, _)| self.dart_edit(relative, source, at, &used))
            .collect()
    }

    fn dart_edit(
        &self,
        relative: &str,
        source: &str,
        at: usize,
        used: &[&str],
    ) -> Option<TextEdit> {
        let open = at + Self::DART_IMPORT.len();
        let spec = Self::quoted(&source[open..])?;
        let line_end = open + source[open..].find('\n').unwrap_or(source.len() - open);
        let wanted = [!used.is_empty(), self.names_split_file(relative, spec)];
        let lines = used
            .iter()
            .map(|name| format!("\nimport '{}';", Self::renamed_spec(spec, name)))
            .collect::<Vec<String>>()
            .concat();
        (!wanted.contains(&false)).then(|| TextEdit::insert(line_end, &lines))
    }

    fn quoted(text: &str) -> Option<&str> {
        let quote = text
            .chars()
            .next()
            .filter(|first| Self::QUOTES.contains(first))?;
        let length = text[1..].find(quote)?;
        Some(&text[1..=length])
    }

    fn word(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| ModelText::is_name_byte(*byte))
            .count();
        &text[..length]
    }

    fn names_split_file(&self, relative: &str, spec: &str) -> bool {
        let (folder, file) = Self::spec_parts(spec);
        let folder = folder.trim_end_matches('/');
        let stem = file.split('.').next().unwrap_or(file);
        let same_folder =
            relative[..relative.rfind('/').map_or(0, |at| at + 1)] == self.split.folder;
        let from_here = [same_folder, Self::PARENT_FOLDERS.contains(&folder)] == [true, true];
        let from_models = [
            folder.ends_with(&self.models_folder),
            !self.models_folder.is_empty(),
        ] == [true, true];
        let checks = [
            stem == self.split.stem,
            [from_here, from_models].contains(&true),
        ];
        !checks.contains(&false)
    }

    fn renamed_spec(spec: &str, name: &str) -> String {
        let (folder, file) = Self::spec_parts(spec);
        let extension = file.split_once('.').map_or("", |(_, extension)| extension);
        let dot = [".", ""][usize::from(extension.is_empty())];
        format!("{folder}{name}{dot}{extension}")
    }

    fn spec_parts(spec: &str) -> (&str, &str) {
        spec.rsplit_once('/')
            .map_or(("", spec), |(folder, file)| (&spec[..=folder.len()], file))
    }
}
