#[Test(GreetingTestSetup, {"partition":"tests"})]
struct GreetingControllerTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl GreetingControllerTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }

    fn test_get_greeting_says_hello(&self) {
        let controller = GreetingController::new();
        let greeting = controller.get_greeting();
        Assert::equals(String::from("Hello from Yon"), greeting.message);
    }
}
