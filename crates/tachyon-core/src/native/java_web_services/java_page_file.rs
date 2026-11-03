use super::java_code::JavaCode;
use super::java_enclosure::JavaEnclosure;
use super::java_page_type::JavaPageType;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct JavaPageFile {
    imports: Vec<String>,
    types: Vec<(String, String)>,
}

impl JavaPageFile {
    const FOLDER: &'static str = "tachyon";
    const FILE: &'static str = "TacPage.java";
    const IMPORT: &'static str = "import ";
    const STATIC_IMPORT: &'static str = "import static ";
    const MODEL_IMPORT: &'static str = "import models.";
    const ALL_NAMES: &'static str = ".*";
    const STATEMENT_END: char = ';';
    const NAME_SEPARATOR: char = '.';
    const LINE_ENDS: [u8; 2] = *b"\n\r";
    const BLANK: u8 = b' ';

    pub(super) fn moved(&mut self, source: &str, own: &BTreeSet<String>) -> Option<String> {
        let code = JavaCode::new(source);
        let (opening, types) = Self::types(&code)?;
        self.imports
            .extend(Self::imports(&code.text(&(0..opening))));
        let mut bytes = source.as_bytes().to_vec();
        types
            .iter()
            .filter(|found| !own.contains(&found.name))
            .try_for_each(|moving| self.move_type(source, moving, &mut bytes))?;
        String::from_utf8(bytes).ok()
    }

    pub(super) fn imports_conflict(&self) -> bool {
        let unique: BTreeSet<&str> = self
            .imports
            .iter()
            .map(|statement| statement.trim_end_matches(Self::STATEMENT_END))
            .collect();
        let named: Vec<&str> = unique
            .iter()
            .filter_map(|statement| Self::named_type(statement))
            .collect();
        let distinct: BTreeSet<&&str> = named.iter().collect();
        distinct.len() != named.len()
    }

    pub(super) fn file(&self) -> Option<(PathBuf, String)> {
        let unique: BTreeSet<&String> = self.imports.iter().collect();
        let imports = unique
            .into_iter()
            .fold(String::new(), |text, statement| text + statement + "\n");
        let types = self
            .types
            .iter()
            .fold(String::new(), |text, (_, declared)| {
                text + "\n" + declared + "\n"
            });
        let path = Path::new(Self::FOLDER).join(Self::FILE);
        (!self.types.is_empty()).then(|| (path, JavaEnclosure::page(&format!("{imports}{types}"))))
    }

    fn move_type(&mut self, source: &str, moving: &JavaPageType, bytes: &mut [u8]) -> Option<()> {
        self.keep(&moving.name, source.get(moving.span.clone())?.trim())?;
        Self::blank(&mut bytes[moving.span.clone()]);
        Some(())
    }

    fn keep(&mut self, name: &str, text: &str) -> Option<()> {
        let same = self
            .types
            .iter()
            .find(|(kept, _)| kept == name)
            .map(|(_, kept)| kept == text);
        if let Some(same) = same {
            return same.then_some(());
        }
        self.types.push((name.to_owned(), text.to_owned()));
        Some(())
    }

    fn types(code: &JavaCode) -> Option<(usize, Vec<JavaPageType>)> {
        let declarations = code.top_level_types();
        let opening = declarations
            .first()
            .map_or(0, |first| code.statement_end_before(first.keyword_at));
        let mut start = opening;
        let mut types = Vec::new();
        for declaration in declarations {
            let end = code.body_after(declaration.name_end)?.end + 1;
            types.push(JavaPageType {
                name: declaration.name,
                span: start..end,
            });
            start = end;
        }
        Some((opening, types))
    }

    fn imports(preamble: &str) -> Vec<String> {
        preamble
            .split(Self::STATEMENT_END)
            .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|statement| Self::is_page_import(statement))
            .map(|statement| format!("{statement}{}", Self::STATEMENT_END))
            .collect()
    }

    fn is_page_import(statement: &str) -> bool {
        [
            statement.starts_with(Self::IMPORT),
            statement.starts_with(Self::MODEL_IMPORT),
        ] == [true, false]
    }

    fn named_type(statement: &str) -> Option<&str> {
        let named = [
            statement.starts_with(Self::STATIC_IMPORT),
            statement.ends_with(Self::ALL_NAMES),
        ] == [false, false];
        named
            .then(|| statement.rsplit(Self::NAME_SEPARATOR).next())
            .flatten()
    }

    fn blank(bytes: &mut [u8]) {
        bytes
            .iter_mut()
            .filter(|byte| !Self::LINE_ENDS.contains(byte))
            .for_each(|byte| *byte = Self::BLANK);
    }
}
