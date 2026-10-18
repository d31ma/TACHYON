#[Service]
struct OrdersService {
    behaviors: BehaviorContext,
    tries: std::cell::Cell<f64>,
    attempts: f64,
}
impl OrdersService {
    const FIRST_WORKING_ATTEMPT: f64 = 3.0;

    fn new() -> Self {
        let options = BehaviorOptions::new("orders", BehaviorSignal::new());
        Self {
            behaviors: Behaviors::create::<OrdersSetup>(options).expect("behavior context"),
            tries: std::cell::Cell::new(0.0),
            attempts: 0.0,
        }
    }

    #[Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]
    fn send(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        self.tries.set(self.tries.get() + 1.0);
        Self::fail_first_two_tries(self.tries.get())?;
        Ok(self.report_of(id))
    }

    #[MaxAtOnce({"limit":2,"policy":"block"})]
    fn report(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(self.report_of(id))
    }

    fn report_of(&self, id: BehaviorValue) -> BehaviorValue {
        BehaviorValue::object([("id", id), ("attempts", BehaviorValue::Number(self.tries.get()))])
    }

    fn submit(&mut self, id: String) {
        let _ = self.send(BehaviorValue::from(id));
        self.attempts = self.tries.get();
    }

    fn fail_first_two_tries(tries: f64) -> BehaviorResult<()> {
        if tries < Self::FIRST_WORKING_ATTEMPT {
            return Err(BehaviorError::new("adapter", "the checkout API is unavailable"));
        }
        Ok(())
    }
}
