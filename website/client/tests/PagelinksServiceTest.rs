#[Test(SiteTestSetup, {"partition":"tests"})]
struct PagelinksServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
}
impl PagelinksServiceTest {
    fn new() -> Self {
        Self { behaviors: std::cell::RefCell::new(None) }
    }

    fn testEditHrefPointsAtTheSourceItWasGiven(&self) -> BehaviorResult<BehaviorValue> {
        let links = PagelinksService { source: String::from("website/content/features/routing/feature.json"), path: String::new() };
        if links.edit_href() != "https://github.com/d31ma/Tachyon/edit/main/website/content/features/routing/feature.json" {
            return Err(BehaviorError { code: "assertion", message: "edit links the source it was given", path: None });
        }
        let default_links = PagelinksService { source: String::new(), path: String::new() };
        if !default_links.edit_href().ends_with("/website/content/docs/page.json") {
            return Err(BehaviorError { code: "assertion", message: "the docs page is the default source", path: None });
        }
        Ok(BehaviorValue::Null)
    }

    fn testReportHrefNamesThePageInTheIssueTitle(&self) -> BehaviorResult<BehaviorValue> {
        let links = PagelinksService { source: String::new(), path: String::from("/docs/routing") };
        if links.report_href() != "https://github.com/d31ma/Tachyon/issues/new?title=Docs%3A%20%2Fdocs%2Frouting" {
            return Err(BehaviorError { code: "assertion", message: "the issue title names the page, encoded", path: None });
        }
        Ok(BehaviorValue::Null)
    }

    fn testEncodeComponentPreservesSafeCharactersAndEncodesUtf8(&self) -> BehaviorResult<BehaviorValue> {
        for (input, expected) in [
            ("AZaz09-_.!~*'()", "AZaz09-_.!~*'()"),
            ("a/b c?", "a%2Fb%20c%3F"),
            ("é", "%C3%A9"),
        ] {
            if PagelinksService::encode_component(input) != expected {
                return Err(BehaviorError { code: "assertion", message: "component encoding preserves safe characters and percent-encodes UTF-8", path: None });
            }
        }
        Ok(BehaviorValue::Null)
    }
}
