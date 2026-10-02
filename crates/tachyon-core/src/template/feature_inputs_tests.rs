use super::{ClientViewRenderer, ComponentRegistry, Scope, ServiceRegistry, TemplateFrontend};
use std::collections::BTreeSet;
use std::path::Path;

struct FeatureInputsTests;

impl FeatureInputsTests {
    const SERVICE: &'static str =
        include_str!("../../../../website/client/services/FeatureService.kt");
}

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("feature project");
    for (relative, source) in [
        (
            "client/services/FeatureService.kt",
            FeatureInputsTests::SERVICE,
        ),
        (
            "client/components/docs/feature/tac.html",
            "<article>{feature.entry().title}</article>",
        ),
    ] {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }
    root
}

fn render(root: &Path, page: &str) -> Result<String, crate::Failure> {
    let services = ServiceRegistry::discover(root)?;
    let components = ComponentRegistry::discover(root)?;
    let program = TemplateFrontend::compile(page, "client/pages/tac.html", &components.names())?;
    let data = BTreeSet::from(["features".to_owned()]);
    ClientViewRenderer::new(&components, &services, &data)
        .render(&program, "index.html", &Scope::new(), "/")
        .map(|result| result.html)
}

#[test]
fn website_feature_inputs_are_required_separately_from_the_title_provider() {
    let root = project();
    let services = ServiceRegistry::discover(root.path()).expect("feature service");
    let feature = services.get("feature").expect("feature");
    assert_eq!(feature.required(), ["route", "catalogue"]);
    assert_eq!(feature.injected(), ["documentTitle"]);
    assert!(feature.is_prototype());
    let html = render(
        root.path(),
        "<docs-feature :route=\"page.path\" :catalogue=\"data.features\" hydrate=\"load\" />",
    )
    .expect("real page bindings supply both required inputs");
    assert!(
        html.contains(r#""required":["route","catalogue"]"#),
        "{html}"
    );
}

#[test]
fn website_feature_inputs_refuse_missing_or_differently_spelled_bindings() {
    let root = project();
    for (page, missing) in [
        ("<docs-feature :catalogue=\"data.features\" />", "route"),
        ("<docs-feature :route=\"page.path\" />", "catalogue"),
        (
            "<docs-feature :route-path=\"page.path\" :catalogue=\"data.features\" />",
            "route",
        ),
    ] {
        let failure = render(root.path(), page).expect_err("required input is missing");
        assert!(
            failure.to_string().contains(&format!("needs `{missing}`")),
            "{failure}"
        );
        assert!(
            failure.to_string().contains("client/pages/tac.html"),
            "{failure}"
        );
    }
}

#[test]
fn website_feature_construction_only_requires_the_captured_title_provider() {
    let path = Path::new("client/services/FeatureService.kt");
    let roles = crate::behavior::ServiceRoles::read(path, FeatureInputsTests::SERVICE)
        .expect("service role");
    let source = crate::behavior::lower(path, &roles.source, "tac")
        .expect("declared NeedsAll input syntax")
        .expect("lowered feature service");
    assert!(
        source.contains("constructor(documentTitle: DocumentTitle)"),
        "{source}"
    );
    assert!(!source.contains("constructor(route:"), "{source}");
    assert!(source.contains("lateinit var route: String"), "{source}");
    assert!(
        source.contains("lateinit var catalogue: FeatureCatalogue"),
        "{source}"
    );
}
