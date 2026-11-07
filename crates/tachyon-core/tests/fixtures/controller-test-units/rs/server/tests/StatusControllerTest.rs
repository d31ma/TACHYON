#[Test(StatusTestSetup, {"partition":"tests"})]
struct StatusControllerTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl StatusControllerTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }
    fn testGetStatusSaysOk(&self) {
        Assert::equals("ok", StatusController::new().get_status());
    }
}
