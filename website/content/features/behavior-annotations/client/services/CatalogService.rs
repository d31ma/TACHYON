#[Service]
struct CatalogService {
    behaviors: BehaviorContext,
    offset: f64,
}

impl CatalogService {
    fn new() -> Self {
        let options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        Self { behaviors: Behaviors::create::<ApplicationSetup>(options).expect("behavior context"), offset: 0.0 }
    }

    fn page(&self, #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] input: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(input.get("offset").cloned().unwrap_or(BehaviorValue::Null))
    }

    #[OnStart]
    fn initialize(&mut self) {
        let first = BehaviorValue::object([("page", BehaviorValue::Number(1.0))]);
        self.offset = Self::offset_of(self.page(first));
    }

    fn offset_of(value: BehaviorResult<BehaviorValue>) -> f64 {
        match value {
            Ok(BehaviorValue::Number(offset)) => offset,
            _ => 0.0,
        }
    }
}
