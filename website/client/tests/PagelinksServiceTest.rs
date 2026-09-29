#[Test(SiteTestSetup, {"partition":"tests"})]
struct PagelinksServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
}
impl PagelinksServiceTest {
    fn new() -> Self {
        Self { behaviors: std::cell::RefCell::new(None) }
    }

    fn testEditHrefPointsAtTheSourceItWasGiven(&self) {
        let links = PagelinksService { source: String::from("website/content/features/routing/feature.json"), path: String::new() };
        Assert::equals_with_message(
            "https://github.com/d31ma/Tachyon/edit/main/website/content/features/routing/feature.json",
            links.edit_href(),
            "edit links the source it was given",
        );
        let default_links = PagelinksService { source: String::new(), path: String::new() };
        Assert::is_true_with_message(
            default_links.edit_href().ends_with("/website/content/docs/page.json"),
            "the docs page is the default source",
        );
    }

    fn testReportHrefNamesThePageInTheIssueTitle(&self) {
        let links = PagelinksService { source: String::new(), path: String::from("/docs/routing") };
        Assert::equals_with_message(
            "https://github.com/d31ma/Tachyon/issues/new?title=Docs%3A%20%2Fdocs%2Frouting",
            links.report_href(),
            "the issue title names the page, encoded",
        );
    }

    fn testEncodeComponentKeepsSafeCharacters(&self) {
        Assert::equals("AZaz09-_.!~*'()", PagelinksService::encode_component("AZaz09-_.!~*'()"));
    }

    fn testEncodeComponentEncodesReservedCharacters(&self) {
        Assert::equals("a%2Fb%20c%3F", PagelinksService::encode_component("a/b c?"));
    }

    fn testEncodeComponentPercentEncodesUtf8(&self) {
        Assert::equals("%C3%A9", PagelinksService::encode_component("é"));
    }
}
