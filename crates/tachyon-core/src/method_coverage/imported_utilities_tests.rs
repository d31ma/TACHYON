use super::{CoverageReport, ImportedUtilities, RecordedCalls};
use std::path::{Path, PathBuf};

struct ImportedUtilityProject;

impl ImportedUtilityProject {
    const UTILITY: &'static str =
        "export class NAMEUtils {\n  static label(cents) {\n    return cents\n  }\n}\n";

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, contents).expect("source");
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for name in ["Price", "Title", "Count", "Meta"] {
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
fn explicitly_imported_and_module_client_utilities_are_not_recorded() {
    let root = ImportedUtilityProject::project();
    let unrecorded = ImportedUtilities::unrecorded(root.path());
    for name in ["Price", "Count", "Meta"] {
        let path = PathBuf::from(format!("client/shared/utils/nested/{name}Utils.js"));
        assert!(unrecorded.contains(&path), "{name}: {unrecorded:?}");
    }
    let title = PathBuf::from("client/shared/utils/nested/TitleUtils.js");
    assert!(!unrecorded.contains(&title), "{unrecorded:?}");
}

#[test]
fn coverage_lists_only_the_utilities_whose_calls_it_can_record() {
    let root = ImportedUtilityProject::project();
    let report = CoverageReport::build(root.path(), RecordedCalls::default()).expect("coverage");
    let types: Vec<String> = report
        .uncovered()
        .into_iter()
        .map(|found| found.type_name)
        .collect();
    assert_eq!(types, vec![String::from("TitleUtils")], "{report:?}");
}
