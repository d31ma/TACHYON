#[HTTPClient]
struct CatalogClient { behaviors: BehaviorContext }
impl CatalogClient {
    fn new() -> Self {
        let options = BehaviorOptions::new("catalog", BehaviorSignal::new());
        Self { behaviors: Behaviors::create::<CatalogSetup>(options).expect("catalog context") }
    }

    #[GetRequest("/products")]
    fn all(&self, #[RequestParam("page")] page: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[PostRequest("/products")]
    fn add(&self, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[GetRequest("/products")]
    fn first(&self) -> BehaviorResult<BehaviorValue> {
        let query = BehaviorValue::object([("page", BehaviorValue::Number(1.0))]);
        Ok(BehaviorValue::object([("query", query)]))
    }
}
