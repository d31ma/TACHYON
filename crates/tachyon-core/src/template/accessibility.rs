mod accessibility_check;
mod accessibility_problem;
mod accessible_name;
mod aria_attribute;
mod attribute_state;
mod element_rules;
mod field_label;
mod heading_order;
mod view_audit;
mod view_element;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod accessibility_tests;

pub(crate) use accessibility_check::AccessibilityCheck;
use accessibility_problem::AccessibilityProblem;
use accessible_name::AccessibleName;
use aria_attribute::AriaAttribute;
use attribute_state::AttributeState;
use element_rules::ElementRules;
use field_label::FieldLabel;
use heading_order::HeadingOrder;
use view_audit::ViewAudit;
use view_element::ViewElement;
