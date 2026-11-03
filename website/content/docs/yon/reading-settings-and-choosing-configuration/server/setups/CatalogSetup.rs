#[Setup]
#[Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20,"cacheEnabled":false}})]
#[WhenProfile("development")]
#[WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
struct CatalogSetup {
    #[Min(1)]
    #[Max(100)]
    pageSize: i64,
    cacheEnabled: bool,
}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
