use crate::handler::gateway::GatewayCalls;
use futures_util::future::OptionFuture;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(in crate::handler) struct PipedResources {
    pub(super) gateways: Option<Arc<GatewayCalls>>,
}

impl PipedResources {
    const RESOURCE: &'static str = "resource";
    const GATEWAY: &'static str = "gateway";
    const FAILED: &'static [u8] = br#"{"error":"adapter"}"#;

    pub(in crate::handler) fn command(frame: &[u8]) -> Option<Value> {
        let mut message = serde_json::from_slice::<Value>(frame).ok()?;
        let asks = message.get("kind").and_then(Value::as_str) == Some(Self::RESOURCE);
        asks.then(|| message["command"].take())
    }

    pub(in crate::handler) async fn answer(&self, command: &Value) -> Vec<u8> {
        let gateway = command.get("operation").and_then(Value::as_str) == Some(Self::GATEWAY);
        let calls = self.gateways.as_ref().filter(|_| gateway);
        OptionFuture::from(calls.map(|calls| calls.answer(command)))
            .await
            .unwrap_or_else(|| Self::FAILED.to_vec())
    }
}
