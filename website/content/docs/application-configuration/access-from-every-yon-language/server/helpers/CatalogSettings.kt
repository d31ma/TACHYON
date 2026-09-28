@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
}
