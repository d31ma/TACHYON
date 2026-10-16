use super::{HookStatement, MethodBody, RoleClass, SourceEdit};
use crate::Failure;
use std::path::Path;

pub(crate) struct CallHooks;

impl CallHooks {
    const TEST_FOLDER: &'static str = "tests";
    const GENERATED_FOLDERS: [&'static str; 2] = ["remotes", "clients"];
    const RECORDED: [(&'static str, &'static str); 27] = [
        ("client", "js"),
        ("client", "ts"),
        ("client", "rb"),
        ("client", "dart"),
        ("client", "rs"),
        ("client", "cs"),
        ("client", "fs"),
        ("client", "go"),
        ("client", "swift"),
        ("client", "kt"),
        ("client", "java"),
        ("client", "lua"),
        ("client", "hpp"),
        ("server", "js"),
        ("server", "ts"),
        ("server", "py"),
        ("server", "rb"),
        ("server", "php"),
        ("server", "java"),
        ("server", "groovy"),
        ("server", "kt"),
        ("server", "cs"),
        ("server", "fs"),
        ("server", "rs"),
        ("server", "dart"),
        ("server", "go"),
        ("server", "lua"),
    ];

    pub(crate) fn statement(language: &str, key: &str) -> String {
        HookStatement::statement(language, key)
    }

    pub(crate) fn records(realm: &str, language: &str) -> bool {
        Self::RECORDED.contains(&(realm, language))
    }

    pub(crate) fn for_entry(entry: &Path, path: &Path, text: String) -> Result<String, Failure> {
        let hooked = Self::is_test_entry(entry).then(|| Self::insert(path, &text));
        hooked.unwrap_or(Ok(text))
    }

    pub(crate) fn insert(relative: &Path, source: &str) -> Result<String, Failure> {
        let role = (!Self::generated(relative))
            .then(|| RoleClass::read(relative, source))
            .transpose()?
            .flatten();
        Ok(role
            .filter(|role| Self::records(&role.realm, &role.language))
            .map_or_else(|| source.to_owned(), |role| Self::hooked(&role, source)))
    }

    fn generated(relative: &Path) -> bool {
        relative
            .iter()
            .nth(1)
            .is_some_and(|folder| Self::GENERATED_FOLDERS.iter().any(|name| folder == *name))
    }

    fn is_test_entry(entry: &Path) -> bool {
        entry
            .iter()
            .nth(1)
            .is_some_and(|folder| folder == Self::TEST_FOLDER)
    }

    fn hooked(role: &RoleClass, source: &str) -> String {
        let edits = role
            .methods
            .iter()
            .flat_map(|method| MethodBody::edits(role, source, method))
            .collect();
        SourceEdit::apply_all(source, edits)
    }
}
