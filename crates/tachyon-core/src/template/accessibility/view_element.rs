use super::AttributeState;
use crate::template::model::SourceRange;
use crate::template::{AttributeValue, TemplateAttribute, TemplateNode, TemplateNodeKind};
use std::collections::BTreeMap;

pub(super) struct ViewElement<'a> {
    pub(super) tag: &'a str,
    pub(super) children: &'a [TemplateNode],
    pub(super) range: SourceRange,
    attributes: &'a BTreeMap<String, TemplateAttribute>,
}

impl<'a> ViewElement<'a> {
    const ROLE: &'static str = "role";
    const HIDDEN: &'static str = "aria-hidden";
    const LABEL: &'static str = "aria-label";
    const LABELLED_BY: &'static str = "aria-labelledby";
    const ARIA_PREFIX: &'static str = "aria-";
    const TRUE: &'static str = "true";
    const DECORATIVE_ROLES: [&'static str; 2] = ["presentation", "none"];

    pub(super) fn of(node: &'a TemplateNode) -> Option<Self> {
        Self::from_kind(&node.kind, node.range)
    }

    pub(super) fn is(&self, tag: &str) -> bool {
        self.tag == tag
    }

    pub(super) fn has(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    pub(super) fn text(&self, name: &str) -> Option<&'a str> {
        self.attributes.get(name).and_then(Self::static_text)
    }

    pub(super) fn is_dynamic(&self, name: &str) -> bool {
        self.attributes
            .get(name)
            .is_some_and(|attribute| Self::static_text(attribute).is_none())
    }

    pub(super) fn state(&self, name: &str) -> AttributeState {
        self.attributes
            .get(name)
            .map_or(AttributeState::Missing, |attribute| {
                AttributeState::of(Self::static_text(attribute))
            })
    }

    pub(super) fn range_of(&self, name: &str) -> SourceRange {
        self.attributes
            .get(name)
            .map_or(self.range, |attribute| attribute.range)
    }

    pub(super) fn is_hidden(&self) -> bool {
        self.text(Self::HIDDEN) == Some(Self::TRUE)
    }

    pub(super) fn has_role(&self) -> bool {
        self.has(Self::ROLE)
    }

    pub(super) fn is_decorative(&self) -> bool {
        let role = self.text(Self::ROLE).unwrap_or_default();
        Self::DECORATIVE_ROLES.contains(&role) | self.is_hidden()
    }

    pub(super) fn has_label_attribute(&self) -> bool {
        let label = self.state(Self::LABEL);
        let labelled_by = self.state(Self::LABELLED_BY);
        [label, labelled_by].contains(&AttributeState::Filled)
    }

    pub(super) fn aria_names(&self) -> impl Iterator<Item = &'a str> {
        self.attributes
            .keys()
            .map(String::as_str)
            .filter(|name| name.starts_with(Self::ARIA_PREFIX))
    }

    fn from_kind(kind: &'a TemplateNodeKind, range: SourceRange) -> Option<Self> {
        let TemplateNodeKind::Element {
            tag,
            attributes,
            children,
            ..
        } = kind
        else {
            return None;
        };
        Some(Self {
            tag,
            children,
            range,
            attributes,
        })
    }

    fn static_text(attribute: &'a TemplateAttribute) -> Option<&'a str> {
        let AttributeValue::Static(text) = &attribute.value else {
            return None;
        };
        Some(text)
    }
}
