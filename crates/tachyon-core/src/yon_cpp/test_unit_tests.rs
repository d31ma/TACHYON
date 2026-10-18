use super::CppYon;
use super::policy_tests::Policies;
use super::role_tests::Roles;
use std::path::Path;

struct Units;

impl Units {
    const UNIT_PATH: &'static str = "server/tests/PriceServiceTest.hpp";
    const UNIT: &'static str = r#"#pragma once
#include <string>
#include <vector>

class [[Test(PriceTestSetup, {"partition":"tests"}), TestSettings({"yon.price.scale":3})]] PriceServiceTest {
public:
    BehaviorContext behaviors;
    [[Fake]] RateHelper rates;

    void testUsesTheFake() {
        rates.value = 5;
        PriceService prices = construct<PriceService>();
        Assert::equals(50, prices.price(10));
    }

    void testReachesAControllerWithTheRealObjects() {
        rates.value = 5;
        PriceController controller = construct<PriceController>();
        Assert::equals(20, controller.getPrice(10));
        Assert::equals(1, tachyon::yon::YonNeeds::of<PriceService>().calls);
    }

    void testStartsEachCaseWithNewObjects() {
        PriceController controller = construct<PriceController>();
        Assert::equals(4, controller.getPrice(2));
        Assert::equals(1, tachyon::yon::YonNeeds::of<PriceService>().calls);
        Assert::equals(std::vector<std::string>{"started"}, tachyon::yon::YonNeeds::of<PriceService>().log);
    }

    void testSeesItsContext() {
        Assert::equals("tests", behaviors.partition());
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
        Assert::equals(3, behaviors.properties<PriceSettings>().scale);
    }

    [[ValueSource([1, 2, 3])]]
    void testIsPositive(int amount) {
        Assert::isTrue(amount > 0);
    }

    void testReportsAFailure() {
        Assert::equals(1, 2, "the price");
    }
};
"#;
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "server/models/PriceSettings.hpp",
            "#pragma once\n\nstruct [[Settings({\"prefix\":\"YON_PRICE_\"})]] PriceSettings {\n    int scale = 1;\n};\n",
        ),
        (
            "server/helpers/RateHelper.hpp",
            "#pragma once\n\nclass [[Helper]] RateHelper {\npublic:\n    int value = 2;\n\n    int rate() const { return value; }\n};\n",
        ),
        (
            "server/services/PriceService.hpp",
            r#"#pragma once
#include <string>
#include <vector>

class [[Service]] PriceService {
    [[Needs]] RateHelper& rates;

public:
    int calls = 0;
    std::vector<std::string> log;

    int price(int amount) {
        calls += 1;
        return amount * rates.rate();
    }

    [[OnStart]]
    void start() { log.push_back("started"); }
};
"#,
        ),
        (
            "server/controllers/PriceController.hpp",
            r#"#pragma once

class [[RequestRoute("/prices")]] [[Public]] PriceController {
    [[Needs]] PriceService& prices;

public:
    [[GetRoute("/{amount}")]]
    int getPrice([[FromPath]] int amount) { return prices.price(amount); }
};
"#,
        ),
        (
            "server/tests/PriceTestSetup.hpp",
            "#pragma once\n\nclass [[TestSetup, UseProfiles(\"test\")]] PriceTestSetup {\npublic:\n    static BehaviorAdapters configure(const BehaviorExecution&) {\n        return BehaviorAdapters();\n    }\n};\n",
        ),
        (Self::UNIT_PATH, Self::UNIT),
    ];

    fn project(changes: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES.iter().chain(changes) {
            Roles::write(root.path(), relative, source);
        }
        root
    }

    const STORED_PATH: &'static str = "server/tests/CatalogServiceTest.hpp";
    const STORED: [(&'static str, &'static str); 5] = [
        (
            "server/models/StockLevel.hpp",
            "#pragma once\n#include <string>\n\nstruct StockLevel {\n    std::string sku;\n    int level = 0;\n};\n",
        ),
        (
            "server/remotes/StockClient.hpp",
            "#pragma once\n#include <string>\n\nclass [[HTTPClient(\"/stock\")]] StockClient {\npublic:\n    [[GetRequest(\"/{sku}\")]]\n    StockLevel level([[PathVariable]] std::string sku) const;\n};\n",
        ),
        (
            "server/setups/StockSetup.hpp",
            "#pragma once\n\nclass [[Setup]] StockSetup {\npublic:\n    static BehaviorAdapters configure(const BehaviorExecution&) {\n        BehaviorAdapters adapters;\n        adapters.http[\"stock\"] = [](const tachyon::http::HttpRequestMessage&) {\n            return tachyon::json::JsonNode::parse(\"{\\\"sku\\\":\\\"real\\\",\\\"level\\\":1}\");\n        };\n        return adapters;\n    }\n};\n",
        ),
        (
            "server/tests/CatalogTestSetup.hpp",
            r#"#pragma once

class [[TestSetup]] CatalogTestSetup {
public:
    static BehaviorAdapters configure() {
        BehaviorAdapters adapters;
        adapters.http["stock"] = [](const tachyon::http::HttpRequestMessage& request) {
            tachyon::json::JsonObject answer;
            answer["sku"] = request.requestUri;
            answer["level"] = 7;
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }
};
"#,
        ),
        (
            Self::STORED_PATH,
            r#"#pragma once

class [[Test(CatalogTestSetup)]] CatalogServiceTest {
public:
    void testCachesAndStoresWithTheAdaptersOfTheSetup() {
        CatalogService& catalog = tachyon::yon::YonNeeds::of<CatalogService>();
        Assert::equals(31, catalog.price("tea", "a"));
        Assert::equals(31, catalog.price("tea", "b"));
        Assert::equals(1, catalog.reads);
        catalog.add(ProductEntity{"one", "active", 3});
        ProductStore& products = tachyon::yon::YonNeeds::of<ProductStore>();
        Assert::equals(1, static_cast<int>(products.findByStatus("active").size()));
    }

    void testStartsWithAnEmptyCacheAndNoRows() {
        Assert::equals(31, tachyon::yon::YonNeeds::of<CatalogService>().price("tea", "c"));
        Assert::equals(0, static_cast<int>(tachyon::yon::YonNeeds::of<ProductStore>().findAll().size()));
    }

    void testAnswersHttpClientsWithTheTestSetup() {
        StockLevel level = tachyon::yon::YonNeeds::of<StockClient>().level("tea");
        Assert::equals(7, level.level);
        Assert::equals("/stock/tea", level.sku);
    }
};
"#,
        ),
    ];

    fn stored() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Policies::FILES.iter().chain(&Self::STORED) {
            Roles::write(root.path(), relative, source);
        }
        root
    }

    fn lower(root: &Path) -> Result<crate::yon::Lowered, crate::Failure> {
        let unit = std::fs::read_to_string(root.join(Self::UNIT_PATH)).expect("unit");
        CppYon::lower(Path::new(Self::UNIT_PATH), &unit, root)
    }

    fn refusal(changes: &[(&str, &str)]) -> String {
        let root = Self::project(changes);
        Self::lower(root.path())
            .err()
            .expect("the unit is refused")
            .to_string()
    }

    fn report(root: &Path) -> serde_json::Value {
        let lowered = Self::lower(root).expect("lowered");
        assert!(lowered.routes.is_empty());
        let (answer, errors) = Roles::run(&lowered.source, r#"{"method":"POST","route":"/"}"#);
        assert_eq!(answer["status"], 200, "{answer} {errors}");
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("report")
    }
}

#[test]
fn a_test_unit_runs_each_case_with_its_fakes_context_and_new_objects() {
    if !Roles::compiler_available() {
        return;
    }
    let root = Units::project(&[]);
    let report = Units::report(root.path());
    assert_eq!(report["unit"], "PriceServiceTest");
    let outcomes: Vec<(String, bool)> = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| {
            (
                case["name"].as_str().expect("name").to_owned(),
                case["passed"] == true,
            )
        })
        .collect();
    let passed = |name: &str| (String::from(name), true);
    assert_eq!(
        outcomes,
        [
            passed("testUsesTheFake"),
            passed("testReachesAControllerWithTheRealObjects"),
            passed("testStartsEachCaseWithNewObjects"),
            passed("testSeesItsContext"),
            passed("testIsPositive[1]"),
            passed("testIsPositive[2]"),
            passed("testIsPositive[3]"),
            (String::from("testReportsAFailure"), false),
        ],
        "{report}"
    );
    assert_eq!(
        report["cases"][7]["error"],
        "the price ==> expected: <1> but was: <2>"
    );
    assert_eq!(
        report["cases"][1]["called"],
        serde_json::json!([
            "PriceService.start",
            "PriceController.getPrice",
            "PriceService.price",
            "RateHelper.rate"
        ])
    );
}

#[test]
fn a_test_unit_builds_with_the_error_handlers_and_error_types_of_the_project() {
    if !Roles::compiler_available() {
        return;
    }
    let root = Units::project(&[
        (
            "server/errors/PriceMissing.hpp",
            "#pragma once\n#include <stdexcept>\n\nclass PriceMissing : public std::runtime_error {\npublic:\n    using std::runtime_error::runtime_error;\n};\n",
        ),
        (
            "server/controllers/PriceErrors.hpp",
            "#pragma once\n#include <string>\n\nclass [[ErrorHandler]] PriceErrors {\npublic:\n    [[HandlesError]] [[ResponseStatus(404)]]\n    std::string missing(const PriceMissing& error) { return error.what(); }\n};\n",
        ),
        (
            Units::UNIT_PATH,
            "class [[Test(PriceTestSetup)]] PriceServiceTest {\npublic:\n    void testAnswersAMissingPrice() {\n        PriceErrors errors = construct<PriceErrors>();\n        Assert::equals(\"gone\", errors.missing(PriceMissing(\"gone\")));\n    }\n};\n",
        ),
    ]);
    let report = Units::report(root.path());
    assert_eq!(report["cases"][0]["passed"], true, "{report}");
    assert_eq!(
        report["cases"][0]["called"],
        serde_json::json!(["PriceErrors.missing"])
    );
}

#[test]
fn a_test_unit_caches_stores_and_answers_http_clients() {
    if !Roles::compiler_available() {
        return;
    }
    let root = Units::stored();
    let source = std::fs::read_to_string(root.path().join(Units::STORED_PATH)).expect("unit");
    let lowered =
        CppYon::lower(Path::new(Units::STORED_PATH), &source, root.path()).expect("lowered");
    let (answer, errors) = Roles::run(&lowered.source, r#"{"method":"POST","route":"/"}"#);
    assert_eq!(answer["status"], 200, "{answer} {errors}");
    let report: serde_json::Value =
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("report");
    let passed: Vec<bool> = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| case["passed"] == true)
        .collect();
    assert_eq!(passed, [true, true, true], "{report}");
}

#[test]
fn a_test_unit_is_checked_before_compiling() {
    let cases = [
        (
            "class [[Test(MissingTestSetup)]] PriceServiceTest {};\n",
            "server/tests/TallyTestSetup.hpp declares class [[TestSetup]] TallyTestSetup",
        ),
        (
            "class [[Test(PriceTestSetup)]] PriceServiceTest {\n    [[Fake]] Unknown unknown;\n};\n",
            "server/stores, server/remotes or server/controllers",
        ),
        (
            "class [[Test(PriceTestSetup)]] PriceServiceTest {\n    [[Fake]] RateHelper& rates;\n};\n",
            "no reference, no pointer, no static",
        ),
        (
            "class [[Test(PriceTestSetup), TestSettings({\"tac.price.scale\":3})]] PriceServiceTest {};\n",
            "yon.",
        ),
        (
            "class PriceServiceTest {};\n",
            "one class marked [[Test(<Name>TestSetup)]]",
        ),
    ];
    for (unit, expected) in cases {
        let refusal = Units::refusal(&[(Units::UNIT_PATH, unit)]);
        assert!(refusal.contains(expected), "{unit}: {refusal}");
    }
}

#[test]
fn the_handler_check_reads_a_test_unit_instead_of_a_controller() {
    let check = |source: &str| {
        CppYon::either(Path::new(Units::UNIT_PATH), source, |_, _| Ok(()))
            .err()
            .map(|failure| failure.to_string())
    };
    assert_eq!(check(Units::UNIT), None);
    let refusal = check("class [[RequestRoute(\"/x\")]] PriceServiceTest {};\n").expect("refused");
    assert!(refusal.contains("[[Test(<Name>TestSetup)]]"), "{refusal}");
}
