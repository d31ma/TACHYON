#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::name_forms::NameForms;
use super::need::Need;
use super::role_layer::RoleLayer;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) struct DependencyGraph {
    pub(super) names: Vec<String>,
    pub(super) folders: Vec<String>,
    pub(super) edges: Vec<Vec<usize>>,
    pub(super) needs: Vec<Need>,
    pub(super) utility_needs: Vec<Need>,
}

impl DependencyGraph {
    pub(super) fn read(classes: &[ClassFile]) -> Self {
        let nodes = Self::nodes(classes);
        let keys: Vec<String> = nodes.keys().cloned().collect();
        let forms = Self::forms(classes);
        let needs = Self::needs(classes, &forms, &keys, Self::injected);
        let utility_needs = Self::needs(classes, &forms, &keys, Self::utility);
        Self {
            names: keys.iter().map(|key| Self::name_of(key)).collect(),
            folders: nodes.into_values().collect(),
            edges: Self::edges(keys.len(), &needs),
            needs,
            utility_needs,
        }
    }

    pub(super) fn need_between(&self, from: usize, to: usize) -> Option<&Need> {
        self.needs
            .iter()
            .find(|need| (need.from == from) & (need.to == to))
    }

    fn nodes(classes: &[ClassFile]) -> BTreeMap<String, String> {
        let mut nodes = BTreeMap::new();
        for class in classes.iter().filter(|class| Self::injected(class)) {
            nodes
                .entry(Self::key(class.realm, class.name))
                .or_insert_with(|| class.folder.to_owned());
        }
        nodes
    }

    fn forms(classes: &[ClassFile]) -> HashMap<String, String> {
        classes
            .iter()
            .filter(|class| Self::injected(class))
            .flat_map(|class| {
                NameForms::of(class.name)
                    .map(|form| (Self::key(class.realm, &form), class.name.to_owned()))
            })
            .collect()
    }

    fn needs(
        classes: &[ClassFile],
        forms: &HashMap<String, String>,
        keys: &[String],
        source: fn(&ClassFile) -> bool,
    ) -> Vec<Need> {
        classes
            .iter()
            .enumerate()
            .filter(|(_, class)| source(class))
            .flat_map(|(index, class)| Self::class_needs(index, class, forms, keys))
            .collect()
    }

    fn class_needs(
        index: usize,
        class: &ClassFile,
        forms: &HashMap<String, String>,
        keys: &[String],
    ) -> Vec<Need> {
        let from = Self::position(keys, &Self::key(class.realm, class.name));
        let mut seen = BTreeSet::new();
        (0..class.words.len())
            .filter_map(|at| Self::target_at(class, at, forms).map(|target| (target, at)))
            .filter(|(target, _)| seen.insert(target.clone()))
            .map(|(target, at)| Need {
                class: index,
                from,
                to: Self::position(keys, &Self::key(class.realm, &target)),
                at,
            })
            .collect()
    }

    fn target_at(class: &ClassFile, at: usize, forms: &HashMap<String, String>) -> Option<String> {
        let name = class.words.is_name(at).then(|| class.words.text(at))?;
        let target = forms.get(&Self::key(class.realm, name))?;
        (target != class.name).then(|| target.clone())
    }

    fn edges(count: usize, needs: &[Need]) -> Vec<Vec<usize>> {
        let edges = Self::linked(count, needs);
        Self::sorted(edges)
    }

    fn linked(count: usize, needs: &[Need]) -> Vec<Vec<usize>> {
        let mut edges = vec![Vec::new(); count];
        for need in needs {
            edges[need.from].push(need.to);
        }
        edges
    }

    fn sorted(mut edges: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        for targets in &mut edges {
            targets.sort_unstable();
            targets.dedup();
        }
        edges
    }

    fn injected(class: &ClassFile) -> bool {
        RoleLayer::of(class.folder).is_injected()
    }

    fn utility(class: &ClassFile) -> bool {
        RoleLayer::of(class.folder).is_utility()
    }

    fn key(realm: &str, name: &str) -> String {
        format!("{realm}/{name}")
    }

    fn name_of(key: &str) -> String {
        key.rsplit('/').next().unwrap_or(key).to_owned()
    }

    fn position(keys: &[String], key: &str) -> usize {
        keys.binary_search_by(|probe| probe.as_str().cmp(key))
            .unwrap_or_default()
    }
}
