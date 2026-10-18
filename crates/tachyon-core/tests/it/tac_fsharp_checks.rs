#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::Path;

struct CheckedOrders;

impl CheckedOrders {
    const UNIT: &'static str = "client/tests/OrderServiceTest.fs";

    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/OrderInput.fs",
            "[<Checked>]\ntype OrderInput() =\n    [<NotEmpty>]\n    [<Length({\"min\":2,\"max\":8})>]\n    member val name : BehaviorJSON = BehaviorJSON.Parse(\"null\") with get, set\n\n    [<Required>]\n    [<Min(1)>]\n    [<Max(9)>]\n    member val level : BehaviorJSON = BehaviorJSON.Parse(\"null\") with get, set\n",
        ),
        (
            "client/services/OrderService.fs",
            "[<Service>]\ntype OrderService() =\n    member this.accept([<Check(typeof<OrderInput>)>] order: BehaviorJSON) : BehaviorJSON = order\n",
        ),
        (
            "client/tests/OrderTestSetup.fs",
            "[<TestSetup>]\ntype OrderTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n",
        ),
    ];

    const UNIT_START: &'static str = "[<Test(typeof<OrderTestSetup>, {\"partition\":\"tests\"})>]\ntype OrderServiceTest() =\n    member this.outcome(input: string) : string =\n        try\n            let order = BehaviorJSON.Parse(input)\n            let answered = OrderService().accept(order)\n            if answered.ToString() = order.ToString() then \"accepted\" else answered.ToString()\n        with :? BehaviorError as error -> error.Code + \":\" + error.Path\n";

    const ACCEPTED: &'static str = "accepted";

    const CASES: [(&'static str, &'static str, &'static str); 16] = [
        (
            "testEveryRuleHolds",
            r#"{"name":"ab","level":3}"#,
            Self::ACCEPTED,
        ),
        (
            "testLengthAcceptsTheMaximum",
            r#"{"name":"abcdefgh","level":5}"#,
            Self::ACCEPTED,
        ),
        (
            "testMinAcceptsTheBound",
            r#"{"name":"order","level":1}"#,
            Self::ACCEPTED,
        ),
        (
            "testMaxAcceptsTheBound",
            r#"{"name":"order","level":9}"#,
            Self::ACCEPTED,
        ),
        (
            "testUnknownFieldsStay",
            r#"{"name":"order","level":2,"sku":"A-1"}"#,
            Self::ACCEPTED,
        ),
        (
            "testCheckedRefusesAList",
            r#"[{"name":"order","level":2}]"#,
            "validation:$.object",
        ),
        (
            "testRequiredRefusesAMissingValue",
            r#"{"name":"order"}"#,
            "validation:level.notNull",
        ),
        (
            "testRequiredRefusesNull",
            r#"{"name":"order","level":null}"#,
            "validation:level.notNull",
        ),
        (
            "testNotEmptyRefusesAMissingValue",
            r#"{"level":3}"#,
            "validation:name.notBlank",
        ),
        (
            "testNotEmptyRefusesAnEmptyText",
            r#"{"name":"","level":3}"#,
            "validation:name.notBlank",
        ),
        (
            "testNotEmptyRefusesBlankText",
            r#"{"name":"   ","level":3}"#,
            "validation:name.notBlank",
        ),
        (
            "testLengthRefusesAShortText",
            r#"{"name":"a","level":3}"#,
            "validation:name.size",
        ),
        (
            "testLengthRefusesALongText",
            r#"{"name":"abcdefghi","level":3}"#,
            "validation:name.size",
        ),
        (
            "testMinRefusesASmallerNumber",
            r#"{"name":"order","level":0}"#,
            "validation:level.min",
        ),
        (
            "testMaxRefusesALargerNumber",
            r#"{"name":"order","level":10}"#,
            "validation:level.max",
        ),
        (
            "testMinRefusesAText",
            r#"{"name":"order","level":"3"}"#,
            "validation:level.min",
        ),
    ];

    fn dotnet_available() -> bool {
        let found = std::process::Command::new("dotnet")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !found {
            eprintln!("skipping F# Tac checks: dotnet is unavailable");
        }
        found
    }

    fn quoted(text: &str) -> String {
        serde_json::Value::String(text.to_owned()).to_string()
    }

    fn unit() -> String {
        let mut unit = String::from(Self::UNIT_START);
        for (name, input, expected) in Self::CASES {
            let _ = write!(
                unit,
                "\n    member this.{name}() : unit =\n        Assert.equals({}, this.outcome({}))\n",
                Self::quoted(expected),
                Self::quoted(input)
            );
        }
        unit
    }

    fn write(root: &Path) {
        let unit = Self::unit();
        let files = Self::FILES
            .iter()
            .copied()
            .chain([(Self::UNIT, unit.as_str())]);
        for (relative, source) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            std::fs::write(path, source).expect("source");
        }
    }
}

#[test]
fn a_tac_fsharp_service_checks_its_input_against_a_checked_model() {
    if !CheckedOrders::dotnet_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CheckedOrders::write(root.path());
    let report = tachyon_core::client_tests::run(root.path(), Path::new(CheckedOrders::UNIT))
        .unwrap_or_else(|failure| panic!("F# Tac unit: {failure:?}"));
    assert_eq!(report.unit, "OrderServiceTest");
    let names: Vec<&str> = report.cases.iter().map(|case| case.name.as_str()).collect();
    let expected: Vec<&str> = CheckedOrders::CASES
        .iter()
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(names, expected);
    let failed: Vec<_> = report.cases.iter().filter(|case| !case.passed).collect();
    assert!(failed.is_empty(), "{failed:?}");
}
