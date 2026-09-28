[Setup]
[Settings({"prefix":"TAC_CATALOG_"})]
[WhenProfile("public")]
[WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
public sealed class CatalogSetup {
    [Min(1)]
    [Max(100)]
    public int pageSize = 20;
    public bool cacheEnabled = false;
    public string title = "Catalog";
    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }
}
