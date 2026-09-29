#[Service]
struct CatalogService { catalog: CatalogClient, count: i64 }
impl CatalogService {
    fn new() -> Self {
        Self { catalog: CatalogClient::new(), count: 0 }
    }

    fn all(&self) -> BehaviorResult<BehaviorValue> {
        self.catalog.all()
    }
}
