@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile('public')
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
export default class CatalogSetup {
  @Min(1)
  @Max(100)
  pageSize = 20;
  cacheEnabled = false;
  title = 'Catalog';
  /** @returns {object} */
  static configure() { return {}; }
}
