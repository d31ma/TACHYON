use super::MigrationFix;
use crate::HandlerSource;
use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, contents).expect("file");
}

fn migrated(relative: &str, source: &str, to: &str) -> String {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), relative, source);
    let plan = MigrationFix::plan(project.path()).expect("plan");
    plan.write(project.path()).expect("write");
    let written = fs::read_to_string(project.path().join(to))
        .unwrap_or_else(|_| panic!("{to}: {}", plan.to_text(true)));
    let routes = HandlerSource::discover(project.path(), to)
        .unwrap_or_else(|failure| panic!("{to}: {failure}"));
    assert!(!routes.routes().is_empty(), "{to} has routes");
    assert!(!project.path().join(relative).exists(), "{relative} moved");
    written
}

#[test]
fn the_26_36_01_scaffold_route_becomes_the_current_home_controller() {
    let written = migrated(
        "server/routes/yon.js",
        "@Controller\nexport class RootController {\n  static async GET() {\n    return { ok: true, framework: 'Tachyon' }\n  }\n}\n",
        "server/controllers/HomeController.js",
    );
    assert_eq!(
        written,
        "@RequestRoute(\"/\")\n@Public\nexport class HomeController {\n  @GetRoute\n  async get() {\n    return { ok: true, framework: 'Tachyon' }\n  }\n}\n"
    );
}

#[test]
fn a_controller_class_keeps_its_helpers_and_takes_the_route_class_name() {
    let written = migrated(
        "server/routes/orders/_id/yon.ts",
        "@Controller\nexport class OrderController {\n  static GET() {\n    return OrderController.found()\n  }\n\n  static DELETE() {\n    return {}\n  }\n\n  static found() {\n    return { found: true }\n  }\n}\n",
        "server/controllers/OrdersIdController.ts",
    );
    assert_eq!(
        written,
        "@RequestRoute(\"/orders/{id}\")\n@Public\nexport class OrdersIdController {\n  @GetRoute\n  get() {\n    return OrdersIdController.found()\n  }\n\n  @DeleteRoute\n  delete() {\n    return {}\n  }\n\n  static found() {\n    return { found: true }\n  }\n}\n"
    );
}

#[test]
fn a_controller_route_that_uses_this_or_has_no_marked_class_stays_with_a_manual_step() {
    for (source, problem) in [
        (
            "@Controller\nexport class RootController {\n  static GET() {\n    return this.found()\n  }\n}\n",
            "Its static methods use 'this'",
        ),
        (
            "export class RootController {\n  static GET() {\n    return {}\n  }\n}\n",
            "no one class marked @Controller",
        ),
    ] {
        let project = tempfile::tempdir().expect("project");
        write(project.path(), "server/routes/yon.js", source);
        let text = MigrationFix::plan(project.path())
            .expect("plan")
            .to_text(false);
        assert!(text.contains(problem), "{text}");
        assert!(!text.contains("HomeController.js\n"), "{text}");
    }
}
