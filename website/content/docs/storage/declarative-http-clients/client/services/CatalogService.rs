#[Service]
struct CatalogService {
    #[Needs]
    catalog_client: std::sync::Arc<CatalogClient>,
    products: Vec<String>,
    error: String,
}
impl CatalogService {
    pub fn load(&mut self) {
        let answer = self.catalog_client.all();
        self.answer(answer);
    }

    fn answer(&mut self, answer: BehaviorResult<BehaviorValue>) {
        match answer {
            Ok(BehaviorValue::Array(items)) => self.show(&items),
            _ => self.show_failure(),
        }
    }

    fn show(&mut self, items: &[BehaviorValue]) {
        self.products = items.iter().filter_map(Self::name).collect();
        self.error = String::new();
    }

    fn name(item: &BehaviorValue) -> Option<String> {
        match item {
            BehaviorValue::String(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn show_failure(&mut self) {
        self.error = String::from("Products could not be loaded. Please retry.");
    }
}
