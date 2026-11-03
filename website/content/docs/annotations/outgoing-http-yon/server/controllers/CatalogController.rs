#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
  #[Needs]
  catalog: std::sync::Arc<CatalogService>,
}
impl CatalogController {
    #[GetRoute]
    fn get_catalog(&self) -> Json { self.catalog.products().unwrap_or(Json::Null) }
}
