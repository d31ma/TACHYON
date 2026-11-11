use crate::behavior::{ReconnectPolicy, RemoteOrigin};
use serde_json::Value;

pub(crate) struct StreamArguments;

impl StreamArguments {
    const PATH: &'static str = "path";
    const RECONNECT: &'static str = "reconnect";
    const KEYS: [&'static str; 2] = [Self::PATH, Self::RECONNECT];
    const QUOTES: [char; 2] = ['"', '\''];

    pub(crate) fn read(
        written: &str,
        text: &str,
    ) -> Result<(String, Option<ReconnectPolicy>), String> {
        let value = Self::parsed(written, text)?;
        Self::check_keys(written, &value)?;
        let path = Self::path(written, &value)?;
        RemoteOrigin::check_written(&path)?;
        let reconnect = value
            .get(Self::RECONNECT)
            .map(ReconnectPolicy::read)
            .transpose()
            .map_err(|message| format!("@{written} {message}"))?;
        Ok((path, reconnect))
    }

    fn form(written: &str) -> String {
        format!(
            "@{written} takes an optional path in quotes, like @{written}(\"/ticks\"), or an object such as {{\"path\": \"/ticks\", \"reconnect\": {{\"maxTries\": 0}}}}. The path is added to the @HTTPClient path"
        )
    }

    fn parsed(written: &str, text: &str) -> Result<Value, String> {
        let empty = text.is_empty().then_some(Value::Null);
        let quoted = Self::quoted(text).map(|path| Value::String(path.to_owned()));
        empty
            .or(quoted)
            .map_or_else(|| Self::object(written, text), Ok)
    }

    fn object(written: &str, text: &str) -> Result<Value, String> {
        crate::behavior::literal::parse(text)
            .ok()
            .filter(Value::is_object)
            .ok_or_else(|| Self::form(written))
    }

    fn quoted(text: &str) -> Option<&str> {
        Self::QUOTES
            .iter()
            .find_map(|mark| text.strip_prefix(*mark)?.strip_suffix(*mark))
    }

    fn check_keys(written: &str, value: &Value) -> Result<(), String> {
        let known = value
            .as_object()
            .into_iter()
            .flat_map(|object| object.keys())
            .all(|key| Self::KEYS.contains(&key.as_str()));
        known.then_some(()).ok_or_else(|| Self::form(written))
    }

    fn path(written: &str, value: &Value) -> Result<String, String> {
        let path = match value {
            Value::String(path) => Some(path.as_str()),
            Value::Object(object) => object.get(Self::PATH).map_or(Some(""), Value::as_str),
            _ => Some(""),
        };
        path.map(str::to_owned).ok_or_else(|| Self::form(written))
    }
}
