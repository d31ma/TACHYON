use serde_json::{Map, Value, json};

use super::collection_name::CollectionName;
use super::scroll_request::ScrollRequest;
use super::{FyloError, FyloStore, document_id, rows_of};
use crate::Failure;
use crate::storage::{failure, object_keys, text};

pub(super) struct DocumentCommands<'store> {
    store: &'store FyloStore,
}

impl<'store> DocumentCommands<'store> {
    const ID_FIELD: &'static str = "id";
    const MAX_FIELDS: usize = 64;
    const DOCUMENT_KEYS: [&'static str; 5] = ["operation", "command", "collection", "id", "value"];
    const COLLECTION_KEYS: [&'static str; 3] = ["operation", "command", "collection"];

    pub(super) fn new(store: &'store FyloStore) -> Self {
        Self { store }
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let name = text(command, "command")?;
        self.dispatch(name, command).await
    }

    async fn dispatch(&self, name: &str, command: &Value) -> Result<Value, Failure> {
        match name {
            "findById" => self.find_by_id(command).await,
            "insert" => self.insert(command).await,
            "patch" => self.patch(command).await,
            "deleteById" => self.delete_by_id(command).await,
            "scroll" => self.scroll(command).await,
            "createCollection" => self.create_collection(command).await,
            "dropCollection" => self.drop_collection(command).await,
            "collectionExists" => self.collection_exists(command).await,
            _ => Err(failure("Unsupported FYLO document command.")),
        }
    }

    async fn find_by_id(&self, command: &Value) -> Result<Value, Failure> {
        let answer = self
            .store
            .call("getLatest", Self::document_request(command)?)
            .await;
        answer.map_or_else(
            |error| Self::answer_when_absent(error.missing(), error, Value::Null),
            Self::first_document,
        )
    }

    async fn insert(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let mut document = Self::new_document(command)?;
        let request = json!({ "collection": collection, "data": Value::Object(document.clone()) });
        let id = self
            .store
            .call("putData", request)
            .await
            .map_err(FyloError::into_failure)?;
        document.insert(Self::ID_FIELD.to_owned(), id);
        Ok(Value::Object(document))
    }

    async fn patch(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let id = Self::id(command)?;
        let fields = Self::changes(command, id)?;
        let change = json!({ "collection": collection, "id": id, "newDoc": Value::Object(fields) });
        let reading = json!({ "collection": collection, "id": id });
        let requests = vec![("patchDoc", change), ("getDoc", reading)];
        let answers = self.store.call_together(requests).await;
        answers.map_or_else(
            |error| Self::answer_when_absent(error.missing(), error, Value::Null),
            Self::last_document,
        )
    }

    async fn delete_by_id(&self, command: &Value) -> Result<Value, Failure> {
        let request = Self::document_request(command)?;
        self.remove("delDoc", request, FyloError::missing).await
    }

    async fn remove(
        &self,
        operation: &str,
        request: Value,
        absent: fn(&FyloError) -> bool,
    ) -> Result<Value, Failure> {
        let answer = self.store.call(operation, request).await;
        answer.map_or_else(
            |error| Self::answer_when_absent(absent(&error), error, json!(false)),
            |_| Ok(json!(true)),
        )
    }

    async fn scroll(&self, command: &Value) -> Result<Value, Failure> {
        let collection = Self::collection(command)?;
        let scroll = ScrollRequest::parse(command)?;
        let scope = json!({ "collection": collection, "where": scroll.filter() }).to_string();
        let cursor = self.cursor_for(collection, &scroll, scope.clone()).await?;
        let page = self
            .store
            .scrolls
            .lock()
            .await
            .page(&cursor, &scope, scroll.size())?;
        Ok(json!({ "content": page.documents, "nextCursor": page.next_cursor }))
    }

    async fn cursor_for(
        &self,
        collection: &str,
        scroll: &ScrollRequest,
        scope: String,
    ) -> Result<String, Failure> {
        match scroll.cursor() {
            Some(cursor) => Ok(cursor.to_owned()),
            None => self.open_snapshot(collection, scroll, scope).await,
        }
    }

    async fn open_snapshot(
        &self,
        collection: &str,
        scroll: &ScrollRequest,
        scope: String,
    ) -> Result<String, Failure> {
        let request = json!({ "collection": collection, "query": scroll.filter() });
        let answer = self
            .store
            .call("findDocs", request)
            .await
            .map_err(Self::snapshot_failure)?;
        let documents = Self::sorted_documents(&answer);
        self.store.scrolls.lock().await.open(scope, documents)
    }

    async fn create_collection(&self, command: &Value) -> Result<Value, Failure> {
        self.store
            .call("createCollection", Self::collection_request(command)?)
            .await
            .map_err(FyloError::into_failure)?;
        Ok(json!(true))
    }

    async fn drop_collection(&self, command: &Value) -> Result<Value, Failure> {
        let request = Self::collection_request(command)?;
        self.remove("dropCollection", request, FyloError::collection_missing)
            .await
    }

    async fn collection_exists(&self, command: &Value) -> Result<Value, Failure> {
        let answer = self
            .store
            .call("inspectCollection", Self::collection_request(command)?)
            .await;
        answer.map_or_else(
            |error| Self::answer_when_absent(error.collection_missing(), error, json!(false)),
            |inspected| Ok(json!(inspected["exists"] == json!(true))),
        )
    }

    fn document_request(command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let id = Self::id(command)?;
        Ok(json!({ "collection": collection, "id": id }))
    }

    fn collection_request(command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::COLLECTION_KEYS)?;
        Ok(json!({ "collection": Self::collection(command)? }))
    }

    fn collection(command: &Value) -> Result<&str, Failure> {
        CollectionName::checked(text(command, "collection")?)
    }

    fn id(command: &Value) -> Result<&str, Failure> {
        let value = command
            .get("id")
            .ok_or_else(|| failure("This FYLO document command requires an id."))?;
        document_id(value)
    }

    fn fields(command: &Value) -> Result<Map<String, Value>, Failure> {
        let fields = command
            .get("value")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let bounded = (1..=Self::MAX_FIELDS).contains(&fields.len());
        bounded
            .then_some(fields)
            .ok_or_else(|| failure("This FYLO document command requires a bounded object value."))
    }

    fn new_document(command: &Value) -> Result<Map<String, Value>, Failure> {
        let mut document = Self::fields(command)?;
        let assigned = document.remove(Self::ID_FIELD).unwrap_or(Value::Null);
        let unassigned = [Value::Null, json!("")].contains(&assigned);
        unassigned
            .then_some(document)
            .ok_or_else(|| failure("FYLO assigns identifiers; insert a document without one."))
    }

    fn changes(command: &Value, id: &str) -> Result<Map<String, Value>, Failure> {
        let mut fields = Self::fields(command)?;
        let named = fields.remove(Self::ID_FIELD).unwrap_or_else(|| json!(id));
        let checks = [named == json!(id), !fields.is_empty()];
        let valid = checks == [true, true];
        valid.then_some(fields).ok_or_else(|| {
            failure("A patch names at least one field besides the id, and it cannot change the id.")
        })
    }

    fn sorted_documents(answer: &Value) -> Vec<Value> {
        let items = answer.as_object().cloned().unwrap_or_default();
        let mut documents = Vec::with_capacity(items.len());
        for (id, fields) in items {
            documents.push(Self::document_with_id(id, &fields));
        }
        documents.sort_by(|left, right| Self::id_of(left).cmp(Self::id_of(right)));
        documents
    }

    fn document_with_id(id: String, fields: &Value) -> Value {
        let mut document = fields.as_object().cloned().unwrap_or_default();
        document.insert(Self::ID_FIELD.to_owned(), Value::String(id));
        Value::Object(document)
    }

    fn id_of(document: &Value) -> &str {
        document[Self::ID_FIELD].as_str().unwrap_or_default()
    }

    fn first_document(answer: Value) -> Result<Value, Failure> {
        let documents = rows_of(answer, Self::ID_FIELD)?;
        Ok(documents.into_iter().next().unwrap_or(Value::Null))
    }

    fn last_document(answers: Vec<Value>) -> Result<Value, Failure> {
        let last = answers.into_iter().last().unwrap_or(Value::Null);
        Self::first_document(last)
    }

    fn answer_when_absent(absent: bool, error: FyloError, answer: Value) -> Result<Value, Failure> {
        absent.then_some(answer).ok_or_else(|| error.into_failure())
    }

    fn snapshot_failure(error: FyloError) -> Failure {
        if error.code == "EFRAME_RESPONSE_TOO_LARGE" {
            return failure(
                "The documents of one scroll must fit in 8 MiB; narrow the scroll with a filter.",
            );
        }
        error.into_failure()
    }
}
