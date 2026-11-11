use crate::Failure;
use crate::events::{EventModule, EventTimer};
use crate::failure::diagnostic;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) struct ServiceTimers;

impl ServiceTimers {
    const MARKER_TEXT: &'static str = "Every";
    const UNREADABLE_SOURCE: u16 = 1605;
    const REALM: &'static str = "tac";

    pub(crate) fn read(
        snapshot_root: &Path,
        source: &Path,
        class: &str,
    ) -> Result<Vec<EventTimer>, Failure> {
        let path = snapshot_root.join(source);
        let text =
            std::fs::read_to_string(&path).map_err(|error| Self::unreadable(&path, &error))?;
        Self::timers_of(&path, &text, class)
    }

    pub(crate) fn described(timers: &[EventTimer]) -> Value {
        Value::Array(timers.iter().map(Self::entry).collect())
    }

    fn timers_of(path: &Path, text: &str, class: &str) -> Result<Vec<EventTimer>, Failure> {
        if !text.contains(Self::MARKER_TEXT) {
            return Ok(Vec::new());
        }
        let module = crate::events::extract(path, text, Self::REALM)?;
        Ok(Self::owned(module, class))
    }

    fn owned(module: Option<EventModule>, class: &str) -> Vec<EventTimer> {
        module
            .into_iter()
            .flat_map(|module| module.types)
            .filter(|event_type| event_type.name == class)
            .flat_map(|event_type| event_type.timers)
            .collect()
    }

    fn entry(timer: &EventTimer) -> Value {
        json!({ "name": timer.method, "every": timer.every })
    }

    fn unreadable(path: &Path, error: &std::io::Error) -> Failure {
        let message = format!(
            "{}: the service source cannot be read: {error}",
            path.display()
        );
        Failure::one(diagnostic(Self::UNREADABLE_SOURCE, &message, None, None))
    }
}
