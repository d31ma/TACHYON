[<Setup>]
[<Settings({"prefix":"TAC_CATALOG_"})>]
[<WhenProfile("public")>]
[<WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})>]
type CatalogSetup() =
    [<Min(1)>]
    [<Max(100)>]
    member val pageSize : int = 20 with get, set

    member val cacheEnabled : bool = false with get, set
    member val title : string = "Catalog" with get, set

    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()
