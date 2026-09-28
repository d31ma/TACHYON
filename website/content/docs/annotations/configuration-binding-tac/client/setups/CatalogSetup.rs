#[Setup]
#[Settings({"prefix":"TAC_CATALOG_","defaults":{"pageSize":20,"cacheEnabled":false,"title":"Catalog"}})]
#[WhenProfile("public")]
#[WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
struct CatalogSetup {
    #[Min(1)]
    #[Max(100)]
    pageSize: i64,
    cacheEnabled: bool,
    title: String,
}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
