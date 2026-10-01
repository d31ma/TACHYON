use super::{
    ClientViewRenderer, ComponentRegistry, Scope, ServiceReach, ServiceRegistry, TemplateFrontend,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("source");
}

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in [
        (
            "client/services/RootService.ts",
            "@Service\nexport default class RootService { middle: MiddleService; count = 1; }",
        ),
        (
            "client/services/MiddleService.js",
            "@Service\nexport default class MiddleService { leaf; count = 2; }",
        ),
        (
            "client/services/LeafService.js",
            "@Service\nexport default class LeafService { count = 3; }",
        ),
        (
            "client/services/UnusedService.java",
            "@Service\nclass UnusedService { int count = 4; }",
        ),
        (
            "client/services/ShowcaseService.py",
            "@service\nclass ShowcaseService:\n    count: int = 5\n",
        ),
        (
            "client/components/reachable/tac.html",
            "<p>{root.count}</p>",
        ),
        ("client/components/other/tac.html", "<p>{unused.count}</p>"),
    ] {
        write(root.path(), relative, source);
    }
    root
}

fn render(root: &Path, view: &str) -> super::client::ClientRenderedView {
    let components = ComponentRegistry::discover(root).expect("components");
    let services = ServiceRegistry::discover(root).expect("services");
    let program = TemplateFrontend::compile(view, "client/pages/tac.html", &components.names())
        .expect("page");
    ClientViewRenderer::new(&components, &services, &BTreeSet::new())
        .render(&program, "index.html", &Scope::new(), "/")
        .expect("rendered page")
}

fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_route_catalog_keeps_transitive_undefined_fields_without_needs_all() {
    let root = project();
    let page = render(root.path(), "<reachable />");
    assert_eq!(page.services, names(&["root", "middle", "leaf"]));
    for class in ["RootService", "MiddleService", "LeafService"] {
        assert!(
            page.html
                .contains(&format!("/.tachyon/services/{class}.js"))
        );
    }
    assert!(!page.html.contains("UnusedService.js"));
    assert!(!page.html.contains("ShowcaseService.js"));
    assert!(page.html.contains(r#""scope":"singleton""#));
}

#[test]
fn explicitly_undefined_fields_and_constructor_assignments_keep_injected_services() {
    for source in [
        "@Service\nexport default class RootService { middle = undefined; count = 1; }",
        "@Service\nexport default class RootService { count = 1; constructor() { this.middle = undefined; } }",
        "@Service\nexport default class RootService { middle: MiddleService = undefined; count = 1; }",
    ] {
        let root = project();
        write(root.path(), "client/services/RootService.ts", source);
        let page = render(root.path(), "<reachable />");
        assert_eq!(
            page.services,
            names(&["root", "middle", "leaf"]),
            "{source}"
        );
        assert!(!page.html.contains("ShowcaseService.js"));
    }
}

#[test]
fn reflective_and_computed_property_writes_conservatively_keep_all_providers() {
    for body in [
        "this['middle'] = undefined",
        "const key = 'middle'; this[key] = undefined",
        "const key = 'middle'; this[key] ??= undefined",
        "Object.assign(this, values)",
        "Object.defineProperty(this, 'middle', { value: undefined })",
        "Reflect.set(this, key, undefined)",
    ] {
        let root = project();
        let source = format!(
            "@Service\nexport default class RootService {{ count = 1; constructor(values) {{ {body}; }} }}"
        );
        write(root.path(), "client/services/RootService.ts", &source);
        let services = ServiceRegistry::discover(root.path()).expect("services");
        let page = render(root.path(), "<reachable />");
        assert_eq!(page.services, services.names(), "{body}");
    }
}

#[test]
fn comments_strings_and_computed_method_reads_do_not_force_unused_providers() {
    let root = project();
    write(
        root.path(),
        "client/services/RootService.ts",
        "@Service\nexport default class RootService { middle: MiddleService; count = 1; hint = 'showcase'; tap(key) { /* unused showcase */ return this[key]() === undefined; } }",
    );
    let page = render(root.path(), "<reachable />");
    assert_eq!(page.services, names(&["root", "middle", "leaf"]));
}

#[test]
fn conditionals_and_delayed_components_keep_every_potential_service() {
    let root = project();
    let page = render(
        root.path(),
        r#"<logic :if="false"><other hydrate="visible" /></logic><reachable hydrate="idle" />"#,
    );
    assert_eq!(page.services, names(&["root", "middle", "leaf", "unused"]));
    assert!(!page.html.contains("ShowcaseService.js"));
}

#[test]
fn event_only_service_use_keeps_the_dependency_closure_and_prototype_metadata() {
    let root = project();
    write(
        root.path(),
        "client/services/RootService.ts",
        "@Service\n@Lifetime({\"value\":\"prototype\"})\nexport default class RootService { middle: MiddleService; tap() {} }",
    );
    write(
        root.path(),
        "client/components/reachable/tac.html",
        r#"<button type="button" on:click="root.tap()">Tap</button>"#,
    );
    let page = render(root.path(), "<reachable />");
    assert_eq!(page.services, names(&["root", "middle", "leaf"]));
    assert!(page.html.contains(r#""scope":"prototype""#));
    assert!(!page.html.contains("ShowcaseService.js"));
}

#[test]
fn exported_elements_include_their_dependencies_without_the_global_registry() {
    let root = project();
    let components = ComponentRegistry::discover(root.path()).expect("components");
    let services = ServiceRegistry::discover(root.path()).expect("services");
    let mut used = BTreeSet::new();
    let definition = ClientViewRenderer::new(&components, &services, &BTreeSet::new())
        .element(
            "shop-reachable",
            components.get("reachable").expect("component"),
            &mut used,
        )
        .expect("export");
    let catalog = definition["services"].as_object().expect("catalog");
    assert_eq!(catalog.keys().cloned().collect::<BTreeSet<_>>(), used);
    assert_eq!(used, names(&["root", "middle", "leaf"]));
    assert_eq!(catalog["root"]["scope"], "singleton");
}

#[test]
fn dependency_cycles_terminate_and_pages_without_services_keep_an_empty_catalog() {
    let root = project();
    write(
        root.path(),
        "client/services/LeafService.js",
        "@Service\nexport default class LeafService { root; count = 3; }",
    );
    let services = ServiceRegistry::discover(root.path()).expect("services");
    assert_eq!(
        ServiceReach::expand(&services, &names(&["root"])),
        names(&["root", "middle", "leaf"])
    );
    let page = render(root.path(), "<p>Static content</p>");
    assert!(page.services.is_empty());
    assert!(page.html.contains(r#""services":{}"#));
}

#[test]
fn unreadable_dependency_sources_conservatively_keep_the_registry() {
    let root = project();
    let services = ServiceRegistry::discover(root.path()).expect("services");
    fs::remove_file(root.path().join("client/services/MiddleService.js"))
        .expect("remove test source");
    assert_eq!(
        ServiceReach::expand(&services, &names(&["root"])),
        services.names()
    );
}

#[test]
fn an_unreachable_runtime_does_not_consume_the_unchanged_route_budget() {
    let root = project();
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"budgets":{"check":true,"script":320,"webassembly":3584}}}"#,
    );
    let page = render(root.path(), "<reachable />");
    let mut files = vec![
        (PathBuf::from("index.html"), page.html.into_bytes()),
        (PathBuf::from(".tachyon/tac-client.js"), Vec::new()),
        (
            PathBuf::from(".tachyon/services/UnusedService.js"),
            b"import './java/services.wasm'".to_vec(),
        ),
        (
            PathBuf::from(".tachyon/services/java/services.wasm"),
            vec![0; 4 * 1_024 * 1_024],
        ),
    ];
    for class in ["RootService", "MiddleService", "LeafService"] {
        files.push((
            PathBuf::from(format!(".tachyon/services/{class}.js")),
            Vec::new(),
        ));
    }
    crate::compiler::BudgetCheck::verify(root.path(), &files, true, &|_| None)
        .expect("unused language is outside this route");
    files[0].1 = render(root.path(), "<other />").html.into_bytes();
    let failure = crate::compiler::BudgetCheck::verify(root.path(), &files, true, &|_| None)
        .expect_err("the same runtime exceeds the budget when actually referenced");
    assert!(
        failure
            .diagnostics()
            .iter()
            .any(|problem| problem.code.to_string() == "TY1337")
    );
}
