#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::receiver_forms::ReceiverForms;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct FormFiles {
    root: PathBuf,
    found: Vec<(String, String)>,
}

impl FormFiles {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const MIDDLEWARE_PREFIX: &'static str = "middleware.";
    const MAX_FILES: usize = 10_000;
    const MAX_FILE_BYTES: u64 = 256 * 1024;
    const IGNORED_FOLDERS: [&'static str; 7] = [
        "assets",
        "dist",
        "dist-bin",
        "node_modules",
        "target",
        "__pycache__",
        "vendor",
    ];

    pub(super) fn read(root: &Path) -> Vec<(String, String)> {
        let mut files = Self {
            root: root.to_path_buf(),
            found: Vec::new(),
        };
        files.walk_realms();
        files.add_middleware();
        files.found
    }

    fn walk_realms(&mut self) {
        let mut pending: Vec<PathBuf> = Self::REALMS.iter().map(PathBuf::from).collect();
        while let Some(folder) = pending.pop() {
            let children = Self::names(&self.root.join(&folder));
            pending.extend(
                children
                    .iter()
                    .filter_map(|name| self.child(&folder.join(name))),
            );
        }
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| Self::is_visible(name))
            .collect();
        names.sort();
        names
    }

    fn is_visible(name: &str) -> bool {
        !name.starts_with('.') & !Self::IGNORED_FOLDERS.contains(&name)
    }

    fn child(&mut self, relative: &Path) -> Option<PathBuf> {
        let metadata = fs::symlink_metadata(self.root.join(relative)).ok()?;
        let length = metadata.is_file().then_some(metadata.len());
        self.add(relative, length);
        metadata.is_dir().then(|| relative.to_path_buf())
    }

    fn add(&mut self, relative: &Path, length: Option<u64>) {
        let extension = relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let wanted = (CodeLanguage::of(extension).is_some() | ReceiverForms::supports(extension))
            & length.is_some_and(|bytes| bytes <= Self::MAX_FILE_BYTES)
            & (self.found.len() < Self::MAX_FILES);
        let text = wanted
            .then(|| fs::read_to_string(self.root.join(relative)).ok())
            .flatten();
        let path = relative.to_string_lossy().replace('\\', "/");
        self.found.extend(text.map(|text| (path, text)));
    }

    fn add_middleware(&mut self) {
        let names = Self::names(&self.root);
        for name in names
            .iter()
            .filter(|name| name.starts_with(Self::MIDDLEWARE_PREFIX))
        {
            self.add(Path::new(name), Some(0));
        }
    }
}
