use super::{AccessibilityProblem, AttributeState, ViewElement};

pub(super) struct ElementRules;

impl ElementRules {
    const IMAGE: &'static str = "img";
    const FRAME: &'static str = "iframe";
    const ALT: &'static str = "alt";
    const TITLE: &'static str = "title";
    const TAB_INDEX: &'static str = "tabindex";
    const IMAGE_HELP: &'static str = "Describe the picture in alt, as in alt=\"A red bicycle\". A picture that only decorates the page takes alt=\"\" together with role=\"presentation\" or aria-hidden=\"true\".";
    const FRAME_HELP: &'static str = "Say what the frame shows in title, as in title=\"Map of the store\", or in aria-label. Screen readers read this name before they enter the frame.";
    const TAB_HELP: &'static str = "Use tabindex=\"0\" to let the keyboard reach the element in page order, or tabindex=\"-1\" to reach it only from code. Change the order of the elements in the view to change the keyboard order.";

    pub(super) fn image(element: &ViewElement) -> Option<AccessibilityProblem> {
        Self::only_for(element, Self::IMAGE, Self::image_problem)
    }

    pub(super) fn frame(element: &ViewElement) -> Option<AccessibilityProblem> {
        Self::only_for(element, Self::FRAME, Self::frame_problem)
    }

    pub(super) fn tab_order(element: &ViewElement) -> Option<AccessibilityProblem> {
        let order = element.text(Self::TAB_INDEX)?.trim().parse::<i64>().ok()?;
        let message = format!(
            "<{}> has tabindex=\"{order}\". A tabindex above 0 moves the element out of the keyboard order of the page.",
            element.tag
        );
        let range = element.range_of(Self::TAB_INDEX);
        order
            .is_positive()
            .then(|| AccessibilityProblem::new(range, message, Self::TAB_HELP))
    }

    fn only_for(
        element: &ViewElement,
        tag: &str,
        rule: fn(&ViewElement) -> Option<AccessibilityProblem>,
    ) -> Option<AccessibilityProblem> {
        element.is(tag).then(|| rule(element)).flatten()
    }

    fn image_problem(element: &ViewElement) -> Option<AccessibilityProblem> {
        match element.state(Self::ALT) {
            AttributeState::Missing => Some(Self::image_text_problem(element, "has no alt text")),
            AttributeState::Blank => Self::blank_image_text(element),
            AttributeState::Filled => None,
        }
    }

    fn blank_image_text(element: &ViewElement) -> Option<AccessibilityProblem> {
        let problem = Self::image_text_problem(
            element,
            "has an empty alt, but it is not marked as decorative",
        );
        (!element.is_decorative()).then_some(problem)
    }

    fn image_text_problem(element: &ViewElement, fault: &str) -> AccessibilityProblem {
        let message = format!("An <img> {fault}. A screen reader cannot say what it shows.");
        AccessibilityProblem::new(element.range, message, Self::IMAGE_HELP)
    }

    fn frame_problem(element: &ViewElement) -> Option<AccessibilityProblem> {
        let message = String::from(
            "An <iframe> has no title. A screen reader cannot say what the frame shows.",
        );
        let problem = AccessibilityProblem::new(element.range, message, Self::FRAME_HELP);
        let titled = element.state(Self::TITLE) == AttributeState::Filled;
        (!titled & !element.has_label_attribute()).then_some(problem)
    }
}
