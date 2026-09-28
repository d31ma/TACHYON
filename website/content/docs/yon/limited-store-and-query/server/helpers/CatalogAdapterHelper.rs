#[Helper]
struct CatalogAdapterHelper;
impl BehaviorEntityAdapter for CatalogAdapterHelper {
    fn execute(&self, command: &BehaviorEntityCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let rows = [("active", 1.0), ("retired", 2.0)];
        let query = command.query.as_ref().ok_or_else(behavior_invalid)?;
        let Some(BehaviorValue::Array(conditions)) = query.get("where") else {
            return Err(behavior_invalid());
        };
        let condition = conditions.first().ok_or_else(behavior_invalid)?;
        let keep_unequal_rows = condition.get("operator") == Some(&BehaviorValue::String("ne".into()));
        let wanted_status = condition.get("value").cloned().unwrap_or(BehaviorValue::Null);
        let mut matching_rows = Vec::new();
        for (status, id) in rows {
            let is_equal = BehaviorValue::from(status) == wanted_status;
            let is_match = if keep_unequal_rows { !is_equal } else { is_equal };
            if !is_match {
                continue;
            }
            let row = BehaviorValue::object([("id", BehaviorValue::Number(id)), ("status", BehaviorValue::from(status))]);
            matching_rows.push(row);
        }
        match query.get("kind") {
            Some(BehaviorValue::String(kind)) if kind == "count" => Ok(BehaviorValue::Number(matching_rows.len() as f64)),
            _ => Ok(BehaviorValue::Array(matching_rows)),
        }
    }
}

impl CatalogAdapterHelper {
    fn new() -> Self { Self }
}
