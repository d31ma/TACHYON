use super::ManualStep;
use super::conversion_plan::ConversionPlan;
use super::edits::TextEdit;
use super::markers::{Marker, MarkerLine};
use super::scan::SourceText;
use crate::route_types::{ModelTable, Shape, TypeText};
use serde_json::{Map, Value};
use std::path::Path;

pub(super) struct EventConversion<'a> {
    relative: &'a str,
    extension: &'a str,
    text: SourceText<'a>,
    models: Vec<String>,
}

struct Publication {
    name_start: usize,
    value_start: usize,
    value_end: usize,
    name: String,
}

impl<'a> EventConversion<'a> {
    const NAMES: [&'static str; 1] = ["onevent"];
    const CALLS: [&'static str; 3] = ["publishEvent(", "publish_event(", "PublishEvent("];

    pub(super) fn plan(root: &Path, relative: &'a str, source: &'a str) -> ConversionPlan {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(text) = SourceText::read(extension, source) else {
            return ConversionPlan::default();
        };
        let conversion = Self {
            relative,
            extension,
            text,
            models: Self::model_names(root, relative, extension, source),
        };
        conversion.convert()
    }

    fn model_names(root: &Path, relative: &str, extension: &str, source: &str) -> Vec<String> {
        let realm = if relative.starts_with("client/") {
            "client"
        } else {
            "server"
        };
        let Ok(mut table) = ModelTable::read_realm(root, realm, extension) else {
            return Vec::new();
        };
        if extension == "js" {
            table.add_documented(source);
        }
        table.record_names()
    }

    fn convert(&self) -> ConversionPlan {
        let mut plan = ConversionPlan::default();
        for marker in Marker::lines(&self.text, self.extension, &Self::NAMES) {
            self.listener(&marker, &mut plan);
        }
        for publication in self.publications() {
            self.publication(&publication, &mut plan);
        }
        plan
    }

    fn listener(&self, marker: &MarkerLine, plan: &mut ConversionPlan) {
        let Some((open, close)) = marker.arguments else {
            return;
        };
        let Ok(Value::Object(mut options)) =
            crate::behavior::literal::parse(&self.text.source[open..close])
        else {
            return;
        };
        let names = Self::event_names(&options);
        if names.is_empty() {
            return;
        }
        let [name] = names.as_slice() else {
            plan.steps.push(self.step(
                "An OnEvent listener names several events; a typed listener hears one model.",
                "Write one OnEvent method per model, each with a parameter of that model type (ADR 0055).",
            ));
            return;
        };
        let Some(model) = self.model_for(name) else {
            plan.steps.push(self.missing_model(name));
            return;
        };
        let condition = options.get("condition").and_then(Value::as_str);
        if condition.is_some_and(|condition| condition.contains("event")) {
            plan.steps.push(self.step(
                "An OnEvent condition compares `event` with the old event name.",
                &format!("The event name is now the model name '{model}'; update or remove that part of the condition."),
            ));
        }
        let Some(typing) = self.payload_typing(marker, &model) else {
            plan.steps.push(self.step(
                &format!("The OnEvent listener for '{name}' reads a JSON value, not the model '{model}'."),
                &format!("Change its parameter type to {model} and read the payload through the model's fields, then remove `event` from OnEvent (ADR 0055)."),
            ));
            return;
        };
        options.remove("event");
        options.remove("events");
        let arguments = if options.is_empty() {
            String::new()
        } else {
            Value::Object(options).to_string()
        };
        let spelled = Marker::spelled(self.extension, "OnEvent", &arguments);
        plan.edits
            .push(TextEdit::replace(marker.start, marker.end, &spelled));
        plan.edits.extend(typing);
        plan.notes.push(format!(
            "OnEvent now listens by the model type {model} instead of the name '{name}'."
        ));
    }

    fn event_names(options: &Map<String, Value>) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(Value::String(name)) = options.get("event") {
            names.push(name.clone());
        }
        if let Some(Value::Array(items)) = options.get("events") {
            for item in items {
                if let Some(name) = item.as_str() {
                    names.push(String::from(name));
                }
            }
        }
        names
    }

    fn payload_typing(&self, marker: &MarkerLine, model: &str) -> Option<Vec<TextEdit>> {
        let payload = crate::events::listener_payload(
            Path::new(self.relative),
            self.text.source,
            marker.end,
        )?;
        let shape = TypeText::shape(self.extension, &payload.type_text).ok();
        if shape == Some(Shape::Model(String::from(model))) {
            return Some(Vec::new());
        }
        let is_json_value = shape == Some(Shape::Any);
        if !is_json_value {
            return None;
        }
        match self.extension {
            "ts" => self.typescript_type(&payload, model),
            "js" => Some(vec![self.javascript_type(marker, &payload.name, model)]),
            _ => None,
        }
    }

    fn typescript_type(
        &self,
        payload: &crate::events::ListenerPayload,
        model: &str,
    ) -> Option<Vec<TextEdit>> {
        let signature = &self.text.source[payload.signature.clone()];
        let open = payload.signature.start + signature.find('(')?;
        let name_start = open + 1 + self.text.source[open + 1..].find(&payload.name)?;
        let name_end = name_start + payload.name.len();
        let after = &self.text.source[name_end..];
        let Some(colon) = after.trim_start().strip_prefix(':') else {
            return Some(vec![TextEdit::insert(name_end, &format!(": {model}"))]);
        };
        let type_start = self.text.source.len() - colon.len();
        let type_length = colon.find([',', ')']).unwrap_or(colon.len());
        Some(vec![TextEdit::replace(
            type_start,
            type_start + type_length,
            &format!(" {model}"),
        )])
    }

    fn javascript_type(&self, marker: &MarkerLine, name: &str, model: &str) -> TextEdit {
        let indentation = Marker::indentation(&self.text, marker.line_start);
        TextEdit::insert(
            marker.line_start,
            &format!("{indentation}/** @param {{{model}}} {name} */\n"),
        )
    }

    fn model_for(&self, name: &str) -> Option<String> {
        let candidates = [String::from(name), Self::pascal_case(name)];
        candidates
            .into_iter()
            .find(|candidate| self.models.contains(candidate))
    }

    fn pascal_case(name: &str) -> String {
        let mut result = String::new();
        for part in name.split(['.', '-', '_', ':']) {
            let mut characters = part.chars();
            if let Some(first) = characters.next() {
                result.extend(first.to_uppercase());
                result.push_str(characters.as_str());
            }
        }
        result
    }

    fn publications(&self) -> Vec<Publication> {
        let mut found = Vec::new();
        for call in Self::CALLS {
            for (position, _) in self.text.source.match_indices(call) {
                let joined = position > 0 && matches!(self.text.bytes[position - 1], b'.' | b'>');
                if !self.text.is_code(position) || !joined {
                    continue;
                }
                if let Some(publication) = self.publication_at(position + call.len()) {
                    found.push(publication);
                }
            }
        }
        found
    }

    fn publication_at(&self, open: usize) -> Option<Publication> {
        let name_start = self.next_text(open)?;
        let quote = self.text.bytes[name_start];
        if !matches!(quote, b'\'' | b'"') {
            return None;
        }
        let name_end =
            name_start + 1 + self.text.source[name_start + 1..].find(char::from(quote))?;
        let comma = self.text.next_code(name_end + 1)?;
        if self.text.bytes[comma] != b',' {
            return None;
        }
        let value_start = self.next_text(comma + 1)?;
        let value_end = self.argument_end(value_start)?;
        Some(Publication {
            name_start,
            value_start,
            value_end,
            name: String::from(&self.text.source[name_start + 1..name_end]),
        })
    }

    fn next_text(&self, from: usize) -> Option<usize> {
        let rest = self.text.source.get(from..)?;
        Some(from + rest.find(|character: char| !character.is_whitespace())?)
    }

    fn argument_end(&self, start: usize) -> Option<usize> {
        let mut depth = 0_i32;
        for at in start..self.text.len() {
            if !self.text.is_code(at) {
                continue;
            }
            match self.text.bytes[at] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' if depth > 0 => depth -= 1,
                b',' | b')' if depth == 0 => {
                    let written = self.text.source[start..at].trim_end();
                    return Some(start + written.len());
                }
                _ => {}
            }
        }
        None
    }

    fn publication(&self, publication: &Publication, plan: &mut ConversionPlan) {
        let name = &publication.name;
        let Some(model) = self.model_for(name) else {
            plan.steps.push(self.missing_model(name));
            return;
        };
        let value = &self.text.source[publication.value_start..publication.value_end];
        let replacement = match self.extension {
            "ts" => format!("{value} satisfies {model}"),
            "js" => format!("/** @type {{{model}}} */ ({value})"),
            "rb" => {
                plan.steps.push(self.step(
                    &format!("publish_event names the event '{name}'; it now takes a {model} value."),
                    &format!("Pass a {model} value instead of the name and the JSON value, for example publish_event({model}.new(...)) (ADR 0055)."),
                ));
                return;
            }
            _ => {
                plan.steps.push(self.step(
                    &format!("publishEvent names the event '{name}'; it now takes a {model} value."),
                    &format!("Pass a {model} value instead of the name and the JSON value, for example publishEvent(new {model}(...)) (ADR 0055)."),
                ));
                return;
            }
        };
        plan.edits.push(TextEdit::replace(
            publication.name_start,
            publication.value_end,
            &replacement,
        ));
        plan.notes.push(format!(
            "publishEvent('{name}', ...) now publishes the model {model}."
        ));
    }

    fn missing_model(&self, name: &str) -> ManualStep {
        let model = Self::pascal_case(name);
        self.step(
            &format!("The event name '{name}' has no model with the same name."),
            &format!("Declare a model named {model} in the models folder with the payload's fields, give the listener parameter that type and publish a {model} value (ADR 0055)."),
        )
    }

    fn step(&self, problem: &str, action: &str) -> ManualStep {
        ManualStep::new(self.relative, String::from(problem), String::from(action))
    }
}
