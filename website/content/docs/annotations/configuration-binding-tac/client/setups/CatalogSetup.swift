@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("public")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
final class CatalogSetup {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
    var cacheEnabled: Bool = false
    var title: String = "Catalog"
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }
}
