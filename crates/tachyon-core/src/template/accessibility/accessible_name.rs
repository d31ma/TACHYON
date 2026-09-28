use super::{AccessibilityProblem, AttributeState, ViewElement};
use crate::template::{TemplateNode, TemplateNodeKind, is_trivia};

pub(super) struct AccessibleName;

impl AccessibleName {
    const BUTTON: &'static str = "button";
    const LINK: &'static str = "a";
    const LINK_TARGET: &'static str = "href";
    const IMAGE: &'static str = "img";
    const IMAGE_TEXT: &'static str = "alt";
    const CUSTOM_ELEMENT_MARK: char = '-';
    const HELP: &'static str = "Put text inside the element, or give it aria-label=\"...\" or aria-labelledby=\"id-of-a-label\". A picture inside it counts when its alt is not empty.";

    pub(super) fn problem(element: &ViewElement) -> Option<AccessibilityProblem> {
        let unnamed = Self::needs_name(element) & !Self::has_name(element);
        let message = format!(
            "<{}> has no accessible name. A screen reader cannot say what it does.",
            element.tag
        );
        unnamed.then(|| AccessibilityProblem::new(element.range, message, Self::HELP))
    }

    fn needs_name(element: &ViewElement) -> bool {
        let link = element.is(Self::LINK) & element.has(Self::LINK_TARGET);
        element.is(Self::BUTTON) | link
    }

    fn has_name(element: &ViewElement) -> bool {
        element.has_label_attribute() | Self::has_content(element.children)
    }

    fn has_content(nodes: &[TemplateNode]) -> bool {
        nodes.iter().any(Self::gives_name)
    }

    fn gives_name(node: &TemplateNode) -> bool {
        match &node.kind {
            TemplateNodeKind::Text(_) | TemplateNodeKind::Comment(_) => !is_trivia(node),
            TemplateNodeKind::Element { .. } => {
                ViewElement::of(node).is_some_and(|element| Self::element_gives_name(&element))
            }
            TemplateNodeKind::Component { .. } | TemplateNodeKind::Slot => true,
            _ => Self::has_content(node.kind.children().unwrap_or_default()),
        }
    }

    fn element_gives_name(element: &ViewElement) -> bool {
        let image =
            element.is(Self::IMAGE) & (element.state(Self::IMAGE_TEXT) == AttributeState::Filled);
        let custom = element.tag.contains(Self::CUSTOM_ELEMENT_MARK);
        let named = element.has_label_attribute() | Self::has_content(element.children);
        image | custom | named
    }
}
