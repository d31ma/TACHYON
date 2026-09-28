use super::{
    AccessibilityProblem, AccessibleName, AriaAttribute, ElementRules, FieldLabel, HeadingOrder,
    ViewElement,
};
use crate::template::{TemplateNode, TemplateProgram};
use tachyon_diagnostics::Diagnostic;

pub(super) struct ViewAudit {
    labels: FieldLabel,
    headings: HeadingOrder,
    problems: Vec<AccessibilityProblem>,
}

impl ViewAudit {
    pub(super) fn diagnostics(program: &TemplateProgram) -> Vec<Diagnostic> {
        let mut audit = Self {
            labels: FieldLabel::collect(&program.nodes),
            headings: HeadingOrder::default(),
            problems: Vec::new(),
        };
        audit.visit_all(&program.nodes, false);
        audit
            .problems
            .iter()
            .map(|problem| problem.diagnostic(&program.source_path))
            .collect()
    }

    fn visit_all(&mut self, nodes: &[TemplateNode], inside_label: bool) {
        for node in nodes {
            self.visit(node, inside_label);
        }
    }

    fn visit(&mut self, node: &TemplateNode, inside_label: bool) {
        let element = ViewElement::of(node);
        if let Some(found) = &element {
            self.check(found, inside_label);
        }
        let starts_label = element.as_ref().is_some_and(FieldLabel::is_label);
        let children = node.kind.children().unwrap_or_default();
        self.visit_all(children, inside_label | starts_label);
    }

    fn check(&mut self, element: &ViewElement, inside_label: bool) {
        let found = [
            ElementRules::image(element),
            ElementRules::frame(element),
            ElementRules::tab_order(element),
            AccessibleName::problem(element),
            self.labels.problem(element, inside_label),
            self.headings.problem(element),
        ];
        self.problems.extend(found.into_iter().flatten());
        self.problems.extend(AriaAttribute::problems(element));
    }
}
