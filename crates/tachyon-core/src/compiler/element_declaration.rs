use super::element_events::ElementEvents;
use super::typescript_field_type::TypeScriptFieldType;
use serde_json::{Map, Value};
use std::fmt::Write as _;

pub(super) struct ElementDeclaration<'a> {
    name: &'a str,
    props: &'a [String],
    events: &'a ElementEvents,
}

impl<'a> ElementDeclaration<'a> {
    const PROP_TYPE: &'static str = "unknown";
    const ADD_OPTIONS: &'static str = "boolean | AddEventListenerOptions";
    const REMOVE_OPTIONS: &'static str = "boolean | EventListenerOptions";

    pub(super) const fn new(name: &'a str, props: &'a [String], events: &'a ElementEvents) -> Self {
        Self {
            name,
            props,
            events,
        }
    }

    pub(super) fn text(&self) -> String {
        let mut text = String::new();
        for (model, described) in &self.events.models {
            text.push_str(&Self::model_interface(model, described));
        }
        text.push_str(&self.event_map());
        text.push_str(&self.element_class());
        text.push_str(&self.tag_map());
        text
    }

    pub(super) fn event_name(model: &str) -> String {
        let mut name = String::with_capacity(model.len() + 4);
        let mut previous_joins = false;
        for character in model.chars() {
            if previous_joins && character.is_ascii_uppercase() {
                name.push('-');
            }
            previous_joins = character.is_ascii_lowercase() || character.is_ascii_digit();
            name.push(character.to_ascii_lowercase());
        }
        name
    }

    fn model_interface(model: &str, described: &Value) -> String {
        let empty = Map::new();
        let fields = described
            .get("fields")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let mut text = format!("export interface {model} {{\n");
        for (field, written) in fields {
            let field_type = TypeScriptFieldType::new(written.as_str().unwrap_or_default());
            let optional = if field_type.is_required() { "" } else { "?" };
            let _ = writeln!(
                text,
                "  {}{optional}: {}",
                Self::member_name(field),
                field_type.type_text()
            );
        }
        text.push_str("}\n\n");
        text
    }

    fn event_map(&self) -> String {
        let mut text = format!(
            "export interface {} extends HTMLElementEventMap {{\n",
            self.event_map_name()
        );
        for model in &self.events.names {
            let detail = if self.events.models.contains_key(model) {
                model.as_str()
            } else {
                Self::PROP_TYPE
            };
            let _ = writeln!(
                text,
                "  {}: CustomEvent<{detail}>",
                Self::quoted(&Self::event_name(model))
            );
        }
        text.push_str("}\n\n");
        text
    }

    fn element_class(&self) -> String {
        let class = self.class_name();
        let map = self.event_map_name();
        let mut text = format!("export declare class {class} extends HTMLElement {{\n");
        for prop in self.props {
            let _ = writeln!(text, "  {}: {}", Self::member_name(prop), Self::PROP_TYPE);
        }
        let typed_listener = format!("listener: (this: {class}, event: {map}[K]) => unknown");
        let plain_listener = "listener: EventListenerOrEventListenerObject";
        for (method, options) in [
            ("addEventListener", Self::ADD_OPTIONS),
            ("removeEventListener", Self::REMOVE_OPTIONS),
        ] {
            let _ = writeln!(
                text,
                "  {method}<K extends keyof {map}>(type: K, {typed_listener}, options?: {options}): void"
            );
            let _ = writeln!(
                text,
                "  {method}(type: string, {plain_listener}, options?: {options}): void"
            );
        }
        text.push_str("}\n\n");
        text
    }

    fn tag_map(&self) -> String {
        format!(
            "declare global {{\n  interface HTMLElementTagNameMap {{\n    {}: {}\n  }}\n}}\n",
            Self::quoted(self.name),
            self.class_name()
        )
    }

    fn class_name(&self) -> String {
        format!("{}Element", self.pascal_name())
    }

    fn event_map_name(&self) -> String {
        format!("{}EventMap", self.pascal_name())
    }

    fn pascal_name(&self) -> String {
        self.name
            .split('-')
            .map(Self::capitalized)
            .collect::<String>()
    }

    fn capitalized(part: &str) -> String {
        let mut characters = part.chars();
        characters.next().map_or_else(String::new, |first| {
            first.to_ascii_uppercase().to_string() + characters.as_str()
        })
    }

    fn member_name(name: &str) -> String {
        let plain = name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '$'
        });
        let starts_well = !name.starts_with(|first: char| first.is_ascii_digit());
        if plain && starts_well && !name.is_empty() {
            return String::from(name);
        }
        Self::quoted(name)
    }

    fn quoted(text: &str) -> String {
        Value::String(String::from(text)).to_string()
    }
}

#[cfg(test)]
#[path = "element_declaration_tests.rs"]
mod tests;
