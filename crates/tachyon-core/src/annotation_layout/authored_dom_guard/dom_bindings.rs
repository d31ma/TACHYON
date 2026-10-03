use super::dom_kind::DomKind;
use std::collections::BTreeMap;

pub(super) struct DomBindings {
    scopes: Vec<BTreeMap<String, DomKind>>,
}

impl DomBindings {
    pub(super) fn new() -> Self {
        Self {
            scopes: vec![BTreeMap::new()],
        }
    }

    pub(super) fn kind(&self, name: &str) -> DomKind {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
            .unwrap_or_else(|| DomKind::global(name))
    }

    pub(super) fn enter(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    pub(super) fn leave(&mut self) {
        self.scopes.pop();
        self.restore_root();
    }

    pub(super) fn assign(&mut self, name: String, kind: DomKind, declaration: bool) {
        let last = self.scopes.len().saturating_sub(1);
        let scope = Self::scope(&self.scopes, &name, declaration).unwrap_or(last);
        self.scopes[scope].insert(name, kind);
    }

    fn scope(scopes: &[BTreeMap<String, DomKind>], name: &str, declaration: bool) -> Option<usize> {
        (!declaration)
            .then(|| scopes.iter().rposition(|scope| scope.contains_key(name)))
            .flatten()
    }

    fn restore_root(&mut self) {
        if self.scopes.is_empty() {
            self.scopes.push(BTreeMap::new());
        }
    }
}
