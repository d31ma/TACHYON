use super::element_events::ElementEvents;
use super::typescript_field_type::TypescriptFieldType;
use serde_json::{Map, Value};

pub(super) struct ElementDeclaration<'a> {
    name: &'a str,
    props: &'a [String],
    events: &'a ElementEvents,
}

impl<'a> ElementDeclaration<'a> {
    const PROP_TYPE: &'static str = "unknown";
    const ADD_OPTIONS: &'static str = "boolean | AddEventListenerOptions";
    const REMOVE_OPTIONS: &'static str = "boolean | EventListenerOptions";
    const LISTENER_METHODS: [(&'static str, &'static str); 2] = [
        ("addEventListener", Self::ADD_OPTIONS),
        ("removeEventListener", Self::REMOVE_OPTIONS),
    ];
    const NO_CHARACTER: char = ' ';
    const IDENTIFIER_SYMBOLS: [char; 2] = ['_', '$'];

    pub(super) const fn new(name: &'a str, props: &'a [String], events: &'a ElementEvents) -> Self {
        Self {
            name,
            props,
            events,
        }
    }

    pub(super) fn text(&self) -> String {
        let models: String = self
            .events
            .models
            .iter()
            .map(|(model, described)| Self::model_interface(model, described))
            .collect();
        [
            models,
            self.event_map(),
            self.element_class(),
            self.tag_map(),
        ]
        .concat()
    }

    pub(super) fn event_name(model: &str) -> String {
        let before = std::iter::once(Self::NO_CHARACTER).chain(model.chars());
        before
            .zip(model.chars())
            .map(|(previous, current)| Self::event_name_piece(previous, current))
            .collect()
    }

    fn event_name_piece(previous: char, current: char) -> String {
        let lower = current.to_ascii_lowercase();
        let joins = [previous.is_ascii_lowercase(), previous.is_ascii_digit()].contains(&true);
        if [joins, current.is_ascii_uppercase()] == [true, true] {
            return format!("-{lower}");
        }
        lower.to_string()
    }

    fn model_interface(model: &str, described: &Value) -> String {
        let empty = Map::new();
        let fields = described
            .get("fields")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let members: String = fields
            .iter()
            .map(|(field, written)| Self::field_line(field, written))
            .collect();
        format!("export interface {model} {{\n{members}}}\n\n")
    }

    fn field_line(field: &str, written: &Value) -> String {
        let field_type = TypescriptFieldType::new(written.as_str().unwrap_or_default());
        let optional = if field_type.is_required() { "" } else { "?" };
        format!(
            "  {}{optional}: {}\n",
            Self::member_name(field),
            field_type.type_text()
        )
    }

    fn event_map(&self) -> String {
        let lines: String = self
            .events
            .names
            .iter()
            .map(|model| self.event_line(model))
            .collect();
        format!(
            "export interface {} extends HTMLElementEventMap {{\n{lines}}}\n\n",
            self.event_map_name()
        )
    }

    fn event_line(&self, model: &str) -> String {
        let detail = if self.events.models.contains_key(model) {
            model
        } else {
            Self::PROP_TYPE
        };
        format!(
            "  {}: CustomEvent<{detail}>\n",
            Self::quoted(&Self::event_name(model))
        )
    }

    fn element_class(&self) -> String {
        let class = self.class_name();
        let props: String = self
            .props
            .iter()
            .map(|prop| Self::prop_line(prop))
            .collect();
        let listeners: String = Self::LISTENER_METHODS
            .iter()
            .map(|(method, options)| self.listener_lines(&class, method, options))
            .collect();
        format!("export declare class {class} extends HTMLElement {{\n{props}{listeners}}}\n\n")
    }

    fn prop_line(prop: &str) -> String {
        format!("  {}: {}\n", Self::member_name(prop), Self::PROP_TYPE)
    }

    fn listener_lines(&self, class: &str, method: &str, options: &str) -> String {
        let map = self.event_map_name();
        format!(
            "  {method}<K extends keyof {map}>(type: K, listener: (this: {class}, event: {map}[K]) => unknown, options?: {options}): void\n  {method}(type: string, listener: EventListenerOrEventListenerObject, options?: {options}): void\n"
        )
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
        Some(name)
            .filter(|name| Self::is_identifier(name))
            .map_or_else(|| Self::quoted(name), String::from)
    }

    fn is_identifier(name: &str) -> bool {
        let plain = name.chars().all(Self::is_identifier_character);
        let starts_well = name
            .chars()
            .next()
            .is_some_and(|first| !first.is_ascii_digit());
        [plain, starts_well] == [true, true]
    }

    fn is_identifier_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            Self::IDENTIFIER_SYMBOLS.contains(&character),
        ]
        .contains(&true)
    }

    fn quoted(text: &str) -> String {
        Value::String(String::from(text)).to_string()
    }
}

#[cfg(test)]
#[path = "element_declaration_tests.rs"]
mod tests;
