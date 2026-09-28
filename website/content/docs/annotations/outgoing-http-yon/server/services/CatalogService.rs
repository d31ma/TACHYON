#[yon::service]
struct CatalogService {
  #[Needs]
  catalog: std::sync::Arc<CatalogClient>,
}
impl CatalogService {
    fn products(&self) -> BehaviorResult<Json> {
        let answer = self.catalog.all(BehaviorValue::Number(1.0))?;
        let answer_text = answer.to_json(65536)?;
        Json::parse(&answer_text).map_err(|_| behavior_invalid())
    }
}
