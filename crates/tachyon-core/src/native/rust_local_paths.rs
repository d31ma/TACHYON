use crate::Failure;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) struct RustLocalPaths {
    names: BTreeSet<String>,
    path: PathBuf,
}

impl RustLocalPaths {
    pub(super) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        let names = crate::companion::RustGraph::type_names(&masked);
        Ok(Self {
            names,
            path: path.to_path_buf(),
        })
    }

    pub(super) fn source(&self, source: &str) -> Result<String, Failure> {
        let masked = crate::annotation_layout::masked_source(&self.path, source)?;
        let mut rewritten = source.to_owned();
        self.references(&masked).into_iter().rev().for_each(|at| {
            rewritten.replace_range(at..at + "crate".len(), "self");
        });
        Ok(rewritten)
    }

    fn references(&self, masked: &str) -> Vec<usize> {
        masked
            .match_indices("crate::")
            .map(|(at, _)| at)
            .filter(|at| self.local_reference(masked, *at))
            .collect()
    }

    fn local_reference(&self, masked: &str, at: usize) -> bool {
        let preceding = masked[..at].bytes().next_back();
        let standalone = preceding.is_none_or(|byte| !Self::identifier_byte(byte));
        let name = masked[at + "crate::".len()..]
            .bytes()
            .take_while(|byte| Self::identifier_byte(*byte))
            .count();
        let name = &masked[at + "crate::".len()..at + "crate::".len() + name];
        [standalone, self.names.contains(name)]
            .into_iter()
            .all(|valid| valid)
    }

    fn identifier_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    pub(super) fn import(&self, statement: &str) -> Option<String> {
        statement.strip_prefix("use crate::").map_or_else(
            || Some(statement.to_owned()),
            |path| self.local_import(path),
        )
    }

    fn local_import(&self, path: &str) -> Option<String> {
        let path = path.trim_end_matches(';');
        let (name, alias) = path
            .split_once(" as ")
            .map_or((path, None), |(name, alias)| (name, Some(alias)));
        match (self.names.contains(name), alias) {
            (false, _) => Some(format!("use crate::{path};")),
            (true, None) => None,
            (true, Some(alias)) if alias == name => None,
            (true, Some(alias)) => Some(format!("use self::{name} as {alias};")),
        }
    }
}
