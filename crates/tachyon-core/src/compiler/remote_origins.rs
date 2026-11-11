use crate::Failure;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) struct RemoteOrigins;

impl RemoteOrigins {
    const MOST_FOLDER_DEPTH: usize = 32;
    const MOST_FILE_BYTES: u64 = 1_024 * 1_024;
    const HIDDEN_PREFIX: char = '.';
    const MARKERS: [&'static str; 2] = ["HTTPClient(", "http_client("];
    const ORIGIN_KEY: &'static str = "origin";

    pub(super) fn check(root: &Path, declared: &BTreeMap<String, String>) -> Result<(), Failure> {
        let named = Self::named(root);
        named
            .iter()
            .try_for_each(|(file, name)| Self::check_declared(root, file, name, declared))
    }

    fn named(root: &Path) -> Vec<(PathBuf, String)> {
        let mut files = Vec::new();
        for folder in crate::Project::REMOTE_ROOTS {
            Self::collect(&root.join(folder), 0, &mut files);
        }
        files
            .into_iter()
            .flat_map(|file| {
                Self::names_in(&file)
                    .into_iter()
                    .map(move |name| (file.clone(), name))
            })
            .collect()
    }

    fn collect(folder: &Path, depth: usize, files: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(folder).into_iter().flatten().flatten();
        for entry in entries.filter(|entry| !Self::is_hidden(&entry.file_name())) {
            Self::collect_entry(&entry, depth, files);
        }
    }

    fn collect_entry(entry: &std::fs::DirEntry, depth: usize, files: &mut Vec<PathBuf>) {
        let kind = entry.file_type().ok();
        let file = kind.is_some_and(|kind| kind.is_file());
        let folder = kind.is_some_and(|kind| kind.is_dir());
        files.extend(file.then(|| entry.path()));
        if [folder, depth < Self::MOST_FOLDER_DEPTH] == [true, true] {
            Self::collect(&entry.path(), depth + 1, files);
        }
    }

    fn is_hidden(name: &std::ffi::OsStr) -> bool {
        name.to_string_lossy().starts_with(Self::HIDDEN_PREFIX)
    }

    fn names_in(file: &Path) -> Vec<String> {
        let small = std::fs::metadata(file).is_ok_and(|found| found.len() <= Self::MOST_FILE_BYTES);
        let text = small.then(|| std::fs::read_to_string(file).ok()).flatten();
        text.unwrap_or_default()
            .lines()
            .filter_map(Self::origin_on)
            .collect()
    }

    fn origin_on(line: &str) -> Option<String> {
        let start = Self::MARKERS
            .iter()
            .find_map(|marker| line.find(marker).map(|at| at + marker.len()))?;
        let rest = &line[start..];
        rest.match_indices(')')
            .find_map(|(end, _)| Self::origin_in(&rest[..end]))
    }

    fn origin_in(arguments: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(arguments).ok()?;
        value.get(Self::ORIGIN_KEY)?.as_str().map(str::to_owned)
    }

    fn check_declared(
        root: &Path,
        file: &Path,
        name: &str,
        declared: &BTreeMap<String, String>,
    ) -> Result<(), Failure> {
        if declared.contains_key(name) {
            return Ok(());
        }
        let written = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string();
        Err(crate::failure::Failure::one(crate::failure::diagnostic(
            1308,
            format!(
                "{written} names origin \"{name}\", but client/manifest.json has no tac.yon.origins.{name}."
            ),
            Some(format!(
                "Add the origin and its host, for example {{\"tac\": {{\"yon\": {{\"origins\": {{\"{name}\": \"https://rt.example.com\"}}}}, \"outgoing\": {{\"hosts\": [\"rt.example.com\"]}}}}}}."
            )),
            None,
        )))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RemoteOrigins;
    use std::collections::BTreeMap;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in files {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
            std::fs::write(path, text).expect("file");
        }
        root
    }

    fn declared() -> BTreeMap<String, String> {
        BTreeMap::from([(
            String::from("realtime"),
            String::from("https://rt.example.com"),
        )])
    }

    #[test]
    fn a_build_accepts_clients_in_every_language_that_name_a_declared_origin() {
        let written = r#"{"path": "/chat", "origin": "realtime"}"#;
        let files = [
            (
                "client/remotes/A.ts",
                format!("@HTTPClient({written})\nexport class A {{}}\n"),
            ),
            (
                "client/remotes/B.go",
                format!("// @HTTPClient({written})\ntype B interface {{}}\n"),
            ),
            (
                "client/remotes/C.rs",
                format!("#[HTTPClient({written})]\ntrait C {{}}\n"),
            ),
            (
                "client/remotes/D.cs",
                format!("[HTTPClient({written})]\npublic interface D {{}}\n"),
            ),
            (
                "client/remotes/E.py",
                format!("@http_client({written})\nclass E(Protocol): ...\n"),
            ),
            (
                "client/remotes/F.lua",
                format!("-- @HTTPClient({written})\nF = {{}}\n"),
            ),
            (
                "client/remotes/G.hpp",
                format!("class [[HTTPClient({written})]] G {{\n}};\n"),
            ),
            (
                "client/remotes/H.fs",
                format!("[<HTTPClient({written})>]\ntype H =\n    abstract X: unit -> unit\n"),
            ),
            (
                "client/clients/nested/I.rb",
                format!("#[HTTPClient({written})]\nclass I\nend\n"),
            ),
        ];
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        let root = project(&files);
        assert!(RemoteOrigins::check(root.path(), &declared()).is_ok());
    }

    #[test]
    fn a_build_refuses_a_client_that_names_an_undeclared_origin() {
        for (path, text) in [
            (
                "client/remotes/A.ts",
                "@HTTPClient({\"path\": \"/chat\", \"origin\": \"missing\"})\nexport class A {}\n",
            ),
            (
                "client/remotes/G.hpp",
                "class [[HTTPClient({\"origin\": \"missing\"})]] G {\n};\n",
            ),
            (
                "client/remotes/H.fs",
                "[<HTTPClient({\"origin\": \"missing\"})>]\ntype H =\n    abstract X: unit -> unit\n",
            ),
        ] {
            let root = project(&[(path, text)]);
            let refused = RemoteOrigins::check(root.path(), &declared())
                .expect_err(path)
                .to_string();
            assert!(refused.contains("TY1308"), "{refused}");
            assert!(
                refused.contains(&format!("{path} names origin \"missing\"")),
                "{refused}"
            );
            assert!(refused.contains("tac.yon.origins.missing"), "{refused}");
        }
    }
}
