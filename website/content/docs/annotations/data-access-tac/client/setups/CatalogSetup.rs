#[Setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct CatalogAdapter;
        impl BehaviorEntityAdapter for CatalogAdapter {
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
        configuration.entities.insert("publicCatalog".into(), std::sync::Arc::new(CatalogAdapter));
        configuration.authorize_entity = Some(std::sync::Arc::new(|command, _execution| Ok(command.entity == "ProductEntity")));
        Ok(configuration)
    }
}
