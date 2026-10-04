use super::{CoverageReport, RecordedCalls};
use std::path::Path;

struct SharedUtilityProject;

impl SharedUtilityProject {
    const UTILITY: &'static str =
        "export class NAMEUtils {\n  static label(cents) {\n    return cents\n  }\n}\n";

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, contents).expect("source");
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for name in ["Price", "Title", "Count"] {
            Self::write(
                root.path(),
                &format!("client/shared/utils/nested/{name}Utils.js"),
                &Self::UTILITY.replace("NAME", name),
            );
        }
        Self::write(
            root.path(),
            "client/shared/utils/nested/MetaUtils.js",
            "export class MetaUtils {\n  static here() {\n    return import.meta.url\n  }\n}\n",
        );
        Self::write(
            root.path(),
            "client/services/BasketService.js",
            "import { PriceUtils } from '/shared/utils/nested/PriceUtils.js';\nimport { CountUtils } from '../shared/utils/nested/CountUtils.js';\n@Service\nexport default class BasketService {}\n",
        );
        root
    }
}

#[test]
fn coverage_lists_imported_module_and_inline_client_utilities() {
    let root = SharedUtilityProject::project();
    let report = CoverageReport::build(root.path(), RecordedCalls::default()).expect("coverage");
    let mut types: Vec<String> = report
        .uncovered()
        .into_iter()
        .map(|found| found.type_name)
        .collect();
    types.sort();
    let expected = ["CountUtils", "MetaUtils", "PriceUtils", "TitleUtils"].map(String::from);
    assert_eq!(types, expected, "{report:?}");
}
