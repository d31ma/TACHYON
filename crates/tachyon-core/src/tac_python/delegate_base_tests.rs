use super::page_tests::PythonPage;

struct PythonDelegateBase;

impl PythonDelegateBase {
    const TAX: &'static str = "@service\nclass TaxService:\n    def rate(self, region: str) -> int:\n        return 2\n\n    \
         def total(self, count: int) -> int:\n        return count * 10\n";

    const DELEGATE: &'static str = "from typing import Annotated\n\n@delegate\nclass TaxDelegate:\n    tax_service: Annotated[TaxService, needs]\n\n    \
         def rate(self, region: str) -> int:\n        return self.tax_service.rate(region) * 2\n\n    \
         def total(self, count: int, extra: int) -> int:\n        return count * 10 + extra\n";

    const QUOTE: &'static str = "from typing import Annotated\n\n@service\nclass QuoteService:\n    tax_delegate: Annotated[TaxDelegate, needs]\n\n    \
         def rate(self) -> int:\n        return self.tax_delegate.rate(\"north\")\n\n    \
         def total(self) -> int:\n        return self.tax_delegate.total(3)\n\n    \
         def extra(self) -> int:\n        return self.tax_delegate.total(3, 4)\n";

    const DRIVER: &'static str = "const { default: QuoteService } = await import('./QuoteService.js')\n\
         const quote = new QuoteService({})\n\
         const quoted = { rate: await quote.rate(), total: await quote.total(), extra: await quote.extra() }\n\
         console.log(JSON.stringify({ quoted }))\n\
         process.exit(0)\n";

    fn staged_delegate() -> String {
        let files = [
            ("client/services/TaxService.py", Self::TAX),
            ("client/delegates/TaxDelegate.py", Self::DELEGATE),
        ];
        let staged = crate::delegate_forwards::DelegateForwards::staged_project(&files);
        staged
            .into_iter()
            .find(|(path, _)| path.contains("delegates"))
            .map(|(_, text)| text)
            .unwrap_or_else(|| panic!("the staged TaxDelegate"))
    }
}

#[tokio::test]
async fn a_python_delegate_overrides_and_overloads_its_base_by_argument_count() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    PythonPage::write(
        root.path(),
        "client/delegates/TaxDelegate.py",
        &PythonDelegateBase::staged_delegate(),
    );
    PythonPage::publish(
        root.path(),
        &[
            ("quote", "QuoteService", PythonDelegateBase::QUOTE),
            ("tax", "TaxService", PythonDelegateBase::TAX),
        ],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonDelegateBase::DRIVER);
    assert_eq!(
        report["quoted"],
        serde_json::json!({"rate": 4, "total": 30, "extra": 34}),
        "{report}"
    );
}
