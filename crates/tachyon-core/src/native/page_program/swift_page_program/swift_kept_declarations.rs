use crate::Failure;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::Path;

#[derive(Default)]
pub(super) struct SwiftKeptDeclarations {
    kept: BTreeMap<(Option<String>, Vec<String>), String>,
    source: String,
    conflicting: bool,
}

impl SwiftKeptDeclarations {
    pub(super) fn with(self, identity: &Path, declaration: &str) -> Result<Self, Failure> {
        let declared = crate::annotation_layout::declaration_names(identity, declaration)?;
        Ok(self.keep(declared, declaration))
    }

    fn keep(mut self, declared: Vec<String>, declaration: &str) -> Self {
        let text = declaration.trim().to_owned();
        let key = (declared.is_empty().then(|| text.clone()), declared);
        match self.kept.entry(key) {
            Entry::Occupied(seen) => self.conflicting |= *seen.get() != text,
            Entry::Vacant(slot) => {
                slot.insert(text);
                self.source.push_str(declaration);
                self.source.push('\n');
            }
        }
        self
    }

    pub(super) fn source(self) -> Option<String> {
        (!self.conflicting).then_some(self.source)
    }
}
