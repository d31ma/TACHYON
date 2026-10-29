use super::VisibilityProject;

struct DelegateCases;

impl DelegateCases {
    const SERVICE: &'static str = "client/services/TaxService";
    const SCRIPT_SERVICE: &'static str = "@Service\nexport default class TaxService {\n  rate(region) {\n    return 2;\n  }\n\n  total(count) {\n    return count * 10;\n  }\n\n  spare() {\n    return 1;\n  }\n}\n";
    const SCRIPT_DELEGATE: &'static str = "@Delegate\nexport default class TaxDelegate {\n  /** @type {TaxService} */\n  @Needs\n  tax;\n\n  rate(region) {\n    return this.tax.rate(region) * 2;\n  }\n}\n";
    const TYPESCRIPT_DELEGATE: &'static str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  tax!: TaxService;\n\n  rate(region: string): number {\n    return this.tax.rate(region) * 2;\n  }\n}\n";
    const SCRIPT_PLAIN_DELEGATE: &'static str = "@Delegate\nexport default class TaxDelegate {\n  rate(region) {\n    return 4;\n  }\n\n  total(count) {\n    return count;\n  }\n}\n";
    const SCRIPT_CALLER: &'static str = "@Service\nexport default class QuoteService {\n  /** @type {TaxDelegate} */\n  @Needs\n  taxDelegate;\n\n  quote() {\n    return this.taxDelegate.rate('north') + this.taxDelegate.total(3);\n  }\n}\n";
    const PYTHON_SERVICE: &'static str = "@service\nclass TaxService:\n    def rate(self, region: str) -> int:\n        return 2\n\n    def total(self, count: int) -> int:\n        return count * 10\n\n    def spare(self) -> int:\n        return 1\n";
    const PYTHON_DELEGATE: &'static str = "from typing import Annotated\n\n@delegate\nclass TaxDelegate:\n    tax_service: Annotated[TaxService, needs]\n\n    def rate(self, region: str) -> int:\n        return self.tax_service.rate(region) * 2\n";
    const PYTHON_CALLER: &'static str = "from typing import Annotated\n\n@service\nclass QuoteService:\n    tax_delegate: Annotated[TaxDelegate, needs]\n\n    def quote(self) -> int:\n        return self.tax_delegate.rate(\"north\") + self.tax_delegate.total(3)\n";

    fn unused(extension: &str, (service, delegate, caller): (&str, &str, &str)) -> Vec<String> {
        let service_path = format!("{}.{extension}", Self::SERVICE);
        let delegate_path = format!("client/delegates/TaxDelegate.{extension}");
        let caller_path = format!("client/services/QuoteService.{extension}");
        let files = [
            (service_path.as_str(), service),
            (delegate_path.as_str(), delegate),
            (caller_path.as_str(), caller),
        ];
        VisibilityProject::names_in(&files, &service_path)
    }
}

#[test]
fn ty1333_a_call_through_a_delegate_forward_keeps_the_base_method_public() {
    let script = (
        DelegateCases::SCRIPT_SERVICE,
        DelegateCases::SCRIPT_DELEGATE,
        DelegateCases::SCRIPT_CALLER,
    );
    assert_eq!(DelegateCases::unused("js", script), ["spare"]);
    let typescript = (
        DelegateCases::SCRIPT_SERVICE,
        DelegateCases::TYPESCRIPT_DELEGATE,
        DelegateCases::SCRIPT_CALLER,
    );
    assert_eq!(DelegateCases::unused("ts", typescript), ["spare"]);
    let python = (
        DelegateCases::PYTHON_SERVICE,
        DelegateCases::PYTHON_DELEGATE,
        DelegateCases::PYTHON_CALLER,
    );
    assert_eq!(DelegateCases::unused("py", python), ["spare"]);
}

#[test]
fn ty1333_a_delegate_without_a_base_keeps_no_service_method_public() {
    let script = (
        DelegateCases::SCRIPT_SERVICE,
        DelegateCases::SCRIPT_PLAIN_DELEGATE,
        DelegateCases::SCRIPT_CALLER,
    );
    assert_eq!(
        DelegateCases::unused("js", script),
        ["rate", "total", "spare"]
    );
}

#[test]
fn ty1333_a_delegate_in_another_realm_keeps_no_client_method_public() {
    let files = [
        (
            "client/services/TaxService.js",
            DelegateCases::SCRIPT_SERVICE,
        ),
        (
            "server/services/TaxService.js",
            DelegateCases::SCRIPT_SERVICE,
        ),
        (
            "server/delegates/TaxDelegate.js",
            DelegateCases::SCRIPT_DELEGATE,
        ),
        (
            "server/services/QuoteService.js",
            DelegateCases::SCRIPT_CALLER,
        ),
    ];
    assert_eq!(
        VisibilityProject::names_in(&files, "client/services/TaxService.js"),
        ["rate", "total", "spare"]
    );
    assert_eq!(
        VisibilityProject::names_in(&files, "server/services/TaxService.js"),
        ["spare"]
    );
}
