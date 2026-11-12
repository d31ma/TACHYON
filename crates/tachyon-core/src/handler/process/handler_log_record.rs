use super::HandlerLogLevel;
use serde_json::{Map, Value};

#[derive(Debug, PartialEq)]
pub(in crate::handler) struct HandlerLogRecord {
    level: HandlerLogLevel,
    level_name: String,
    message: String,
    fields: Value,
    truncated: bool,
}

impl HandlerLogRecord {
    const LINE_EVENT: &'static str = "yon.log";
    const EVENT: &'static str = "handler.log";
    const MAX_MESSAGE_CHARACTERS: usize = 2_000;
    const MAX_FIELDS_BYTES: usize = 8 * 1024;
    const KEPT_CONTROLS: [char; 2] = ['\n', '\t'];

    pub(in crate::handler) fn parse(line: &str) -> Option<Self> {
        Self::mentions_log(line)?;
        let value = serde_json::from_str::<Value>(line.trim()).ok()?;
        Self::from_value(&value)
    }

    pub(in crate::handler) fn reaches(&self, threshold: HandlerLogLevel) -> bool {
        self.level.reaches(threshold)
    }

    pub(in crate::handler) fn event(&self) -> Value {
        serde_json::json!({
            "event": Self::EVENT,
            "level": self.level_name,
            "message": self.message,
            "fields": self.fields,
            "truncated": self.truncated,
        })
    }

    fn mentions_log(line: &str) -> Option<()> {
        line.contains(Self::LINE_EVENT).then_some(())
    }

    fn from_value(value: &Value) -> Option<Self> {
        Self::named_log(value)?;
        let level_name = value.get("level")?.as_str()?;
        let level = HandlerLogLevel::of_line(level_name)?;
        let message = value.get("message")?.as_str()?;
        let fields = Self::fields_of(value.get("fields"))?;
        let (message, message_cut) = Self::bounded_message(message);
        let (fields, fields_cut) = Self::bounded_fields(fields);
        Some(Self {
            level,
            level_name: level_name.to_owned(),
            message,
            fields,
            truncated: [message_cut, fields_cut].contains(&true),
        })
    }

    fn named_log(value: &Value) -> Option<()> {
        let name = value.get("event")?.as_str()?;
        (name == Self::LINE_EVENT).then_some(())
    }

    fn fields_of(fields: Option<&Value>) -> Option<Value> {
        match fields {
            None | Some(Value::Null) => Some(Value::Object(Map::new())),
            Some(Value::Object(fields)) => Some(Value::Object(fields.clone())),
            Some(_) => None,
        }
    }

    fn bounded_message(message: &str) -> (String, bool) {
        let visible: Vec<char> = message
            .chars()
            .filter(|character| Self::is_visible(*character))
            .collect();
        let truncated = visible.len() > Self::MAX_MESSAGE_CHARACTERS;
        let kept = visible
            .into_iter()
            .take(Self::MAX_MESSAGE_CHARACTERS)
            .collect();
        (kept, truncated)
    }

    fn is_visible(character: char) -> bool {
        !character.is_control() || Self::KEPT_CONTROLS.contains(&character)
    }

    fn bounded_fields(fields: Value) -> (Value, bool) {
        if fields.to_string().len() > Self::MAX_FIELDS_BYTES {
            return (Value::Object(Map::new()), true);
        }
        (fields, false)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::HandlerLogRecord;
    use crate::handler::process::HandlerLogLevel;

    #[test]
    fn a_log_line_becomes_a_handler_log_event() {
        let record = HandlerLogRecord::parse(
            r#"{"event":"yon.log","level":"warn","message":"slow","fields":{"ms":812}}"#,
        )
        .expect("log line");
        let event = record.event();
        assert_eq!(event["event"], "handler.log");
        assert_eq!(event["level"], "warn");
        assert_eq!(event["message"], "slow");
        assert_eq!(event["fields"]["ms"], 812);
        assert_eq!(event["truncated"], false);
        assert!(record.reaches(HandlerLogLevel::Info));
        assert!(!record.reaches(HandlerLogLevel::Error));
    }

    #[test]
    fn fields_are_optional_and_empty_when_left_out() {
        let record =
            HandlerLogRecord::parse(r#"{"event":"yon.log","level":"info","message":"ok"}"#)
                .expect("log line without fields");
        assert_eq!(record.event()["fields"], serde_json::json!({}));
        let null_fields = HandlerLogRecord::parse(
            r#"{"event":"yon.log","level":"info","message":"ok","fields":null}"#,
        )
        .expect("log line with null fields");
        assert_eq!(null_fields.event()["fields"], serde_json::json!({}));
    }

    #[test]
    fn other_lines_are_not_log_lines() {
        for line in [
            "plain text",
            "{\"event\":\"handler.relay_failed\",\"request_id\":\"0ABC\"}",
            r#"{"event":"yon.log","level":"loud","message":"x"}"#,
            r#"{"event":"yon.log","level":"info","message":7}"#,
            r#"{"event":"yon.log","level":"info","message":"x","fields":[1]}"#,
            r#"{"event":"yon.log","level":"info""#,
        ] {
            assert!(HandlerLogRecord::parse(line).is_none(), "{line}");
        }
    }

    #[test]
    fn long_messages_and_large_fields_are_cut_and_marked() {
        let long = "m".repeat(2_500);
        let line = serde_json::json!({
            "event": "yon.log",
            "level": "info",
            "message": format!("{long}\u{1b}[31m"),
        })
        .to_string();
        let event = HandlerLogRecord::parse(&line).expect("long line").event();
        assert_eq!(event["message"].as_str().map(str::len), Some(2_000));
        assert_eq!(event["truncated"], true);
        let large = serde_json::json!({
            "event": "yon.log",
            "level": "info",
            "message": "large\tfields\u{0}",
            "fields": {"blob": "b".repeat(9 * 1024)},
        })
        .to_string();
        let event = HandlerLogRecord::parse(&large).expect("large line").event();
        assert_eq!(event["message"], "large\tfields");
        assert_eq!(event["fields"], serde_json::json!({}));
        assert_eq!(event["truncated"], true);
    }
}
