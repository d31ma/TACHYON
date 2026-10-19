#[Service]
struct CatalogService {
    #[Needs]
    products_client: std::sync::Arc<ProductsClient>,
    count: usize,
}

impl CatalogService {
    pub fn load(&mut self) {
        self.count = Self::count_items(self.products_client.list());
    }

    pub fn add(&self) -> BehaviorResult<BehaviorValue> {
        self.products_client.save(BehaviorValue::object([("name", "anvil".into())]))
    }

    fn count_items(answer: BehaviorResult<BehaviorValue>) -> usize {
        match answer.ok().as_ref().and_then(|value| value.get("items")) {
            Some(BehaviorValue::Array(items)) => items.len(),
            _ => 0,
        }
    }
}
