use super::MigrationFix;
use super::visibility_off::VisibilityOff;
use std::fs;
use std::path::Path;

struct SharedUtilsProject;

impl SharedUtilsProject {
    const UTILITY: &'static str =
        "export class TextUtils {\n  static shout(text) {\n    return `${text}!`\n  }\n}\n";
    const RUST_UTILITY: &'static str = "pub struct TitleUtils;\nimpl TitleUtils {\n    pub fn title() -> String { String::from(\"Ready\") }\n}\n";
    const RUST_SERVICE: &'static str = "use crate::utils::TitleUtils;\n#[Service]\npub struct TitleService;\nimpl TitleService {\n    pub fn title(&self) -> String { TitleUtils::title() }\n}\n";

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    fn read(root: &Path, relative: &str) -> String {
        fs::read_to_string(root.join(relative)).expect("source")
    }

    fn migrated(files: &[(&str, &str)]) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        VisibilityOff::write(project.path());
        for (relative, contents) in files {
            Self::write(project.path(), relative, contents);
        }
        MigrationFix::plan(project.path())
            .expect("plan")
            .write(project.path())
            .expect("write");
        project
    }
}

#[test]
fn every_file_in_client_utils_moves_to_client_shared_utils_with_its_nested_folders() {
    let extensions = [
        "js", "ts", "dart", "rs", "go", "rb", "swift", "kt", "java", "cs", "fs", "lua", "py",
    ];
    let files: Vec<(String, &str)> = extensions
        .iter()
        .map(|extension| (format!("client/utils/text/TextUtils.{extension}"), "kept\n"))
        .collect();
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(relative, contents)| (relative.as_str(), *contents))
        .collect();
    let project = SharedUtilsProject::migrated(&borrowed);
    for extension in extensions {
        let moved = format!("client/shared/utils/text/TextUtils.{extension}");
        assert_eq!(SharedUtilsProject::read(project.path(), &moved), "kept\n");
        let old = format!("client/utils/text/TextUtils.{extension}");
        assert!(!project.path().join(old).exists());
    }
}

#[test]
fn imports_of_a_moved_utility_point_at_client_shared_utils() {
    let project = SharedUtilsProject::migrated(&[
        ("client/utils/TextUtils.js", SharedUtilsProject::UTILITY),
        (
            "client/utils/text/TitleUtils.js",
            "import { TextUtils } from '../TextUtils.js';\nimport Item from '../../models/Item.js';\nexport class TitleUtils {\n  static title() {\n    return TextUtils.shout('x')\n  }\n}\n",
        ),
        (
            "client/services/BasketService.js",
            "import { TextUtils } from '../utils/TextUtils.js';\nimport { TitleUtils } from \"/utils/text/TitleUtils.js\";\nconst where = 'client/utils/TextUtils.js';\n@Service\nexport default class BasketService {}\n",
        ),
        (
            "client/components/basket/BasketDelegate.ts",
            "import { TextUtils } from '../../utils/TextUtils.js';\nimport { Other } from '../utils/Other.js';\n",
        ),
        (
            "server/services/OrderService.js",
            "import { PriceUtils } from '../utils/PriceUtils.js';\n@Service\nexport default class OrderService {}\n",
        ),
        ("server/utils/PriceUtils.js", "export class PriceUtils {}\n"),
    ]);
    let root = project.path();
    let service = SharedUtilsProject::read(root, "client/services/BasketService.js");
    assert!(
        service.contains("from '../shared/utils/TextUtils.js'"),
        "{service}"
    );
    assert!(
        service.contains("from \"/shared/utils/text/TitleUtils.js\""),
        "{service}"
    );
    assert!(
        service.contains("'client/shared/utils/TextUtils.js'"),
        "{service}"
    );
    let helper = SharedUtilsProject::read(root, "client/components/basket/BasketDelegate.ts");
    assert!(
        helper.contains("from '../../shared/utils/TextUtils.js'"),
        "{helper}"
    );
    assert!(helper.contains("from '../utils/Other.js'"), "{helper}");
    let title = SharedUtilsProject::read(root, "client/shared/utils/text/TitleUtils.js");
    assert!(title.contains("from '../TextUtils.js'"), "{title}");
    assert!(title.contains("from '../../../models/Item.js'"), "{title}");
    let order = SharedUtilsProject::read(root, "server/services/OrderService.js");
    assert!(order.contains("from '../utils/PriceUtils.js'"), "{order}");
    assert!(root.join("server/utils/PriceUtils.js").exists());
}

#[test]
fn a_native_rust_utility_moves_and_its_crate_utils_path_stays() {
    let project = SharedUtilsProject::migrated(&[
        (
            "client/utils/nested/TitleUtils.rs",
            SharedUtilsProject::RUST_UTILITY,
        ),
        (
            "client/services/TitleService.rs",
            SharedUtilsProject::RUST_SERVICE,
        ),
    ]);
    let root = project.path();
    assert_eq!(
        SharedUtilsProject::read(root, "client/shared/utils/nested/TitleUtils.rs"),
        SharedUtilsProject::RUST_UTILITY
    );
    assert_eq!(
        SharedUtilsProject::read(root, "client/services/TitleService.rs"),
        SharedUtilsProject::RUST_SERVICE
    );
    assert!(!root.join("client/utils/nested/TitleUtils.rs").exists());
}

#[test]
fn a_taken_shared_utils_destination_is_a_manual_step() {
    let project = tempfile::tempdir().expect("project");
    VisibilityOff::write(project.path());
    let root = project.path();
    SharedUtilsProject::write(root, "client/utils/TextUtils.js", "old\n");
    SharedUtilsProject::write(root, "client/shared/utils/TextUtils.js", "new\n");
    let planned = MigrationFix::plan(root).expect("plan");
    assert!(planned.to_text(false).contains("already exists"));
    planned.write(root).expect("write");
    assert_eq!(
        SharedUtilsProject::read(root, "client/utils/TextUtils.js"),
        "old\n"
    );
    assert_eq!(
        SharedUtilsProject::read(root, "client/shared/utils/TextUtils.js"),
        "new\n"
    );
}
