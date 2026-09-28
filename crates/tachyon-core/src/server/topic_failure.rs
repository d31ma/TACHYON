#[derive(Clone, Copy)]
pub(in crate::server) enum TopicFailure {
    Open,
    Read,
    LogLimit,
    RecordLimit,
    InvalidJson,
}

impl TopicFailure {
    pub(in crate::server) fn frame(self) -> String {
        let (code, message, guidance) = self.details();
        Self::error_frame(code, message, guidance)
    }

    pub(in crate::server) fn error_frame(code: &str, message: &str, guidance: &str) -> String {
        let payload = serde_json::json!({
            "category": "topic",
            "code": code,
            "guidance": guidance,
            "message": message,
            "terminal": true,
        });
        format!("event: topic-error\ndata: {payload}\n\n")
    }

    const fn details(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Open => (
                "TY_TOPIC_OPEN",
                "Topic stream cannot be opened.",
                "Check that the topic log is a project-contained regular file.",
            ),
            Self::Read => (
                "TY_TOPIC_READ",
                "Topic stream read failed.",
                "Reconnect after the topic log is readable.",
            ),
            Self::LogLimit => (
                "TY_TOPIC_LOG_LIMIT",
                "Topic stream exceeded its byte limit.",
                "Rotate the topic log before reconnecting.",
            ),
            Self::RecordLimit => (
                "TY_TOPIC_RECORD_LIMIT",
                "Topic record exceeded its byte limit.",
                "Publish a smaller JSON record before reconnecting.",
            ),
            Self::InvalidJson => (
                "TY_TOPIC_INVALID_JSON",
                "Topic record is not valid JSON.",
                "Repair the NDJSON record before reconnecting.",
            ),
        }
    }
}
