use super::edits::TextEdit;
use crate::annotation_layout::UtilityLayout;

pub(super) struct SharedUtilsMove;

impl SharedUtilsMove {
    const QUOTES: [char; 3] = ['\'', '"', '`'];
    const RELATIVE_STARTS: [&'static str; 2] = ["./", "../"];
    const WEB_FOLDER: &'static str = "/utils/";
    const SHARED_WEB_FOLDER: &'static str = "/shared/utils/";
    const CLIENT_FOLDER: &'static str = "client/";

    pub(super) fn target(relative: &str) -> Option<String> {
        let rest = Self::folder_rest(relative).filter(|rest| !rest.is_empty())?;
        Some(format!("{}/{rest}", UtilityLayout::CLIENT_ROOT))
    }

    pub(super) fn moves_folder(relative: &str) -> bool {
        Self::folder_rest(&format!("{relative}/")).is_some()
    }

    pub(super) fn path_edits(importer: &str, source: &str) -> Vec<TextEdit> {
        source
            .char_indices()
            .filter_map(|(at, quote)| Self::literal_edit(importer, source, at, quote))
            .collect()
    }

    fn folder_rest(relative: &str) -> Option<&str> {
        relative
            .strip_prefix(UtilityLayout::REMOVED_CLIENT_ROOT)?
            .strip_prefix('/')
    }

    fn literal_edit(importer: &str, source: &str, at: usize, quote: char) -> Option<TextEdit> {
        let start = Self::QUOTES
            .contains(&quote)
            .then(|| at + quote.len_utf8())?;
        let length = source[start..].find([quote, '\n'])?;
        let changed = Self::changed_literal(importer, &source[start..start + length])?;
        Some(TextEdit::replace(start, start + length, &changed))
    }

    fn changed_literal(importer: &str, literal: &str) -> Option<String> {
        Self::target(literal)
            .or_else(|| Self::web_literal(importer, literal))
            .or_else(|| Self::relative_literal(importer, literal))
    }

    fn web_literal(importer: &str, literal: &str) -> Option<String> {
        let rest = literal.strip_prefix(Self::WEB_FOLDER)?;
        importer
            .starts_with(Self::CLIENT_FOLDER)
            .then(|| format!("{}{rest}", Self::SHARED_WEB_FOLDER))
    }

    fn relative_literal(importer: &str, literal: &str) -> Option<String> {
        Self::RELATIVE_STARTS
            .iter()
            .find(|start| literal.starts_with(**start))?;
        let resolved = Self::resolve(Self::directory(importer), literal)?;
        let new_importer = Self::target(importer);
        let new_target = Self::target(&resolved);
        new_importer.as_ref().or(new_target.as_ref())?;
        let changed = Self::relative(
            Self::directory(new_importer.as_deref().unwrap_or(importer)),
            new_target.as_deref().unwrap_or(&resolved),
        );
        (changed != literal).then_some(changed)
    }

    fn directory(relative: &str) -> &str {
        relative
            .rsplit_once('/')
            .map_or("", |(directory, _)| directory)
    }

    fn parts(path: &str) -> Vec<&str> {
        path.split('/').filter(|part| !part.is_empty()).collect()
    }

    fn resolve(directory: &str, literal: &str) -> Option<String> {
        let mut parts = Self::parts(directory);
        for part in literal.split('/') {
            Self::step(&mut parts, part)?;
        }
        Some(parts.join("/"))
    }

    fn step<'a>(parts: &mut Vec<&'a str>, part: &'a str) -> Option<()> {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
        Some(())
    }

    fn relative(directory: &str, target: &str) -> String {
        let from = Self::parts(directory);
        let to = Self::parts(target);
        let shared = from
            .iter()
            .zip(&to)
            .take_while(|(left, right)| left == right)
            .count();
        let ups = "../".repeat(from.len() - shared);
        let rest = to[shared..].join("/");
        if ups.is_empty() {
            return format!("./{rest}");
        }
        format!("{ups}{rest}")
    }
}
