use super::project_source::ProjectSource;
use std::fs;
use std::path::{Path, PathBuf};

pub(in super::super) struct ProjectFiles {
    root: PathBuf,
    found: Vec<ProjectSource>,
}

impl ProjectFiles {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const MIDDLEWARE_PREFIX: &'static str = "middleware.";
    const MAX_FILES: usize = 10_000;
    const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
    const IGNORED_FOLDERS: [&'static str; 7] = [
        ".git",
        ".tachyon",
        "dist",
        "dist-bin",
        "node_modules",
        "target",
        "__pycache__",
    ];

    pub(in super::super) fn read(root: &Path) -> Vec<ProjectSource> {
        let mut files = Self {
            root: root.to_path_buf(),
            found: Vec::new(),
        };
        files.walk_realms();
        files.add_middleware();
        files.found
    }

    fn walk_realms(&mut self) {
        for realm in Self::REALMS {
            self.walk(Path::new(realm));
        }
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| !Self::IGNORED_FOLDERS.contains(&name.as_str()))
            .collect();
        names.sort();
        names
    }

    fn walk(&mut self, folder: &Path) {
        for name in Self::names(&self.root.join(folder)) {
            self.visit(&folder.join(name));
        }
    }

    fn visit(&mut self, relative: &Path) {
        let kind = fs::symlink_metadata(self.root.join(relative))
            .ok()
            .map(|metadata| (metadata.is_dir(), metadata.is_file(), metadata.len()));
        match kind {
            Some((true, _, _)) => self.walk(relative),
            Some((false, true, length)) => self.add(relative, length),
            _ => {}
        }
    }

    fn add(&mut self, relative: &Path, length: u64) {
        let wanted = (length <= Self::MAX_FILE_BYTES) & (self.found.len() < Self::MAX_FILES);
        let text = wanted
            .then(|| fs::read_to_string(self.root.join(relative)).ok())
            .flatten();
        self.found
            .extend(text.map(|text| ProjectSource::new(relative, text)));
    }

    fn add_middleware(&mut self) {
        let names = Self::names(&self.root);
        for name in names
            .iter()
            .filter(|name| name.starts_with(Self::MIDDLEWARE_PREFIX))
        {
            self.add(Path::new(name), 0);
        }
    }
}
