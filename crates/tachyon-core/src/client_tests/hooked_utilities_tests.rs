#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;

struct ImportedUtilityUnit;

impl ImportedUtilityUnit {
    const RUNTIMES: [&'static str; 3] = ["node", "bun", "deno"];
    const EXTENSIONS: [&'static str; 2] = ["js", "ts"];
    const UTILITY: &'static str = "export class NAMEUtils {\n  static label(cents) {\n    return `${cents}`\n  }\n\n  static unused() {\n    return 0\n  }\n}\n";
    const SETUP: &'static str =
        "@TestSetup\nclass BasketTestSetup {\n  static configure() { return {}; }\n}\n";
    const SERVICE: &'static str = "import { PriceUtils } from '/shared/utils/PriceUtils.EXT';\nimport { CountUtils } from '../shared/utils/nested/CountUtils.EXT';\n\n@Service\nexport default class BasketService {\n  total() {\n    return PriceUtils.label(2) + CountUtils.label(3)\n  }\n}\n";
    const UNIT: &'static str = "@Test(BasketTestSetup, {\"partition\":\"tests\"})\nexport default class BasketServiceTest {\n  async testTotal() {\n    Assert.equals('23', this.construct(BasketService).total(), 'priced')\n  }\n}\n";
    const RELATIVE: &'static str = "client/tests/BasketServiceTest.EXT";

    fn available(program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn write(root: &Path, relative: &str, source: &str, extension: &str) {
        let path = root.join(relative.replace("EXT", extension));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source.replace("EXT", extension)).expect("write");
    }

    fn project(extension: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let price = Self::UTILITY.replace("NAME", "Price");
        let count = Self::UTILITY.replace("NAME", "Count");
        let files = [
            ("client/shared/utils/PriceUtils.EXT", price.as_str()),
            ("client/shared/utils/nested/CountUtils.EXT", count.as_str()),
            ("client/tests/BasketTestSetup.EXT", Self::SETUP),
            ("client/services/BasketService.EXT", Self::SERVICE),
            (Self::RELATIVE, Self::UNIT),
        ];
        for (relative, source) in files {
            Self::write(root.path(), relative, source, extension);
        }
        root
    }

    fn called(runtime: &str, extension: &str) -> Vec<String> {
        let root = Self::project(extension);
        let relative = Self::RELATIVE.replace("EXT", extension);
        let report = super::run_with(root.path(), Path::new(&relative), Some(Path::new(runtime)))
            .expect("the unit runs");
        let case = report.cases.first().expect("one case");
        let failure = case.error.clone().unwrap_or_default();
        assert!(case.passed, "{runtime} {extension}: {failure}");
        case.called.clone()
    }
}

#[test]
fn a_script_unit_records_the_calls_of_imported_shared_utilities() {
    for runtime in ImportedUtilityUnit::RUNTIMES {
        if !ImportedUtilityUnit::available(runtime) {
            eprintln!("skipping {runtime}: it is unavailable");
            continue;
        }
        for extension in ImportedUtilityUnit::EXTENSIONS {
            let called = ImportedUtilityUnit::called(runtime, extension);
            for key in [
                "BasketService.total",
                "PriceUtils.label",
                "CountUtils.label",
            ] {
                let label = format!("{runtime} {extension} {key}");
                assert!(called.contains(&String::from(key)), "{label}: {called:?}");
            }
            let unused = String::from("PriceUtils.unused");
            assert!(
                !called.contains(&unused),
                "{runtime} {extension}: {called:?}"
            );
        }
    }
}
