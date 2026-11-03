#[Service]
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
struct CatalogService {
    behaviors: BehaviorContext,
    sku: String,
    reads: std::cell::Cell<f64>,
}
impl CatalogService {
    fn new() -> Self {
        let options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        Self {
            behaviors: Behaviors::create::<CatalogSetup>(options).expect("behavior context"),
            sku: "anvil".into(),
            reads: std::cell::Cell::new(0.0),
        }
    }

    #[Cached]
    fn find(&self, sku: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let reads = self.reads.get() + 1.0;
        self.reads.set(reads);
        Ok(Self::entry(sku, BehaviorValue::Number(reads)))
    }

    #[UpdateCache({"method":"find"})]
    fn write(&self, sku: BehaviorValue, reads: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(Self::entry(sku, reads))
    }

    #[ClearCache({"method":"find"})]
    fn invalidate(&self, sku: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(Self::entry(sku, BehaviorValue::Number(self.reads.get())))
    }

    fn entry(sku: BehaviorValue, reads: BehaviorValue) -> BehaviorValue {
        BehaviorValue::object([("sku", sku), ("reads", reads)])
    }
}
