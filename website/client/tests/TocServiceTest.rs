#[Test(SiteTestSetup, {"partition":"tests"})]
struct TocServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
}
impl TocServiceTest {
    fn new() -> Self {
        Self { behaviors: std::cell::RefCell::new(None) }
    }

    fn toc(count: usize) -> TocService {
        TocService { entries: TacValue::List(vec![TacValue::Null; count]) }
    }

    fn testNoEntryIsNotWorthShowing(&self) {
        Assert::is_true(!Self::toc(0).worth_showing());
    }

    fn testOneEntryIsNotWorthShowing(&self) {
        Assert::is_true(!Self::toc(1).worth_showing());
    }

    fn testTwoEntriesAreWorthShowing(&self) {
        Assert::is_true(Self::toc(2).worth_showing());
    }
}
