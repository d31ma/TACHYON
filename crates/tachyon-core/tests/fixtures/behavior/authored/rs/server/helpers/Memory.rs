#[Description("Behavior conformance Memory")]
#[derive(Default)]
struct Memory {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorCacheEntry>>,
}

impl BehaviorCacheAdapter for Memory {
    fn get(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<Option<BehaviorCacheEntry>> {
        execution.check()?;
        let entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        Ok(entries.get(key).cloned())
    }
    fn set(&self, key: &str, value: BehaviorValue, write: &BehaviorCacheWrite) -> BehaviorResult<()> {
        write.execution.check()?;
        let mut entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        let entry = BehaviorCacheEntry { value, expires_at: write.expires_at };
        entries.insert(key.to_owned(), entry);
        Ok(())
    }
    fn delete(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<()> {
        execution.check()?;
        let mut entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        entries.remove(key);
        Ok(())
    }
}

impl BehaviorHttpAdapter for Memory {
    fn exchange(&self, request: &BehaviorHttpRequest, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        assert_eq!(request.credentials, "omit");
        assert_eq!(request.redirect, "error");
        Ok(BehaviorValue::from(request.path.as_str()))
    }
}

impl BehaviorEntityAdapter for Memory {
    fn transaction(&self, command: &BehaviorTransactionCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let entry = match &command.transaction {
            Some(id) => format!("{}:{id}", command.action),
            None => command.action.to_owned(),
        };
        CatalogChecks::entity_log().lock().expect("log").push(entry);
        Ok(BehaviorValue::from("tx-1"))
    }
    fn execute(&self, command: &BehaviorEntityCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let transaction = command.transaction.as_deref().unwrap_or("none");
        let entry = format!("{}@{}", command.operation, transaction);
        CatalogChecks::entity_log().lock().expect("log").push(entry);
        let Some(query) = &command.query else {
            return Ok(BehaviorValue::object([("operation", command.operation.into()), ("table", command.table.into())]));
        };
        assert_eq!(command.operation, "query");
        Memory::answer_query(query)
    }
}

impl Memory {
    fn answer_query(query: &BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let Some(BehaviorValue::String(kind)) = query.get("kind") else {
            return Err(behavior_invalid());
        };
        if kind == "find" {
            let row = BehaviorValue::object([("id", "one".into()), ("status", "active".into())]);
            return Ok(BehaviorValue::Array(vec![row]));
        }
        if kind == "count" {
            return Ok(1.into());
        }
        if kind == "exists" {
            return Ok(true.into());
        }
        Err(behavior_invalid())
    }
}
