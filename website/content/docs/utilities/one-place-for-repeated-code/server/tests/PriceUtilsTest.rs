#[Test(PriceTestSetup, {"partition":"tests"})]
struct PriceUtilsTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl PriceUtilsTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }

    fn test_label_shows_dollars(&self) {
        Assert::equals(String::from("$12.50"), PriceUtils::label(1250));
    }
}
