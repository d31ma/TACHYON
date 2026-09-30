#[Test(HelloTestSetup, {"partition":"tests"})]
struct CounterServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl CounterServiceTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }

    fn testIncrementAddsOne(&self) {
        let mut counter = CounterService::new();
        counter.increment();
        Assert::equals(1, counter.count);
    }
}
