use super::super::{Input, Marker};
use super::{CsharpHelperInterfaces, CsharpTacHelpers, Type};
use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) struct CsharpHelperGraph {
    helpers: BTreeSet<String>,
    constructors: BTreeMap<String, BTreeSet<String>>,
    references: BTreeMap<String, BTreeSet<String>>,
}

impl CsharpHelperGraph {
    const LANGUAGE: &'static str = "cs";

    pub(crate) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        Self::parsed(path, source)
    }

    pub(crate) fn helpers(&self) -> &BTreeSet<String> {
        &self.helpers
    }

    pub(crate) fn owned(&self, roots: &BTreeSet<String>) -> BTreeSet<String> {
        Self::closure(roots, &self.constructors)
    }

    pub(crate) fn referenced(&self, roots: &BTreeSet<String>) -> BTreeSet<String> {
        Self::closure(roots, &self.references)
    }

    fn parsed(path: &Path, source: &str) -> Result<Self, Failure> {
        CsharpTacHelpers::source(path, source)?;
        let mask = crate::lexical::code_mask(Self::LANGUAGE, source);
        let mut input = Input {
            path,
            source,
            code: super::visible_code(source, mask.code),
        };
        let markers = super::annotations(&input, Self::LANGUAGE, "tac")?;
        super::mask_annotations(&mut input, &markers, Self::LANGUAGE);
        let types = super::declarations(&input, Self::LANGUAGE)?;
        let helpers: Vec<&Type> = types
            .iter()
            .filter(|ty| Self::helper_type(ty, &markers))
            .collect();
        helpers
            .iter()
            .try_for_each(|ty| CsharpHelperInterfaces::require(&input, ty))?;
        let mut graph = Self {
            helpers: helpers.iter().map(|ty| ty.name.clone()).collect(),
            constructors: BTreeMap::new(),
            references: BTreeMap::new(),
        };
        types.iter().for_each(|ty| graph.add_type(&input, ty));
        Ok(graph)
    }

    fn helper_type(ty: &Type, markers: &[Marker]) -> bool {
        markers.iter().any(|marker| Self::marks_helper(marker, ty))
    }

    fn marks_helper(marker: &Marker, ty: &Type) -> bool {
        (marker.name.as_str(), marker.target) == ("Helper", ty.declaration)
    }

    fn add_type(&mut self, input: &Input<'_>, ty: &Type) {
        let body = &input.code[ty.body.clone()];
        let other_helpers: BTreeSet<String> = self
            .helpers
            .iter()
            .filter(|name| *name != &ty.name)
            .cloned()
            .collect();
        let references: BTreeSet<String> = other_helpers
            .iter()
            .filter(|name| Self::mentions(body, name))
            .cloned()
            .collect();
        let constructors: BTreeSet<String> = other_helpers
            .iter()
            .filter(|name| CsharpTacHelpers::external_new(body, name))
            .cloned()
            .collect();
        self.references.insert(ty.name.clone(), references);
        self.constructors.insert(ty.name.clone(), constructors);
    }

    fn mentions(body: &str, name: &str) -> bool {
        body.split(|character: char| !character.is_alphanumeric() && character != '_')
            .any(|word| word == name)
    }

    fn closure(
        roots: &BTreeSet<String>,
        edges: &BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let mut found = roots.clone();
        let mut pending: Vec<String> = roots.iter().cloned().collect();
        while let Some(name) = pending.pop() {
            Self::visit(edges.get(&name), &mut found, &mut pending);
        }
        found
    }

    fn visit(
        children: Option<&BTreeSet<String>>,
        found: &mut BTreeSet<String>,
        pending: &mut Vec<String>,
    ) {
        children.into_iter().flatten().for_each(|name| {
            Self::enqueue(name, found, pending);
        });
    }

    fn enqueue(name: &str, found: &mut BTreeSet<String>, pending: &mut Vec<String>) {
        if found.insert(name.to_owned()) {
            pending.push(name.to_owned());
        }
    }
}
