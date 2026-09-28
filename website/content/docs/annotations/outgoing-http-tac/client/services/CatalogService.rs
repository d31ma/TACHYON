#[Service]
#[HTTPClient]
struct CatalogService { behaviors: BehaviorContext, count: i64 }
impl CatalogService {
    fn new() -> Self {
        let options = BehaviorOptions::new("catalog", BehaviorSignal::new());
        let behaviors = Behaviors::create::<CatalogSetup>(options).expect("catalog context");
        Self { behaviors, count: 0 }
    }

    #[GetRequest("/api/products")]
    fn all(&self) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([] as [(&str, BehaviorValue); 0]))
    }
}
