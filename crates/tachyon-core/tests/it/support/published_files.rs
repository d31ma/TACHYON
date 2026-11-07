use std::fs;
use std::path::{Path, PathBuf};

pub struct PublishedFiles;

impl PublishedFiles {
    pub fn folder(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content")
            .join(relative)
    }

    pub fn read(folder: &Path, realm: &str) -> Vec<(String, String)> {
        let mut found = Vec::new();
        Self::collect(folder, folder, realm, &mut found);
        found
    }

    fn collect(base: &Path, folder: &Path, realm: &str, found: &mut Vec<(String, String)>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(folder)
            .expect("published folder")
            .map(|entry| entry.expect("folder entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                Self::collect(base, &path, realm, found);
                continue;
            }
            let relative = path
                .strip_prefix(base)
                .expect("inside the published folder")
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with(realm) {
                found.push((relative, fs::read_to_string(&path).expect("source")));
            }
        }
    }
}
