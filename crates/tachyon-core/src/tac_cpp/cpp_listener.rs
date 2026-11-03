use super::{CppAttribute, CppFailure, CppMember, CppParameter, CppTypeText};
use crate::Failure;
use serde_json::{Map, Value, json};
use std::path::Path;

pub(crate) struct CppListener {
    pub(crate) method: String,
    pub(crate) event: String,
    pub(crate) replay: bool,
    pub(crate) condition: Value,
    pub(crate) id: Value,
    pub(crate) order: i64,
    pub(crate) background: bool,
    pub(crate) returns: Value,
}

impl CppListener {
    const OPTIONS: [&'static str; 3] = ["replay", "condition", "id"];
    const NOTHING: [&'static str; 2] = ["", "void"];
    const NO_OPTIONS: &'static str = "{}";
    const VECTOR: &'static str = "std::vector<";
    const ONE_PARAMETER: &'static str = "An [[OnEvent]] method takes one parameter: the event model, for example `void placed(const PlacedOrder& order)`";
    const OPTIONS_FORM: &'static str = "[[OnEvent]] takes no arguments or one JSON object with replay, condition and id, for example [[OnEvent({\"replay\":true})]]";
    const ORDER_FORM: &'static str = "[[Order]] takes one whole number, for example [[Order(10)]]";
    const CONDITION_FORM: &'static str = "OnEvent.condition must be a string";

    pub(crate) fn read(path: &Path, member: &CppMember) -> Result<Self, Failure> {
        let marker = member
            .markers
            .iter()
            .find(|marker| marker.name == "OnEvent");
        let at = marker.map_or(member.name_at, |marker| marker.at);
        let parameter = Self::only_parameter(path, at, &member.parameters)?;
        let arguments = marker.map_or("", |marker| marker.arguments.as_str());
        let options = Self::options(path, at, arguments)?;
        Ok(Self {
            method: member.name.clone(),
            event: CppTypeText::plain(&parameter.type_text),
            replay: options
                .get("replay")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            condition: Self::condition(path, at, options.get("condition"))?,
            id: options.get("id").cloned().unwrap_or(Value::Null),
            order: Self::order(path, member)?,
            background: member.marked("InBackground"),
            returns: Self::returned(&member.type_text),
        })
    }

    pub(crate) fn page(&self) -> Value {
        json!({
            "key": self.method,
            "events": [self.event],
            "replay": self.replay,
            "order": self.order,
            "asynchronous": self.background,
            "condition": self.condition,
            "id": self.id,
        })
    }

    pub(crate) fn program(&self) -> Value {
        json!({
            "method": self.method,
            "events": [self.event],
            "asynchronous": self.background,
            "returns": self.returns,
        })
    }

    fn only_parameter<'p>(
        path: &Path,
        at: usize,
        parameters: &'p [CppParameter],
    ) -> Result<&'p CppParameter, Failure> {
        match parameters {
            [parameter] => Ok(parameter),
            _ => Err(CppFailure::unreadable(path, at, Self::ONE_PARAMETER)),
        }
    }

    fn options(path: &Path, at: usize, arguments: &str) -> Result<Map<String, Value>, Failure> {
        let trimmed = arguments.trim();
        let written = [trimmed, Self::NO_OPTIONS][usize::from(trimmed.is_empty())];
        let refused = || CppFailure::unreadable(path, at, Self::OPTIONS_FORM);
        let parsed: Value = serde_json::from_str(written).map_err(|_| refused())?;
        let object = parsed.as_object().cloned().ok_or_else(refused)?;
        let known = object
            .keys()
            .all(|key| Self::OPTIONS.contains(&key.as_str()));
        known.then_some(object).ok_or_else(refused)
    }

    fn condition(path: &Path, at: usize, written: Option<&Value>) -> Result<Value, Failure> {
        written.map_or(Ok(Value::Null), |written| Self::compiled(path, at, written))
    }

    fn compiled(path: &Path, at: usize, written: &Value) -> Result<Value, Failure> {
        let source = written
            .as_str()
            .ok_or_else(|| CppFailure::unreadable(path, at, Self::CONDITION_FORM))?;
        crate::events::condition::compile(source).map_err(|message| {
            CppFailure::unreadable(path, at, &format!("OnEvent.condition: {message}"))
        })
    }

    fn order(path: &Path, member: &CppMember) -> Result<i64, Failure> {
        member
            .markers
            .iter()
            .find(|marker| marker.name == "Order")
            .map_or(Ok(0), |marker| Self::written_order(path, marker))
    }

    fn written_order(path: &Path, marker: &CppAttribute) -> Result<i64, Failure> {
        marker
            .arguments
            .trim()
            .parse::<i64>()
            .map_err(|_| CppFailure::unreadable(path, marker.at, Self::ORDER_FORM))
    }

    fn returned(type_text: &str) -> Value {
        let plain = CppTypeText::plain(type_text);
        let item = plain
            .strip_prefix(Self::VECTOR)
            .and_then(|rest| rest.strip_suffix('>'))
            .unwrap_or(&plain)
            .trim();
        Some(item)
            .filter(|item| !Self::NOTHING.contains(item))
            .map_or(Value::Null, |item| Value::String(item.to_owned()))
    }
}
