use super::slotted_children::SlottedChildren;
use super::{TemplateNode, TemplateNodeKind};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

pub(super) struct ComponentSlots {
    unnamed: bool,
    named: BTreeSet<String>,
}

impl ComponentSlots {
    const CODE: u16 = 1404;

    pub(super) fn of(template_nodes: &[TemplateNode]) -> Self {
        let mut slots = Self {
            unnamed: false,
            named: BTreeSet::new(),
        };
        slots.collect(template_nodes);
        slots
    }

    pub(super) fn check(
        &self,
        node: &TemplateNode,
        component: &str,
        children: &SlottedChildren,
    ) -> Result<(), Failure> {
        self.check_unnamed(node, component, children)?;
        self.check_named(node, component, children)
    }

    fn check_unnamed(
        &self,
        node: &TemplateNode,
        component: &str,
        children: &SlottedChildren,
    ) -> Result<(), Failure> {
        let refused = [children.has_unnamed(), self.unnamed] == [true, false];
        (!refused).then_some(()).ok_or_else(|| {
            Self::failure(
                node,
                format!("Component '<{component}>' received children but declares no <slot>."),
            )
        })
    }

    fn check_named(
        &self,
        node: &TemplateNode,
        component: &str,
        children: &SlottedChildren,
    ) -> Result<(), Failure> {
        let missing = children
            .named
            .keys()
            .find(|name| !self.named.contains(*name));
        missing.map_or(Ok(()), |name| {
            Err(Self::failure(
                node,
                format!(
                    "Component '<{component}>' received children for slot '{name}' but declares no <slot name=\"{name}\">."
                ),
            ))
        })
    }

    pub(super) fn native_unnamed(&self) -> Vec<Value> {
        self.unnamed
            .then(|| Self::native_slot(None))
            .into_iter()
            .collect()
    }

    pub(super) fn native_named(&self) -> Map<String, Value> {
        self.named
            .iter()
            .map(|name| (name.clone(), json!([Self::native_slot(Some(name))])))
            .collect()
    }

    fn native_slot(name: Option<&str>) -> Value {
        let attributes: Vec<Value> = name
            .map(|name| json!({ "name": "name", "value": name }))
            .into_iter()
            .collect();
        json!({ "k": "element", "tag": "slot", "attributes": attributes, "children": [], "void": false })
    }

    fn collect(&mut self, nodes: &[TemplateNode]) {
        nodes.iter().for_each(|node| self.add(&node.kind));
    }

    fn add(&mut self, kind: &TemplateNodeKind) {
        match kind {
            TemplateNodeKind::Slot(Some(name)) => {
                self.named.insert(name.clone());
            }
            TemplateNodeKind::Slot(None) => self.unnamed = true,
            kind => self.collect(kind.children().unwrap_or_default()),
        }
    }

    fn failure(node: &TemplateNode, message: String) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            None,
            source_span(&node.source_path, node.range.start, node.range.end),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "component_slots_tests.rs"]
mod tests;
