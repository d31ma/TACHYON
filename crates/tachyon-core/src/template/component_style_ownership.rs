use super::{
    AttributeValue, Expression, TemplateAttribute, TemplateFrontend, TemplateNode,
    TemplateNodeKind, TemplateProgram,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct ComponentStyleOwnership;

impl ComponentStyleOwnership {
    const CODE: u16 = 1403;
    const MAX_DIAGNOSTICS: usize = 100;
    const REL: &'static str = "rel";
    const AS: &'static str = "as";
    const STYLE: &'static str = "style";
    const LINK: &'static str = "link";
    const STYLESHEET: &'static str = "stylesheet";
    const PRELOAD: &'static str = "preload";
    const KIND: &'static str = "k";
    const LITERAL: &'static str = "lit";
    const VALUE: &'static str = "v";
    const HELP: &'static str = "Move component CSS beside its HTML in tac.css. Declare global colors, fonts, resets and pinned vendors through client/manifest.json tac.styles and audited tac.styleSources; component HTML cannot load a separate global stylesheet.";

    pub(super) fn compile(
        path: &Path,
        source_path: &str,
        components: &BTreeSet<String>,
    ) -> Result<TemplateProgram, Failure> {
        let program = TemplateFrontend::compile_file(path, source_path, components)?;
        Self::validate(&program)?;
        Ok(program)
    }

    fn validate(program: &TemplateProgram) -> Result<(), Failure> {
        let mut found = Vec::new();
        Self::walk(&program.nodes, &mut found);
        found
            .is_empty()
            .then_some(())
            .ok_or_else(|| Failure::new(found))
    }

    fn walk(nodes: &[TemplateNode], found: &mut Vec<Diagnostic>) {
        for node in nodes {
            Self::check_node(node, found);
        }
    }

    fn check_node(node: &TemplateNode, found: &mut Vec<Diagnostic>) {
        let failure = Self::node_failure(node).filter(|_| found.len() < Self::MAX_DIAGNOSTICS);
        found.extend(failure);
        Self::walk(node.kind.children().unwrap_or_default(), found);
    }

    fn node_failure(node: &TemplateNode) -> Option<Diagnostic> {
        match &node.kind {
            TemplateNodeKind::Element { tag, .. } if tag == Self::STYLE => Some(Self::failure(node, "Component HTML cannot contain a style element outside its owned tac.css.")),
            TemplateNodeKind::Element { tag, attributes, .. } if tag == Self::LINK => Self::stylesheet_link(attributes).then(|| Self::failure(node, "Component HTML cannot declare a stylesheet or a link whose dynamic resource role may load stylesheet content.")),
            _ => None,
        }
    }

    fn stylesheet_link(attributes: &BTreeMap<String, TemplateAttribute>) -> bool {
        attributes
            .get(Self::REL)
            .is_some_and(|rel| Self::relation(rel, attributes))
    }

    fn relation(rel: &TemplateAttribute, attributes: &BTreeMap<String, TemplateAttribute>) -> bool {
        Self::literal_attribute(rel).map_or(true, |value| Self::static_relation(&value, attributes))
    }

    fn static_relation(value: &str, attributes: &BTreeMap<String, TemplateAttribute>) -> bool {
        let stylesheet = Self::has_relation(value, Self::STYLESHEET);
        let preload = Self::has_relation(value, Self::PRELOAD);
        let style = attributes.get(Self::AS).is_some_and(Self::style_resource);
        stylesheet | (preload & style)
    }

    fn has_relation(value: &str, expected: &str) -> bool {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case(expected))
    }

    fn style_resource(attribute: &TemplateAttribute) -> bool {
        Self::literal_attribute(attribute)
            .map_or(true, |value| value.trim().eq_ignore_ascii_case(Self::STYLE))
    }

    fn literal_attribute(attribute: &TemplateAttribute) -> Option<String> {
        match &attribute.value {
            AttributeValue::Static(value) => Some(value.clone()),
            AttributeValue::Dynamic(expression) => Self::literal_expression(expression),
            _ => None,
        }
    }

    fn literal_expression(expression: &Expression) -> Option<String> {
        let value: Value = serde_json::from_str(&expression.to_client_json()).ok()?;
        value
            .get(Self::KIND)?
            .as_str()
            .filter(|kind| *kind == Self::LITERAL)?;
        let literal = value.get(Self::VALUE)?;
        Some(literal.as_str().unwrap_or_default().to_owned())
    }

    fn failure(node: &TemplateNode, message: &str) -> Diagnostic {
        diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(&node.source_path, node.range.start, node.range.end),
        )
    }
}
