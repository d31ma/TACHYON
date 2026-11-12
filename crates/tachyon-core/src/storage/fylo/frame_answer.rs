use serde_json::{Value, json};

use super::FyloError;

pub(super) struct FrameAnswer;

impl FrameAnswer {
    pub(super) fn from_line(line: &str) -> Result<Value, FyloError> {
        let envelope: Value = serde_json::from_str(line.trim_end())
            .map_err(|_| FyloError::protocol("FYLO returned a malformed frame."))?;
        Self::result_of(&envelope)
    }

    fn result_of(envelope: &Value) -> Result<Value, FyloError> {
        if envelope["ok"] == json!(true) {
            return Ok(envelope["result"].clone());
        }
        let code = envelope["error"]["code"].as_str().unwrap_or("EUNKNOWN");
        Err(FyloError::coded(code))
    }
}
