use crate::Failure;
use crate::failure::diagnostic;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SocketOutput {
    Send { to: Option<String>, data: String },
    AskSessions,
    Close { code: u16 },
}

impl SocketOutput {
    pub(crate) const MAX_MESSAGE_BYTES: usize = 64 * 1024;
    const CLOSE_CODES: [u16; 2] = [1000, 1011];

    pub(super) fn read(frame: &Value, request_id: &str) -> Result<Self, Failure> {
        Self::check_owner(frame, request_id)?;
        Self::read_kind(frame)
    }

    pub(super) fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            2107,
            format!("Socket handler failed: {message}"),
            Some(String::from(
                "Fix the @SocketRoute class or its protocol adapter, then open the socket again.",
            )),
            None,
        ))
    }

    fn check_owner(frame: &Value, request_id: &str) -> Result<(), Failure> {
        let version = frame.get("protocol_version").and_then(Value::as_u64);
        let owner = frame.get("request_id").and_then(Value::as_str);
        if version != Some(1) || owner != Some(request_id) {
            return Err(Self::failure("A socket frame breaks Handler Protocol v1."));
        }
        Ok(())
    }

    fn read_kind(frame: &Value) -> Result<Self, Failure> {
        match frame.get("kind").and_then(Value::as_str) {
            Some("event") => Self::event(frame),
            Some("sessions") => Ok(Self::AskSessions),
            Some("close") => Ok(Self::close(frame)),
            Some("response") => Err(Self::failure(
                "The socket handler stopped before it could hold the connection.",
            )),
            _ => Err(Self::failure("A socket frame has an unknown kind.")),
        }
    }

    fn close(frame: &Value) -> Self {
        let code = frame
            .get("code")
            .and_then(Value::as_u64)
            .and_then(|code| u16::try_from(code).ok())
            .filter(|code| Self::CLOSE_CODES.contains(code))
            .unwrap_or(1000);
        Self::Close { code }
    }

    fn event(frame: &Value) -> Result<Self, Failure> {
        let data = Self::utf8_body(frame)?;
        Self::check_message_size(data)?;
        let to = frame.get("to").and_then(Value::as_str).map(str::to_owned);
        Ok(Self::Send {
            to,
            data: data.to_owned(),
        })
    }

    fn utf8_body(frame: &Value) -> Result<&str, Failure> {
        let encoding = frame.pointer("/body/encoding").and_then(Value::as_str);
        let data = frame.pointer("/body/data").and_then(Value::as_str);
        let (Some("utf8"), Some(data)) = (encoding, data) else {
            return Err(Self::failure("A socket message has no UTF-8 body."));
        };
        Ok(data)
    }

    fn check_message_size(data: &str) -> Result<(), Failure> {
        if data.len() > Self::MAX_MESSAGE_BYTES {
            return Err(Self::failure(
                "A socket message from the handler is larger than 64 KiB.",
            ));
        }
        Ok(())
    }
}
