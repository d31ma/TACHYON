use super::FileReference;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct LoadedFiles<'a> {
    files: &'a BTreeMap<String, &'a [u8]>,
    found: BTreeSet<String>,
    waiting: Vec<String>,
}

impl<'a> LoadedFiles<'a> {
    const SERVICE_WORKER: &'static str = "tachyon-sw.js";

    pub(super) fn of(page: &str, files: &'a BTreeMap<String, &'a [u8]>) -> BTreeSet<String> {
        let mut loaded = Self {
            files,
            found: BTreeSet::new(),
            waiting: vec![String::from(page)],
        };
        loaded.walk();
        loaded.found
    }

    fn walk(&mut self) {
        while let Some(path) = self.waiting.pop() {
            self.visit(&path);
        }
    }

    fn visit(&mut self, path: &str) {
        if self.found.insert(path.to_owned()) {
            let next = self.references(path);
            self.waiting.extend(next);
        }
    }

    fn references(&self, path: &str) -> Vec<String> {
        let bytes = self.files.get(path).copied().unwrap_or_default();
        FileReference::resolved(path, bytes)
            .into_iter()
            .filter(|reference| self.is_loaded(reference))
            .collect()
    }

    fn is_loaded(&self, reference: &str) -> bool {
        self.files.contains_key(reference) && reference != Self::SERVICE_WORKER
    }
}
