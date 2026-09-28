use super::renames::FileMove;
use super::{MigrationFix, MigrationFixPlan};
use crate::behavior::RenamedAnnotation;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct Language;

impl Language {
    const SPELLINGS: [(&str, &str, &str, &str); 11] = [
        ("js", "@", "", "class PlainService {}\n"),
        ("ts", "@", "", "class PlainService {}\n"),
        ("py", "@", "", "class PlainService:\n    pass\n"),
        ("java", "@", "", "class PlainService {}\n"),
        ("kt", "@", "", "class PlainService\n"),
        ("dart", "@", "", "class PlainService {}\n"),
        ("swift", "@", "", "final class PlainService {}\n"),
        ("cs", "[", "]", "public class PlainService {}\n"),
        ("php", "#[", "]", "class PlainService {}\n"),
        ("rs", "#[", "]", "struct PlainService;\n"),
        ("go", "#[", "]", "type PlainService struct{}\n"),
    ];

    fn every_old_marker(prefix: &str, suffix: &str, declaration: &str) -> String {
        let mut source = String::new();
        for (old, _) in RenamedAnnotation::NAMES {
            source.push_str(&[prefix, old, suffix, "\n"].concat());
        }
        source.push_str(declaration);
        source
    }

    fn every_new_marker(prefix: &str, suffix: &str, declaration: &str) -> String {
        let (prefix, suffix) = if declaration.starts_with("type ") {
            ("// @", "")
        } else {
            (prefix, suffix)
        };
        let mut source = String::new();
        for (_, new) in RenamedAnnotation::NAMES {
            source.push_str(&[prefix, new, suffix, "\n"].concat());
        }
        source.push_str(declaration);
        source
    }
}

fn project() -> TempDir {
    tempfile::tempdir().expect("project")
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("source");
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).expect("source")
}

fn plan(root: &Path) -> MigrationFixPlan {
    MigrationFix::plan(root).expect("plan")
}

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(root).expect("inside the project");
            files.insert(
                relative.to_string_lossy().replace('\\', "/"),
                fs::read(&path).expect("file"),
            );
        }
    }
    files
}

fn write_role_project(root: &Path) {
    write(
        root,
        "client/repositories/BasketRepository.js",
        "@Repository\nexport default class BasketRepository {\n  items = []\n}\n",
    );
    write(
        root,
        "client/configurations/ShopConfiguration.js",
        "@Configuration\nexport default class ShopConfiguration {\n  static configure() {\n    return {}\n  }\n}\n",
    );
    write(
        root,
        "client/services/BasketService.js",
        "@Service\nexport default class BasketService {\n  @Autowired\n  basketRepository\n\n  @PostConstruct\n  start() {\n    this.basketRepository.items.push(1)\n  }\n}\n",
    );
    write(
        root,
        "client/tests/ShopTestConfiguration.js",
        "@TestConfiguration\nexport default class ShopTestConfiguration {\n  static configure() {\n    return {}\n  }\n}\n",
    );
    write(
        root,
        "client/tests/BasketServiceTest.js",
        "@TachyonTest({\"configuration\":\"ShopTestConfiguration\"})\nexport default class BasketServiceTest {\n  async testStarts() {}\n}\n",
    );
    write(root, "client/repositories/notes.txt", "kept as it is\n");
}

#[test]
fn every_old_annotation_is_renamed_in_every_language() {
    let project = project();
    for (extension, prefix, suffix, declaration) in Language::SPELLINGS {
        let relative = format!("server/services/PlainService.{extension}");
        write(
            project.path(),
            &relative,
            &Language::every_old_marker(prefix, suffix, declaration),
        );
    }
    let planned = plan(project.path());
    assert_eq!(
        planned.change_count(),
        Language::SPELLINGS.len(),
        "{}",
        planned.to_text(false)
    );
    assert!(
        planned
            .to_text(false)
            .contains("Renamed 'Autowired' to 'Needs' on line 1 (ADR 0047).")
    );
    planned.write(project.path()).expect("write");
    for (extension, prefix, suffix, declaration) in Language::SPELLINGS {
        let relative = format!("server/services/PlainService.{extension}");
        assert_eq!(
            read(project.path(), &relative),
            Language::every_new_marker(prefix, suffix, declaration),
            "{extension}"
        );
    }
    let again = plan(project.path());
    assert_eq!(again.change_count(), 0, "{}", again.to_text(false));
}

#[test]
fn snake_case_and_namespaced_markers_keep_their_spelling() {
    let project = project();
    write(
        project.path(),
        "server/services/PlainService.py",
        "@service\n@cache_config({\"name\":\"plain\"})\nclass PlainService:\n    @post_construct\n    def start(self):\n        pass\n\n    @property\n    def value(self):\n        return 1\n\n    @value.setter\n    def value(self, item):\n        pass\n",
    );
    write(
        project.path(),
        "server/services/PlainService.rs",
        "#[yon::service]\n#[yon::required_args_constructor]\nstruct PlainService;\n\nimpl PlainService {\n    #[test]\n    fn works() {}\n}\n",
    );
    plan(project.path()).write(project.path()).expect("write");
    let python = read(project.path(), "server/services/PlainService.py");
    assert!(
        python.contains("@cache_settings({\"name\":\"plain\"})"),
        "{python}"
    );
    assert!(python.contains("    @on_start\n"), "{python}");
    assert!(python.contains("@value.setter"), "{python}");
    let rust = read(project.path(), "server/services/PlainService.rs");
    assert!(rust.contains("#[yon::needs_all]"), "{rust}");
    assert!(rust.contains("    #[test]\n"), "{rust}");
}

#[test]
fn old_names_in_comments_strings_and_other_decorators_stay() {
    let project = project();
    let source = "@Service\nexport default class PlainService {\n  // @Autowired\n  label = '@Scope'\n  note = `\n@Value\n`\n}\n";
    write(project.path(), "client/services/PlainService.js", source);
    let planned = plan(project.path());
    assert_eq!(planned.change_count(), 0, "{}", planned.to_text(false));
}

#[test]
fn role_folders_move_and_their_types_are_renamed_everywhere() {
    let project = project();
    write_role_project(project.path());
    let planned = plan(project.path());
    let text = planned.to_text(false);
    assert!(
        text.contains(
            "move     client/repositories/BasketRepository.js -> client/stores/BasketStore.js"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "move     client/configurations/ShopConfiguration.js -> client/setups/ShopSetup.js"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "move     client/tests/ShopTestConfiguration.js -> client/tests/ShopTestSetup.js"
        ),
        "{text}"
    );
    assert!(
        text.contains("move     client/repositories/notes.txt -> client/stores/notes.txt"),
        "{text}"
    );
    planned.write(project.path()).expect("write");
    assert_eq!(
        read(project.path(), "client/stores/BasketStore.js"),
        "@Store\nexport default class BasketStore {\n  items = []\n}\n"
    );
    assert_eq!(
        read(project.path(), "client/services/BasketService.js"),
        "@Service\nexport default class BasketService {\n  @Needs\n  basketStore\n\n  @OnStart\n  start() {\n    this.basketStore.items.push(1)\n  }\n}\n"
    );
    assert!(
        read(project.path(), "client/tests/BasketServiceTest.js")
            .starts_with("@Test(ShopTestSetup)\n")
    );
    assert!(read(project.path(), "client/setups/ShopSetup.js").contains("class ShopSetup "));
    assert_eq!(
        read(project.path(), "client/stores/notes.txt"),
        "kept as it is\n"
    );
    assert!(
        !project
            .path()
            .join("client/repositories/BasketRepository.js")
            .exists()
    );
    let again = plan(project.path());
    assert_eq!(
        again.change_count() + again.move_count(),
        0,
        "{}",
        again.to_text(false)
    );
}

#[test]
fn moving_keeps_every_file_and_its_content() {
    let project = project();
    write_role_project(project.path());
    write(project.path(), "VERSION", "26.36.01\n");
    write(
        project.path(),
        "docs/repositories/Guide.js",
        "@Repository\nclass Guide {}\n",
    );
    let before = snapshot(project.path());
    plan(project.path()).write(project.path()).expect("write");
    let after = snapshot(project.path());
    assert_eq!(before.len(), after.len());
    assert_eq!(after.get("VERSION"), before.get("VERSION"));
    assert_eq!(
        after.get("docs/repositories/Guide.js"),
        before.get("docs/repositories/Guide.js")
    );
    assert_eq!(
        after.get("client/stores/notes.txt"),
        before.get("client/repositories/notes.txt")
    );
}

#[test]
fn a_taken_destination_is_a_manual_step_and_nothing_moves_over_it() {
    let project = project();
    write_role_project(project.path());
    write(
        project.path(),
        "client/stores/BasketStore.js",
        "@Store\nexport default class BasketStore {}\n",
    );
    let planned = plan(project.path());
    let text = planned.to_text(false);
    assert!(
        text.contains("manual   client/repositories/BasketRepository.js"),
        "{text}"
    );
    assert!(
        text.contains("'client/stores/BasketStore.js' already exists"),
        "{text}"
    );
    planned.write(project.path()).expect("write");
    assert_eq!(
        read(project.path(), "client/stores/BasketStore.js"),
        "@Store\nexport default class BasketStore {}\n"
    );
    assert!(
        project
            .path()
            .join("client/repositories/BasketRepository.js")
            .exists()
    );
}

#[test]
fn a_linked_role_folder_is_never_followed() {
    let project = project();
    let outside = tempfile::tempdir().expect("outside");
    write(
        outside.path(),
        "OrdersRepository.js",
        "@Repository\nclass OrdersRepository {}\n",
    );
    fs::create_dir_all(project.path().join("server")).expect("server");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), project.path().join("server/repositories"))
        .expect("link");
    let planned = plan(project.path());
    assert_eq!(planned.move_count(), 0, "{}", planned.to_text(false));
    planned.write(project.path()).expect("write");
    assert!(outside.path().join("OrdersRepository.js").exists());
    assert_eq!(
        read(outside.path(), "OrdersRepository.js"),
        "@Repository\nclass OrdersRepository {}\n"
    );
}

#[test]
fn a_tampered_move_writes_and_moves_nothing() {
    let project = project();
    write(project.path(), "VERSION", "26.36.01\n");
    write(
        project.path(),
        "client/repositories/BasketRepository.js",
        "@Store\nclass BasketStore {}\n",
    );
    let tampered = [
        ("VERSION", "client/stores/VERSION"),
        (
            "client/repositories/BasketRepository.js",
            "../BasketStore.js",
        ),
        (
            "client/repositories/BasketRepository.js",
            "client/stores/Other.js",
        ),
        (
            "client/repositories/BasketRepository.js",
            "docs/BasketStore.js",
        ),
        ("client/services/../../VERSION", "client/stores/VERSION"),
    ];
    for (from, to) in tampered {
        let valid = FileMove {
            from: String::from("client/repositories/BasketRepository.js"),
            to: String::from("client/stores/BasketStore.js"),
        };
        let broken = FileMove {
            from: String::from(from),
            to: String::from(to),
        };
        let planned = MigrationFixPlan {
            changes: Vec::new(),
            moves: vec![valid, broken],
            deletes: Vec::new(),
            legacy_identity: None,
            manual_steps: Vec::new(),
            unused_members: Vec::new(),
            missing_types: Vec::new(),
        };
        let failure = planned.write(project.path()).expect_err(to);
        assert!(
            failure
                .to_string()
                .contains("is not a file that the fix may move"),
            "{failure}"
        );
    }
    assert_eq!(read(project.path(), "VERSION"), "26.36.01\n");
    assert!(
        project
            .path()
            .join("client/repositories/BasketRepository.js")
            .exists()
    );
    assert!(!project.path().join("client/stores").exists());
}

#[test]
fn a_class_that_hides_its_new_annotation_is_a_manual_step() {
    let project = project();
    write(
        project.path(),
        "server/helpers/Settings.ts",
        "@ConfigurationProperties({\"prefix\":\"YON_SHOP_\",\"fields\":{}})\nexport default class Settings {}\n",
    );
    let text = plan(project.path()).to_text(false);
    assert!(
        text.contains("manual   server/helpers/Settings.ts"),
        "{text}"
    );
    assert!(
        text.contains("the same name as the global annotation @Settings"),
        "{text}"
    );
}

#[test]
fn a_component_becomes_a_helper_in_the_helpers_folder() {
    let project = project();
    let root = project.path();
    write(
        root,
        "client/delegates/PricingComponent.js",
        "@Component\nexport default class PricingComponent {\n  total(items) {\n    return items.length\n  }\n}\n",
    );
    write(
        root,
        "client/services/BasketService.js",
        "@Service\nexport default class BasketService {\n  @Needs\n  pricingComponent\n\n  total() {\n    return this.pricingComponent.total([])\n  }\n}\n",
    );
    write(
        root,
        "server/delegates/TaxComponent.py",
        "@component\nclass TaxComponent:\n    pass\n",
    );
    write(
        root,
        "server/services/OrderService.py",
        "@service\n@needs_all\nclass OrderService:\n    tax_component: TaxComponent\n",
    );
    write(root, "server/delegates/report.rb", "puts 1\n");
    write(
        root,
        "server/clients/ReportClient.py",
        "@http_client\nclass ReportClient:\n    @gateway({\"command\":[\"ruby\",\"server/delegates/report.rb\"]})\n    def summarize(self): ...\n",
    );
    plan(root).write(root).expect("write");
    assert_eq!(
        read(root, "client/helpers/PricingHelper.js"),
        "@Helper\nexport default class PricingHelper {\n  total(items) {\n    return items.length\n  }\n}\n"
    );
    let service = read(root, "client/services/BasketService.js");
    assert!(service.contains("  pricingHelper\n"), "{service}");
    assert!(service.contains("this.pricingHelper.total"), "{service}");
    assert_eq!(
        read(root, "server/helpers/TaxHelper.py"),
        "@helper\nclass TaxHelper:\n    pass\n"
    );
    let order = read(root, "server/services/OrderService.py");
    assert!(order.contains("tax_helper: TaxHelper"), "{order}");
    let client = read(root, "server/clients/ReportClient.py");
    assert!(client.contains("\"server/helpers/report.rb\""), "{client}");
    assert_eq!(read(root, "server/helpers/report.rb"), "puts 1\n");
    assert!(!root.join("client/delegates/PricingComponent.js").exists());
    assert!(!root.join("server/delegates/report.rb").exists());
}
