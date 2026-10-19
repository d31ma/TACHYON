#[CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":10})]
#[Service]
struct CatalogService {
    behaviors: BehaviorContext,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl CatalogService {
    fn new() -> Self {
        let options = BehaviorOptions::new("tenant-a", BehaviorSignal::new());
        let behaviors = Behaviors::create::<ConfigSetup>(options).expect("catalog context");
        Self { behaviors, calls: std::sync::Arc::default() }
    }

    #[Cached]
    fn list(&self, #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let count = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(BehaviorValue::object([("page", page), ("calls", BehaviorValue::Number(count as f64))]))
    }

    #[Cached]
    fn price(&self, #[CacheKey] sku: BehaviorValue, note: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = (sku, note);
        let count = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(BehaviorValue::Number(count as f64))
    }

    #[UpdateCache({"method":"list"})]
    fn refresh(&self, page: BehaviorValue, value: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = page;
        Ok(value)
    }

    #[ClearCache({"method":"list"})]
    fn invalidate(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = page;
        Ok(true.into())
    }

    #[Cached]
    fn validated(&self, #[Check(ProductInput)] input: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(input) }
}

