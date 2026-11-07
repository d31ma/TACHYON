#[Test(CatalogTestSetup, {"partition":"tests"})]
struct CatalogServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl CatalogServiceTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }
    fn testOpensAContext(&self) {
        Assert::is_true_with_message(self.behaviors.borrow().is_some(), "the case has a context");
    }
}
