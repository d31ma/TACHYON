@Settings({"prefix":"TAC_CATALOG_"})
struct CatalogSettings {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
}
