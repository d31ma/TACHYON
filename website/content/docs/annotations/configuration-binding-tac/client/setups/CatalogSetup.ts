@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile('public')
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
  cacheEnabled: boolean = false;
  title: string = 'Catalog';
  static configure() { return {}; }
}
