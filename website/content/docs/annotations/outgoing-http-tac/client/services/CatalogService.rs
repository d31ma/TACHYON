#[Service]
struct CatalogService {
    catalog: CatalogClient,
    count: i64,
}
impl CatalogService {
    fn new() -> Self {
        Self {
            catalog: CatalogClient::new(),
            count: 0,
        }
    }

    fn load(&mut self) {
        let answer = self.catalog.all().unwrap_or(BehaviorValue::Null);
        self.count = Self::count_items(&answer);
    }

    fn count_items(answer: &BehaviorValue) -> i64 {
        match answer.get("items") {
            Some(BehaviorValue::Array(items)) => items.len() as i64,
            _ => 0,
        }
    }
}
