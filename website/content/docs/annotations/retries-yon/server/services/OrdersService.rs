#[yon::service]
struct OrdersService {
    behaviors: BehaviorContext,
    attempts: std::cell::Cell<f64>,
}
impl OrdersService {
    const FIRST_WORKING_ATTEMPT: f64 = 3.0;

    fn new() -> Self {
        let options = BehaviorOptions::new("orders", BehaviorSignal::new());
        Self {
            behaviors: Behaviors::create::<OrdersSetup>(options).expect("behavior context"),
            attempts: std::cell::Cell::new(0.0),
        }
    }

    #[Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]
    fn submit(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let attempts = self.attempts.get() + 1.0;
        self.attempts.set(attempts);
        Self::fail_first_two_attempts(attempts)?;
        Ok(Self::report_of(id, attempts))
    }

    #[MaxAtOnce({"limit":2,"policy":"block"})]
    fn report(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(Self::report_of(id, self.attempts.get()))
    }

    fn report_of(id: BehaviorValue, attempts: f64) -> BehaviorValue {
        BehaviorValue::object([("id", id), ("attempts", BehaviorValue::Number(attempts))])
    }

    fn fail_first_two_attempts(attempts: f64) -> BehaviorResult<()> {
        if attempts < Self::FIRST_WORKING_ATTEMPT {
            return Err(BehaviorError::new("adapter", "the payment gateway is unavailable"));
        }
        Ok(())
    }
}
