#[Service]
struct CatalogService { catalog: CatalogClient, products: Vec<String>, error: String }
impl CatalogService {
    fn new() -> Self {
        Self { catalog: CatalogClient::new(), products: Vec::new(), error: String::new() }
    }

    fn load(&mut self) {
        match self.catalog.all() {
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
