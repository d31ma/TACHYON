use super::edits::TextEdit;
use crate::annotation_layout::UtilityLayout;

pub(super) struct SharedUtilsMove;

impl SharedUtilsMove {
    const QUOTES: [char; 3] = ['\'', '"', '`'];
    const RELATIVE_STARTS: [&'static str; 2] = ["./", "../"];
    const WEB_FOLDER: &'static str = "/utils/";
    const SHARED_WEB_FOLDER: &'static str = "/shared/utils/";

    pub(super) fn target(relative: &str) -> Option<String> {
        let rest = Self::removed_tail(relative)?;
        Some(format!("{}/{rest}", UtilityLayout::CLIENT_ROOT))
    }

    pub(super) fn moves_folder(relative: &str) -> bool {
        relative
            .strip_prefix(UtilityLayout::REMOVED_CLIENT_ROOT)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }

    pub(super) fn path_edits(importer: &str, source: &str) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        for (at, quote) in source.char_indices() {
            if !Self::QUOTES.contains(&quote) {
                continue;
            }
            let start = at + quote.len_utf8();
            let Some(length) = source[start..].find([quote, '\n']) else {
                continue;
            };
            let literal = &source[start..start + length];
            if let Some(changed) = Self::changed_literal(importer, literal) {
                edits.push(TextEdit::replace(start, start + length, &changed));
            }
        }
        edits
    }

    fn removed_tail(relative: &str) -> Option<&str> {
        relative
            .strip_prefix(UtilityLayout::REMOVED_CLIENT_ROOT)?
            .strip_prefix('/')
            .filter(|rest| !rest.is_empty())
    }

    fn changed_literal(importer: &str, literal: &str) -> Option<String> {
        if let Some(target) = Self::target(literal) {
            return Some(target);
        }
        let client_importer = importer.starts_with("client/");
        if let Some(rest) = literal.strip_prefix(Self::WEB_FOLDER) {
            return client_importer.then(|| format!("{}{rest}", Self::SHARED_WEB_FOLDER));
        }
        let relative = Self::RELATIVE_STARTS
            .iter()
            .any(|start| literal.starts_with(start));
        if !relative {
            return None;
        }
        let resolved = Self::resolve(Self::directory(importer), literal)?;
        let moved_target = Self::target(&resolved);
        let moved_importer = Self::target(importer);
        if moved_target.is_none() && moved_importer.is_none() {
            return None;
        }
        let new_importer = moved_importer.unwrap_or_else(|| String::from(importer));
        let new_target = moved_target.unwrap_or(resolved);
        let changed = Self::relative(Self::directory(&new_importer), &new_target);
        (changed != literal).then_some(changed)
    }

    fn directory(relative: &str) -> &str {
        relative
            .rsplit_once('/')
            .map_or("", |(directory, _)| directory)
    }

    fn resolve(directory: &str, literal: &str) -> Option<String> {
        let mut parts: Vec<&str> = directory
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        for part in literal.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                name => parts.push(name),
            }
        }
        Some(parts.join("/"))
    }

    fn relative(directory: &str, target: &str) -> String {
        let from: Vec<&str> = directory
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        let to: Vec<&str> = target.split('/').collect();
        let shared = from
            .iter()
            .zip(&to)
            .take_while(|(left, right)| left == right)
            .count();
        let ups = from.len() - shared;
        let rest = to[shared..].join("/");
        if ups == 0 {
            return format!("./{rest}");
        }
        format!("{}{rest}", "../".repeat(ups))
    }
}
