struct Resilient {
    #[Needs]
    behaviors: BehaviorContext,
    #[Needs]
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Resilient {
    #[Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
    fn flaky(&self) -> BehaviorResult<BehaviorValue> {
        let count = self.attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        if count < 3 {
            return Err(BehaviorError::new("adapter", "transient"));
        }
        Ok(BehaviorValue::Number(count as f64))
    }

    #[MaxAtOnce({"limit":2,"policy":"block"})]
    fn bounded(&self, value: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(value) }
}
