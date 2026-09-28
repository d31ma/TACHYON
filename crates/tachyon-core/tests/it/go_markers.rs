#![allow(clippy::expect_used)]

use std::fs;
use tachyon_core::{MigrationFix, ProjectDiscovery};

struct GoProject;

impl GoProject {
    const CONTROLLER: &str = include_str!("../fixtures/go-markers/OrdersController.go");

    fn create(source: &str) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        fs::create_dir_all(project.path().join("server/controllers")).expect("controllers");
        fs::write(
            project
                .path()
                .join("server/controllers/OrdersController.go"),
            source,
        )
        .expect("controller");
        project
    }
}

#[test]
fn go_comment_markers_discover_routes_and_parameter_types() {
    let project = GoProject::create(GoProject::CONTROLLER);
    let discovered = ProjectDiscovery::discover(project.path()).expect("comment annotations");
    assert!(!discovered.route_graph().manifest().routes.is_empty());
}

#[test]
fn go_unknown_comment_markers_are_not_silently_ignored() {
    let project = GoProject::create(&GoProject::CONTROLLER.replace("@GetRoute", "@GetRote"));
    let failure = ProjectDiscovery::discover(project.path()).expect_err("unknown annotation");
    let message = format!("{failure:?}");
    assert!(
        message.contains("unknown annotation `@GetRote`"),
        "{message}"
    );
    assert!(message.contains("GetRoute"), "{message}");
}

#[test]
fn go_bare_at_markers_are_refused_with_comment_guidance() {
    let project = GoProject::create(&GoProject::CONTROLLER.replace("// @", "@"));
    let failure = ProjectDiscovery::discover(project.path()).expect_err("bare annotations");
    assert!(format!("{failure:?}").contains("Go annotations are comments"));
}

#[test]
fn go_unknown_only_provider_markers_are_not_skipped() {
    let project = GoProject::create(GoProject::CONTROLLER);
    fs::create_dir_all(project.path().join("server/services")).expect("services");
    fs::write(
        project.path().join("server/services/MoneyService.go"),
        "// @Servce\ntype MoneyService struct{}\n",
    )
    .expect("provider");
    let failure = ProjectDiscovery::discover(project.path()).expect_err("unknown provider marker");
    let message = format!("{failure:?}");
    assert!(
        message.contains("unknown annotation `@Servce`"),
        "{message}"
    );
    assert!(message.contains("// @Service"), "{message}");
}

#[test]
fn go_old_markers_are_refused_and_migrate_without_touching_literals() {
    let source = "package main\n#[RequestRoute(\"/orders\")]\n#[Public]\ntype OrdersController struct{}\n#[GetRoute]\nfunc (controller *OrdersController) GetOrder() string { return `#[Service]` }\n";
    let project = GoProject::create(source);
    let failure = ProjectDiscovery::discover(project.path()).expect_err("old Go spelling");
    assert!(format!("{failure:?}").contains("not `#[Name]`"));
    MigrationFix::plan(project.path())
        .expect("migration plan")
        .write(project.path())
        .expect("migration");
    let migrated = fs::read_to_string(
        project
            .path()
            .join("server/controllers/OrdersController.go"),
    )
    .expect("migrated source");
    assert!(migrated.contains("// @RequestRoute(\"/orders\")"));
    assert!(migrated.contains("`#[Service]`"));
    assert_eq!(
        MigrationFix::plan(project.path())
            .expect("second plan")
            .change_count(),
        0
    );
    ProjectDiscovery::discover(project.path()).expect("migrated controller");
}

#[test]
fn go_marker_shaped_text_inside_comments_and_strings_stays_text() {
    let source = GoProject::CONTROLLER.replace(
        "\treturn id",
        "\t/*\n\t// @UnknownBlock\n\t*/\n\ttext := `\n// @UnknownRaw\n#[Service]\n`\n\t_ = text\n\t_ = \"// @UnknownQuoted\"\n\t// @param id\n\treturn id",
    );
    let project = GoProject::create(&source);
    ProjectDiscovery::discover(project.path()).expect("ordinary comments and literals");
    assert_eq!(
        MigrationFix::plan(project.path())
            .expect("plan")
            .change_count(),
        0
    );
}

#[test]
fn go_lowercase_documentation_tags_stay_comments() {
    let source = GoProject::CONTROLLER
        .replace(
            "// @RequestRoute",
            "// @description Orders API for the shop\n// @RequestRoute",
        )
        .replace(
            "\treturn id",
            "\t// @id is assigned by the database\n\treturn id",
        );
    let project = GoProject::create(&source);
    ProjectDiscovery::discover(project.path()).expect("lowercase documentation tags");
}

#[test]
fn go_old_inline_parameter_markers_migrate_to_comment_lines() {
    let source = "package main\n#[RequestRoute(\"/orders\")]\n#[Public]\ntype OrdersController struct{}\n#[GetRoute(\"/{id}\")]\nfunc (controller *OrdersController) GetOrder(#[FromPath] id int, #[FromQuery] limit int) int { return id + limit }\n";
    let project = GoProject::create(source);
    MigrationFix::plan(project.path())
        .expect("migration plan")
        .write(project.path())
        .expect("migration");
    let migrated = fs::read_to_string(
        project
            .path()
            .join("server/controllers/OrdersController.go"),
    )
    .expect("migrated source");
    assert!(migrated.contains("// @FromPath\n") && migrated.contains("// @FromQuery\n"));
    assert!(!migrated.contains("#["));
    ProjectDiscovery::discover(project.path()).expect("migrated parameters");
    assert_eq!(
        MigrationFix::plan(project.path())
            .expect("second plan")
            .change_count(),
        0
    );
}

fn refusal(source: &str) -> String {
    let project = GoProject::create(source);
    let failure = ProjectDiscovery::discover(project.path()).expect_err("refused source");
    format!("{failure:?}")
}

#[test]
fn go_swag_documentation_comments_stay_ordinary_comments() {
    let source = GoProject::CONTROLLER.replace(
        "// @GetRoute",
        "// GetOrder godoc\n// @Summary Get one order\n// @Description Returns the order with this id\n// @Tags orders\n// @ID get-order\n// @Accept json\n// @Produce json\n// @Param id path int true \"Order ID\"\n// @Success 200 {object} int\n// @Failure 404 {object} string\n// @Router /orders/{id} [get]\n// @GetRoute",
    );
    let project = GoProject::create(&source);
    let discovered = ProjectDiscovery::discover(project.path()).expect("swag comments");
    assert!(!discovered.route_graph().manifest().routes.is_empty());
}

#[test]
fn go_near_miss_markers_are_refused_with_the_written_namespace() {
    let case = refusal(&GoProject::CONTROLLER.replace("@GetRoute", "@Getroute"));
    assert!(
        case.contains("unknown annotation `@Getroute`; did you mean `// @GetRoute`?"),
        "{case}"
    );
    let namespaced = refusal(&GoProject::CONTROLLER.replace("@GetRoute", "@yon::GetRote"));
    assert!(
        namespaced
            .contains("unknown annotation `@yon::GetRote`; did you mean `// @yon::GetRoute`?"),
        "{namespaced}"
    );
    let capital = refusal(&GoProject::CONTROLLER.replace("@GetRoute", "@Yon::GetRoute"));
    assert!(
        capital.contains("namespaces are lowercase: write `// @yon::GetRoute`"),
        "{capital}"
    );
}

#[test]
fn go_markers_need_exactly_one_space_after_the_slashes() {
    for written in [
        "//@GetRoute",
        "//  @GetRoute",
        "//\t@GetRoute",
        "//@FromPath",
    ] {
        let prefix = if written.ends_with("FromPath") {
            "// @FromPath"
        } else {
            "// @GetRoute"
        };
        let message = refusal(&GoProject::CONTROLLER.replace(prefix, written));
        assert!(
            message.contains("exactly one space after `//`"),
            "{written}: {message}"
        );
    }
}

#[test]
fn go_text_after_a_marker_is_refused_on_its_own_line() {
    let note = refusal(&GoProject::CONTROLLER.replace(
        "// @GetRoute(\"/{id}\")",
        "// @GetRoute(\"/{id}\") serves one order",
    ));
    assert!(note.contains("holds only the annotation"), "{note}");
    let prose = refusal(&GoProject::CONTROLLER.replace(
        "\treturn id",
        "\t// @Needs is resolved at startup, so don't check it\n\treturn id",
    ));
    assert!(prose.contains("holds only the annotation"), "{prose}");
    assert!(!prose.contains("Unterminated"), "{prose}");
    let unclosed = refusal(&GoProject::CONTROLLER.replace(
        "// @RequestRoute(\"/orders\")",
        "// @RequestRoute(\"/orders)",
    ));
    assert!(
        unclosed.contains("open and close on the annotation's own line"),
        "{unclosed}"
    );
}

#[test]
fn go_a_byte_order_mark_does_not_hide_a_first_line_marker() {
    let source = format!(
        "\u{feff}{}",
        GoProject::CONTROLLER.trim_start_matches("package main\n\n")
    );
    let project = GoProject::create(&source);
    let discovered = ProjectDiscovery::discover(project.path()).expect("marker after a BOM");
    assert!(!discovered.route_graph().manifest().routes.is_empty());
}

fn migrated(source: &str) -> (tempfile::TempDir, String) {
    let project = GoProject::create(source);
    MigrationFix::plan(project.path())
        .expect("migration plan")
        .write(project.path())
        .expect("migration");
    let text = fs::read_to_string(
        project
            .path()
            .join("server/controllers/OrdersController.go"),
    )
    .expect("migrated source");
    (project, text)
}

#[test]
fn go_multi_line_and_grouped_old_markers_migrate_to_one_line_each() {
    let source = "package main\n\n#[RequestRoute(\n    \"/orders\"\n)]\n#[Public]\ntype OrdersController struct{}\n\n#[GetRoute(\"/{id}\"), ResponseStatus(201)]\nfunc (controller *OrdersController) GetOrder(#[FromPath] id int, #[FromQuery] limit int) int {\n\treturn id + limit\n}\n";
    let (project, text) = migrated(source);
    assert!(
        text.contains("\n// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController"),
        "{text}"
    );
    assert!(
        text.contains("\n// @GetRoute(\"/{id}\")\n// @ResponseStatus(201)\nfunc"),
        "{text}"
    );
    assert!(
        text.contains("GetOrder(\n\t// @FromPath\n\tid int,\n\t// @FromQuery\n\tlimit int) int {"),
        "{text}"
    );
    ProjectDiscovery::discover(project.path()).expect("migrated controller");
    assert_eq!(
        MigrationFix::plan(project.path())
            .expect("second plan")
            .change_count(),
        0
    );
}

#[test]
fn go_multi_line_old_marker_arguments_join_one_comment_line() {
    let source = "package main\n\n#[Schedule({\n  \"cron\": \"* * * * *\",\n  \"zone\": \"UTC\"\n})]\ntype OrdersController struct{}\n";
    let (_project, text) = migrated(source);
    assert!(
        text.contains(
            "\n// @Schedule({\"cron\": \"* * * * *\", \"zone\": \"UTC\"})\ntype OrdersController"
        ),
        "{text}"
    );
    let raw = "package main\n\n#[Schedule(`\n* * * * *\n`)]\ntype OrdersController struct{}\n";
    let project = GoProject::create(raw);
    let plan = MigrationFix::plan(project.path()).expect("migration plan");
    assert!(
        format!("{plan:?}").contains("cannot become one `// @Name(...)` line"),
        "{plan:?}"
    );
}
