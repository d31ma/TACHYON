#![allow(clippy::expect_used)]

use super::DelegateForwards;
use std::path::Path;

pub(super) struct Project;

impl Project {
    const JS_SERVICE: &str = "@Service\nexport default class TaxService {\n  rate(region) {\n    return 2;\n  }\n\n  async total(items) {\n    return items.length;\n  }\n\n  _secret() {\n    return 1;\n  }\n\n  static made() {\n    return new TaxService();\n  }\n\n  @OnStart\n  start() {\n    return true;\n  }\n}\n";
    const JS_DELEGATE: &str = "@Delegate\nexport default class TaxDelegate {\n  /** @type {TaxService} */\n  @Needs\n  taxService;\n\n  rate(region) {\n    return this.taxService.rate(region) * 2;\n  }\n}\n";
    const TS_SERVICE: &str = "@Service\nexport default class TaxService {\n  rate(region: string): number {\n    return 2;\n  }\n\n  total(items: string[]): number {\n    return items.length;\n  }\n}\n";
    const TS_DELEGATE: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  tax!: TaxService;\n\n  rate(region: string): number {\n    return this.tax.rate(region) * 2;\n  }\n}\n";
    const PY_SERVICE: &str = "@service\nclass TaxService:\n    def rate(self, region: str) -> int:\n        return 2\n\n    def total(self, items: list) -> int:\n        return len(items)\n\n    @on_start\n    async def start(self):\n        return None\n";
    const PY_DELEGATE: &str = "from typing import Annotated\n\n@delegate\nclass TaxDelegate:\n    tax: Annotated[TaxService, needs]\n\n    def rate(self, region: str) -> int:\n        return self.tax.rate(region) * 2\n";
    const PHP_SERVICE: &str = "<?php\n#[Service]\nclass TaxService {\n  public function rate(string $region): int {\n    return 2;\n  }\n\n  public function total(array $items): int {\n    return count($items);\n  }\n\n  private function secret(): int {\n    return 1;\n  }\n}\n";
    const PHP_DELEGATE: &str = "<?php\n#[Delegate]\nclass TaxDelegate {\n  #[Needs]\n  private TaxService $tax;\n\n  public function rate(string $region): int {\n    return $this->tax->rate($region) * 2;\n  }\n}\n";
    const RUBY_SERVICE: &str = "#[Service]\nclass TaxService\n  def rate(region)\n    2\n  end\n\n  def total(items)\n    items.length\n  end\nend\n";
    const RUBY_DELEGATE: &str = "#[Delegate]\nclass TaxDelegate\n  #[Needs]\n  # @return [TaxService]\n  attr_reader :tax\n\n  def rate(region)\n    tax.rate(region) * 2\n  end\n\n  private\n\n  def doubled(value)\n    value * 2\n  end\nend\n";
    const PRICE_SERVICE: &str =
        "@Service\nexport default class PriceService {\n  price() {\n    return 3;\n  }\n}\n";
    const TWO_BASES: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n\n  @Needs\n  priceService;\n}\n";
    const NO_BASE: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  orderStore;\n\n  rate() {\n    return 2;\n  }\n}\n";

    pub(super) fn staged(files: &[(&str, &str)], delegate: &str) -> Option<String> {
        let forwards = Self::forwards(files);
        let source = Self::source(files, delegate);
        forwards
            .staged(Path::new(delegate), source.as_bytes())
            .expect("the delegate is staged")
    }

    pub(super) fn refusal(files: &[(&str, &str)], delegate: &str) -> String {
        let forwards = Self::forwards(files);
        let source = Self::source(files, delegate);
        let Err(failure) = forwards.staged(Path::new(delegate), source.as_bytes()) else {
            panic!("the delegate was accepted");
        };
        failure.to_string()
    }

    fn forwards<'a>(files: &'a [(&'a str, &'a str)]) -> DelegateForwards<'a> {
        DelegateForwards::new(
            files
                .iter()
                .map(|(path, source)| (Path::new(*path), source.as_bytes())),
        )
    }

    fn source<'a>(files: &[(&str, &'a str)], delegate: &str) -> &'a str {
        files
            .iter()
            .find(|(path, _)| *path == delegate)
            .map(|(_, source)| *source)
            .expect("the delegate is one of the files")
    }
}

#[test]
fn a_javascript_delegate_takes_on_the_public_methods_its_service_declares() {
    let files = [
        ("server/services/TaxService.js", Project::JS_SERVICE),
        ("server/delegates/TaxDelegate.js", Project::JS_DELEGATE),
    ];
    let staged = Project::staged(&files, "server/delegates/TaxDelegate.js").expect("forwards");
    assert!(
        staged
            .contains("\n  total(...args) {\n    return this.taxService.total(...args);\n  }\n}\n"),
        "{staged}"
    );
    assert_eq!(staged.matches("rate(").count(), 2, "{staged}");
    for name in ["_secret", "made", "start"] {
        assert!(!staged.contains(name), "{name}: {staged}");
    }
    let written = Project::JS_DELEGATE
        .strip_suffix("}\n")
        .expect("the class ends the file");
    assert!(staged.starts_with(written), "{staged}");
}

#[test]
fn a_typescript_forward_keeps_the_parameter_and_return_types_of_the_base() {
    let files = [
        ("client/services/TaxService.ts", Project::TS_SERVICE),
        ("client/delegates/TaxDelegate.ts", Project::TS_DELEGATE),
    ];
    let staged = Project::staged(&files, "client/delegates/TaxDelegate.ts").expect("forwards");
    assert!(
        staged.contains(
            "  total(...args: Parameters<TaxService['total']>): ReturnType<TaxService['total']> {\n    return this.tax.total(...args);\n  }\n"
        ),
        "{staged}"
    );
    assert!(!staged.contains("rate(...args"), "{staged}");
}

#[test]
fn a_python_forward_passes_every_argument_on() {
    let files = [
        ("server/services/TaxService.py", Project::PY_SERVICE),
        ("server/delegates/TaxDelegate.py", Project::PY_DELEGATE),
    ];
    let staged = Project::staged(&files, "server/delegates/TaxDelegate.py").expect("forwards");
    assert!(
        staged.ends_with(
            "        return self.tax.rate(region) * 2\n\n    def total(self, *args, **kwargs):\n        return self.tax.total(*args, **kwargs)\n"
        ),
        "{staged}"
    );
    assert!(!staged.contains("def start"), "{staged}");
}

#[test]
fn a_php_forward_is_a_public_method() {
    let files = [
        ("server/services/TaxService.php", Project::PHP_SERVICE),
        ("server/delegates/TaxDelegate.php", Project::PHP_DELEGATE),
    ];
    let staged = Project::staged(&files, "server/delegates/TaxDelegate.php").expect("forwards");
    assert!(
        staged.contains(
            "  public function total(...$args) {\n    return $this->tax->total(...$args);\n  }\n}\n"
        ),
        "{staged}"
    );
    assert!(!staged.contains("secret"), "{staged}");
}

#[test]
fn a_ruby_forward_stays_public_after_a_private_section() {
    let files = [
        ("server/services/TaxService.rb", Project::RUBY_SERVICE),
        ("server/delegates/TaxDelegate.rb", Project::RUBY_DELEGATE),
    ];
    let staged = Project::staged(&files, "server/delegates/TaxDelegate.rb").expect("forwards");
    assert!(
        staged.ends_with("\n  public\n\n  def total(...)\n    tax.total(...)\n  end\nend\n"),
        "{staged}"
    );
    assert_eq!(staged.matches("def rate").count(), 1, "{staged}");
}

#[test]
fn a_delegate_with_two_services_is_refused() {
    let files = [
        ("server/services/TaxService.js", Project::JS_SERVICE),
        ("server/services/PriceService.js", Project::PRICE_SERVICE),
        ("server/delegates/TaxDelegate.js", Project::TWO_BASES),
    ];
    let refusal = Project::refusal(&files, "server/delegates/TaxDelegate.js");
    assert!(refusal.contains("TY1373"), "{refusal}");
    assert!(
        refusal.contains("TaxDelegate needs two services, TaxService and PriceService"),
        "{refusal}"
    );
}

#[test]
fn a_delegate_without_a_service_and_other_files_stay_as_written() {
    let files = [
        ("server/services/TaxService.js", Project::JS_SERVICE),
        ("server/delegates/TaxDelegate.js", Project::NO_BASE),
    ];
    assert_eq!(
        Project::staged(&files, "server/delegates/TaxDelegate.js"),
        None
    );
    assert_eq!(
        Project::staged(&files, "server/services/TaxService.js"),
        None
    );
}

#[test]
fn a_service_of_the_other_realm_is_not_a_base() {
    let files = [
        ("client/services/TaxService.js", Project::JS_SERVICE),
        ("server/delegates/TaxDelegate.js", Project::JS_DELEGATE),
    ];
    assert_eq!(
        Project::staged(&files, "server/delegates/TaxDelegate.js"),
        None
    );
}

#[test]
fn a_tac_ruby_delegate_cannot_take_on_a_service() {
    let files = [
        ("client/services/TaxService.rb", Project::RUBY_SERVICE),
        ("client/delegates/TaxDelegate.rb", Project::RUBY_DELEGATE),
    ];
    let refusal = Project::refusal(&files, "client/delegates/TaxDelegate.rb");
    assert!(refusal.contains("TY1406"), "{refusal}");
    assert!(
        refusal.contains("a Ruby browser service cannot need another service"),
        "{refusal}"
    );
}
