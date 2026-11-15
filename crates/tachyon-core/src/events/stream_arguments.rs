use super::StreamOptions;
use crate::behavior::{ReconnectPolicy, RemoteOrigin, StreamEventName, StreamQuery};
use serde_json::Value;

pub(crate) struct StreamArguments;

impl StreamArguments {
    const PATH: &'static str = "path";
    const RECONNECT: &'static str = "reconnect";
    const EVENT: &'static str = "event";
    const QUERY: &'static str = "query";
    const SOCKET: &'static str = "SocketRequest";
    const KEYS: [&'static str; 4] = [Self::PATH, Self::RECONNECT, Self::EVENT, Self::QUERY];
    const QUOTES: [char; 2] = ['"', '\''];
    const SOCKET_EVENT: &'static str = "@SocketRequest takes no event name: a WebSocket message has no event type. Use event on @StreamRequest";
    const SOCKET_QUERY: &'static str = "@SocketRequest takes no query: a socket sends its own messages with send(message). Use query on @StreamRequest";

    pub(crate) fn read(written: &str, text: &str) -> Result<StreamOptions, String> {
        let value = Self::parsed(written, text)?;
        Self::check_keys(written, &value)?;
        let path = Self::path(written, &value)?;
        RemoteOrigin::check_written(&path)?;
        let reconnect = value
            .get(Self::RECONNECT)
            .map(ReconnectPolicy::read)
            .transpose()
            .map_err(|message| format!("@{written} {message}"))?;
        let event = Self::event(written, &value)?;
        let query = Self::query(written, &value)?;
        StreamQuery::check_path(&path, query.is_some())
            .map_err(|message| format!("@{written}: {message}"))?;
        Ok(StreamOptions {
            path,
            reconnect,
            event,
            query,
        })
    }

    fn form(written: &str) -> String {
        format!(
            "@{written} takes an optional path in quotes, like @{written}(\"/ticks\"), or an object with \"path\", \"reconnect\" and, on a stream, \"event\" and \"query\", such as {{\"path\": \"/ticks\", \"event\": \"tick\", \"reconnect\": {{\"maxTries\": 0}}}}. The path is added to the @HTTPClient path"
        )
    }

    fn event(written: &str, value: &Value) -> Result<Option<StreamEventName>, String> {
        let event = value.get(Self::EVENT);
        Self::refuse_on_socket(written, event.is_some(), Self::SOCKET_EVENT)?;
        event
            .map(StreamEventName::read)
            .transpose()
            .map_err(|message| format!("@{written} {message}"))
    }

    fn query(written: &str, value: &Value) -> Result<Option<StreamQuery>, String> {
        let query = value.get(Self::QUERY);
        Self::refuse_on_socket(written, query.is_some(), Self::SOCKET_QUERY)?;
        query
            .map(StreamQuery::read)
            .transpose()
            .map_err(|message| format!("@{written} {message}"))
    }

    fn refuse_on_socket(written: &str, given: bool, refusal: &str) -> Result<(), String> {
        let refused = [written == Self::SOCKET, given] == [true, true];
        (!refused)
            .then_some(())
            .ok_or_else(|| String::from(refusal))
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
