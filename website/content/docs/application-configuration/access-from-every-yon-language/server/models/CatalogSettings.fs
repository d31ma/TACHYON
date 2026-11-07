[<Settings({"prefix":"YON_CATALOG_"})>]
type CatalogSettings() =
    [<Min(1)>]
    [<Max(100)>]
    member val pageSize : int = 20 with get, set
