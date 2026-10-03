#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use tachyon_core::ComposedTypes;

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("source");
}

fn generated(root: &Path, realm: &str) -> String {
    ComposedTypes::write(root).expect("composed types");
    fs::read_to_string(
        root.join(".tachyon/types")
            .join(realm)
            .join("composed.d.ts"),
    )
    .expect("generated file")
}

fn declares(text: &str, name: &str, module: &str, member: &str) -> bool {
    let target = format!("import(\"../../../{module}\").{member}");
    text.contains(&format!("  type {name} = {target};\n"))
        && text.contains(&format!("  const {name}: typeof {target};\n"))
}

fn names(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("  const ") {
            found.push(rest.split(':').next().unwrap_or(""));
        }
    }
    found
}

#[test]
fn client_roles_become_globals_with_their_source_types() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/services/TopicService.ts",
        "import type { Guide } from '/models/Guide.ts'\n\n@Service\nexport default class TopicService {\n  entry(): Guide | null { return null }\n}\n",
    );
    write(
        root.path(),
        "client/stores/CartStore.js",
        "@Store\nexport class CartStore {\n  count() { return 1 }\n}\n",
    );
    write(
        root.path(),
        "client/helpers/PricingHelper.ts",
        "@Helper\nexport default class PricingHelper {}\n",
    );
    let text = generated(root.path(), "client");
    assert!(
        text.starts_with("export {};\n\ndeclare global {\n"),
        "{text}"
    );
    assert!(declares(
        &text,
        "TopicService",
        "client/services/TopicService.ts",
        "default"
    ));
    assert!(declares(
        &text,
        "CartStore",
        "client/stores/CartStore.js",
        "CartStore"
    ));
    assert!(declares(
        &text,
        "PricingHelper",
        "client/helpers/PricingHelper.ts",
        "default"
    ));
    assert_eq!(names(&text), ["CartStore", "PricingHelper", "TopicService"]);
}

#[test]
fn only_exported_annotated_role_classes_are_declared() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "client/services/ScriptService.js",
            "@Service\nclass ScriptService {}\n",
        ),
        (
            "client/services/Helper.js",
            "export default class Helper {}\n",
        ),
        (
            "client/services/notes.md",
            "export default class Notes {}\n",
        ),
        (
            "client/services/CheckService.py",
            "@Service\nclass CheckService: pass\n",
        ),
        ("client/models/Guide.ts", "export class Guide {}\n"),
        (
            "client/tests/ShellServiceTest.js",
            "@Test(ShellTestSetup)\nexport default class ShellServiceTest {}\n",
        ),
        (
            "client/components/app/tac.js",
            "export default class Shell {}\n",
        ),
        ("client/services/nested/tac.html", "<p>entry</p>\n"),
        (
            "client/services/nested/HiddenService.js",
            "@Service\nexport default class HiddenService {}\n",
        ),
        (
            "client/services/RealService.js",
            "@Service\nexport default class RealService {}\n",
        ),
    ];
    for (name, source) in files {
        write(root.path(), name, source);
    }
    let text = generated(root.path(), "client");
    assert_eq!(names(&text), ["RealService"], "{text}");
}

#[test]
fn server_globals_include_errors_error_handlers_and_route_controllers_for_test_units() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "server/services/OrdersService.js",
            "@Service\nexport default class OrdersService {}\n",
        ),
        (
            "server/controllers/OrdersController.js",
            "@RequestRoute(\"/orders\")\n@Public\nexport class OrdersController {\n  @GetRoute\n  get() {}\n}\n",
        ),
        (
            "server/controllers/OrdersErrorHandler.js",
            "@ErrorHandler\nexport class OrdersErrorHandler {}\n",
        ),
        (
            "server/errors/OrderNotFound.js",
            "export class OrderNotFound extends Error {}\n",
        ),
    ];
    for (name, source) in files {
        write(root.path(), name, source);
    }
    let text = generated(root.path(), "server");
    assert_eq!(
        names(&text),
        [
            "OrderNotFound",
            "OrdersController",
            "OrdersErrorHandler",
            "OrdersService"
        ],
        "{text}"
    );
    assert!(declares(
        &text,
        "OrderNotFound",
        "server/errors/OrderNotFound.js",
        "OrderNotFound"
    ));
}

#[test]
fn client_and_server_realms_are_kept_apart() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/entities/ProductEntity.ts",
        "@Entity({\"adapter\":\"data\"})\nexport default class ProductEntity {\n  @Id\n  id = ''\n}\n",
    );
    write(
        root.path(),
        "server/services/PriceService.ts",
        "@Service\nexport default class PriceService {}\n",
    );
    let client = generated(root.path(), "client");
    let server = generated(root.path(), "server");
    assert_eq!(names(&client), ["ProductEntity"], "{client}");
    assert_eq!(names(&server), ["PriceService"], "{server}");
    assert!(!client.contains("server/"), "{client}");
    assert!(!server.contains("client/"), "{server}");
}

#[test]
fn exported_test_setups_are_declared_in_their_realm_for_test_units() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        (
            "client/tests/SiteTestSetup.ts",
            "@TestSetup\nexport default class SiteTestSetup {\n  static configure() { return {} }\n}\n",
        ),
        (
            "client/tests/TopicServiceTest.ts",
            "@Test(SiteTestSetup)\nexport default class TopicServiceTest {}\n",
        ),
        (
            "client/tests/HiddenTestSetup.ts",
            "@TestSetup\nclass HiddenTestSetup {}\n",
        ),
        (
            "server/tests/OrdersTestSetup.js",
            "@TestSetup\nexport default class OrdersTestSetup {}\n",
        ),
    ];
    for (name, source) in files {
        write(root.path(), name, source);
    }
    let client = generated(root.path(), "client");
    let server = generated(root.path(), "server");
    assert_eq!(names(&client), ["SiteTestSetup"], "{client}");
    assert!(
        declares(
            &client,
            "SiteTestSetup",
            "client/tests/SiteTestSetup.ts",
            "default"
        ),
        "{client}"
    );
    assert_eq!(names(&server), ["OrdersTestSetup"], "{server}");
}

#[test]
fn a_deleted_class_leaves_no_stale_declaration() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/services/KeptService.js",
        "@Service\nexport default class KeptService {}\n",
    );
    write(
        root.path(),
        "client/services/GoneService.js",
        "@Service\nexport default class GoneService {}\n",
    );
    assert_eq!(
        names(&generated(root.path(), "client")),
        ["GoneService", "KeptService"]
    );
    fs::remove_file(root.path().join("client/services/GoneService.js")).expect("delete");
    assert_eq!(names(&generated(root.path(), "client")), ["KeptService"]);
    fs::remove_file(root.path().join("client/services/KeptService.js")).expect("delete");
    assert_eq!(generated(root.path(), "client"), "export {};\n");
}

#[test]
fn a_project_without_classes_gets_empty_declaration_files() {
    let root = tempfile::tempdir().expect("project");
    assert_eq!(generated(root.path(), "client"), "export {};\n");
    assert_eq!(generated(root.path(), "server"), "export {};\n");
}

#[test]
fn the_build_writes_the_composed_types() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<home-view />\n");
    write(
        root.path(),
        "client/components/home/view/tac.html",
        "<main>Home</main>\n",
    );
    write(
        root.path(),
        "client/services/HomeService.js",
        "@Service\nexport default class HomeService {}\n",
    );
    let options = tachyon_core::BuildOptions::default();
    tachyon_core::WebCompiler::build(root.path(), &options).expect("build");
    let text = fs::read_to_string(root.path().join(".tachyon/types/client/composed.d.ts"))
        .expect("generated by the build");
    assert_eq!(names(&text), ["HomeService"]);
}

#[cfg(unix)]
#[test]
fn a_symlinked_types_folder_fails_without_writing_outside() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("project");
    let outside = tempfile::tempdir().expect("outside");
    fs::create_dir(root.path().join(".tachyon")).expect("tachyon");
    symlink(outside.path(), root.path().join(".tachyon/types")).expect("types link");
    let failure = ComposedTypes::write(root.path()).expect_err("symlink refused");
    assert!(failure.to_string().contains("TY1410"), "{failure}");
    assert_eq!(fs::read_dir(outside.path()).expect("outside").count(), 0);
}

#[test]
fn utility_classes_are_declared_in_their_own_realm() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/shared/utils/PriceUtils.js",
        "export default class PriceUtils {\n  static label(cents) { return String(cents) }\n}\n",
    );
    let client = generated(root.path(), "client");
    assert!(declares(
        &client,
        "PriceUtils",
        "client/shared/utils/PriceUtils.js",
        "default"
    ));
    let server = generated(root.path(), "server");
    assert!(!server.contains("PriceUtils"), "{server}");
}

#[test]
fn javascript_models_become_global_types_in_their_realm() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/models/CartTotal.js",
        "/**\n * @typedef {Object} CartTotal\n * @property {number} total\n */\nexport {}\n",
    );
    write(
        root.path(),
        "server/models/orders/Order.js",
        "/**\n * @typedef {Object} Order\n * @property {string} sku\n */\nexport {}\n",
    );
    write(
        root.path(),
        "client/models/Guide.ts",
        "export interface Guide {\n  title: string\n}\n",
    );
    let client = generated(root.path(), "client");
    assert!(
        client.contains(
            "  type CartTotal = import(\"../../../client/models/CartTotal.js\").CartTotal;\n"
        ),
        "{client}"
    );
    assert!(!client.contains("const CartTotal"), "{client}");
    assert!(!client.contains("Guide"), "{client}");
    assert!(!client.contains("Order"), "{client}");
    let server = generated(root.path(), "server");
    assert!(
        server
            .contains("  type Order = import(\"../../../server/models/orders/Order.js\").Order;\n"),
        "{server}"
    );
}
