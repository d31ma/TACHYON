use super::{AttributeValue, TemplateNode, TemplateNodeKind, is_trivia};
use std::collections::BTreeMap;

pub(super) struct SlottedChildren {
    pub(super) unnamed: Vec<TemplateNode>,
    pub(super) named: BTreeMap<String, Vec<TemplateNode>>,
}

impl SlottedChildren {
    const SLOT_ATTRIBUTE: &'static str = "slot";

    pub(super) fn split(children: &[TemplateNode]) -> Self {
        let mut slotted = Self {
            unnamed: Vec::new(),
            named: BTreeMap::new(),
        };
        for child in children {
            match Self::slot_name(child) {
                Some(name) => slotted.named.entry(name).or_default().push(child.clone()),
                None => slotted.unnamed.push(child.clone()),
            }
        }
        slotted
    }

    pub(super) fn has_unnamed(&self) -> bool {
        self.unnamed.iter().any(|child| !is_trivia(child))
    }

    fn slot_name(child: &TemplateNode) -> Option<String> {
        let attributes = match &child.kind {
            TemplateNodeKind::Element { attributes, .. } => attributes,
            TemplateNodeKind::Component { properties, .. } => properties,
            _ => return None,
        };
        match &attributes.get(Self::SLOT_ATTRIBUTE)?.value {
            AttributeValue::Static(name) if !name.is_empty() => Some(name.clone()),
            _ => None,
        }
    }
}
