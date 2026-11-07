@Setup
@Settings({"prefix":"YON_CATALOG_"})
@WhenProfile("development")
@WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
    var cacheEnabled: Boolean = false
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
