use super::Input;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::{Component, Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct ImportBoundary;

impl ImportBoundary {
    pub(crate) const CODE: u16 = 1369;
    const LANGUAGES: [&'static str; 3] = ["js", "ts", "dart"];
    const REALMS: [(&'static str, &'static str); 2] =
        [("client", "Tac (client)"), ("server", "Yon (server)")];
    const KEYWORDS: [&'static str; 5] = ["from", "import", "require", "export", "part"];
    const QUOTES: [char; 3] = ['\'', '"', '`'];
    const NAME_SIGNS: &'static str = "_$.";
    const OPENING: [char; 1] = ['('];
    const RELATIVE: [&'static str; 2] = ["./", "../"];
    const HELP: &'static str = "Tac (client) code runs in the browser and Yon (server) code runs on the server: each realm is built on its own and never imports a file of the other realm (ADR 0091). Reach the server from the client through an HTTPClient in client/remotes, and give each realm its own model, utility or service.";

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let realm = Self::checked_realm(path);
        let problems = realm.map_or(Ok(Vec::new()), |realm| Self::problems(path, source, realm))?;
        Self::verdict(problems)
    }

    fn verdict(problems: Vec<Diagnostic>) -> Result<(), Failure> {
        if problems.is_empty() {
            return Ok(());
        }
        Err(Failure::new(problems))
    }

    fn checked_realm(path: &Path) -> Option<&'static str> {
        let extension = path.extension()?.to_str()?;
        let first = path.components().next()?.as_os_str().to_str()?;
        let realm = Self::REALMS.iter().find(|(name, _)| *name == first)?.0;
        Self::LANGUAGES.contains(&extension).then_some(realm)
    }

    fn problems(path: &Path, source: &str, realm: &str) -> Result<Vec<Diagnostic>, Failure> {
        let code = Input::new(path, source).masked_code()?;
        Ok(Self::specifiers(source, &code)
            .into_iter()
            .filter_map(|(at, specifier)| Self::problem(path, realm, at, specifier))
            .collect())
    }

    fn specifiers<'a>(source: &'a str, code: &str) -> Vec<(usize, &'a str)> {
        Self::KEYWORDS
            .iter()
            .flat_map(|keyword| code.match_indices(keyword))
            .filter(|(at, keyword)| Self::is_word(code, *at, keyword.len()))
            .filter_map(|(at, keyword)| Self::quoted_after(source, code, at + keyword.len()))
            .collect()
    }

    fn is_word(code: &str, at: usize, length: usize) -> bool {
        let before = code[..at].chars().next_back();
        let after = code[at + length..].chars().next();
        ![Self::joins(before), Self::joins(after)].contains(&true)
    }

    fn joins(letter: Option<char>) -> bool {
        letter.is_some_and(|letter| {
            [letter.is_alphanumeric(), Self::NAME_SIGNS.contains(letter)].contains(&true)
        })
    }

    fn quoted_after<'a>(source: &'a str, code: &str, end: usize) -> Option<(usize, &'a str)> {
        let gap = source[end..]
            .find(|letter: char| !Self::is_gap(letter))
            .unwrap_or(source.len() - end);
        let start = end + gap;
        let quote = source[start..].chars().next()?;
        let opened = [
            Self::QUOTES.contains(&quote),
            code[start..].starts_with(' '),
        ];
        let text = opened
            .iter()
            .all(|holds| *holds)
            .then(|| &source[start + 1..])?;
        let length = text.find(quote)?;
        Some((start + 1, &text[..length]))
    }

    fn is_gap(letter: char) -> bool {
        [letter.is_whitespace(), Self::OPENING.contains(&letter)].contains(&true)
    }

    fn problem(path: &Path, realm: &str, at: usize, specifier: &str) -> Option<Diagnostic> {
        let target = Self::target(path, specifier)?;
        let other = Self::REALMS
            .iter()
            .filter(|(name, _)| *name != realm)
            .find(|(name, _)| target.starts_with(name))?;
        let portable = path.to_string_lossy().replace('\\', "/");
        let imported = target.to_string_lossy().replace('\\', "/");
        Some(diagnostic(
            Self::CODE,
            format!(
                "'{portable}' imports '{imported}', a file of the {} realm",
                other.1
            ),
            Some(Self::HELP.to_owned()),
            source_span(&portable, at, at + specifier.len()),
        ))
    }

    fn target(path: &Path, specifier: &str) -> Option<PathBuf> {
        let relative = Self::RELATIVE
            .iter()
            .any(|start| specifier.starts_with(start));
        let parent = relative.then(|| path.parent())??.to_path_buf();
        Path::new(specifier)
            .components()
            .try_fold(parent, Self::step)
    }

    fn step(mut resolved: PathBuf, part: Component<'_>) -> Option<PathBuf> {
        match part {
            Component::Normal(name) => resolved.push(name),
            Component::ParentDir => resolved.pop().then_some(())?,
            _ => {}
        }
        Some(resolved)
    }
}
