@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("public")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
  @Min(1)
  @Max(100)
  int pageSize = 20;
  bool cacheEnabled = false;
  String title = 'Catalog';
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters();
  }
}
