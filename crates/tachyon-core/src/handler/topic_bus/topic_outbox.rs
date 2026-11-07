use super::{TopicBus, TopicDelivery};
use crate::handler::source::HandlerSource;
use crate::topics::Topics;
use serde_json::Value;
use std::sync::Arc;
use tachyon_contracts::HandlerRequest;

pub(crate) struct TopicOutbox {
    bus: Arc<TopicBus>,
    topics: Vec<String>,
    origin: String,
    hops: u64,
}

impl TopicOutbox {
    pub(crate) fn for_invocation(
        bus: &Arc<TopicBus>,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) -> Option<Self> {
        let topics = source.topic_names().to_vec();
        let has_topics = !topics.is_empty();
        has_topics.then(|| Self {
            bus: Arc::clone(bus),
            topics,
            origin: source.relative_path().to_owned(),
            hops: Self::delivered_hops(request),
        })
    }

    pub(crate) fn publish(&self, command: &Value) -> Result<(), &'static str> {
        let topic = command
            .get("topic")
            .and_then(Value::as_str)
            .ok_or("A topic publication names its model.")?;
        self.check_topic_is_known(topic)?;
        let slug = Topics::slug(topic).ok_or("A topic name must be a bounded model name.")?;
        let written = command
            .get("payload_json")
            .and_then(Value::as_str)
            .and_then(|text| serde_json::from_str::<Value>(text).ok());
        let payload = written
            .as_ref()
            .or_else(|| command.get("payload"))
            .filter(|value| value.is_object())
            .ok_or("A topic payload is a model value.")?;
        self.check_hops()?;
        let record = serde_json::to_string(payload).map_err(|_| "Topic payload is invalid.")?;
        Self::check_record_size(&record)?;
        self.bus.append(&slug, &record)?;
        self.bus.queue(TopicDelivery {
            topic: topic.to_owned(),
            payload: payload.clone(),
            origin: self.origin.clone(),
            hops: self.hops + 1,
        });
        Ok(())
    }

    fn check_topic_is_known(&self, topic: &str) -> Result<(), &'static str> {
        if !self.topics.iter().any(|known| known == topic) {
            return Err("Only a @Topic model of this language is published to the topic log.");
        }
        Ok(())
    }

    fn check_hops(&self) -> Result<(), &'static str> {
        if self.hops >= TopicBus::MAX_HOPS {
            return Err("Topic events republished too many times.");
        }
        Ok(())
    }

    fn check_record_size(record: &str) -> Result<(), &'static str> {
        if record.len() > TopicBus::MAX_RECORD_BYTES {
            return Err("Topic payload exceeds 64 KiB.");
        }
        Ok(())
    }

    fn delivered_hops(request: &HandlerRequest) -> u64 {
        if request.operation != TopicBus::OPERATION {
            return 0;
        }
        request
            .body
            .as_ref()
            .and_then(|body| serde_json::from_str::<Value>(&body.data).ok())
            .and_then(|body| body.get("hops").and_then(Value::as_u64))
            .unwrap_or(TopicBus::MAX_HOPS)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::TopicBus;
    use super::TopicOutbox;
    use serde_json::json;
    use std::sync::Arc;

    fn outbox(
        root: &std::path::Path,
        hops: u64,
    ) -> (
        TopicOutbox,
        tokio::sync::mpsc::Receiver<super::super::TopicDelivery>,
    ) {
        let project = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())
            .expect("project");
        let folder = super::super::TopicFolder::resolve(Arc::new(project), None).expect("folder");
        let (bus, receiver) = TopicBus::open(folder);
        let outbox = TopicOutbox {
            bus,
            topics: vec![String::from("OrderPlaced")],
            origin: String::from("server/controllers/orders/OrdersController.py"),
            hops,
        };
        (outbox, receiver)
    }

    #[test]
    fn a_topic_publication_is_appended_and_queued_once() {
        let root = tempfile::tempdir().expect("root");
        let (outbox, mut receiver) = outbox(root.path(), 0);
        let command =
            json!({"operation": "topic", "topic": "OrderPlaced", "payload": {"id": "A-1"}});
        outbox.publish(&command).expect("published");
        let written = json!({"operation": "topic", "topic": "OrderPlaced", "payload_json": "{\"id\":\"A-1\"}"});
        outbox.publish(&written).expect("published as text");
        let log = std::fs::read_to_string(root.path().join("data/topics/order-placed.jsonl"))
            .expect("log");
        assert_eq!(log, "{\"id\":\"A-1\"}\n{\"id\":\"A-1\"}\n");
        assert!(!root.path().join(".tachyon").exists());
        let delivery = receiver.try_recv().expect("delivery");
        assert_eq!(delivery.topic, "OrderPlaced");
        assert_eq!(delivery.hops, 1);
        assert_eq!(
            delivery.origin,
            "server/controllers/orders/OrdersController.py"
        );
    }

    #[test]
    fn the_bus_refuses_what_is_not_a_topic_model_of_the_language() {
        let root = tempfile::tempdir().expect("root");
        let (outbox, _receiver) = outbox(root.path(), 0);
        for command in [
            json!({"topic": "OrderShipped", "payload": {}}),
            json!({"topic": "../OrderPlaced", "payload": {}}),
            json!({"topic": "OrderPlaced", "payload": [1, 2]}),
            json!({"topic": "OrderPlaced", "payload": "text"}),
            json!({"topic": "OrderPlaced", "payload": {"text": "x".repeat(70 * 1024)}}),
        ] {
            assert!(outbox.publish(&command).is_err(), "{command}");
        }
        assert!(!root.path().join("data/topics/order-placed.jsonl").exists());
    }

    #[test]
    fn republishing_stops_after_eight_hops() {
        let root = tempfile::tempdir().expect("root");
        let (outbox, _receiver) = outbox(root.path(), 8);
        let command = json!({"topic": "OrderPlaced", "payload": {"id": "A-1"}});
        assert!(outbox.publish(&command).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_topic_directory_is_refused() {
        let root = tempfile::tempdir().expect("root");
        let elsewhere = tempfile::tempdir().expect("elsewhere");
        std::fs::create_dir(root.path().join("data")).expect("data");
        std::os::unix::fs::symlink(elsewhere.path(), root.path().join("data/topics"))
            .expect("link");
        let (outbox, _receiver) = outbox(root.path(), 0);
        let command = json!({"topic": "OrderPlaced", "payload": {"id": "A-1"}});
        assert!(outbox.publish(&command).is_err());
        assert!(
            std::fs::read_dir(elsewhere.path())
                .expect("read")
                .next()
                .is_none()
        );
    }
}
