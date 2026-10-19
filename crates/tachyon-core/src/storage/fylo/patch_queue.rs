use std::sync::{Mutex, PoisonError};

use serde_json::{Map, Value, json};
use tokio::sync::oneshot;

use super::FyloError;

pub(super) struct PatchQueue {
    waiting: Mutex<Vec<QueuedPatch>>,
}

pub(super) type PatchAnswer = Result<Vec<Value>, FyloError>;

struct QueuedPatch {
    collection: String,
    id: String,
    fields: Map<String, Value>,
    answer: oneshot::Sender<PatchAnswer>,
}

pub(super) struct CombinedPatch {
    collection: String,
    id: String,
    fields: Map<String, Value>,
    answers: Vec<oneshot::Sender<PatchAnswer>>,
}

impl PatchQueue {
    pub(super) fn new() -> Self {
        Self {
            waiting: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn add(
        &self,
        collection: &str,
        id: &str,
        fields: Map<String, Value>,
    ) -> oneshot::Receiver<PatchAnswer> {
        let (answer, receiver) = oneshot::channel();
        let patch = QueuedPatch {
            collection: collection.to_owned(),
            id: id.to_owned(),
            fields,
            answer,
        };
        self.waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(patch);
        receiver
    }

    pub(super) fn take_combined(&self) -> Vec<CombinedPatch> {
        let waiting =
            std::mem::take(&mut *self.waiting.lock().unwrap_or_else(PoisonError::into_inner));
        waiting
            .into_iter()
            .filter(QueuedPatch::still_awaited)
            .fold(Vec::new(), Self::combine)
    }

    pub(super) fn received(mut receiver: oneshot::Receiver<PatchAnswer>) -> PatchAnswer {
        receiver
            .try_recv()
            .unwrap_or_else(|_| Err(FyloError::protocol("FYLO patch was interrupted.")))
    }

    fn combine(mut combined: Vec<CombinedPatch>, patch: QueuedPatch) -> Vec<CombinedPatch> {
        let same_document = combined.iter_mut().find(|earlier| earlier.targets(&patch));
        match same_document {
            Some(earlier) => earlier.absorb(patch),
            None => combined.push(CombinedPatch::starting_with(patch)),
        }
        combined
    }
}

impl QueuedPatch {
    fn still_awaited(&self) -> bool {
        !self.answer.is_closed()
    }
}

impl CombinedPatch {
    pub(super) fn requests(&self) -> Vec<(&'static str, Value)> {
        let change = json!({
            "collection": self.collection,
            "id": self.id,
            "newDoc": Value::Object(self.fields.clone()),
        });
        let reading = json!({ "collection": self.collection, "id": self.id });
        vec![("patchDoc", change), ("getDoc", reading)]
    }

    pub(super) fn answer(self, answer: &PatchAnswer) {
        for sender in self.answers {
            let _ = sender.send(answer.clone());
        }
    }

    fn targets(&self, patch: &QueuedPatch) -> bool {
        [self.collection == patch.collection, self.id == patch.id] == [true, true]
    }

    fn absorb(&mut self, patch: QueuedPatch) {
        self.fields.extend(patch.fields);
        self.answers.push(patch.answer);
    }

    fn starting_with(patch: QueuedPatch) -> Self {
        Self {
            collection: patch.collection,
            id: patch.id,
            fields: patch.fields,
            answers: vec![patch.answer],
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{CombinedPatch, PatchQueue};

    fn fields(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn waiting_patches_to_one_document_become_one_write_in_arrival_order() {
        let queue = PatchQueue::new();
        let first = queue.add("counters", "A", fields(&json!({"x": 1, "y": 1})));
        let other = queue.add("counters", "B", fields(&json!({"z": 3})));
        let second = queue.add("counters", "A", fields(&json!({"y": 2})));
        let abandoned = queue.add("counters", "A", fields(&json!({"gone": true})));
        drop(abandoned);
        let combined = queue.take_combined();
        assert_eq!(combined.len(), 2);
        let requests = combined
            .iter()
            .map(CombinedPatch::requests)
            .collect::<Vec<_>>();
        assert_eq!(requests[0][0].1["newDoc"], json!({"x": 1, "y": 2}));
        assert_eq!(requests[0][1].0, "getDoc");
        assert_eq!(requests[1][0].1["id"], json!("B"));
        assert!(queue.take_combined().is_empty());
        for patch in combined {
            patch.answer(&Ok(vec![json!("saved")]));
        }
        for receiver in [first, other, second] {
            let answer = PatchQueue::received(receiver).ok();
            assert_eq!(answer, Some(vec![json!("saved")]));
        }
    }

    #[test]
    fn a_patch_whose_writer_stopped_reports_an_interrupted_write() {
        let queue = PatchQueue::new();
        let receiver = queue.add("counters", "A", fields(&json!({"x": 1})));
        drop(queue.take_combined());
        let answer = PatchQueue::received(receiver);
        assert_eq!(
            answer.err().map(|error| error.code),
            Some("EPROTOCOL".to_owned())
        );
    }
}
