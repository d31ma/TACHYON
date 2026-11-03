@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("public")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
    var cacheEnabled: Boolean = false
    var title: String = "Catalog"
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
