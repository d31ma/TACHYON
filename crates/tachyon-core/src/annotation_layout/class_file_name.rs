use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) struct ClassFileName;

impl ClassFileName {
    pub(crate) fn matches(path: &Path, name: &str) -> bool {
        let stem = path.file_stem().and_then(|stem| stem.to_str());
        [
            stem == Some(name),
            stem == Some(Self::preferred(path, name).as_str()),
        ]
        .contains(&true)
    }

    pub(crate) fn preferred(path: &Path, name: &str) -> String {
        Self::render(path, name)
    }

    pub(crate) fn declared(path: &Path, source: &str) -> Option<String> {
        Self::read_name(path, source)
    }

    pub(crate) fn matching_declaration(path: &Path, source: &str) -> Option<String> {
        Self::find_declaration(path, source)
    }

    fn find_declaration(path: &Path, source: &str) -> Option<String> {
        super::declaration_names(path, source)
            .ok()?
            .into_iter()
            .find(|name| Self::matches(path, name))
    }

    fn render(path: &Path, name: &str) -> String {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("rs") => Self::snake_case(name),
            _ => name.to_owned(),
        }
    }

    fn read_name(path: &Path, source: &str) -> Option<String> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("rs") => super::single_class::Parser::declared_name(path, source)
                .ok()
                .flatten()
                .or_else(|| {
                    super::declaration_names(path, source)
                        .ok()?
                        .into_iter()
                        .next()
                }),
            _ => path.file_stem()?.to_str().map(str::to_owned),
        }
    }

    fn snake_case(name: &str) -> String {
        let bytes = name.as_bytes();
        (0..bytes.len()).map(|at| Self::piece(bytes, at)).collect()
    }

    fn piece(bytes: &[u8], at: usize) -> String {
        let lower = char::from(bytes[at]).to_ascii_lowercase();
        if Self::boundary(bytes, at) {
            format!("_{lower}")
        } else {
            lower.to_string()
        }
    }

    fn boundary(bytes: &[u8], at: usize) -> bool {
        let before = at
            .checked_sub(1)
            .and_then(|at| bytes.get(at))
            .copied()
            .unwrap_or(b' ');
        let after = bytes.get(at + 1).copied().unwrap_or(b' ');
        let after_word = [before.is_ascii_lowercase(), before.is_ascii_digit()].contains(&true);
        let acronym_end = [before.is_ascii_uppercase(), after.is_ascii_lowercase()] == [true, true];
        [
            bytes[at].is_ascii_uppercase(),
            [after_word, acronym_end].contains(&true),
        ] == [true, true]
    }
}
