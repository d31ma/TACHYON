use super::{AttributeValue, TemplateAttribute, TemplateNode, TemplateNodeKind, is_trivia};
use std::collections::BTreeMap;

pub(super) struct SlottedChildren {
    pub(super) unnamed: Vec<TemplateNode>,
    pub(super) named: BTreeMap<String, Vec<TemplateNode>>,
}

impl SlottedChildren {
    const SLOT_ATTRIBUTE: &'static str = "slot";

    pub(super) fn split(children: &[TemplateNode]) -> Self {
        let (named, unnamed): (Vec<&TemplateNode>, Vec<&TemplateNode>) = children
            .iter()
            .partition(|child| Self::slot_name(child).is_some());
        Self {
            unnamed: unnamed.into_iter().cloned().collect(),
            named: named.into_iter().fold(BTreeMap::new(), Self::add_named),
        }
    }

    fn add_named(
        mut named: BTreeMap<String, Vec<TemplateNode>>,
        child: &TemplateNode,
    ) -> BTreeMap<String, Vec<TemplateNode>> {
        let name = Self::slot_name(child).unwrap_or_default();
        named.entry(name).or_default().push(child.clone());
        named
    }

    pub(super) fn has_unnamed(&self) -> bool {
        self.unnamed.iter().any(|child| !is_trivia(child))
    }

    fn slot_name(child: &TemplateNode) -> Option<String> {
        let attribute = Self::attributes(child)?.get(Self::SLOT_ATTRIBUTE)?;
        Self::static_name(&attribute.value)
    }

    fn attributes(child: &TemplateNode) -> Option<&BTreeMap<String, TemplateAttribute>> {
        match &child.kind {
            TemplateNodeKind::Element { attributes, .. } => Some(attributes),
            TemplateNodeKind::Component { properties, .. } => Some(properties),
            _ => None,
        }
    }

    fn static_name(value: &AttributeValue) -> Option<String> {
        match value {
            AttributeValue::Static(name) if !name.is_empty() => Some(name.clone()),
            _ => None,
        }
    }
}
