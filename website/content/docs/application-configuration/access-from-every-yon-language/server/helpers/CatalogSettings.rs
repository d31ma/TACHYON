#[Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})]
struct CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    pageSize: i64,
}
