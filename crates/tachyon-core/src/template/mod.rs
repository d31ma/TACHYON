mod accessibility;
mod braced_attribute;
mod client;
mod component;
mod component_slots;
mod component_style_ownership;
#[cfg(test)]
mod component_style_ownership_tests;
mod context;
mod expression;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod feature_inputs_tests;
mod frontend;
mod model;
pub(crate) mod required_fields;
mod script_service_references;
mod service;
mod service_reach;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod service_reach_tests;
mod shipped_script;
#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod shipped_script_tests;
mod slotted_children;

pub(crate) use accessibility::AccessibilityCheck;
pub(crate) use braced_attribute::BracedAttribute;
pub(crate) use client::ClientViewRenderer;
pub(crate) use component::{ComponentDefinition, ComponentRegistry};
pub(crate) use context::client_route_context;
pub(crate) use expression::{Expression, Scope};
pub(crate) use frontend::TemplateFrontend;
pub(crate) use model::{
    AttributeValue, HydrationPolicy, TemplateAttribute, TemplateNode, TemplateNodeKind,
    TemplateProgram, TextPart, is_trivia,
};
pub(crate) use service::{ServiceDefinition, ServiceRegistry, instance_name};
pub(crate) use service_reach::ServiceReach;
pub(crate) use shipped_script::ShippedScript;

pub(crate) use required_fields::RequiredFields;
