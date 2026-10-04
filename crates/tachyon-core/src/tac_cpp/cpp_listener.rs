use super::{CppFailure, CppMember, CppTypeText};
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
    const VECTOR: &'static str = "std::vector<";
    const ONE_PARAMETER: &'static str = "An [[OnEvent]] method takes one parameter: the event model, for example `void placed(const PlacedOrder& order)`";
    const OPTIONS_FORM: &'static str = "[[OnEvent]] takes no arguments or one JSON object with replay, condition and id, for example [[OnEvent({\"replay\":true})]]";
    const ORDER_FORM: &'static str = "[[Order]] takes one whole number, for example [[Order(10)]]";

    pub(crate) fn read(path: &Path, member: &CppMember) -> Result<Self, Failure> {
        let marker = member
            .markers
            .iter()
            .find(|marker| marker.name == "OnEvent");
        let at = marker.map_or(member.name_at, |marker| marker.at);
        let [parameter] = member.parameters.as_slice() else {
            return Err(CppFailure::unreadable(path, at, Self::ONE_PARAMETER));
        };
        let options = Self::options(
            path,
            at,
            marker.map_or("", |marker| marker.arguments.as_str()),
        )?;
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

    fn options(path: &Path, at: usize, arguments: &str) -> Result<Map<String, Value>, Failure> {
        if arguments.trim().is_empty() {
            return Ok(Map::new());
        }
        let parsed: Value = serde_json::from_str(arguments)
            .map_err(|_| CppFailure::unreadable(path, at, Self::OPTIONS_FORM))?;
        let Value::Object(object) = parsed else {
            return Err(CppFailure::unreadable(path, at, Self::OPTIONS_FORM));
        };
        if object
            .keys()
            .any(|key| !Self::OPTIONS.contains(&key.as_str()))
        {
            return Err(CppFailure::unreadable(path, at, Self::OPTIONS_FORM));
        }
        Ok(object)
    }

    fn condition(path: &Path, at: usize, written: Option<&Value>) -> Result<Value, Failure> {
        let Some(written) = written else {
            return Ok(Value::Null);
        };
        let source = written.as_str().ok_or_else(|| {
            CppFailure::unreadable(path, at, "OnEvent.condition must be a string")
        })?;
        crate::events::condition::compile(source).map_err(|message| {
            CppFailure::unreadable(path, at, &format!("OnEvent.condition: {message}"))
        })
    }

    fn order(path: &Path, member: &CppMember) -> Result<i64, Failure> {
        let Some(marker) = member.markers.iter().find(|marker| marker.name == "Order") else {
            return Ok(0);
        };
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
        if item.is_empty() || item == "void" {
            return Value::Null;
        }
        Value::String(item.to_owned())
    }
}
