#[Setup]
struct ConfigSetup {}

impl ConfigSetup {
    const PRODUCTS: &str = "products";
    const TENANT: &str = "tenant-a";

    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let memory = std::sync::Arc::new(Memory::default());
        let mut config = BehaviorConfiguration::default();
        config.caches.insert(Self::PRODUCTS.into(), memory.clone());
        config.http.insert("catalog".into(), memory.clone());
        config.entities.insert("data".into(), memory);
        config.authorize = Some(std::sync::Arc::new(|call, execution| {
            execution.check()?;
            assert_eq!(call.cache, Self::PRODUCTS);
            assert_eq!(call.owner, "CatalogService");
            Ok(execution.partition == Self::TENANT)
        }));
        config.authorize_entity = Some(std::sync::Arc::new(|command, execution| {
            execution.check()?;
            assert_eq!(command.operation, "query");
            assert_eq!(command.table, Self::PRODUCTS);
            Ok(execution.partition == Self::TENANT)
        }));
        Ok(config)
    }
}

