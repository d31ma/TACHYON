#[Service]
struct CatalogService {
    #[Needs]
    catalog_client: std::sync::Arc<CatalogClient>,
    count: i64,
}
impl CatalogService {
    pub fn load(&mut self) {
        let answer = self.catalog_client.all().unwrap_or(BehaviorValue::Null);
        self.count = Self::count_items(&answer);
    }

    fn count_items(answer: &BehaviorValue) -> i64 {
        match answer.get("items") {
            Some(BehaviorValue::Array(items)) => items.len() as i64,
            _ => 0,
        }
    }
}
