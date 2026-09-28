#[Setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut configuration = BehaviorConfiguration::default();
        configuration.entities.insert("catalog".into(), std::sync::Arc::new(CatalogAdapterHelper));
        configuration.authorize_entity = Some(std::sync::Arc::new(|command, _execution| Ok(command.entity == "ProductEntity")));
        Ok(configuration)
    }
}
