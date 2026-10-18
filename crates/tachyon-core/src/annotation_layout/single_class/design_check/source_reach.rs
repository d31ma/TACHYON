#![allow(clippy::needless_bitwise_bool)]

use super::super::duplication::ProjectSource;
use super::access_scan::AccessScan;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct SourceReach<'a> {
    path: &'a str,
    realm: &'static str,
    view: bool,
    hands_over: bool,
    text: &'a str,
    words: BTreeSet<&'a str>,
    accesses: Vec<(&'a str, &'a str)>,
}

impl<'a> SourceReach<'a> {
    const CODE_EXTENSIONS: [&'static str; 19] = [
        "js", "mjs", "ts", "java", "groovy", "kt", "cs", "fs", "dart", "swift", "php", "rs", "go",
        "py", "rb", "lua", "hpp", "html", "tsx",
    ];
    const HANDING_FOLDERS: [&'static str; 2] = ["/setups/", "/configurations/"];
    const VIEW_EXTENSION: &'static str = "html";
    const TESTS_FOLDER: &'static str = "tests";
    const GENERATED_MARK: &'static str = ".d.ts";
    const CLIENT_REALM: &'static str = "client";
    const CLIENT_SERVICES: &'static str = "client/services/";
    const CSHARP_EXTENSION: &'static str = "cs";
    const SCRIPT_SUFFIXES: [&'static str; 2] = [".js", ".ts"];
    const SCRIPT_SELF: &'static str = "this";

    pub(super) fn read_all(sources: &'a [ProjectSource]) -> Vec<Self> {
        sources
            .iter()
            .filter(|source| Self::is_counted(source))
            .map(Self::read)
            .collect()
    }

    pub(super) fn beside(&self, path: &str, realm: &str) -> bool {
        let other_class = Self::without_extension(self.path) != Self::without_extension(path);
        other_class & (self.realm == realm)
    }

    pub(super) fn sibling_bridge_names(
        &self,
        path: &str,
        realm: &str,
        authored_extension: &str,
    ) -> Vec<&'a str> {
        let service = (realm == Self::CLIENT_REALM) & path.starts_with(Self::CLIENT_SERVICES);
        let csharp = authored_extension == Self::CSHARP_EXTENSION;
        let script = Self::SCRIPT_SUFFIXES
            .iter()
            .any(|suffix| self.path.ends_with(suffix));
        let sibling = Self::without_extension(self.path) == Self::without_extension(path);
        let same_realm = self.realm == realm;
        if !(service & csharp & script & sibling & same_realm) {
            return Vec::new();
        }
        self.accesses
            .iter()
            .filter(|(receiver, _)| *receiver == Self::SCRIPT_SELF)
            .map(|(_, name)| *name)
            .collect()
    }

    fn without_extension(path: &str) -> &str {
        path.rsplit_once('.').map_or(path, |(stem, _)| stem)
    }

    pub(super) fn used_names(&self, forms: &[String]) -> Vec<&'a str> {
        let bound = self.bound_names(forms);
        let mentioned = self.mentioned_names(forms);
        let chosen = [mentioned, bound].into_iter().nth(usize::from(self.view));
        chosen.unwrap_or_default()
    }

    pub(super) fn hands_over(&self, forms: &[String], path: &str) -> bool {
        let handed_by_folder = self.hands_over & self.mentions(forms);
        handed_by_folder | self.text.contains(path)
    }

    fn mentions(&self, forms: &[String]) -> bool {
        forms.iter().any(|form| self.words.contains(form.as_str()))
    }

    fn read(source: &'a ProjectSource) -> Self {
        let scan = AccessScan::read(&source.text);
        Self {
            path: &source.path,
            realm: source.realm(),
            view: source.extension() == Self::VIEW_EXTENSION,
            hands_over: Self::HANDING_FOLDERS
                .iter()
                .any(|folder| source.path.contains(folder)),
            text: &source.text,
            words: scan.words().into_iter().collect(),
            accesses: scan.accesses(),
        }
    }

    fn is_counted(source: &ProjectSource) -> bool {
        let code = Self::CODE_EXTENSIONS.contains(&source.extension());
        let test = Path::new(&source.path)
            .components()
            .nth(1)
            .is_some_and(|folder| folder.as_os_str() == Self::TESTS_FOLDER);
        let generated = source.path.ends_with(Self::GENERATED_MARK);
        code & !test & !generated
    }

    fn bound_names(&self, forms: &[String]) -> Vec<&'a str> {
        self.accesses
            .iter()
            .filter(|(receiver, _)| forms.iter().any(|form| form == receiver))
            .map(|(_, name)| *name)
            .collect()
    }

    fn mentioned_names(&self, forms: &[String]) -> Vec<&'a str> {
        let mentioned = self.mentions(forms);
        let names = self.accesses.iter().map(|(_, name)| *name);
        names.filter(|_| mentioned).collect()
    }
}
