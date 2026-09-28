use super::edits::TextEdit;
use super::imports::ImportRemoval;
use super::middleware::{MiddlewareConversion, MiddlewarePlan};
use super::{MigrationFix, MigrationFixPlan};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct OldMiddleware {
    extension: &'static str,
    source: &'static str,
}

impl OldMiddleware {
    const ALL: [Self; 9] = [
        Self {
            extension: "js",
            source: "export class Handler {\n  static GET(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static POST(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n",
        },
        Self {
            extension: "ts",
            source: "export class Handler {\n  static async GET(request: { method: string }) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static async POST(request: { method: string }) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n",
        },
        Self {
            extension: "py",
            source: "class Handler:\n    @staticmethod\n    def GET(request):\n        return {\"status\": 204, \"headers\": {}, \"body\": \"\"}\n\n    @staticmethod\n    def POST(request):\n        return {\"status\": 204, \"headers\": {}, \"body\": \"\"}\n",
        },
        Self {
            extension: "php",
            source: "<?php\nfinal class Handler\n{\n    public static function GET(YonRequest $request): YonResponse\n    {\n        return YonResponse::empty(204);\n    }\n\n    public static function POST(YonRequest $request): YonResponse\n    {\n        return YonResponse::empty(204);\n    }\n}\n",
        },
        Self {
            extension: "java",
            source: "class Handler {\n    static YonResponse GET(YonRequest request) {\n        return YonResponse.empty(204);\n    }\n\n    static YonResponse POST(YonRequest request) {\n        return YonResponse.empty(204);\n    }\n}\n",
        },
        Self {
            extension: "kt",
            source: "object Handler {\n    fun GET(request: YonRequest): YonResponse =\n        YonResponse.empty(204)\n\n    fun POST(request: YonRequest): YonResponse =\n        YonResponse.empty(204)\n}\n",
        },
        Self {
            extension: "cs",
            source: "sealed class Handler {\n    public static YonResponse GET(YonRequest request) =>\n        YonResponse.Empty(204);\n\n    public static YonResponse POST(YonRequest request) =>\n        YonResponse.Empty(204);\n}\n",
        },
        Self {
            extension: "rs",
            source: "struct Handler;\nimpl Handler {\n    #[allow(non_snake_case)]\n    fn GET(_request: &YonRequest) -> YonResponse {\n        YonResponse::empty(204)\n    }\n\n    #[allow(non_snake_case)]\n    fn POST(_request: &YonRequest) -> YonResponse {\n        YonResponse::empty(204)\n    }\n}\n",
        },
        Self {
            extension: "dart",
            source: "class Handler {\n  YonResponse GET(YonRequest request) =>\n      YonResponse.empty(204);\n\n  YonResponse POST(YonRequest request) =>\n      YonResponse.empty(204);\n}\n",
        },
    ];

    fn file_name(&self) -> String {
        format!("middleware.{}", self.extension)
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

fn without_imports(relative: &str, source: &str) -> String {
    let removal = ImportRemoval::plan(relative, source);
    assert!(removal.manual_steps.is_empty(), "{source}");
    TextEdit::apply(source, &removal.edits)
}

fn import_problem(source: &str) -> String {
    let removal = ImportRemoval::plan("client/services/CartService.ts", source);
    assert!(removal.edits.is_empty(), "{source}");
    let step = removal.manual_steps.first().expect("manual step");
    format!("{} {} {}", step.source, step.problem, step.action)
}

fn converted(relative: &str, source: &str) -> String {
    match MiddlewareConversion::plan(relative, source) {
        MiddlewarePlan::Converted { edits, .. } => TextEdit::apply(source, &edits),
        MiddlewarePlan::Manual { problem, .. } => panic!("{relative}: {problem}"),
        MiddlewarePlan::Unchanged => panic!("{relative} was not changed"),
    }
}

fn refused(relative: &str, source: &str) -> String {
    match MiddlewareConversion::plan(relative, source) {
        MiddlewarePlan::Manual { problem, action } => format!("{problem} {action}"),
        MiddlewarePlan::Converted { .. } => panic!("{relative} was converted"),
        MiddlewarePlan::Unchanged => panic!("{relative} was not changed"),
    }
}

#[test]
fn a_named_import_line_is_deleted_and_the_other_imports_stay() {
    let source = "import { helper } from '../shared/helper.js'\nimport { Service } from 'tachyon:behaviors'\n\n@Service\nexport class CartService {}\n";
    assert_eq!(
        without_imports("client/services/CartService.js", source),
        "import { helper } from '../shared/helper.js'\n\n@Service\nexport class CartService {}\n"
    );
}

#[test]
fn type_multiline_and_side_effect_imports_are_deleted() {
    let source = "import type { BehaviorContext } from \"tachyon:behaviors\";\nimport {\n  Service,\n  type Inject,\n} from 'tachyon:behaviors';\nimport 'tachyon:behaviors'\n\n@Service\nexport class CartService {}\n";
    assert_eq!(
        without_imports("server/services/CartService.ts", source),
        "@Service\nexport class CartService {}\n"
    );
}

#[test]
fn an_import_that_shares_its_line_keeps_the_rest_of_the_line() {
    let source =
        "import { Service } from 'tachyon:behaviors'; // global now\nexport class CartService {}\n";
    assert_eq!(
        without_imports("client/services/CartService.js", source),
        "// global now\nexport class CartService {}\n"
    );
}

#[test]
fn mentions_outside_an_import_are_not_changed() {
    let source =
        "// import { Service } from 'tachyon:behaviors'\nconst name = 'tachyon:behaviors'\n";
    let removal = ImportRemoval::plan("client/services/CartService.js", source);
    assert!(removal.edits.is_empty());
    assert!(removal.manual_steps.is_empty());
}

#[test]
fn imports_that_rename_or_reexport_are_manual_steps() {
    let cases = [
        (
            "import * as Behavior from 'tachyon:behaviors'",
            "local name: * as Behavior",
        ),
        (
            "import Behaviors from 'tachyon:behaviors'",
            "local name: Behaviors",
        ),
        (
            "import { Service as Role } from 'tachyon:behaviors'",
            "local name: { Service as Role }",
        ),
        (
            "export { Service } from 'tachyon:behaviors'",
            "re-exports names",
        ),
        (
            "const behaviors = await import('tachyon:behaviors')",
            "with import()",
        ),
    ];
    for (line, expected) in cases {
        let message = import_problem(&format!("\n{line}\n"));
        assert!(
            message.starts_with("client/services/CartService.ts:2 "),
            "{message}"
        );
        assert!(message.contains(expected), "{message}");
        assert!(message.contains("ADR 0046"), "{message}");
    }
}

#[test]
fn old_middleware_in_every_language_becomes_the_middleware_class() {
    for old in &OldMiddleware::ALL {
        let name = old.file_name();
        let new = converted(&name, old.source);
        let path = Path::new(&name);
        let names = crate::annotation_layout::declaration_names(path, &new).expect("names");
        assert_eq!(names, vec![String::from("Middleware")], "{name}:\n{new}");
        crate::annotation_layout::has_primary_role(path, &new)
            .unwrap_or_else(|failure| panic!("{name}: {failure}\n{new}"));
        assert!(!new.contains("Handler"), "{name}:\n{new}");
        assert!(
            !new.contains("GET") && !new.contains("POST"),
            "{name}:\n{new}"
        );
        assert_eq!(new.matches("before(").count(), 1, "{name}:\n{new}");
        assert_eq!(new.matches("after(").count(), 1, "{name}:\n{new}");
    }
}

#[test]
fn javascript_middleware_keeps_one_copy_of_the_code_in_each_phase() {
    let old = &OldMiddleware::ALL[0];
    assert_eq!(
        converted("middleware.js", old.source),
        "export class Middleware {\n  static before(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static after(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n"
    );
}

#[test]
fn python_middleware_keeps_its_decorators() {
    let old = &OldMiddleware::ALL[2];
    assert_eq!(
        converted("middleware.py", old.source),
        "class Middleware:\n    @staticmethod\n    def before(request):\n        return {\"status\": 204, \"headers\": {}, \"body\": \"\"}\n\n    @staticmethod\n    def after(request):\n        return {\"status\": 204, \"headers\": {}, \"body\": \"\"}\n"
    );
}

#[test]
fn references_to_the_old_class_name_follow_the_rename() {
    let source = "export class Handler {\n  static GET(request) {\n    return Handler.pass(request)\n  }\n\n  static pass(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n";
    let new = converted("middleware.js", source);
    assert_eq!(new.matches("Middleware.pass(request)").count(), 2, "{new}");
    assert!(new.contains("static pass(request)"), "{new}");
}

#[test]
fn middleware_that_cannot_be_converted_safely_is_left_alone() {
    let cases = [
        (
            "middleware.js",
            "export class Handler {\n  static GET(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static POST(request) {\n    return { status: 401, headers: {}, body: '' }\n  }\n}\n",
            "hold different code",
        ),
        (
            "middleware.js",
            "export class Handler {\n  static pass(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n",
            "no function named after an HTTP method",
        ),
        (
            "middleware.js",
            "export class Handler {\n  static GET(request) {\n    return Middleware.check(request)\n  }\n}\n",
            "already uses the name Middleware",
        ),
        (
            "middleware.js",
            "export class Handler {\n  static GET(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static check(request) {\n    return Handler.GET(request)\n  }\n}\n",
            "refers to an HTTP method function by name",
        ),
        (
            "middleware.kt",
            "class Handler {\n    companion object {\n        fun GET(request: YonRequest): YonResponse = YonResponse.empty(204)\n    }\n}\n",
            "no function named after an HTTP method",
        ),
        (
            "middleware.py",
            "class Handler:\n    pass\n\n\nclass Extra:\n    pass\n",
            "more than one type",
        ),
    ];
    for (name, source, reason) in cases {
        let message = refused(name, source);
        assert!(message.contains(reason), "{name}: {message}");
        assert!(
            message.contains("Rename the class to Middleware"),
            "{message}"
        );
    }
}

#[test]
fn middleware_that_already_follows_the_contract_is_unchanged() {
    let source = "export class Middleware {\n  static before(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n";
    let plan = MiddlewareConversion::plan("middleware.js", source);
    assert!(matches!(plan, MiddlewarePlan::Unchanged));
}

fn write_old_project(root: &Path) {
    write(root, "middleware.js", OldMiddleware::ALL[0].source);
    write(
        root,
        "client/services/CartService.ts",
        "import { Service } from 'tachyon:behaviors'\n\n@Service\nexport class CartService {}\n",
    );
    write(
        root,
        "server/routes/cart/yon.js",
        "export class Handler {\n  static GET() { return {} }\n}\n\nexport function helper() {}\n",
    );
}

#[test]
fn the_plan_lists_every_change_without_writing_it() {
    let project = project();
    write_old_project(project.path());
    let plan = plan(project.path());
    let text = plan.to_text(false);
    assert_eq!(plan.change_count(), 3, "{text}");
    assert!(
        text.contains("change   client/services/CartService.ts"),
        "{text}"
    );
    assert!(
        text.contains("- import { Service } from 'tachyon:behaviors'"),
        "{text}"
    );
    assert!(text.contains("change   middleware.js"), "{text}");
    assert!(text.contains("+ export class Middleware {"), "{text}");
    assert!(text.contains("Run 'ty migrate fix --write'"), "{text}");
    assert_eq!(
        read(project.path(), "middleware.js"),
        OldMiddleware::ALL[0].source
    );
}

#[test]
fn the_one_class_rule_is_reported_with_the_split_to_do() {
    let project = project();
    write_old_project(project.path());
    let text = plan(project.path()).to_text(false);
    assert!(
        text.contains("manual   server/controllers/CartController.js:8"),
        "{text}"
    );
    assert!(
        text.contains("An annotated or handler file holds one class and its imports; found"),
        "{text}"
    );
    assert!(
        text.contains("Split: keep class CartController in server/controllers/CartController.js."),
        "{text}"
    );
    assert!(text.contains("into 'server/shared/'"), "{text}");
}

#[test]
fn writing_applies_the_plan_once() {
    let project = project();
    write_old_project(project.path());
    plan(project.path()).write(project.path()).expect("write");
    assert!(read(project.path(), "middleware.js").starts_with("export class Middleware {"));
    assert_eq!(
        read(project.path(), "client/services/CartService.ts"),
        "@Service\nexport class CartService {}\n"
    );
    let second = plan(project.path());
    assert_eq!(second.change_count(), 0, "{}", second.to_text(false));
    assert_eq!(second.manual_step_count(), 1);
}

#[test]
fn a_file_changed_after_planning_is_not_overwritten() {
    let project = project();
    write_old_project(project.path());
    let plan = plan(project.path());
    write(
        project.path(),
        "middleware.js",
        "export class Middleware {}\n",
    );
    let failure = plan.write(project.path()).expect_err("stale plan");
    assert!(
        failure
            .to_string()
            .contains("changed after the fix was planned")
    );
    assert_eq!(
        read(project.path(), "middleware.js"),
        "export class Middleware {}\n"
    );
}

#[cfg(unix)]
#[test]
fn writing_keeps_file_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let project = project();
    write_old_project(project.path());
    let path = project.path().join("middleware.js");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("permissions");
    plan(project.path()).write(project.path()).expect("write");
    let mode = fs::metadata(&path).expect("metadata").permissions().mode();
    assert_eq!(mode & 0o777, 0o640);
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_never_followed() {
    let outside = project();
    let project = project();
    let source = "import { Service } from 'tachyon:behaviors'\nexport class Outside {}\n";
    write(outside.path(), "client/services/Outside.ts", source);
    write(
        outside.path(),
        "middleware.js",
        OldMiddleware::ALL[0].source,
    );
    std::os::unix::fs::symlink(outside.path().join("client"), project.path().join("client"))
        .expect("folder link");
    std::os::unix::fs::symlink(
        outside.path().join("middleware.js"),
        project.path().join("middleware.js"),
    )
    .expect("file link");
    let plan = plan(project.path());
    assert_eq!(plan.change_count(), 0, "{}", plan.to_text(false));
    assert_eq!(read(outside.path(), "client/services/Outside.ts"), source);
}

fn snapshot(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(root).expect("inside the project");
            let relative = relative.to_string_lossy().replace('\\', "/");
            files.insert(relative, fs::read(&path).expect("file"));
        }
    }
    files
}

fn write_files_the_fix_must_not_touch(root: &Path) {
    let import = "import { Service } from 'tachyon:behaviors'\n";
    write(root, "VERSION", "26.36.01\n");
    write(root, "package.json", "{\"name\":\"shop\"}\n");
    write(root, "README.md", import);
    write(root, "index.js", import);
    write(root, "scripts/build.js", import);
    write(root, "client/shared/scripts/tool.mjs", import);
    write(root, "client/pages/tac.html", "<main>Home</main>\n");
}

#[test]
fn writing_changes_and_moves_only_the_reported_files() {
    let project = project();
    write_old_project(project.path());
    write_files_the_fix_must_not_touch(project.path());
    let mut before = snapshot(project.path());
    let plan = plan(project.path());
    plan.write(project.path()).expect("write");
    let after = snapshot(project.path());
    for file_move in &plan.moves {
        let original = before.remove(&file_move.from).expect("moved source");
        before.insert(file_move.to.clone(), original);
    }
    let before_names: Vec<&String> = before.keys().collect();
    let after_names: Vec<&String> = after.keys().collect();
    assert_eq!(before_names, after_names);
    let mut changed = Vec::new();
    for (name, contents) in &after {
        if before.get(name) != Some(contents) {
            changed.push(name.clone());
        }
    }
    let mut reported: Vec<String> = plan
        .changes
        .iter()
        .map(|change| {
            plan.moves
                .iter()
                .find(|file_move| file_move.from == change.source)
                .map_or_else(|| change.source.clone(), |file_move| file_move.to.clone())
        })
        .collect();
    reported.sort();
    assert_eq!(changed, reported);
    assert_eq!(
        changed,
        vec![
            String::from("client/services/CartService.ts"),
            String::from("middleware.js"),
            String::from("server/controllers/CartController.js")
        ]
    );
}

#[test]
fn a_plan_that_names_a_file_outside_the_fix_targets_writes_nothing() {
    let project = project();
    write(project.path(), "VERSION", "26.36.01\n");
    write(
        project.path(),
        "client/services/CartService.ts",
        "export class CartService {}\n",
    );
    for target in [
        "VERSION",
        "../outside.ts",
        "/tmp/outside.ts",
        "client/../VERSION",
        "docs/guide.ts",
    ] {
        let change = super::FileChange {
            source: String::from(target),
            notes: Vec::new(),
            hunks: Vec::new(),
            original: String::from("26.36.01\n"),
            updated: String::new(),
        };
        let valid = super::FileChange {
            source: String::from("client/services/CartService.ts"),
            notes: Vec::new(),
            hunks: Vec::new(),
            original: String::from("export class CartService {}\n"),
            updated: String::new(),
        };
        let plan = MigrationFixPlan {
            changes: vec![valid, change],
            moves: Vec::new(),
            deletes: Vec::new(),
            legacy_identity: None,
            manual_steps: Vec::new(),
            unused_members: Vec::new(),
            missing_types: Vec::new(),
        };
        let failure = plan.write(project.path()).expect_err(target);
        assert!(
            failure
                .to_string()
                .contains("is not a file that the fix may change"),
            "{failure}"
        );
    }
    assert_eq!(read(project.path(), "VERSION"), "26.36.01\n");
    assert_eq!(
        read(project.path(), "client/services/CartService.ts"),
        "export class CartService {}\n"
    );
}
