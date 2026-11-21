use serde_json::{Map, Value, json};

use super::collection_name::CollectionName;
use super::store_query::StoreQuery;
use super::store_rows::StoreRows;
use super::{FyloError, FyloStore, document_id, identifier};
use crate::Failure;
use crate::storage::dialect::{page_with_query_order, reject_count_page_or_order};
use crate::storage::{ManagedStorage, failure, object_keys, text};

pub(super) struct RepositoryCommands<'store> {
    store: &'store FyloStore,
    joined: Option<&'store str>,
}

impl<'store> RepositoryCommands<'store> {
    const MAX_FIELDS: usize = 64;
    const QUERY_KEYS: [&'static str; 4] = ["kind", "where", "orderBy", "page"];
    const FIND: &'static str = "find";

    pub(super) fn new(store: &'store FyloStore, joined: Option<&'store str>) -> Self {
        Self { store, joined }
    }

    pub(super) async fn execute(&self, command: &Value, operation: &str) -> Result<Value, Failure> {
        object_keys(command, &ManagedStorage::REPOSITORY_COMMAND_KEYS)?;
        let collection = CollectionName::checked(text(command, "table")?)?;
        let id_field = identifier(text(command, "idField")?)?;
        self.dispatch(operation, command, collection, id_field)
            .await
    }

    pub(super) fn deleted(answer: Result<Value, FyloError>) -> Result<Value, Failure> {
        Self::deleted_count_answer(answer)
    }

    fn deleted_count_answer(answer: Result<Value, FyloError>) -> Result<Value, Failure> {
        match answer {
            Ok(count) => Ok(json!(count.as_u64().unwrap_or_default() > 0)),
            Err(error) if error.missing() => Ok(json!(false)),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn dispatch(
        &self,
        operation: &str,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        match operation {
            "save" => self.save(command, collection, id_field).await,
            "delete" => self.delete(command, collection).await,
            "find" => self.find_page(command, collection, id_field).await,
            "query" => self.query(command, collection, id_field).await,
            _ => Err(failure("Unsupported managed storage operation.")),
        }
    }

    async fn save(
        &self,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        let entity = Self::bounded_entity(command)?;
        let assigned = Self::assigned_id(entity, id_field)?;
        let request = json!({
            "collection": collection,
            "idField": id_field,
            "entity": entity,
            "requireExisting": assigned.is_some(),
        });
        let answer = self
            .call("storeSave", request)
            .await
            .map_err(Self::save_failure)?;
        StoreRows::entity(answer)
    }

    fn save_failure(error: FyloError) -> Failure {
        if error.read_found_no_collection() {
            return failure("FYLO assigns identifiers; save a new entity without one.");
        }
        error.into_failure()
    }

    fn bounded_entity(command: &Value) -> Result<&Map<String, Value>, Failure> {
        command
            .get("value")
            .ok_or_else(|| failure("Store save requires an entity."))?
            .as_object()
            .filter(|entity| (1..=Self::MAX_FIELDS).contains(&entity.len()))
            .ok_or_else(|| failure("Store save requires a bounded entity object."))
    }

    fn assigned_id<'entity>(
        entity: &'entity Map<String, Value>,
        id_field: &str,
    ) -> Result<Option<&'entity str>, Failure> {
        entity
            .get(id_field)
            .filter(|id| !id.is_null())
            .map(document_id)
            .transpose()
    }

    async fn delete(&self, command: &Value, collection: &str) -> Result<Value, Failure> {
        let id = command
            .get("id")
            .ok_or_else(|| failure("Store delete requires an identifier."))?;
        let request = json!({ "collection": collection, "id": document_id(id)? });
        Self::deleted(self.call("storeDeleteById", request).await)
    }

    async fn find_page(
        &self,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        let page = Self::page_or_first(command.get("page"));
        self.find(collection, id_field, Vec::new(), &page).await
    }

    fn page_or_first(page: Option<&Value>) -> Value {
        page.filter(|page| !page.is_null())
            .cloned()
            .unwrap_or_else(|| json!({}))
    }

    async fn find(
        &self,
        collection: &str,
        id_field: &str,
        conditions: Vec<Value>,
        page: &Value,
    ) -> Result<Value, Failure> {
        let order = StoreQuery::new(id_field).order(page)?;
        let window = StoreQuery::page(page)?;
        let query = StoreQuery::bounded(json!({ "where": conditions, "orderBy": order }))?;
        let request = json!({
            "collection": collection,
            "idField": id_field,
            "query": query,
            "page": window,
        });
        let answer = self
            .call("storeFind", request)
            .await
            .map_err(FyloError::into_failure)?;
        Ok(Value::Array(StoreRows::found(answer)?))
    }

    async fn query(
        &self,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        let query = &command["query"];
        object_keys(query, &Self::QUERY_KEYS)?;
        let kind = text(query, "kind")?;
        let conditions = StoreQuery::conditions(query)?;
        let nothing = StoreQuery::matches_nothing(&conditions);
        match (nothing, kind == Self::FIND) {
            (true, _) => Self::empty_result(kind),
            (false, true) => {
                let page = page_with_query_order(query)?;
                self.find(collection, id_field, conditions, &page).await
            }
            (false, false) => {
                let request = Self::counting_request(collection, id_field, &conditions)?;
                self.counted(kind, query, request).await
            }
        }
    }

    async fn counted(&self, kind: &str, query: &Value, request: Value) -> Result<Value, Failure> {
        reject_count_page_or_order(query)?;
        let operation = Self::counting_operation(kind)?;
        self.call(operation, request)
            .await
            .map_err(FyloError::into_failure)
    }

    fn counting_request(
        collection: &str,
        id_field: &str,
        conditions: &[Value],
    ) -> Result<Value, Failure> {
        let query = StoreQuery::bounded(json!({ "where": conditions }))?;
        Ok(json!({ "collection": collection, "idField": id_field, "query": query }))
    }

    fn counting_operation(kind: &str) -> Result<&'static str, Failure> {
        match kind {
            "count" => Ok("storeCount"),
            "exists" => Ok("storeExists"),
            "delete" => Ok("storeDelete"),
            _ => Err(failure("Unsupported repository query kind.")),
        }
    }

    fn empty_result(kind: &str) -> Result<Value, Failure> {
        match kind {
            "find" => Ok(json!([])),
            "count" | "delete" => Ok(json!(0)),
            "exists" => Ok(json!(false)),
            _ => Err(failure("Unsupported repository query kind.")),
        }
    }

    async fn call(&self, operation: &str, fields: Value) -> Result<Value, FyloError> {
        self.store.call_joined(self.joined, operation, fields).await
    }
}
