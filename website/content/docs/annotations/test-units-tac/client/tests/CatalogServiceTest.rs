#[Test(CatalogTestSetup, {"partition":"tests"})]
struct CatalogServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl CatalogServiceTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }
    fn testHasAContext(&self) -> BehaviorResult<BehaviorValue> {
        if self.behaviors.borrow().is_none() {
            return Err(BehaviorError { code: "adapter", message: "no context", path: None });
        }
        Ok(BehaviorValue::Null)
    }
}
