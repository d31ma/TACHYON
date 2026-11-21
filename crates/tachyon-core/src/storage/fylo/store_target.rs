use serde_json::Value;

use crate::Failure;
use crate::storage::failure;

pub(super) struct StoreTarget {
    collection: Option<String>,
    writes: bool,
    joins: bool,
    sql_writes: bool,
}

impl StoreTarget {
    const DOCUMENT_WRITES: [&'static str; 5] = [
        "insert",
        "patch",
        "deleteById",
        "createCollection",
        "dropCollection",
    ];
    const COLLECTION_COMMANDS: [&'static str; 2] = ["createCollection", "dropCollection"];
    const REPOSITORY_WRITES: [&'static str; 2] = ["save", "delete"];
    const QUERY_DELETE: &'static str = "delete";
    const SQL_READ_MODE: &'static str = "rows";

    pub(super) fn of(command: &Value) -> Self {
        Self::classified(command["operation"].as_str().unwrap_or_default(), command)
    }

    fn classified(operation: &str, command: &Value) -> Self {
        match operation {
            "sql" => Self::sql(command),
            "queue" => Self::queue(),
            "document" => Self::document(command),
            operation => Self::repository(command, operation),
        }
    }

    pub(super) fn collection(&self) -> Option<&str> {
        self.collection.as_deref()
    }

    pub(super) fn writes(&self) -> bool {
        self.writes
    }

    pub(super) fn sql_writes(&self) -> bool {
        self.sql_writes
    }

    pub(super) fn refuse_unless_it_joins(&self) -> Result<(), Failure> {
        Self::refuse_outsider(self.joins)
    }

    fn refuse_outsider(joins: bool) -> Result<(), Failure> {
        if !joins {
            return Err(failure(
                "Queue, SQL and collection commands cannot join a FYLO transaction; run them outside it.",
            ));
        }
        Ok(())
    }

    fn sql(command: &Value) -> Self {
        let reads = command["mode"].as_str() == Some(Self::SQL_READ_MODE);
        Self {
            collection: None,
            writes: false,
            joins: false,
            sql_writes: !reads,
        }
    }

    fn queue() -> Self {
        Self {
            collection: None,
            writes: false,
            joins: false,
            sql_writes: false,
        }
    }

    fn document(command: &Value) -> Self {
        let name = command["command"].as_str().unwrap_or_default();
        Self {
            collection: Self::named(command, "collection"),
            writes: Self::DOCUMENT_WRITES.contains(&name),
            joins: !Self::COLLECTION_COMMANDS.contains(&name),
            sql_writes: false,
        }
    }

    fn repository(command: &Value, operation: &str) -> Self {
        let deletes = command["query"]["kind"].as_str() == Some(Self::QUERY_DELETE);
        let saves = Self::REPOSITORY_WRITES.contains(&operation);
        Self {
            collection: Self::named(command, "table"),
            writes: [saves, deletes].contains(&true),
            joins: true,
            sql_writes: false,
        }
    }

    fn named(command: &Value, key: &str) -> Option<String> {
        command[key].as_str().map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StoreTarget;

    #[test]
    fn a_repository_save_writes_to_its_table_and_joins() {
        let target = StoreTarget::of(&json!({"operation": "save", "table": "orders"}));
        assert_eq!(target.collection(), Some("orders"));
        assert!(target.writes());
        assert!(target.refuse_unless_it_joins().is_ok());
    }

    #[test]
    fn a_delete_query_writes_and_a_count_query_reads() {
        let delete = json!({"operation": "query", "table": "orders", "query": {"kind": "delete"}});
        assert!(StoreTarget::of(&delete).writes());
        let count = json!({"operation": "query", "table": "orders", "query": {"kind": "count"}});
        assert!(!StoreTarget::of(&count).writes());
    }

    #[test]
    fn a_document_patch_writes_and_a_scroll_reads() {
        let patch = json!({"operation": "document", "command": "patch", "collection": "people"});
        assert!(StoreTarget::of(&patch).writes());
        let scroll = json!({"operation": "document", "command": "scroll", "collection": "people"});
        assert!(!StoreTarget::of(&scroll).writes());
    }

    #[test]
    fn collection_queue_and_sql_commands_do_not_join() {
        let drop =
            json!({"operation": "document", "command": "dropCollection", "collection": "people"});
        assert!(StoreTarget::of(&drop).refuse_unless_it_joins().is_err());
        let queue = json!({"operation": "queue", "command": "publish", "topic": "outbox"});
        assert!(StoreTarget::of(&queue).refuse_unless_it_joins().is_err());
        let sql = json!({"operation": "sql", "mode": "rows", "sql": "SELECT * FROM people"});
        assert!(StoreTarget::of(&sql).refuse_unless_it_joins().is_err());
    }

    #[test]
    fn only_an_sql_statement_outside_rows_mode_counts_as_an_sql_write() {
        let update =
            json!({"operation": "sql", "mode": "execute", "sql": "UPDATE people SET a = 1"});
        assert!(StoreTarget::of(&update).sql_writes());
        let select = json!({"operation": "sql", "mode": "rows", "sql": "SELECT * FROM people"});
        assert!(!StoreTarget::of(&select).sql_writes());
    }
}
