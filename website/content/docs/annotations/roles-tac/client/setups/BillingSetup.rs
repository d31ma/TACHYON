#[Setup]
struct BillingSetup {}
impl BillingSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct BillingAdapter;
        impl BehaviorEntityAdapter for BillingAdapter {
            fn execute(&self, command: &BehaviorEntityCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                execution.check()?;
                let query = command.query.as_ref().ok_or_else(behavior_invalid)?;
                match query.get("kind") {
                    Some(BehaviorValue::String(kind)) if kind == "count" => Ok(BehaviorValue::Number(0.0)),
                    _ => Ok(BehaviorValue::Array(Vec::new())),
                }
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.entities.insert("publicBilling".into(), std::sync::Arc::new(BillingAdapter));
        configuration.authorize_entity = Some(std::sync::Arc::new(|command, _execution| Ok(command.entity == "BillingEntity")));
        Ok(configuration)
    }
}
