#[Service]
struct CatalogService {
    products: ProductsClient,
    count: usize,
}

impl CatalogService {
    fn new() -> Self {
        Self { products: ProductsClient::new(), count: 0 }
    }

    fn load(&mut self) {
        self.count = Self::count_items(self.products.list());
    }

    fn add(&self) -> BehaviorResult<BehaviorValue> {
        self.products.save(BehaviorValue::object([("name", "anvil".into())]))
    }

    fn count_items(answer: BehaviorResult<BehaviorValue>) -> usize {
        match answer.ok().as_ref().and_then(|value| value.get("items")) {
            Some(BehaviorValue::Array(items)) => items.len(),
            _ => 0,
        }
    }
}
