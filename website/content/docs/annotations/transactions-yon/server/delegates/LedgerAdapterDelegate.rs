#[derive(Default)]
#[Delegate]
struct LedgerAdapterDelegate {
    sequence: std::sync::Mutex<u32>,
}
impl LedgerAdapterDelegate {
    fn commands() -> &'static std::sync::Mutex<Vec<String>> {
        static COMMANDS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &COMMANDS
    }
}
impl BehaviorEntityAdapter for LedgerAdapterDelegate {
    fn transaction(&self, command: &BehaviorTransactionCommand, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        let entry = match &command.transaction {
            Some(id) => format!("{}:{id}", command.action),
            None => command.action.to_owned(),
        };
        Self::commands().lock().expect("record the transaction").push(entry);
        let mut sequence = self.sequence.lock().expect("sequence");
        *sequence += 1;
        Ok(BehaviorValue::from(format!("t{sequence}")))
    }

    fn execute(&self, command: &BehaviorEntityCommand, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        let query = command.query.as_ref().ok_or_else(behavior_invalid)?;
        let Some(BehaviorValue::String(kind)) = query.get("kind") else {
            return Err(behavior_invalid());
        };
        let transaction = command.transaction.as_deref().unwrap_or("none");
        Self::commands().lock().expect("record the query").push(format!("{kind}@{transaction}"));
        if kind == "count" {
            return Ok(BehaviorValue::Number(1.0));
        }
        let row = BehaviorValue::object([("id", BehaviorValue::Number(1.0)), ("status", BehaviorValue::from("open"))]);
        Ok(BehaviorValue::Array(vec![row]))
    }
}

impl LedgerAdapterDelegate {
    fn new() -> Self { Self::default() }
}
