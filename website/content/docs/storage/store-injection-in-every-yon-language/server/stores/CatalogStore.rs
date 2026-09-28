#[Store]
struct CatalogStore {
  #[Needs]
  data_source: std::sync::Arc<DataSource>,
}
impl CatalogStore {
  fn find(&self) -> Result<Json, YonResourceError> {
    self.data_source.query("SELECT 42 AS amount", vec![])
  }
}