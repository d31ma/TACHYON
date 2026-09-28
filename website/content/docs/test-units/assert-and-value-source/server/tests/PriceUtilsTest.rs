#[Test(PriceTestSetup)]
struct PriceUtilsTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }
impl PriceUtilsTest {
    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }

    fn test_adds_the_tax(&self) {
        Assert::equals(Some(1200), PriceUtils::with_tax(1000).ok());
    }

    #[ValueSource([0, 5, 1000])]
    fn test_never_lowers_a_price(&self, cents: i64) {
        assert!(PriceUtils::with_tax(cents).unwrap_or_default() >= cents, "the tax never lowers a price");
    }

    fn test_refuses_a_negative_price(&self) {
        Assert::throws(|| PriceUtils::with_tax(-1));
    }
}
