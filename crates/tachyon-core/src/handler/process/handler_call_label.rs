use crate::handler::HandlerSource;
use serde_json::Value;
use tachyon_contracts::HandlerRequest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::handler) struct HandlerCallLabel {
    pub(in crate::handler) request_id: String,
    pub(in crate::handler) route: String,
    pub(in crate::handler) handler: String,
}

impl HandlerCallLabel {
    pub(in crate::handler) fn of(source: &HandlerSource, request: &HandlerRequest) -> Self {
        Self {
            request_id: request.request_id.clone(),
            route: request.route.clone(),
            handler: source.relative_path().to_owned(),
        }
    }

    pub(in crate::handler) fn stamp(&self, event: &mut Value) {
        event["request_id"] = Value::from(self.request_id.as_str());
        event["route"] = Value::from(self.route.as_str());
        event["handler"] = Value::from(self.handler.as_str());
    }
}
