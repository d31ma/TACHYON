#[Test(StatusTestSetup, {"partition":"tests"})]
struct StatusControllerTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl StatusControllerTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }
    fn testGetStatusSaysOk(&self) -> BehaviorResult<BehaviorValue> {
        if StatusController::new().get_status() != "ok" {
            return Err(BehaviorError { code: "adapter", message: "expected ok", path: None });
        }
        Ok(BehaviorValue::Null)
    }
}
