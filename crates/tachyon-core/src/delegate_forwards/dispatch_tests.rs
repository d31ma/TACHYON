#![allow(clippy::expect_used)]

use super::tests::Project;

struct Dispatch;

impl Dispatch {
    const JS_SERVICE: &str = "@Service\nexport default class TaxService {\n  rate(region) {\n    return 2;\n  }\n\n  total(items) {\n    return items.length;\n  }\n}\n";
    const JS_OVERLOAD: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n\n  rate(region, day) {\n    return this.taxService.rate(region) + day;\n  }\n}\n";
    const JS_OVERLAP: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n\n  rate(region, day = 1) {\n    return this.taxService.rate(region) + day;\n  }\n}\n";
    const JS_OVERLOAD_STAGED: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n\n  _own_rate(region, day) {\n    return this.taxService.rate(region) + day;\n  }\n\n  total(...args) {\n    return this.taxService.total(...args);\n  }\n\n  rate(...args) {\n    if (args.length >= 2 && args.length <= 2) {\n      return this._own_rate(...args);\n    }\n    return this.taxService.rate(...args);\n  }\n}\n";
    const TS_SERVICE: &str = "@Service\nexport default class TaxService {\n  rate(region: string): number {\n    return 2;\n  }\n}\n";
    const TS_OVERLOAD: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  tax!: TaxService;\n\n  rate(region: string, day: number, month?: number): number {\n    return this.tax.rate(region) + day;\n  }\n}\n";
    const TS_DISPATCH: &str = "\n  rate(...args: Parameters<TaxService['rate']>): ReturnType<TaxService['rate']>;\n  rate(...args: Parameters<TaxDelegate['_own_rate']>): ReturnType<TaxDelegate['_own_rate']>;\n  rate(...args: any[]): any {\n    if (args.length >= 2 && args.length <= 3) {\n      return (this._own_rate as (...values: any[]) => any).apply(this, args);\n    }\n    return (this.tax.rate as (...values: any[]) => any).apply(this.tax, args);\n  }\n}\n";
    const PY_SERVICE: &str =
        "@service\nclass TaxService:\n    def rate(self, region: str) -> int:\n        return 2\n";
    const PY_OVERLOAD: &str = "from typing import Annotated\n\n@delegate\nclass TaxDelegate:\n    tax: Annotated[TaxService, needs]\n\n    def rate(self, region: str, day: int, month: int = 1) -> int:\n        return self.tax.rate(region) + day\n";
    const PY_DISPATCH: &str = "\n\n    def rate(self, *args, **kwargs):\n        if 2 <= len(args) + len(kwargs) <= 3:\n            return self._own_rate(*args, **kwargs)\n        return self.tax.rate(*args, **kwargs)\n";
    const LUA_SERVICE: &str = "-- @Service\n---@class TaxService\nTaxService = {}\n\nfunction TaxService:rate(region)\n  return 2\nend\n";
    const LUA_OVERLOAD: &str = "-- @Delegate\n---@class TaxDelegate\nTaxDelegate = {\n  -- @Needs\n  ---@type TaxService\n  tax_service = nil,\n}\n\nfunction TaxDelegate:rate(region, day)\n  return self.tax_service:rate(region) + day\nend\n";
    const LUA_DISPATCH: &str = "\nfunction TaxDelegate:rate(...)\n  local count = select('#', ...)\n  if count >= 2 and count <= 2 then\n    return self:_own_rate(...)\n  end\n  return self.tax_service:rate(...)\nend\n";
    const RUBY_SERVICE: &str =
        "#[Service]\nclass TaxService\n  def rate(region)\n    2\n  end\nend\n";
    const RUBY_REST: &str = "#[Delegate]\nclass TaxDelegate\n  #[Needs]\n  # @return [TaxService]\n  attr_reader :tax\n\n  def rate(region, *days)\n    tax.rate(region) + days.length\n  end\nend\n";
    const PHP_SERVICE: &str = "<?php\n#[Service]\nclass TaxService {\n  public function rate(string $region): int {\n    return 2;\n  }\n}\n";
    const PHP_OVERLOAD: &str = "<?php\n#[Delegate]\nclass TaxDelegate {\n  #[Needs]\n  private TaxService $tax;\n\n  public function rate(string $region, int $day, int $month = 1): int {\n    return $this->tax->rate($region) + $day;\n  }\n}\n";
    const PHP_DISPATCH: &str = "\n  public function rate(...$args) {\n    if (count($args) >= 2 && count($args) <= 3) {\n      return $this->_own_rate(...$args);\n    }\n    return $this->tax->rate(...$args);\n  }\n}\n";

    fn staged(realm: &str, extension: &str, (service, delegate): (&str, &str)) -> String {
        let service_path = format!("{realm}/services/TaxService.{extension}");
        let delegate_path = format!("{realm}/delegates/TaxDelegate.{extension}");
        let files = [
            (service_path.as_str(), service),
            (delegate_path.as_str(), delegate),
        ];
        Project::staged(&files, &delegate_path).expect("forwards")
    }

    fn refusal(extension: &str, (service, delegate): (&str, &str)) -> String {
        let service_path = format!("server/services/TaxService.{extension}");
        let delegate_path = format!("server/delegates/TaxDelegate.{extension}");
        let files = [
            (service_path.as_str(), service),
            (delegate_path.as_str(), delegate),
        ];
        Project::refusal(&files, &delegate_path)
    }
}

#[test]
fn a_javascript_overload_is_renamed_and_a_dispatcher_picks_it_by_argument_count() {
    let staged = Dispatch::staged(
        "server",
        "js",
        (Dispatch::JS_SERVICE, Dispatch::JS_OVERLOAD),
    );
    assert_eq!(staged, Dispatch::JS_OVERLOAD_STAGED);
}

#[test]
fn a_javascript_method_whose_argument_counts_overlap_its_base_is_refused() {
    let refusal = Dispatch::refusal("js", (Dispatch::JS_SERVICE, Dispatch::JS_OVERLAP));
    assert!(refusal.contains("TY1374"), "{refusal}");
    assert!(
        refusal.contains("TaxDelegate.rate takes 1 to 2 arguments and TaxService.rate takes 1, so a call could match both."),
        "{refusal}"
    );
}

#[test]
fn a_typescript_dispatcher_keeps_both_signatures_for_the_type_checker() {
    let staged = Dispatch::staged(
        "server",
        "ts",
        (Dispatch::TS_SERVICE, Dispatch::TS_OVERLOAD),
    );
    assert!(
        staged.contains("  _own_rate(region: string, day: number, month?: number): number {"),
        "{staged}"
    );
    assert!(staged.ends_with(Dispatch::TS_DISPATCH), "{staged}");
}

#[test]
fn a_python_dispatcher_counts_keyword_arguments_too() {
    let staged = Dispatch::staged(
        "server",
        "py",
        (Dispatch::PY_SERVICE, Dispatch::PY_OVERLOAD),
    );
    assert!(
        staged.contains("    def _own_rate(self, region: str, day: int, month: int = 1) -> int:"),
        "{staged}"
    );
    assert!(staged.ends_with(Dispatch::PY_DISPATCH), "{staged}");
}

#[test]
fn a_lua_dispatcher_counts_the_values_it_gets() {
    let staged = Dispatch::staged(
        "client",
        "lua",
        (Dispatch::LUA_SERVICE, Dispatch::LUA_OVERLOAD),
    );
    assert!(
        staged.contains("function TaxDelegate:_own_rate(region, day)"),
        "{staged}"
    );
    assert!(staged.ends_with(Dispatch::LUA_DISPATCH), "{staged}");
}

#[test]
fn a_php_dispatcher_counts_its_arguments() {
    let staged = Dispatch::staged(
        "server",
        "php",
        (Dispatch::PHP_SERVICE, Dispatch::PHP_OVERLOAD),
    );
    assert!(
        staged.contains(
            "  public function _own_rate(string $region, int $day, int $month = 1): int {"
        ),
        "{staged}"
    );
    assert!(staged.ends_with(Dispatch::PHP_DISPATCH), "{staged}");
}

#[test]
fn a_ruby_rest_parameter_overlaps_every_longer_call() {
    let refusal = Dispatch::refusal("rb", (Dispatch::RUBY_SERVICE, Dispatch::RUBY_REST));
    assert!(refusal.contains("TY1374"), "{refusal}");
    assert!(
        refusal.contains("TaxDelegate.rate takes 1 or more arguments and TaxService.rate takes 1"),
        "{refusal}"
    );
}
