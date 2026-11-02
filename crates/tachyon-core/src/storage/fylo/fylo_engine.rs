use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value, json};

use super::FyloError;
use crate::sorted_json::SortedJson;
use crate::storage::admission_permit::AdmissionPermit;

pub(super) struct FyloLauncher {
    engine: Mutex<Option<Arc<FyloEngine>>>,
}

pub(super) struct FyloProcess {
    engine: Arc<FyloEngine>,
}

struct FyloEngine {
    root: fylo::Root,
    turns: Arc<tokio::sync::Mutex<()>>,
}

impl FyloLauncher {
    pub(super) async fn start(root: &Path) -> Result<(Self, FyloProcess), FyloError> {
        let path = root.to_path_buf();
        let opened = tokio::task::spawn_blocking(move || {
            fylo::Root::open(path, fylo::OpenOptions::default())
        })
        .await
        .map_err(|_| FyloError::protocol("FYLO could not open the root."))?
        .map_err(|error| FyloError::from_engine(&error))?;
        let engine = Arc::new(FyloEngine {
            root: opened,
            turns: Arc::new(tokio::sync::Mutex::new(())),
        });
        let process = FyloProcess {
            engine: Arc::clone(&engine),
        };
        let launcher = Self {
            engine: Mutex::new(Some(engine)),
        };
        Ok((launcher, process))
    }

    pub(super) fn launch(&self) -> Result<FyloProcess, FyloError> {
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| FyloError::protocol("FYLO datasource is closed."))?;
        Ok(FyloProcess { engine })
    }

    pub(super) async fn stop(&self) {
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let closing = tokio::task::spawn_blocking(move || engine.map(FyloEngine::close_if_last));
        let _ = closing.await;
    }
}

impl FyloEngine {
    fn close_if_last(engine: Arc<Self>) {
        if let Some(engine) = Arc::into_inner(engine) {
            let _ = engine.root.close();
        }
    }
}

impl FyloProcess {
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

    pub(super) fn stop(self) -> std::future::Ready<()> {
        drop(self.engine);
        std::future::ready(())
    }

    pub(super) async fn request(&mut self, request: Value) -> Result<Value, FyloError> {
        let request = SortedJson::into_sorted(request);
        self.run(move |root| Self::answer(root, &request)).await
    }

    async fn run<Work>(&self, work: Work) -> Result<Value, FyloError>
    where
        Work: FnOnce(&fylo::Root) -> Result<Value, FyloError> + Send + 'static,
    {
        let engine = Arc::clone(&self.engine);
        let permit = AdmissionPermit::shared();
        let turn = Arc::clone(&engine.turns).lock_owned().await;
        let call = tokio::task::spawn_blocking(move || {
            let _held_until_done = (permit, turn);
            let answer = work(&engine.root);
            FyloEngine::close_if_last(engine);
            answer
        });
        call.await
            .unwrap_or_else(|_| Err(FyloError::protocol("FYLO call stopped unexpectedly.")))
    }

    fn answer(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        match Self::field(request, "op")? {
            "putData" => Self::put(root, request),
            "patchDoc" => Self::patch(root, request),
            "getDoc" => Self::read(root, request, fylo::Collection::get),
            "getLatest" => Self::read(root, request, fylo::Collection::latest),
            "delDoc" => Self::delete(root, request),
            "findDocs" => Self::find(root, request),
            "createCollection" => Self::create_collection(root, request),
            "dropCollection" => Self::drop_collection(root, request),
            "inspectCollection" => Self::inspect_collection(root, request),
            "queuePublish" => Self::publish(root, request),
            "queueClaim" => Self::claim(root, request),
            "queueAck" => Self::ack(root, request),
            "queueNack" => Self::nack(root, request),
            "executeSQL" => Err(FyloError::sql_unavailable()),
            _ => Err(FyloError::bad_request()),
        }
    }

    fn put(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        let data = Self::object(request, "data")?;
        let id = root
            .collection(collection)
            .put(data)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(Value::String(id))
    }

    fn patch(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        let id = Self::field(request, "id")?;
        let changes = Self::object(request, "newDoc")?;
        root.collection(collection)
            .patch(id, &changes)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(Value::String(id.to_owned()))
    }

    fn read<'call, Read>(
        root: &'call fylo::Root,
        request: &'call Value,
        read: Read,
    ) -> Result<Value, FyloError>
    where
        Read: FnOnce(&fylo::Collection<'call>, &str) -> Result<Option<fylo::Document>, fylo::Error>,
    {
        let collection = root.collection(Self::field(request, "collection")?);
        let id = Self::field(request, "id")?;
        let found = read(&collection, id).map_err(|error| FyloError::from_engine(&error))?;
        Ok(found.map_or_else(|| json!({}), |document| json!({ id: document })))
    }

    fn delete(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        let id = Self::field(request, "id")?;
        root.collection(collection)
            .delete(id)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(json!({ "deleted": true }))
    }

    fn find(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        let query = request.get("query").cloned().unwrap_or_else(|| json!({}));
        let query =
            fylo::Query::from_json(&query).map_err(|error| FyloError::from_engine(&error))?;
        let rows = root
            .collection(collection)
            .find(&query)
            .map_err(|error| FyloError::from_engine(&error))?;
        let documents: Map<String, Value> = rows
            .into_iter()
            .map(|(id, document)| (id, Value::Object(document)))
            .collect();
        Self::within_response_limit(Value::Object(documents))
    }

    fn within_response_limit(answer: Value) -> Result<Value, FyloError> {
        let size = serde_json::to_vec(&answer).map_or(usize::MAX, |encoded| encoded.len());
        if size > Self::MAX_RESPONSE_BYTES {
            return Err(FyloError::response_too_large());
        }
        Ok(answer)
    }

    fn create_collection(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        root.create_collection(collection)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(json!({ "collection": collection, "kind": "document" }))
    }

    fn drop_collection(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        root.drop_collection(collection)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(json!({ "collection": collection }))
    }

    fn inspect_collection(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let collection = Self::field(request, "collection")?;
        let inspected = root
            .inspect_collection(collection)
            .map_err(|error| FyloError::from_engine(&error))?;
        Ok(json!({
            "collection": inspected.collection,
            "exists": true,
            "docsStored": inspected.document_count + inspected.file_count,
            "deletedDocs": inspected.deleted_count,
        }))
    }

    fn publish(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let topic = Self::field(request, "topic")?;
        let payload = request.get("payload").cloned().unwrap_or(Value::Null);
        let options = fylo::PublishOptions {
            delay_ms: Self::number(request, "delayMs", 0)?,
            idempotency_key: request
                .get("idempotencyKey")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        };
        let published = root
            .queue(topic)
            .publish(payload, &options)
            .map_err(|error| FyloError::from_engine(&error))?;
        serde_json::to_value(published).map_err(|_| FyloError::bad_request())
    }

    fn claim(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let topic = Self::field(request, "topic")?;
        let group = Self::field(request, "group")?;
        let defaults = fylo::ClaimOptions::default();
        let options = fylo::ClaimOptions {
            max_messages: Self::count(request, "maxMessages", defaults.max_messages)?,
            visibility_timeout_ms: Self::number(
                request,
                "visibilityTimeoutMs",
                defaults.visibility_timeout_ms,
            )?,
            max_attempts: u32::try_from(Self::number(
                request,
                "maxAttempts",
                u64::from(defaults.max_attempts),
            )?)
            .map_err(|_| FyloError::bad_request())?,
            max_bytes: defaults.max_bytes,
        };
        let deliveries = root
            .queue(topic)
            .claim(group, options)
            .map_err(|error| FyloError::from_engine(&error))?;
        serde_json::to_value(deliveries).map_err(|_| FyloError::bad_request())
    }

    fn ack(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let (queue, settlement) = Self::settlement(root, request)?;
        let [group, id, receipt] = settlement;
        let acknowledged = queue
            .ack(group, id, receipt)
            .map_err(|error| FyloError::from_engine(&error))?;
        serde_json::to_value(acknowledged).map_err(|_| FyloError::bad_request())
    }

    fn nack(root: &fylo::Root, request: &Value) -> Result<Value, FyloError> {
        let (queue, settlement) = Self::settlement(root, request)?;
        let [group, id, receipt] = settlement;
        let delay_ms = Self::number(request, "delayMs", 0)?;
        let returned = queue
            .nack(group, id, receipt, delay_ms, "")
            .map_err(|error| FyloError::from_engine(&error))?;
        serde_json::to_value(returned).map_err(|_| FyloError::bad_request())
    }

    fn settlement<'call>(
        root: &'call fylo::Root,
        request: &'call Value,
    ) -> Result<(fylo::Queue<'call>, [&'call str; 3]), FyloError> {
        let queue = root.queue(Self::field(request, "topic")?);
        let settlement = [
            Self::field(request, "group")?,
            Self::field(request, "id")?,
            Self::field(request, "receipt")?,
        ];
        Ok((queue, settlement))
    }

    fn field<'request>(request: &'request Value, key: &str) -> Result<&'request str, FyloError> {
        request
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(FyloError::bad_request)
    }

    fn object(request: &Value, key: &str) -> Result<Map<String, Value>, FyloError> {
        request
            .get(key)
            .and_then(Value::as_object)
            .cloned()
            .ok_or_else(FyloError::bad_request)
    }

    fn number(request: &Value, key: &str, default: u64) -> Result<u64, FyloError> {
        match request.get(key) {
            None => Ok(default),
            Some(value) => value.as_u64().ok_or_else(FyloError::bad_request),
        }
    }

    fn count(request: &Value, key: &str, default: usize) -> Result<usize, FyloError> {
        let fallback = u64::try_from(default).map_err(|_| FyloError::bad_request())?;
        let number = Self::number(request, key, fallback)?;
        usize::try_from(number).map_err(|_| FyloError::bad_request())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::time::Duration;

    use serde_json::{Map, Value, json};
    use tokio::sync::Semaphore;

    use super::{FyloError, FyloLauncher, FyloProcess};
    use crate::storage::admission_permit::AdmissionPermit;

    struct AbandonedWrite {
        admission: Arc<Semaphore>,
        release: Sender<()>,
    }

    impl AbandonedWrite {
        const DEADLINE: Duration = Duration::from_millis(200);

        async fn start(process: &FyloProcess) -> Self {
            let admission = Arc::new(Semaphore::new(16));
            let permit = Arc::clone(&admission)
                .try_acquire_owned()
                .unwrap_or_else(|error| panic!("{error}"));
            let (release, released) = std::sync::mpsc::channel::<()>();
            let slow_write = move |root: &fylo::Root| Self::write_after(root, &released);
            let abandoned = AdmissionPermit::hold_during(permit, process.run(slow_write));
            let timed_out = tokio::time::timeout(Self::DEADLINE, abandoned).await;
            assert!(timed_out.is_err(), "the slow call must time out");
            Self { admission, release }
        }

        fn write_after(root: &fylo::Root, released: &Receiver<()>) -> Result<Value, FyloError> {
            let _ = released.recv();
            let mut document = Map::new();
            document.insert("from".to_owned(), json!("abandoned"));
            root.collection("events")
                .put(document)
                .map(Value::String)
                .map_err(|error| FyloError::from_engine(&error))
        }

        fn finish(&self) {
            self.release
                .send(())
                .unwrap_or_else(|error| panic!("{error}"));
        }

        fn permits(&self) -> usize {
            self.admission.available_permits()
        }

        async fn permits_return(&self) -> bool {
            for _ in 0..100 {
                if self.permits() == 16 {
                    return true;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            false
        }
    }

    fn canonical(directory: &tempfile::TempDir) -> std::path::PathBuf {
        std::fs::canonicalize(directory.path())
            .unwrap_or_else(|error| panic!("canonical root: {error}"))
    }

    async fn started(directory: &tempfile::TempDir) -> (FyloLauncher, FyloProcess) {
        let (launcher, mut process) = FyloLauncher::start(&canonical(directory))
            .await
            .unwrap_or_else(|error| panic!("start: {}", error.code));
        answer(
            &mut process,
            json!({"op": "createCollection", "collection": "events"}),
        )
        .await;
        (launcher, process)
    }

    async fn answer(process: &mut FyloProcess, request: Value) -> Value {
        process
            .request(request)
            .await
            .unwrap_or_else(|error| panic!("request: {}", error.code))
    }

    fn next_call(
        launcher: &FyloLauncher,
        request: Value,
    ) -> tokio::task::JoinHandle<Result<Value, FyloError>> {
        let mut next = launcher
            .launch()
            .unwrap_or_else(|error| panic!("launch: {}", error.code));
        tokio::spawn(async move { next.request(request).await })
    }

    async fn reopens(directory: &tempfile::TempDir) -> bool {
        for _ in 0..100 {
            if let Ok(reopened) =
                fylo::Root::open(canonical(directory), fylo::OpenOptions::default())
            {
                reopened.close().unwrap_or_else(|error| panic!("{error}"));
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        false
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_abandoned_call_keeps_its_admission_permit_until_it_really_ends() {
        let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let (launcher, process) = started(&directory).await;
        let abandoned = AbandonedWrite::start(&process).await;
        assert_eq!(abandoned.permits(), 15, "held right after the deadline");
        tokio::time::sleep(AbandonedWrite::DEADLINE * 2).await;
        assert_eq!(abandoned.permits(), 15, "held while the call still runs");
        abandoned.finish();
        assert!(abandoned.permits_return().await, "returned when it ends");
        drop(process);
        launcher.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_late_result_never_reaches_a_later_caller() {
        let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let (launcher, process) = started(&directory).await;
        let abandoned = AbandonedWrite::start(&process).await;
        let later_write = next_call(
            &launcher,
            json!({"op": "putData", "collection": "events", "data": {"from": "later"}}),
        );
        tokio::time::sleep(AbandonedWrite::DEADLINE).await;
        assert!(
            !later_write.is_finished(),
            "the later call waits for the abandoned one"
        );
        abandoned.finish();
        let later_id = later_write
            .await
            .unwrap_or_else(|error| panic!("{error}"))
            .unwrap_or_else(|error| panic!("putData: {}", error.code));
        let later_id = later_id
            .as_str()
            .unwrap_or_else(|| panic!("putData answers a TTID: {later_id}"))
            .to_owned();
        let read = next_call(
            &launcher,
            json!({"op": "getDoc", "collection": "events", "id": later_id}),
        );
        let read = read
            .await
            .unwrap_or_else(|error| panic!("{error}"))
            .unwrap_or_else(|error| panic!("getDoc: {}", error.code));
        assert_eq!(
            read,
            json!({ later_id.as_str(): {"from": "later"} }),
            "the later write's id names the later document, not the abandoned one"
        );
        let all = next_call(
            &launcher,
            json!({"op": "findDocs", "collection": "events", "query": {}}),
        );
        let all = all
            .await
            .unwrap_or_else(|error| panic!("{error}"))
            .unwrap_or_else(|error| panic!("findDocs: {}", error.code));
        let all = all
            .as_object()
            .unwrap_or_else(|| panic!("findDocs answers an object, not a late TTID: {all}"));
        assert_eq!(all.len(), 2, "the abandoned write still committed");
        drop(process);
        launcher.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn closing_during_an_abandoned_call_releases_the_lease_when_the_call_ends() {
        let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let (launcher, process) = started(&directory).await;
        let abandoned = AbandonedWrite::start(&process).await;
        drop(process);
        launcher.stop().await;
        assert!(
            fylo::Root::open(canonical(&directory), fylo::OpenOptions::default()).is_err(),
            "the running call still holds the lease"
        );
        assert!(
            launcher.launch().is_err(),
            "a closed datasource takes no new calls"
        );
        abandoned.finish();
        assert!(abandoned.permits_return().await);
        assert!(
            reopens(&directory).await,
            "the lease returns when the call ends"
        );
    }

    #[tokio::test]
    async fn closing_an_idle_datasource_releases_the_lease_at_once() {
        let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let (launcher, process) = started(&directory).await;
        drop(process);
        launcher.stop().await;
        let reopened = fylo::Root::open(canonical(&directory), fylo::OpenOptions::default())
            .unwrap_or_else(|error| panic!("the lease returns on close: {error}"));
        reopened.close().unwrap_or_else(|error| panic!("{error}"));
    }
}
