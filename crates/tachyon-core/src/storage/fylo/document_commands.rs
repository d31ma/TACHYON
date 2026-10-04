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
        match text(command, "command")? {
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
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let id = Self::id(command)?;
        let request = json!({ "collection": collection, "id": id });
        match self.store.call("getLatest", request).await {
            Ok(answer) => Self::first_document(answer),
            Err(error) if error.missing() => Ok(Value::Null),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn insert(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let mut document = Self::fields(command)?;
        let assigned = document.remove(Self::ID_FIELD);
        let has_identifier = assigned.is_some_and(|id| !id.is_null() && id != json!(""));
        if has_identifier {
            return Err(failure(
                "FYLO assigns identifiers; insert a document without one.",
            ));
        }
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
        let mut fields = Self::fields(command)?;
        let named = fields.remove(Self::ID_FIELD);
        if named.is_some_and(|named| named != json!(id)) {
            return Err(failure("A patch cannot change the id of a document."));
        }
        if fields.is_empty() {
            return Err(failure("A patch names at least one field besides the id."));
        }
        let change = json!({ "collection": collection, "id": id, "newDoc": Value::Object(fields) });
        let reading = json!({ "collection": collection, "id": id });
        let requests = vec![("patchDoc", change), ("getDoc", reading)];
        match self.store.call_together(requests).await {
            Ok(mut answers) => Self::first_document(answers.pop().unwrap_or(Value::Null)),
            Err(error) if error.missing() => Ok(Value::Null),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn delete_by_id(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let id = Self::id(command)?;
        let request = json!({ "collection": collection, "id": id });
        match self.store.call("delDoc", request).await {
            Ok(_) => Ok(json!(true)),
            Err(error) if error.missing() => Ok(json!(false)),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn scroll(&self, command: &Value) -> Result<Value, Failure> {
        let collection = Self::collection(command)?;
        let scroll = ScrollRequest::parse(command)?;
        let scope = json!({ "collection": collection, "where": scroll.filter() }).to_string();
        let cursor = match scroll.cursor() {
            Some(cursor) => cursor.to_owned(),
            None => {
                self.open_snapshot(collection, &scroll, scope.clone())
                    .await?
            }
        };
        let page = self
            .store
            .scrolls
            .lock()
            .await
            .page(&cursor, &scope, scroll.size())?;
        Ok(json!({ "content": page.documents, "nextCursor": page.next_cursor }))
    }

    async fn open_snapshot(
        &self,
        collection: &str,
        scroll: &ScrollRequest,
        scope: String,
    ) -> Result<String, Failure> {
        let request = json!({ "collection": collection, "query": scroll.filter() });
        let answer = self.store.call("findDocs", request).await.map_err(|error| {
            if error.code == "EFRAME_RESPONSE_TOO_LARGE" {
                failure("The documents of one scroll must fit in 8 MiB; narrow the scroll with a filter.")
            } else {
                error.into_failure()
            }
        })?;
        let mut documents = Vec::new();
        for (id, fields) in answer.as_object().into_iter().flatten() {
            let Value::Object(mut document) = fields.clone() else {
                continue;
            };
            document.insert(Self::ID_FIELD.to_owned(), json!(id));
            documents.push(Value::Object(document));
        }
        documents.sort_by(|left, right| Self::id_of(left).cmp(Self::id_of(right)));
        self.store.scrolls.lock().await.open(scope, documents)
    }

    async fn create_collection(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::COLLECTION_KEYS)?;
        let collection = Self::collection(command)?;
        self.store
            .call("createCollection", json!({ "collection": collection }))
            .await
            .map_err(FyloError::into_failure)?;
        Ok(json!(true))
    }

    async fn drop_collection(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::COLLECTION_KEYS)?;
        let collection = Self::collection(command)?;
        let answer = self
            .store
            .call("dropCollection", json!({ "collection": collection }))
            .await;
        match answer {
            Ok(_) => Ok(json!(true)),
            Err(error) if error.collection_missing() => Ok(json!(false)),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn collection_exists(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::COLLECTION_KEYS)?;
        let collection = Self::collection(command)?;
        let answer = self
            .store
            .call("inspectCollection", json!({ "collection": collection }))
            .await;
        match answer {
            Ok(inspected) => Ok(json!(inspected["exists"] == json!(true))),
            Err(error) if error.collection_missing() => Ok(json!(false)),
            Err(error) => Err(error.into_failure()),
        }
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
        command
            .get("value")
            .and_then(Value::as_object)
            .filter(|fields| !fields.is_empty() && fields.len() <= Self::MAX_FIELDS)
            .cloned()
            .ok_or_else(|| failure("This FYLO document command requires a bounded object value."))
    }

    fn id_of(document: &Value) -> &str {
        document[Self::ID_FIELD].as_str().unwrap_or_default()
    }

    fn first_document(answer: Value) -> Result<Value, Failure> {
        let documents = rows_of(answer, Self::ID_FIELD)?;
        Ok(documents.into_iter().next().unwrap_or(Value::Null))
    }
}
