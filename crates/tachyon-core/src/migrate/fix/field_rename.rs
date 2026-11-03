use super::edits::TextEdit;
use super::field_use::FieldUse;
use super::scan::SourceText;
use crate::annotation_rules::AnnotationRules;
use std::fs;
use std::path::Path;

pub(super) struct FieldRename {
    pub(super) edits: Vec<TextEdit>,
    pub(super) note: String,
    from: String,
    to: String,
}

impl FieldRename {
    const SERVICE_FOLDER: &'static str = "client/services";

    pub(super) fn plan(relative: &str, source: &str) -> Vec<Self> {
        let language = Self::language(relative);
        let fields = AnnotationRules::injected_fields(Path::new(relative), source);
        let found = fields
            .iter()
            .map(|field| (field.at, field.written.as_str(), field.fixed.as_str()))
            .collect::<Vec<_>>();
        SourceText::read(language, source)
            .map(|text| Self::planned_all(&text, language, &found))
            .unwrap_or_default()
    }

    pub(super) fn view_bindings(root: &Path) -> Vec<(String, String)> {
        let services: Vec<_> = fs::read_dir(root.join(Self::SERVICE_FOLDER))
            .map_or_else(|_| Vec::new(), |entries| entries.flatten().collect());
        services
            .iter()
            .map(fs::DirEntry::path)
            .flat_map(|path| Self::service_bindings(root, &path))
            .collect()
    }

    pub(super) fn view_edits(source: &str, bindings: &[(String, String)]) -> Vec<TextEdit> {
        bindings
            .iter()
            .flat_map(|(from, to)| Self::binding_edits(source, from, to))
            .collect()
    }

    fn planned_all(
        text: &SourceText<'_>,
        language: &str,
        found: &[(usize, &str, &str)],
    ) -> Vec<Self> {
        found
            .iter()
            .filter_map(|field| Self::planned(text, language, *field))
            .collect()
    }

    fn planned(text: &SourceText<'_>, language: &str, field: (usize, &str, &str)) -> Option<Self> {
        let (at, from, to) = field;
        let uses = text
            .code_words(from)
            .into_iter()
            .filter(|place| *place != at)
            .map(|place| (place, FieldUse::of(text, place, from, language)))
            .collect::<Vec<_>>();
        let unsure = uses.iter().any(|(_, kind)| *kind == FieldUse::Unsure);
        let taken = !text.code_words(to).is_empty();
        let mut places = vec![at];
        places.extend(
            uses.iter()
                .filter(|(_, kind)| *kind == FieldUse::Rename)
                .map(|(place, _)| *place),
        );
        let rename = Self {
            edits: Self::replacements(&places, from.len(), to),
            note: Self::note(text, at, from, to),
            from: from.to_owned(),
            to: to.to_owned(),
        };
        [unsure, taken].eq(&[false, false]).then_some(rename)
    }

    fn replacements(places: &[usize], length: usize, to: &str) -> Vec<TextEdit> {
        places
            .iter()
            .map(|place| TextEdit::replace(*place, place + length, to))
            .collect()
    }

    fn note(text: &SourceText<'_>, at: usize, from: &str, to: &str) -> String {
        format!(
            "Renamed the injected field `{from}` on line {} to `{to}`, and its uses in this class: an injected field uses the language's field case (TY1332).",
            text.line_number(at)
        )
    }

    fn service_bindings(root: &Path, path: &Path) -> Vec<(String, String)> {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let portable = relative.to_string_lossy().replace('\\', "/");
        let class = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        let instance = crate::template::instance_name(class);
        let source = fs::read_to_string(path).unwrap_or_default();
        Self::plan(&portable, &source)
            .into_iter()
            .map(|rename| {
                (
                    format!("{instance}.{}", rename.from),
                    format!("{instance}.{}", rename.to),
                )
            })
            .collect()
    }

    fn binding_edits(source: &str, from: &str, to: &str) -> Vec<TextEdit> {
        source
            .match_indices(from)
            .map(|(at, _)| at)
            .filter(|at| Self::is_whole_binding(source, *at, from.len()))
            .map(|at| TextEdit::replace(at, at + from.len(), to))
            .collect()
    }

    fn is_whole_binding(source: &str, at: usize, length: usize) -> bool {
        let before = source[..at].chars().next_back();
        let after = source[at + length..].chars().next();
        let open_before = before.is_none_or(|letter| !Self::is_binding_letter(letter));
        let open_after = after.is_none_or(|letter| !Self::is_word_letter(letter));
        [open_before, open_after] == [true, true]
    }

    fn is_binding_letter(letter: char) -> bool {
        letter.is_alphanumeric() || letter == '_' || letter == '.'
    }

    fn is_word_letter(letter: char) -> bool {
        letter.is_alphanumeric() || letter == '_'
    }

    fn language(relative: &str) -> &str {
        Path::new(relative)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
    }
}
