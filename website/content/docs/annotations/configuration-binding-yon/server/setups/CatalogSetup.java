@Setup
@Settings({"prefix":"YON_CATALOG_"})
@WhenProfile("development")
@WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
    @Min(1)
    @Max(100)
    int pageSize = 20;
    boolean cacheEnabled = false;
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration();
    }
}
