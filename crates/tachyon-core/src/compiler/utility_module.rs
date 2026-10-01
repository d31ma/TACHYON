use crate::Failure;
use std::path::Path;

pub(crate) struct UtilityModule;

impl UtilityModule {
    pub(crate) fn needs_module(path: &Path, source: &str) -> Result<bool, Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        Ok(Self::module_meta(&code))
    }

    fn module_meta(code: &str) -> bool {
        code.match_indices("import")
            .filter(|(at, _)| Self::keyword(code, *at, "import"))
            .any(|(at, _)| Self::meta_tail(&code[at + 6..]))
    }

    fn meta_tail(tail: &str) -> bool {
        tail.trim_start()
            .strip_prefix('.')
            .map(str::trim_start)
            .is_some_and(|tail| Self::keyword(tail, 0, "meta"))
    }

    fn keyword(code: &str, at: usize, word: &str) -> bool {
        let tail = &code[at..];
        let before = code[..at].chars().next_back();
        let after = tail.get(word.len()..).and_then(|tail| tail.chars().next());
        [
            tail.starts_with(word),
            !before.is_some_and(Self::word),
            before != Some('.'),
            !after.is_some_and(Self::word),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn word(character: char) -> bool {
        character.is_ascii_alphanumeric() || "_$".contains(character)
    }

    pub(crate) fn exported(path: &Path, source: &str) -> Result<Vec<u8>, Failure> {
        path.starts_with("client/utils")
            .then(|| Self::export_utility(path, source))
            .unwrap_or_else(|| Ok(source.as_bytes().to_vec()))
    }

    fn export_utility(path: &Path, source: &str) -> Result<Vec<u8>, Failure> {
        crate::annotation_layout::UtilityLayout::check(path, source)?;
        let code = crate::annotation_layout::masked_source(path, source)?;
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if Self::named_export(&code, name) {
            return Ok(source.as_bytes().to_vec());
        }
        Ok(format!("{source}\nexport {{ {name} }};\n").into_bytes())
    }

    fn named_export(code: &str, name: &str) -> bool {
        code.match_indices("export")
            .filter(|(at, _)| Self::keyword(code, *at, "export"))
            .any(|(at, _)| Self::export_tail(&code[at + 6..], name))
    }

    fn export_tail(tail: &str, name: &str) -> bool {
        Self::exported_class(tail.trim_start(), name)
    }

    fn exported_class(tail: &str, name: &str) -> bool {
        let tail = Self::without_abstract(tail);
        Self::keyword(tail, 0, "class")
            .then(|| tail.strip_prefix("class"))
            .flatten()
            .map(str::trim_start)
            .is_some_and(|tail| Self::keyword(tail, 0, name))
    }

    fn without_abstract(tail: &str) -> &str {
        Self::keyword(tail, 0, "abstract")
            .then(|| tail[8..].trim_start())
            .unwrap_or(tail)
    }
}
