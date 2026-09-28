use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SocketInput {
    Message(String),
    Sessions(Vec<String>),
    Close,
}

impl SocketInput {
    pub(super) fn frame(&self, request_id: &str) -> Vec<u8> {
        let payload = self.frame_value(request_id).to_string().into_bytes();
        let length = u32::try_from(payload.len()).unwrap_or(u32::MAX);
        let mut frame = length.to_be_bytes().to_vec();
        frame.extend(payload);
        frame
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
