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
    const CALL_JOINERS: &'static [u8] = b".>";
    const QUOTES: &'static [u8] = b"'\"";

    pub(super) fn plan(root: &Path, relative: &'a str, source: &'a str) -> ConversionPlan {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(extension, source);
        text.map(|text| {
            let conversion = Self {
                relative,
                extension,
                text,
                models: Self::model_names(root, relative, extension, source),
            };
            conversion.convert()
        })
        .unwrap_or_default()
    }

    fn model_names(root: &Path, relative: &str, extension: &str, source: &str) -> Vec<String> {
        let table = ModelTable::read_realm(root, Self::realm(relative), extension);
        table
            .map(|table| Self::documented_names(table, extension, source))
            .unwrap_or_default()
    }

    fn realm(relative: &str) -> &'static str {
        if relative.starts_with("client/") {
            return "client";
        }
        "server"
    }

    fn documented_names(mut table: ModelTable, extension: &str, source: &str) -> Vec<String> {
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
        self.add_publications(&mut plan);
        plan
    }

    fn add_publications(&self, plan: &mut ConversionPlan) {
        for publication in self.publications() {
            self.publication(&publication, plan);
        }
    }

    fn listener(&self, marker: &MarkerLine, plan: &mut ConversionPlan) {
        let options = self.listener_options(marker);
        let converted =
            options.map(|(options, names)| self.typed_listener(marker, options, &names, plan));
        plan.steps.extend(converted.and_then(Result::err));
    }

    fn listener_options(&self, marker: &MarkerLine) -> Option<(Map<String, Value>, Vec<String>)> {
        let (open, close) = marker.arguments?;
        let parsed = crate::behavior::literal::parse(&self.text.source[open..close]).ok()?;
        let Value::Object(options) = parsed else {
            return None;
        };
        let names = Self::event_names(&options);
        (!names.is_empty()).then_some((options, names))
    }

    fn typed_listener(
        &self,
        marker: &MarkerLine,
        mut options: Map<String, Value>,
        names: &[String],
        plan: &mut ConversionPlan,
    ) -> Result<(), ManualStep> {
        let single = names.first().filter(|_| names.len() == 1);
        let name = single.ok_or_else(|| {
            self.step(
                "An OnEvent listener names several events; a typed listener hears one model.",
                "Write one OnEvent method per model, each with a parameter of that model type (ADR 0055).",
            )
        })?;
        let model = self
            .model_for(name)
            .ok_or_else(|| self.missing_model(name))?;
        let condition = options.get("condition").and_then(Value::as_str);
        let compares_event = condition.is_some_and(|condition| condition.contains("event"));
        plan.steps.extend(compares_event.then(|| {
            self.step(
                "An OnEvent condition compares `event` with the old event name.",
                &format!("The event name is now the model name '{model}'; update or remove that part of the condition."),
            )
        }));
        let typing = self.payload_typing(marker, &model).ok_or_else(|| {
            self.step(
                &format!("The OnEvent listener for '{name}' reads a JSON value, not the model '{model}'."),
                &format!("Change its parameter type to {model} and read the payload through the model's fields, then remove `event` from OnEvent (ADR 0055)."),
            )
        })?;
        options.remove("event");
        options.remove("events");
        let arguments = Self::option_text(options);
        let spelled = Marker::spelled(self.extension, "OnEvent", &arguments);
        plan.edits
            .push(TextEdit::replace(marker.start, marker.end, &spelled));
        plan.edits.extend(typing);
        plan.notes.push(format!(
            "OnEvent now listens by the model type {model} instead of the name '{name}'."
        ));
        Ok(())
    }

    fn option_text(options: Map<String, Value>) -> String {
        if options.is_empty() {
            return String::new();
        }
        Value::Object(options).to_string()
    }

    fn event_names(options: &Map<String, Value>) -> Vec<String> {
        let single = options
            .get("event")
            .and_then(Value::as_str)
            .map(String::from);
        single
            .into_iter()
            .chain(Self::listed_names(options))
            .collect()
    }

    fn listed_names(options: &Map<String, Value>) -> Vec<String> {
        let items = options.get("events").and_then(Value::as_array);
        items
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    }

    fn payload_typing(&self, marker: &MarkerLine, model: &str) -> Option<Vec<TextEdit>> {
        let payload = crate::events::listener_payload(
            Path::new(self.relative),
            self.text.source,
            marker.end,
        )?;
        let shape = TypeText::shape(self.extension, &payload.type_text).ok()?;
        let already_typed = shape == Shape::Model(String::from(model));
        let json_value = shape == Shape::Any;
        let added = json_value
            .then(|| self.json_value_typing(marker, &payload, model))
            .flatten();
        already_typed.then(Vec::new).or(added)
    }

    fn json_value_typing(
        &self,
        marker: &MarkerLine,
        payload: &crate::events::ListenerPayload,
        model: &str,
    ) -> Option<Vec<TextEdit>> {
        match self.extension {
            "ts" => self.typescript_type(payload, model),
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
        let colon = after.trim_start().strip_prefix(':');
        let edit = colon.map_or_else(
            || TextEdit::insert(name_end, &format!(": {model}")),
            |colon| self.replaced_type(colon, model),
        );
        Some(vec![edit])
    }

    fn replaced_type(&self, colon: &str, model: &str) -> TextEdit {
        let type_start = self.text.source.len() - colon.len();
        let type_length = colon.find([',', ')']).unwrap_or(colon.len());
        TextEdit::replace(type_start, type_start + type_length, &format!(" {model}"))
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
        name.split(['.', '-', '_', ':'])
            .map(Self::capitalized)
            .collect()
    }

    fn capitalized(part: &str) -> String {
        let mut characters = part.chars();
        let first = characters.next();
        let upper: String = first.into_iter().flat_map(char::to_uppercase).collect();
        format!("{upper}{}", characters.as_str())
    }

    fn publications(&self) -> Vec<Publication> {
        Self::CALLS
            .iter()
            .flat_map(|call| self.call_publications(call))
            .collect()
    }

    fn call_publications(&self, call: &str) -> Vec<Publication> {
        self.text
            .source
            .match_indices(call)
            .map(|(position, _)| position)
            .filter(|position| self.is_method_call(*position))
            .filter_map(|position| self.publication_at(position + call.len()))
            .collect()
    }

    fn is_method_call(&self, position: usize) -> bool {
        let before = position
            .checked_sub(1)
            .map(|before| self.text.bytes[before]);
        let joined = before.is_some_and(|byte| Self::CALL_JOINERS.contains(&byte));
        [self.text.is_code(position), joined] == [true, true]
    }

    fn publication_at(&self, open: usize) -> Option<Publication> {
        let name_start = self.next_text(open)?;
        let quote = self.text.bytes[name_start];
        Self::QUOTES.contains(&quote).then_some(())?;
        let name_end =
            name_start + 1 + self.text.source[name_start + 1..].find(char::from(quote))?;
        let comma = self.text.next_code(name_end + 1)?;
        (self.text.bytes[comma] == b',').then_some(())?;
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
        let model = self.model_for(name).ok_or_else(|| self.missing_model(name));
        let typed = model.and_then(|model| self.typed_publication(publication, &model));
        match typed {
            Ok((edit, note)) => {
                plan.edits.push(edit);
                plan.notes.push(note);
            }
            Err(step) => plan.steps.push(step),
        }
    }

    fn typed_publication(
        &self,
        publication: &Publication,
        model: &str,
    ) -> Result<(TextEdit, String), ManualStep> {
        let name = &publication.name;
        let value = &self.text.source[publication.value_start..publication.value_end];
        let replacement = self.typed_value(name, model, value)?;
        let edit = TextEdit::replace(publication.name_start, publication.value_end, &replacement);
        let note = format!("publishEvent('{name}', ...) now publishes the model {model}.");
        Ok((edit, note))
    }

    fn typed_value(&self, name: &str, model: &str, value: &str) -> Result<String, ManualStep> {
        match self.extension {
            "ts" => Ok(format!("{value} satisfies {model}")),
            "js" => Ok(format!("/** @type {{{model}}} */ ({value})")),
            "rb" => Err(self.step(
                &format!("publish_event names the event '{name}'; it now takes a {model} value."),
                &format!("Pass a {model} value instead of the name and the JSON value, for example publish_event({model}.new(...)) (ADR 0055)."),
            )),
            _ => Err(self.step(
                &format!("publishEvent names the event '{name}'; it now takes a {model} value."),
                &format!("Pass a {model} value instead of the name and the JSON value, for example publishEvent(new {model}(...)) (ADR 0055)."),
            )),
        }
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
