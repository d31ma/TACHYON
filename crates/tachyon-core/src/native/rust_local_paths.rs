use crate::Failure;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

pub(super) struct RustLocalPaths {
    names: BTreeSet<String>,
    models: bool,
    utilities: BTreeSet<String>,
    path: PathBuf,
}

impl RustLocalPaths {
    pub(super) fn read(
        path: &Path,
        source: &str,
        models: &str,
        utilities: &BTreeSet<String>,
    ) -> Result<Self, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        let mut names = crate::companion::RustGraph::type_names(&masked);
        names.extend(utilities.iter().cloned());
        let models = crate::annotation_layout::masked_source(path, models)?;
        let models = crate::companion::RustGraph::declares_module(&models, "models");
        Ok(Self {
            names,
            models,
            utilities: utilities.clone(),
            path: path.to_path_buf(),
        })
    }

    pub(super) fn source(&self, source: &str) -> Result<String, Failure> {
        let masked = crate::annotation_layout::masked_source(&self.path, source)?;
        let mut rewritten = source.to_owned();
        self.references(&masked).into_iter().rev().for_each(|at| {
            let prefix = self.prefix(&masked, at);
            rewritten.replace_range(at..at + prefix.len(), "crate::TachyonRustServices");
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
        let name = Self::reference_name(masked, at);
        let known = [
            self.names.contains(name),
            [self.models, name == "models"] == [true; 2],
            self.utility_name(&masked[at..]).is_some(),
        ]
        .contains(&true);
        [standalone, known].into_iter().all(|valid| valid)
    }

    fn reference_name(masked: &str, at: usize) -> &str {
        let name = masked[at + "crate::".len()..]
            .bytes()
            .take_while(|byte| Self::identifier_byte(*byte))
            .count();
        &masked[at + "crate::".len()..at + "crate::".len() + name]
    }

    fn prefix(&self, masked: &str, at: usize) -> &'static str {
        if self.utility_name(&masked[at..]).is_some() {
            "crate::utils"
        } else {
            "crate"
        }
    }

    fn utility_name<'a>(&self, path: &'a str) -> Option<&'a str> {
        let name = path.strip_prefix("crate::utils::")?;
        let length = name
            .bytes()
            .take_while(|byte| Self::identifier_byte(*byte))
            .count();
        let name = &name[..length];
        self.utilities.contains(name).then_some(name)
    }

    fn identifier_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    pub(super) fn import(&self, statement: &str) -> Option<String> {
        let local = statement
            .strip_prefix("use crate::")
            .map(|path| self.utility_import(path));
        local.map_or_else(
            || Some(statement.to_owned()),
            |path| self.local_import(path),
        )
    }

    fn local_import(&self, path: &str) -> Option<String> {
        let path = path.trim_end_matches(';');
        let (name, alias) = path
            .split_once(" as ")
            .map_or((path, None), |(name, alias)| (name, Some(alias)));
        match (self.models, name, self.names.contains(name), alias) {
            (true, "models", _, None | Some("models")) => None,
            (true, "models", _, _) => Some(format!("use crate::TachyonRustServices::{path};")),
            (true, name, _, _) if name.starts_with("models::") => {
                Some(format!("use crate::TachyonRustServices::{path};"))
            }
            (_, _, false, _) => Some(format!("use crate::{path};")),
            (_, _, true, None) => None,
            (_, _, true, Some(alias)) if alias == name => None,
            (_, _, true, Some(alias)) => Some(format!("use self::{name} as {alias};")),
        }
    }

    fn utility_import<'a>(&self, path: &'a str) -> &'a str {
        let qualified = format!("crate::{path}");
        if self.utility_name(&qualified).is_some() {
            return path.trim_start_matches("utils::");
        }
        path
    }
}
