use serde_json::{Map, Value, json};

use super::collection_name::CollectionName;
use super::repository_commands::RepositoryCommands;
use super::scroll_request::ScrollRequest;
use super::store_query::StoreQuery;
use super::store_rows::StoreRows;
use super::{FyloError, FyloStore, document_id};
use crate::Failure;
use crate::storage::{failure, object_keys, text};

pub(super) struct DocumentCommands<'store> {
    store: &'store FyloStore,
    joined: Option<&'store str>,
}

impl<'store> DocumentCommands<'store> {
    const ID_FIELD: &'static str = "id";
    const MAX_FIELDS: usize = 64;
    const DOCUMENT_KEYS: [&'static str; 5] = ["operation", "command", "collection", "id", "value"];
    const COLLECTION_KEYS: [&'static str; 3] = ["operation", "command", "collection"];

    pub(super) fn new(store: &'store FyloStore, joined: Option<&'store str>) -> Self {
        Self { store, joined }
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
        let request = Self::document_request(command)?;
        let answer = self.call("storeFindById", request).await;
        answer.or_else(|error| Self::answer_when_absent(error.missing(), error, Value::Null))
    }

    async fn insert(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let document = Self::new_document(command)?;
        let request = json!({ "collection": collection, "entity": Value::Object(document) });
        let answer = self
            .call("storeSave", request)
            .await
            .map_err(FyloError::into_failure)?;
        StoreRows::entity(answer)
    }

    async fn patch(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::DOCUMENT_KEYS)?;
        let collection = Self::collection(command)?;
        let id = Self::id(command)?;
        let fields = Self::changes(command, id)?;
        let answer = self.patched(collection, id, fields).await;
        answer.or_else(|error| Self::answer_when_absent(error.missing(), error, Value::Null))
    }

    async fn patched(
        &self,
        collection: &str,
        id: &str,
        fields: Map<String, Value>,
    ) -> Result<Value, FyloError> {
        match self.joined {
            Some(_) => {
                let request = json!({ "collection": collection, "id": id, "fields": fields });
                self.call("storePatch", request).await
            }
            None => self.store.patch(collection, id, fields).await.map(Self::last_answer),
        }
    }

    async fn delete_by_id(&self, command: &Value) -> Result<Value, Failure> {
        let request = Self::document_request(command)?;
        RepositoryCommands::deleted(self.call("storeDeleteById", request).await)
    }

    async fn scroll(&self, command: &Value) -> Result<Value, Failure> {
        let collection = Self::collection(command)?;
        let scroll = ScrollRequest::parse(command)?;
        let query = StoreQuery::bounded(json!({ "where": scroll.conditions() }))?;
        let request = json!({ "collection": collection, "query": query, "page": scroll.page() });
        let answer = self
            .call("storeScroll", request)
            .await
            .map_err(FyloError::into_failure)?;
        StoreRows::scroll_page(answer)
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
        let answer = self.store.call("dropCollection", request).await;
        answer.map_or_else(
            |error| Self::answer_when_absent(error.collection_missing(), error, json!(false)),
            |_| Ok(json!(true)),
        )
    }

    async fn collection_exists(&self, command: &Value) -> Result<Value, Failure> {
        let request = Self::collection_request(command)?;
        Self::exists_answer(self.store.call("collectionExists", request).await)
    }

    fn exists_answer(answer: Result<Value, FyloError>) -> Result<Value, Failure> {
        answer.map_or_else(
            |error| Self::answer_when_absent(error.read_found_no_collection(), error, json!(false)),
            |exists| Ok(json!(exists == json!(true))),
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

    fn last_answer(answers: Vec<Value>) -> Value {
        answers.into_iter().last().unwrap_or(Value::Null)
    }

    fn answer_when_absent(absent: bool, error: FyloError, answer: Value) -> Result<Value, Failure> {
        absent.then_some(answer).ok_or_else(|| error.into_failure())
    }

    async fn call(&self, operation: &str, fields: Value) -> Result<Value, FyloError> {
        self.store.call_joined(self.joined, operation, fields).await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::DocumentCommands;
    use crate::storage::fylo::FyloError;

    fn failed(code: &str) -> Result<serde_json::Value, FyloError> {
        Err(FyloError {
            code: code.to_owned(),
        })
    }

    #[test]
    fn fylo_answers_whether_the_collection_exists() {
        let found = DocumentCommands::exists_answer(Ok(json!(true)));
        assert_eq!(found.ok(), Some(json!(true)));
        let missing = DocumentCommands::exists_answer(Ok(json!(false)));
        assert_eq!(missing.ok(), Some(json!(false)));
    }

    #[test]
    fn enative_not_found_also_means_the_collection_does_not_exist() {
        let missing = DocumentCommands::exists_answer(failed("ENATIVE_NOT_FOUND"));
        assert_eq!(missing.ok(), Some(json!(false)));
    }

    #[test]
    fn every_other_error_fails_the_call() {
        for code in [
            "ENOTFOUND",
            "ECOLLECTION_NOT_FOUND",
            "ENATIVE_UNSAFE_PATH",
            "EROOTLOCKED",
            "EPROTOCOL",
        ] {
            assert!(
                DocumentCommands::exists_answer(failed(code)).is_err(),
                "{code}"
            );
        }
    }

    #[test]
    fn a_patch_answers_the_last_written_document() {
        let last = DocumentCommands::last_answer(vec![json!({"id": "4W1", "n": 2})]);
        assert_eq!(last, json!({"id": "4W1", "n": 2}));
        assert_eq!(DocumentCommands::last_answer(Vec::new()), json!(null));
    }
}
