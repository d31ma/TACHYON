@Setup
@Settings({"prefix":"YON_CATALOG_"})
@WhenProfile('development')
@WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
export default class CatalogSetup {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
  cacheEnabled: boolean = false;
  static configure() { return {}; }
}
