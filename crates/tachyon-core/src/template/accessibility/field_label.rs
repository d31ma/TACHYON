use super::{AccessibilityProblem, ViewElement};
use crate::template::TemplateNode;
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct FieldLabel {
    labelled_ids: BTreeSet<String>,
    has_dynamic_target: bool,
}

impl FieldLabel {
    const LABEL: &'static str = "label";
    const TARGET: &'static str = "for";
    const ID: &'static str = "id";
    const INPUT: &'static str = "input";
    const INPUT_KIND: &'static str = "type";
    const TEXT_KIND: &'static str = "text";
    const OTHER_FIELDS: [&'static str; 2] = ["select", "textarea"];
    const KINDS_WITHOUT_LABEL: [&'static str; 5] = ["hidden", "submit", "reset", "button", "image"];
    const HELP: &'static str = "Put the field inside a <label>, point a <label for=\"id\"> at the field's id, or give the field aria-label=\"...\" or aria-labelledby=\"id-of-a-label\". A placeholder is not a label.";

    pub(super) fn collect(nodes: &[TemplateNode]) -> Self {
        let mut labels = Self::default();
        labels.collect_all(nodes);
        labels
    }

    pub(super) fn problem(
        &self,
        element: &ViewElement,
        inside_label: bool,
    ) -> Option<AccessibilityProblem> {
        let message = format!(
            "<{}> has no label. A screen reader cannot say what to enter.",
            element.tag
        );
        self.is_unlabelled(element, inside_label)
            .then(|| AccessibilityProblem::new(element.range, message, Self::HELP))
    }

    pub(super) fn is_label(element: &ViewElement) -> bool {
        element.is(Self::LABEL)
    }

    fn collect_all(&mut self, nodes: &[TemplateNode]) {
        for node in nodes {
            self.collect_node(node);
        }
    }

    fn collect_node(&mut self, node: &TemplateNode) {
        if let Some(label) = ViewElement::of(node).filter(Self::is_label) {
            self.remember(&label);
        }
        self.collect_all(node.kind.children().unwrap_or_default());
    }

    fn is_unlabelled(&self, element: &ViewElement, inside_label: bool) -> bool {
        let labelled = inside_label | element.has_label_attribute() | self.is_target(element);
        Self::needs_label(element) && !labelled
    }

    fn remember(&mut self, label: &ViewElement) {
        self.labelled_ids
            .extend(label.text(Self::TARGET).map(String::from));
        self.has_dynamic_target |= label.is_dynamic(Self::TARGET);
    }

    fn is_target(&self, element: &ViewElement) -> bool {
        let named = element
            .text(Self::ID)
            .is_some_and(|id| self.labelled_ids.contains(id));
        let dynamic = element.is_dynamic(Self::ID) & self.has_dynamic_target;
        named | dynamic
    }

    fn needs_label(element: &ViewElement) -> bool {
        let kind = element
            .text(Self::INPUT_KIND)
            .unwrap_or(Self::TEXT_KIND)
            .to_ascii_lowercase();
        let input = element.is(Self::INPUT) & !Self::KINDS_WITHOUT_LABEL.contains(&kind.as_str());
        input | Self::OTHER_FIELDS.contains(&element.tag)
    }
}
