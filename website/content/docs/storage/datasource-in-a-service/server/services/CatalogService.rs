#[Service]
struct CatalogService {
  #[Needs]
  data_source: std::sync::Arc<DataSource>,
}
impl CatalogService {
  fn find(&self) -> Result<Json, YonResourceError> {
    self.data_source.query("SELECT 42 AS amount", vec![])
  }
}