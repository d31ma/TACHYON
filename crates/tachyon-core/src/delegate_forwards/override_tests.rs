#![allow(clippy::expect_used)]

use super::tests::Project;
use super::typed_tests::Typed;

struct Overrides;

impl Overrides {
    const JAVA_OVERRIDES: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxes;\n\n    int rate(String region) {\n        return taxes.rate(region) * 2;\n    }\n\n    int rate(String region, String day) {\n        return taxes.rate(region);\n    }\n}\n";
    const JAVA_OTHER_RETURN: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxes;\n\n    long rate(String region) {\n        return taxes.rate(region) * 2L;\n    }\n}\n";
    const GO_OVERRIDE: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Rate(region string) int {\n\treturn delegate.taxes.Rate(region) * 2\n}\n";
    const GO_OVERLOAD: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Rate(region string, year int) int {\n\treturn delegate.taxes.Rate(region) + year\n}\n";
    const SWIFT_OTHER_LABEL: &str = "@Delegate\nfinal class TaxDelegate {\n    @Needs\n    private let taxes: TaxService\n\n    func rate(_ region: String, on year: Int) -> Int {\n        return year\n    }\n}\n";
    const JS_SERVICE: &str =
        "@Service\nexport default class TaxService {\n  rate(region) {\n    return 2;\n  }\n}\n";
    const JS_FIELD_CLASH: &str = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n\n  rate = 1;\n}\n";
    const GO_BODY_NAME: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Total(region string) int {\n\tRecord := 2\n\treturn delegate.taxes.Rate(region) * Record\n}\n";
    const GO_FIELD_CLASH: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n\tRate  int\n}\n";
    const JAVA_OVERRIDES_STAGED: &str = "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxes;\n\n    int rate(String region) {\n        return taxes.rate(region) * 2;\n    }\n\n    int rate(String region, String day) {\n        return taxes.rate(region);\n    }\n\n  int rate(String region, int year) {\n    return taxes.rate(region, year);\n  }\n\n  public void record(String sku, int count) {\n    taxes.record(sku, count);\n  }\n}\n";
    const GO_OVERRIDE_STAGED: &str = "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxes *TaxService\n}\n\nfunc (delegate *TaxDelegate) Rate(region string) int {\n\treturn delegate.taxes.Rate(region) * 2\n}\n\nfunc (delegate *TaxDelegate) Record(names ...string) {\n\tdelegate.taxes.Record(names...)\n}\n";
    const SWIFT_BASE_RATE: &str = "\n  func rate(_ region: String, year: Int) -> Int {\n    return taxes.rate(region, year: year)\n  }\n";

    fn files<'a>(
        language: &str,
        (service, delegate): (&'a str, &'a str),
    ) -> [(String, &'a str); 2] {
        [
            (format!("server/services/TaxService.{language}"), service),
            (format!("server/delegates/TaxDelegate.{language}"), delegate),
        ]
    }

    fn staged(language: &str, written: (&str, &str)) -> String {
        let files = Self::files(language, written);
        let named: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, source)| (path.as_str(), *source))
            .collect();
        Project::staged(&named, &files[1].0).expect("forwards")
    }

    fn refusal(language: &str, written: (&str, &str)) -> String {
        let files = Self::files(language, written);
        let named: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, source)| (path.as_str(), *source))
            .collect();
        Project::refusal(&named, &files[1].0)
    }
}

#[test]
fn a_java_override_replaces_its_base_method_and_an_overload_sits_next_to_it() {
    let staged = Overrides::staged("java", (Typed::JAVA_SERVICE, Overrides::JAVA_OVERRIDES));
    assert_eq!(staged, Overrides::JAVA_OVERRIDES_STAGED);
}

#[test]
fn a_java_override_that_returns_another_type_is_refused() {
    let refusal = Overrides::refusal("java", (Typed::JAVA_SERVICE, Overrides::JAVA_OTHER_RETURN));
    assert!(refusal.contains("TY1375"), "{refusal}");
    assert!(
        refusal.contains("TaxDelegate.rate has the same parameters as TaxService.rate, so it overrides it, but it returns long instead of int."),
        "{refusal}"
    );
}

#[test]
fn a_go_override_keeps_the_name_and_the_other_methods_are_forwarded() {
    let staged = Overrides::staged("go", (Typed::GO_SERVICE, Overrides::GO_OVERRIDE));
    assert_eq!(staged, Overrides::GO_OVERRIDE_STAGED);
}

#[test]
fn a_go_overload_is_refused_because_go_has_one_method_per_name() {
    let refusal = Overrides::refusal("go", (Typed::GO_SERVICE, Overrides::GO_OVERLOAD));
    assert!(refusal.contains("TY1374"), "{refusal}");
    assert!(
        refusal.contains(
            "TaxDelegate.Rate has other parameters than TaxService.Rate. Go has one method per name"
        ),
        "{refusal}"
    );
}

#[test]
fn a_swift_method_with_other_argument_labels_is_an_overload() {
    let files = [
        ("client/services/TaxService.swift", Typed::SWIFT_SERVICE),
        (
            "client/delegates/TaxDelegate.swift",
            Overrides::SWIFT_OTHER_LABEL,
        ),
    ];
    let staged = Project::staged(&files, "client/delegates/TaxDelegate.swift").expect("forwards");
    assert!(staged.contains(Overrides::SWIFT_BASE_RATE), "{staged}");
    assert!(
        staged.contains("func rate(_ region: String, on year: Int) -> Int {"),
        "{staged}"
    );
    assert!(
        staged.contains("func fetch(id: Int) async throws -> String {"),
        "{staged}"
    );
}

#[test]
fn a_field_named_like_a_base_method_is_refused() {
    let refusal = Overrides::refusal("js", (Overrides::JS_SERVICE, Overrides::JS_FIELD_CLASH));
    assert!(refusal.contains("TY1375"), "{refusal}");
    assert!(
        refusal.contains(
            "TaxDelegate has a field named rate, and its base TaxService has a method named rate."
        ),
        "{refusal}"
    );
}

#[test]
fn a_go_method_body_line_named_like_a_base_method_is_not_a_field() {
    let staged = Overrides::staged("go", (Typed::GO_SERVICE, Overrides::GO_BODY_NAME));
    assert!(staged.contains("\tRecord := 2\n"), "{staged}");
    assert!(
        staged.contains("func (delegate *TaxDelegate) Rate(region string) int {"),
        "{staged}"
    );
}

#[test]
fn a_go_struct_field_named_like_a_base_method_is_refused() {
    let refusal = Overrides::refusal("go", (Typed::GO_SERVICE, Overrides::GO_FIELD_CLASH));
    assert!(refusal.contains("TY1375"), "{refusal}");
    assert!(
        refusal.contains("TaxDelegate has a field named Rate"),
        "{refusal}"
    );
}
