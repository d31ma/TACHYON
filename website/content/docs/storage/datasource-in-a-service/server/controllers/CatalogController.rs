#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
  #[Needs]
  catalog: std::sync::Arc<CatalogService>,
}
impl CatalogController {
  #[GetRoute]
  fn get_catalog(&self) -> Result<Json, YonResourceError> { self.catalog.find() }
}