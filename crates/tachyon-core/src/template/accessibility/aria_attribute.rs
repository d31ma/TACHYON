use super::{AccessibilityProblem, ViewElement};

pub(super) struct AriaAttribute;

impl AriaAttribute {
    const TAB_INDEX: &'static str = "tabindex";
    const DISABLED: &'static str = "disabled";
    const LINK: &'static str = "a";
    const LINK_TARGET: &'static str = "href";
    const FOCUSABLE_TAGS: [&'static str; 4] = ["button", "input", "select", "textarea"];
    const TAGS_WITHOUT_NAME: [&'static str; 12] = [
        "div", "span", "p", "b", "i", "u", "s", "em", "strong", "code", "sub", "sup",
    ];
    const TRUE_OR_FALSE: [&'static str; 2] = ["true", "false"];
    const TRUE_FALSE_OR_MIXED: [&'static str; 3] = ["true", "false", "mixed"];
    const TRUE_OR_FALSE_NAMES: [&'static str; 9] = [
        "aria-atomic",
        "aria-busy",
        "aria-disabled",
        "aria-hidden",
        "aria-modal",
        "aria-multiline",
        "aria-multiselectable",
        "aria-readonly",
        "aria-required",
    ];
    const TRUE_FALSE_OR_MIXED_NAMES: [&'static str; 2] = ["aria-checked", "aria-pressed"];
    const KNOWN: [&'static str; 53] = [
        "aria-activedescendant",
        "aria-atomic",
        "aria-autocomplete",
        "aria-braillelabel",
        "aria-brailleroledescription",
        "aria-busy",
        "aria-checked",
        "aria-colcount",
        "aria-colindex",
        "aria-colindextext",
        "aria-colspan",
        "aria-controls",
        "aria-current",
        "aria-describedby",
        "aria-description",
        "aria-details",
        "aria-disabled",
        "aria-dropeffect",
        "aria-errormessage",
        "aria-expanded",
        "aria-flowto",
        "aria-grabbed",
        "aria-haspopup",
        "aria-hidden",
        "aria-invalid",
        "aria-keyshortcuts",
        "aria-label",
        "aria-labelledby",
        "aria-level",
        "aria-live",
        "aria-modal",
        "aria-multiline",
        "aria-multiselectable",
        "aria-orientation",
        "aria-owns",
        "aria-placeholder",
        "aria-posinset",
        "aria-pressed",
        "aria-readonly",
        "aria-relevant",
        "aria-required",
        "aria-roledescription",
        "aria-rowcount",
        "aria-rowindex",
        "aria-rowindextext",
        "aria-rowspan",
        "aria-selected",
        "aria-setsize",
        "aria-sort",
        "aria-valuemax",
        "aria-valuemin",
        "aria-valuenow",
        "aria-valuetext",
    ];
    const NAME_HELP: &'static str = "Use a name from the WAI-ARIA 1.2 list, such as aria-label, aria-describedby or aria-expanded. Check the spelling: ARIA names are lower case, as in aria-labelledby.";
    const VALUE_HELP: &'static str = "Write one of the values that the message lists. A value in :attr=\"...\" is checked by the browser, not by the build.";
    const HIDDEN_HELP: &'static str = "Remove aria-hidden=\"true\", or take the element out of the keyboard order with tabindex=\"-1\" or disabled. A keyboard user must not land on an element that a screen reader does not announce.";
    const NAMING_HELP: &'static str = "Give the element a role that takes a name, such as role=\"region\" or role=\"group\", use an element that takes a name, such as <section> or <nav>, or show the text on the page instead.";

    pub(super) fn problems(element: &ViewElement) -> Vec<AccessibilityProblem> {
        let mut found: Vec<AccessibilityProblem> = element
            .aria_names()
            .filter(|name| !Self::KNOWN.contains(name))
            .map(|name| Self::unknown_problem(element, name))
            .collect();
        found.extend(
            element
                .aria_names()
                .filter_map(|name| Self::value_problem(element, name)),
        );
        found.extend(Self::hidden_problem(element));
        found.extend(Self::naming_problem(element));
        found
    }

    fn unknown_problem(element: &ViewElement, name: &str) -> AccessibilityProblem {
        let message = format!(
            "<{}> has {name}, which is not an ARIA attribute.",
            element.tag
        );
        AccessibilityProblem::new(element.range_of(name), message, Self::NAME_HELP)
    }

    fn value_problem(element: &ViewElement, name: &str) -> Option<AccessibilityProblem> {
        let allowed = Self::allowed_values(name)?;
        let value = element.text(name)?;
        let message = format!(
            "<{}> has {name}=\"{value}\". {name} takes {}.",
            element.tag,
            allowed.join(" or ")
        );
        let problem = AccessibilityProblem::new(element.range_of(name), message, Self::VALUE_HELP);
        (!allowed.contains(&value)).then_some(problem)
    }

    fn allowed_values(name: &str) -> Option<&'static [&'static str]> {
        if Self::TRUE_OR_FALSE_NAMES.contains(&name) {
            Some(&Self::TRUE_OR_FALSE)
        } else if Self::TRUE_FALSE_OR_MIXED_NAMES.contains(&name) {
            Some(&Self::TRUE_FALSE_OR_MIXED)
        } else {
            None
        }
    }

    fn hidden_problem(element: &ViewElement) -> Option<AccessibilityProblem> {
        let message = format!(
            "<{}> has aria-hidden=\"true\", but the keyboard can still reach it.",
            element.tag
        );
        let problem = AccessibilityProblem::new(element.range, message, Self::HIDDEN_HELP);
        (element.is_hidden() & Self::is_focusable(element)).then_some(problem)
    }

    fn is_focusable(element: &ViewElement) -> bool {
        let control = Self::FOCUSABLE_TAGS.contains(&element.tag);
        let link = element.is(Self::LINK) & element.has(Self::LINK_TARGET);
        let reachable = (control | link) & !element.has(Self::DISABLED);
        element
            .text(Self::TAB_INDEX)
            .and_then(|order| order.trim().parse::<i64>().ok())
            .map_or(reachable, |order| !order.is_negative())
    }

    fn naming_problem(element: &ViewElement) -> Option<AccessibilityProblem> {
        let nameless = Self::TAGS_WITHOUT_NAME.contains(&element.tag) & !element.has_role();
        let tag = element.tag;
        let message = format!(
            "<{tag}> has aria-label or aria-labelledby, but a <{tag}> without a role cannot have a name. Screen readers ignore the label."
        );
        let problem = AccessibilityProblem::new(element.range, message, Self::NAMING_HELP);
        (nameless & element.has_label_attribute()).then_some(problem)
    }
}
