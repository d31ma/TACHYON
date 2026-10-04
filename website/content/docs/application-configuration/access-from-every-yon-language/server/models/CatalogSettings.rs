#[Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})]
#[allow(non_snake_case)]
pub struct CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    pub pageSize: i64,
}
