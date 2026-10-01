use super::{Failure, folded, import_specifier, import_statements, masked_source};
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
mod tests;

pub(super) struct SharedImports;

impl SharedImports {
    pub(super) fn canonical(entry: &Path, specifier: &str) -> String {
        Self::path(entry, specifier)
            .map(|target| Self::relative_path(entry, &target))
            .unwrap_or_else(|| specifier.to_owned())
    }

    pub(super) fn path(path: &Path, specifier: &str) -> Option<PathBuf> {
        let parent = Self::realm_parent(path, specifier)?;
        Self::resolve(parent, specifier)
    }

    fn realm_parent<'a>(path: &'a Path, specifier: &str) -> Option<&'a Path> {
        let realm = [path.starts_with("server"), path.starts_with("client")].contains(&true);
        if [realm, specifier.starts_with('.'), !specifier.contains('\\')].contains(&false) {
            return None;
        }
        path.parent()
    }

    fn resolve(parent: &Path, specifier: &str) -> Option<PathBuf> {
        let mut resolved = parent.to_path_buf();
        for part in Path::new(specifier).components() {
            Self::add_component(&mut resolved, part)?;
        }
        let realm = parent.components().next()?;
        let same_realm = resolved.components().next() == Some(realm);
        let allowed_folder = [
            resolved.starts_with("server/shared"),
            resolved.starts_with("client/shared"),
            resolved.starts_with("client/utils"),
        ]
        .contains(&true);
        let allowed = [same_realm, allowed_folder].into_iter().all(|holds| holds);
        allowed.then_some(resolved)
    }

    fn add_component(resolved: &mut PathBuf, part: Component<'_>) -> Option<()> {
        match part {
            Component::Normal(name) => {
                resolved.push(name);
                Some(())
            }
            Component::CurDir => Some(()),
            Component::ParentDir => resolved.pop().then_some(()),
            _ => None,
        }
    }

    pub(super) fn rebase(
        entry: &Path,
        path: &Path,
        source: &str,
        extension: &str,
    ) -> Result<String, Failure> {
        let masked = Self::script_source(path, source, extension)?;
        Ok(masked.map_or_else(
            || source.to_owned(),
            |masked| Self::rewrite(entry, path, source, &masked),
        ))
    }

    fn script_source(
        path: &Path,
        source: &str,
        extension: &str,
    ) -> Result<Option<String>, Failure> {
        if !["js", "ts"].contains(&extension) {
            return Ok(None);
        }
        masked_source(path, source).map(Some)
    }

    fn rewrite(entry: &Path, path: &Path, source: &str, masked: &str) -> String {
        let mut output = source.to_owned();
        for range in import_statements(source, masked).into_iter().rev() {
            let replacement = Self::replacement(entry, path, &source[range.clone()]);
            Self::replace(&mut output, range, replacement);
        }
        output
    }

    fn replacement(entry: &Path, path: &Path, statement: &str) -> Option<String> {
        let folded = folded(statement);
        let specifier = import_specifier(&folded)?;
        let target = Self::path(path, specifier)?;
        let relative = Self::relative_path(entry, &target);
        Some(statement.replace(specifier, &relative))
    }

    fn replace(output: &mut String, range: std::ops::Range<usize>, replacement: Option<String>) {
        if let Some(replacement) = replacement {
            output.replace_range(range, &replacement);
        }
    }

    pub(super) fn relative_path(entry: &Path, target: &Path) -> String {
        let parent: Vec<_> = entry
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .collect();
        let target: Vec<_> = target.components().collect();
        let shared = Self::shared_depth(&parent, &target);
        let relative = Self::relative_components(parent.len() - shared, &target[shared..]);
        Self::relative_specifier(relative.to_string_lossy().replace('\\', "/"))
    }

    fn relative_specifier(path: String) -> String {
        if path.starts_with('.') {
            path
        } else {
            format!("./{path}")
        }
    }

    fn shared_depth(parent: &[Component<'_>], target: &[Component<'_>]) -> usize {
        parent
            .iter()
            .zip(target)
            .take_while(|(left, right)| left == right)
            .count()
    }

    fn relative_components(parents: usize, target: &[Component<'_>]) -> PathBuf {
        let parent = std::iter::repeat_n(std::ffi::OsStr::new(".."), parents);
        let target = target.iter().map(|part| part.as_os_str());
        let mut relative = PathBuf::new();
        for part in parent.chain(target) {
            relative.push(part);
        }
        relative
    }
}
