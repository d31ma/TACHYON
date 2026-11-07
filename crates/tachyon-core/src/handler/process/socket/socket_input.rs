use crate::handler::frame::FramePrefix;
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SocketInput {
    Message(String),
    Sessions(Vec<String>),
    Close,
}

impl SocketInput {
    pub(super) fn frame(&self, request_id: &str, style: FramePrefix) -> Vec<u8> {
        let payload = self.frame_value(request_id).to_string().into_bytes();
        [style.encode(payload.len()), payload].concat()
    }

    fn frame_value(&self, request_id: &str) -> Value {
        match self {
            Self::Message(data) => json!({
                "protocol_version": 1,
                "kind": "message",
                "request_id": request_id,
                "body": { "encoding": "utf8", "data": data },
            }),
            Self::Sessions(ids) => json!({
                "protocol_version": 1,
                "kind": "sessions",
                "request_id": request_id,
                "sessions": ids,
            }),
            Self::Close => json!({
                "protocol_version": 1,
                "kind": "close",
                "request_id": request_id,
            }),
        }
    }
}
