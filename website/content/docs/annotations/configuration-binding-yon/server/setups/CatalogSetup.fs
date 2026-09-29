[<Setup>]
[<Settings({"prefix":"YON_CATALOG_"})>]
[<WhenProfile("development")>]
[<WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})>]
type CatalogSetup() =
    [<Min(1)>]
    [<Max(100)>]
    member val pageSize : int = 20 with get, set

    member val cacheEnabled : bool = false with get, set

    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()
