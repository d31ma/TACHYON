use super::{FsharpMarkers, FsharpSource, FsharpView};
use crate::Failure;
use crate::yon::Lowered;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct FsharpTestUnit;

impl FsharpTestUnit {
    const CASE_PREFIX: &'static str = "test";
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
        Ok(unit.map(|_| Self::program(source, contents, view)))
    }

    fn program(source: &Path, contents: &str, view: &FsharpView) -> Lowered {
        let name = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let driver = format!(
            "{}{}",
            view.usings(),
            Self::driver(name, &Self::cases(view.text()))
        );
        Lowered {
            source: FsharpSource::with_generated(&FsharpMarkers::strip(contents), &driver),
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        }
    }

    fn cases(shadow: &str) -> Vec<String> {
        shadow.lines().filter_map(Self::case_name).collect()
    }

    fn case_name(line: &str) -> Option<String> {
        let declaration = line.trim().strip_prefix("public ")?;
        let (before, _) = declaration.split_once('(')?;
        let name = before.rsplit(' ').next()?;
        name.starts_with(Self::CASE_PREFIX).then(|| name.to_owned())
    }

    fn driver(name: &str, cases: &[String]) -> String {
        let quoted = serde_json::Value::String(name.to_owned()).to_string();
        let mut driver = String::from(Self::DRIVER_START);
        for case in cases {
            let _ = writeln!(
                driver,
                "        Run(cases, \"{case}\", () => new {name}().{case}());"
            );
        }
        driver.push_str(&Self::DRIVER_END.replace("__UNIT__", &quoted));
        driver
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpTestUnit;

    #[test]
    fn every_test_member_becomes_one_case_of_the_driver() {
        let shadow = "[Test(typeof(CatalogTestSetup))]\npublic class CatalogServiceTest {\n    public void testDoubles() {  ()\n    public int twice(int value) { value * 2 } }\n";
        let cases = FsharpTestUnit::cases(shadow);
        assert_eq!(cases, vec![String::from("testDoubles")]);
        let driver = FsharpTestUnit::driver("CatalogServiceTest", &cases);
        assert!(driver.contains(
            "Run(cases, \"testDoubles\", () => new CatalogServiceTest().testDoubles());"
        ));
        assert!(driver.contains("[\"unit\"] = \"CatalogServiceTest\""));
    }
}
