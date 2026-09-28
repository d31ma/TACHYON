#[Test(SiteTestSetup, {"partition":"tests"})]
struct TocServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
}
impl TocServiceTest {
    fn new() -> Self {
        Self { behaviors: std::cell::RefCell::new(None) }
    }

    fn testWorthShowingNeedsMoreThanOneEntry(&self) -> BehaviorResult<BehaviorValue> {
        let toc = |count: usize| TocService { entries: TacValue::List(vec![TacValue::Null; count]) };
        if toc(0).worth_showing() || toc(1).worth_showing() {
            return Err(BehaviorError { code: "assertion", message: "no entries, or one, is not a table of contents", path: None });
        }
        if !toc(2).worth_showing() {
            return Err(BehaviorError { code: "assertion", message: "two entries are", path: None });
        }
        Ok(BehaviorValue::Null)
    }
}
