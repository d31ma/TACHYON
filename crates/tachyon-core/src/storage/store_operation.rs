use serde_json::{Value, json};

pub(crate) struct StoreOperation;

impl StoreOperation {
    const UNKNOWN: &'static str = "unknown";
    const SUBCOMMAND_FIELDS: [(&'static str, &'static str); 6] = [
        ("document", "/command"),
        ("queue", "/command"),
        ("redis", "/command"),
        ("query", "/query/kind"),
        ("sql", "/mode"),
        ("transaction", "/action"),
    ];
    const KNOWN: [&'static str; 30] = [
        "save",
        "delete",
        "find",
        "query.find",
        "query.count",
        "query.exists",
        "query.delete",
        "sql.rows",
        "sql.execute",
        "document.findById",
        "document.insert",
        "document.patch",
        "document.deleteById",
        "document.scroll",
        "document.createCollection",
        "document.dropCollection",
        "document.collectionExists",
        "queue.publish",
        "queue.claim",
        "queue.ack",
        "queue.nack",
        "redis.get",
        "redis.set",
        "redis.delete",
        "transaction.begin",
        "transaction.commit",
        "transaction.rollback",
        "sql",
        "document",
        "queue",
    ];
    const REPOSITORY_OPERATIONS: [&'static str; 4] = ["save", "delete", "find", "query"];
    const MAX_NAME_LENGTH: usize = 64;

    pub(crate) fn summary(command: &Value) -> Value {
        json!({
            "operation": command["operation"],
            "command": command["command"],
            "mode": command["mode"],
            "action": command["action"],
            "table": command["table"],
            "query": {"kind": command.pointer("/query/kind")},
        })
    }

    pub(crate) fn kind(command: &Value) -> &'static str {
        let named = Self::full_name(command);
        Self::KNOWN
            .into_iter()
            .find(|known| *known == named)
            .unwrap_or(Self::UNKNOWN)
    }

    pub(crate) fn collection(command: &Value) -> Option<&str> {
        let repository = command["operation"]
            .as_str()
            .is_some_and(|operation| Self::REPOSITORY_OPERATIONS.contains(&operation));
        let table = command["table"].as_str().filter(|_| repository);
        table.filter(|name| Self::is_plain_name(name))
    }

    fn full_name(command: &Value) -> String {
        let operation = command["operation"].as_str().unwrap_or(Self::UNKNOWN);
        let subcommand = Self::subcommand(command, operation);
        subcommand.map_or_else(
            || operation.to_owned(),
            |subcommand| format!("{operation}.{subcommand}"),
        )
    }

    fn subcommand<'command>(command: &'command Value, operation: &str) -> Option<&'command str> {
        Self::SUBCOMMAND_FIELDS
            .into_iter()
            .find(|(named, _)| *named == operation)
            .and_then(|(_, pointer)| command.pointer(pointer))
            .and_then(Value::as_str)
    }

    fn is_plain_name(name: &str) -> bool {
        let bytes = name.as_bytes();
        let checks = [
            (1..=Self::MAX_NAME_LENGTH).contains(&bytes.len()),
            bytes.iter().copied().all(Self::is_name_byte),
        ];
        checks == [true; 2]
    }

    fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || b"-_.".contains(&byte)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StoreOperation;

    #[test]
    fn known_operations_are_named_with_their_command() {
        let cases = [
            (json!({"operation": "save", "table": "people"}), "save"),
            (
                json!({"operation": "query", "query": {"kind": "count"}}),
                "query.count",
            ),
            (
                json!({"operation": "document", "command": "insert"}),
                "document.insert",
            ),
            (
                json!({"operation": "queue", "command": "claim"}),
                "queue.claim",
            ),
            (
                json!({"operation": "sql", "mode": "rows", "sql": "SELECT 1"}),
                "sql.rows",
            ),
            (
                json!({"operation": "redis", "command": "get", "key": "secret"}),
                "redis.get",
            ),
            (
                json!({"operation": "transaction", "action": "begin"}),
                "transaction.begin",
            ),
        ];
        for (command, name) in cases {
            assert_eq!(StoreOperation::kind(&command), name, "{command}");
        }
    }

    #[test]
    fn any_other_text_is_never_copied_into_the_name() {
        for command in [
            json!({"operation": "document", "command": "insert me@example.com"}),
            json!({"operation": "DROP TABLE people"}),
            json!({"operation": "sql", "mode": "SELECT * FROM secrets"}),
            json!({}),
        ] {
            assert_eq!(StoreOperation::kind(&command), "unknown", "{command}");
        }
    }

    #[test]
    fn the_table_of_a_repository_comes_from_the_model_and_is_kept() {
        let save = json!({"operation": "save", "table": "people"});
        assert_eq!(StoreOperation::collection(&save), Some("people"));
        let query =
            json!({"operation": "query", "table": "local_parts", "query": {"kind": "find"}});
        assert_eq!(StoreOperation::collection(&query), Some("local_parts"));
    }

    #[test]
    fn names_that_handler_code_passes_at_run_time_are_left_out() {
        for command in [
            json!({"operation": "document", "command": "insert", "collection": "local-parts"}),
            json!({"operation": "queue", "command": "publish", "topic": "outbox"}),
            json!({"operation": "sql", "mode": "rows", "table": "people"}),
        ] {
            assert_eq!(StoreOperation::collection(&command), None, "{command}");
        }
    }

    #[test]
    fn a_summary_keeps_the_name_and_table_but_no_data() {
        let save =
            json!({"operation": "save", "table": "people", "value": {"email": "a@example.com"}});
        let summary = StoreOperation::summary(&save);
        assert_eq!(StoreOperation::kind(&summary), "save");
        assert_eq!(StoreOperation::collection(&summary), Some("people"));
        assert!(!summary.to_string().contains("example.com"));
        let find = json!({"operation": "query", "query": {"kind": "find", "where": [{"field": "email", "value": "a@example.com"}]}});
        let summary = StoreOperation::summary(&find);
        assert_eq!(StoreOperation::kind(&summary), "query.find");
        assert!(!summary.to_string().contains("example.com"));
    }

    #[test]
    fn keys_values_and_odd_names_are_left_out() {
        for command in [
            json!({"operation": "redis", "command": "get", "key": "session-1"}),
            json!({"operation": "save", "table": "people; DROP"}),
            json!({"operation": "save", "table": "a".repeat(65)}),
            json!({"operation": "sql", "mode": "rows", "sql": "SELECT 1"}),
        ] {
            assert_eq!(StoreOperation::collection(&command), None, "{command}");
        }
    }
}
