use super::{FsharpMarkers, FsharpSource, FsharpView};
use crate::Failure;
use crate::behavior::TestAssert;
use crate::yon::Lowered;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct FsharpTestUnit;

impl FsharpTestUnit {
    const DRIVER_START: &'static str = "\nsealed class Handler\n{\n    public static object POST()\n    {\n        var cases = new List<object>();\n";
    const DRIVER_END: &'static str = "        return new Dictionary<string, object> { [\"unit\"] = __UNIT__, [\"cases\"] = cases };\n    }\n\n    static void Run(List<object> cases, string name, Action test)\n    {\n        try\n        {\n            test();\n            cases.Add(new Dictionary<string, object> { [\"name\"] = name, [\"passed\"] = true, [\"called\"] = Array.Empty<string>() });\n        }\n        catch (Exception error)\n        {\n            cases.Add(new Dictionary<string, object> { [\"name\"] = name, [\"passed\"] = false, [\"error\"] = error.Message, [\"called\"] = Array.Empty<string>() });\n        }\n    }\n}\n";

    pub(crate) fn lower(
        source: &Path,
        contents: &str,
        view: &FsharpView,
        environment: &str,
    ) -> Result<Option<Lowered>, Failure> {
        let unit = crate::behavior::test_configuration_name(view.path(), view.text(), environment)
            .map_err(|failure| view.failure(&failure))?;
        unit.map(|_| Self::program(source, contents, view, environment))
            .transpose()
    }

    fn program(
        source: &Path,
        contents: &str,
        view: &FsharpView,
        environment: &str,
    ) -> Result<Lowered, Failure> {
        let name = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let calls = crate::behavior::csharp_case_calls(view.path(), view.text(), environment)
            .map_err(|failure| view.failure(&failure))?;
        let driver = format!("{}{}", view.usings(), Self::driver(name, &calls));
        let fsharp = format!("{}{}", FsharpMarkers::strip(contents), TestAssert::FSHARP);
        Ok(Lowered {
            source: FsharpSource::with_generated(&fsharp, &driver),
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        })
    }

    fn driver(name: &str, calls: &[(String, String)]) -> String {
        let mut driver = String::from(Self::DRIVER_START);
        for (case, call) in calls {
            let _ = writeln!(
                driver,
                "        Run(cases, {}, () => new {name}().{call});",
                Self::quoted(case)
            );
        }
        driver.push_str(&Self::DRIVER_END.replace("__UNIT__", &Self::quoted(name)));
        driver
    }

    fn quoted(text: &str) -> String {
        serde_json::Value::String(text.to_owned()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{FsharpTestUnit, FsharpView};
    use std::path::Path;

    #[test]
    fn every_case_and_input_becomes_one_run_of_the_driver() {
        let calls = [
            (String::from("testDoubles"), String::from("testDoubles()")),
            (String::from("testKeeps[1]"), String::from("testKeeps(5)")),
        ];
        let driver = FsharpTestUnit::driver("CatalogServiceTest", &calls);
        assert!(driver.contains(
            "Run(cases, \"testDoubles\", () => new CatalogServiceTest().testDoubles());"
        ));
        assert!(driver.contains(
            "Run(cases, \"testKeeps[1]\", () => new CatalogServiceTest().testKeeps(5));"
        ));
        assert!(driver.contains("[\"unit\"] = \"CatalogServiceTest\""));
    }

    #[test]
    fn a_unit_lists_its_value_source_inputs_and_gets_assert() {
        let source = "[<Test(typeof<CatalogTestSetup>)>]\ntype CatalogServiceTest() =\n    member this.Twice(value: int) : int =\n        value * 2\n\n    member this.testDoublesAValue() : unit =\n        Assert.equals(8, this.Twice(4))\n\n    [<ValueSource([1, \"two\"])>]\n    member this.testKeepsAnInput(value: obj) : unit =\n        Assert.isTrue(not (isNull value))\n";
        let path = Path::new("server/tests/CatalogServiceTest.fs");
        let view = FsharpView::new(path, source);
        let lowered = FsharpTestUnit::lower(path, source, &view, "yon")
            .unwrap_or_else(|failure| panic!("{failure:?}"))
            .unwrap_or_else(|| panic!("a test unit"));
        let (fsharp, csharp) = crate::fsharp_source::FsharpSource::split(&lowered.source);
        assert!(fsharp.contains("\ntype Assert =\n"), "{fsharp}");
        for run in [
            "Run(cases, \"testDoublesAValue\", () => new CatalogServiceTest().testDoublesAValue());",
            "Run(cases, \"testKeepsAnInput[1]\", () => new CatalogServiceTest().testKeepsAnInput(1));",
            "Run(cases, \"testKeepsAnInput[2]\", () => new CatalogServiceTest().testKeepsAnInput(\"two\"));",
        ] {
            assert!(csharp.contains(run), "{run} missing from {csharp}");
        }
        assert!(!csharp.contains("Twice"), "{csharp}");
    }
}
