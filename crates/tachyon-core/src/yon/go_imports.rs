use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct GoImports;

impl GoImports {
    const KEYWORD: &'static str = "import";
    const GROUP_OPEN: char = '(';
    const GROUP_CLOSE: char = ')';
    const QUOTE: char = '"';
    const SELECTOR: char = '.';
    const UNDERSCORE: char = '_';
    const PATH_SEPARATOR: char = '/';

    pub(crate) fn for_types(source: &str, type_texts: &[&str]) -> Result<String, String> {
        let declared = Self::declared(source);
        Self::qualifiers(type_texts)
            .iter()
            .map(|qualifier| Self::import_line(&declared, qualifier))
            .collect()
    }

    fn declared(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, path) in Self::specs(source) {
            declared.entry(name).or_default().insert(path);
        }
        declared
    }

    fn specs(source: &str) -> Vec<(String, String)> {
        Self::line_starts(source)
            .into_iter()
            .filter_map(|start| source[start..].strip_prefix(Self::KEYWORD))
            .filter_map(Self::import_body)
            .flat_map(str::lines)
            .filter_map(Self::spec)
            .collect()
    }

    fn line_starts(source: &str) -> Vec<usize> {
        std::iter::once(0)
            .chain(source.match_indices('\n').map(|(index, _)| index + 1))
            .collect()
    }

    fn import_body(rest: &str) -> Option<&str> {
        let trimmed = rest.trim_start_matches([' ', '\t']);
        match trimmed.strip_prefix(Self::GROUP_OPEN) {
            Some(group) => group.split(Self::GROUP_CLOSE).next(),
            None => trimmed.lines().next(),
        }
    }

    fn spec(line: &str) -> Option<(String, String)> {
        let (written, quoted) = line.split_once(Self::QUOTE)?;
        let path = quoted.split(Self::QUOTE).next()?;
        Some((Self::spec_name(written.trim(), path), path.to_owned()))
    }

    fn spec_name(written: &str, path: &str) -> String {
        if written.is_empty() {
            return Self::package_name(path);
        }
        written.to_owned()
    }

    fn package_name(path: &str) -> String {
        path.rsplit(Self::PATH_SEPARATOR)
            .next()
            .unwrap_or(path)
            .to_owned()
    }

    fn qualifiers(type_texts: &[&str]) -> BTreeSet<String> {
        type_texts
            .iter()
            .flat_map(|text| Self::text_qualifiers(text))
            .collect()
    }

    fn text_qualifiers(text: &str) -> Vec<String> {
        text.split(|character: char| !Self::is_name_or_selector(character))
            .map(|word| word.trim_start_matches(Self::SELECTOR))
            .filter_map(|word| word.split_once(Self::SELECTOR))
            .map(|(qualifier, _)| qualifier.to_owned())
            .collect()
    }

    fn is_name_or_selector(character: char) -> bool {
        [Self::SELECTOR, Self::UNDERSCORE].contains(&character) || character.is_alphanumeric()
    }

    fn import_line(
        declared: &BTreeMap<String, BTreeSet<String>>,
        qualifier: &str,
    ) -> Result<String, String> {
        let paths = declared
            .get(qualifier)
            .ok_or_else(|| Self::missing(qualifier))?;
        let path = Self::single_path(paths, qualifier)?;
        Ok(format!("import {qualifier} \"{path}\"\n"))
    }

    fn single_path<'a>(paths: &'a BTreeSet<String>, qualifier: &str) -> Result<&'a str, String> {
        let mut found = paths.iter();
        let first = found.next().map(String::as_str).unwrap_or_default();
        found
            .next()
            .map_or(Ok(first), |_| Err(Self::ambiguous(qualifier, paths)))
    }

    fn missing(qualifier: &str) -> String {
        format!(
            "a Go type names the package '{qualifier}', but no Go file of this handler imports a package with that name. Import the package in the file that uses the type, and give the import the name '{qualifier}' when its last path part differs"
        )
    }

    fn ambiguous(qualifier: &str, paths: &BTreeSet<String>) -> String {
        let listed: Vec<&str> = paths.iter().map(String::as_str).collect();
        format!(
            "the Go files of this handler import different packages with the name '{qualifier}' ({}). Give one of these imports another name",
            listed.join(", ")
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "go_imports_tests.rs"]
mod tests;
