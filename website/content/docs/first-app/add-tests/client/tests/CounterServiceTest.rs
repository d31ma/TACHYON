#[Test(HelloTestSetup, {"partition":"tests"})]
struct CounterServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
    #[Fake]
    greeting_client: std::cell::RefCell<Option<std::sync::Arc<GreetingClient>>>,
}
impl CounterServiceTest {
    fn new() -> Self {
        Self {
            behaviors: std::cell::RefCell::new(None),
            greeting_client: std::cell::RefCell::new(None),
        }
    }

    fn testIncrementAddsOne(&self) {
        let mut counter: CounterService = self.construct();
        counter.increment();
        Assert::equals(1, counter.count);
    }

    async fn testAskServerSavesTheMessage(&self) {
        let mut counter: CounterService = self.construct();
        let answer = counter.ask_server().await;
        Assert::is_true(answer.is_ok());
        Assert::equals("Hi", counter.message);
    }
}
