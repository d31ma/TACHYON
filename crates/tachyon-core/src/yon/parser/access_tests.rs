use super::parse;
use crate::yon::plan::Route;
use crate::yon::route_access::RouteAccess;
use std::path::Path;

fn routes(file: &str, source: &str) -> Vec<Route> {
    parse(Path::new(file), source)
        .expect("parsed controller")
        .expect("managed controller")
        .plan
        .routes
}

fn fsharp_routes(file: &str, source: &str) -> Vec<Route> {
    crate::fsharp_source::FsharpView::run(Path::new(file), source, parse)
        .expect("an F# source")
        .expect("parsed controller")
        .expect("managed controller")
        .plan
        .routes
}

fn refusal(file: &str, source: &str) -> String {
    let Err(failure) = parse(Path::new(file), source) else {
        panic!("the parser accepted {source}");
    };
    format!("{failure:?}")
}

fn roles(names: &[&str]) -> RouteAccess {
    RouteAccess::Allow(names.iter().map(|name| String::from(*name)).collect())
}

#[test]
fn the_method_rule_wins_over_the_class_rule() {
    let found = routes(
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/orders\")\n@SignedIn\nclass OrdersController {\n  @GetRoute\n  @Public\n  getOrders() { return [] }\n  @PostRoute\n  postOrder() { return {} }\n  @DeleteRoute\n  @Allow(Admin, Editor)\n  deleteOrder() { return {} }\n}\n",
    );
    let access: Vec<RouteAccess> = found.into_iter().map(|route| route.access).collect();
    assert_eq!(
        access,
        vec![
            RouteAccess::Public,
            RouteAccess::SignedIn,
            roles(&["Admin", "Editor"])
        ]
    );
}

#[test]
fn a_route_without_a_rule_is_missing() {
    let found = routes(
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  getOrders() { return [] }\n}\n",
    );
    assert!(found[0].access.is_missing());
}

#[test]
fn every_language_writes_role_types() {
    for (file, source) in [
        (
            "server/controllers/OrdersController.java",
            "@RequestRoute(\"/orders\")\n@Allow({Admin.class, Editor.class})\nclass OrdersController {\n    @GetRoute\n    Object getOrders() { return null; }\n}\n",
        ),
        (
            "server/controllers/OrdersController.cs",
            "[RequestRoute(\"/orders\")]\n[Allow(typeof(Admin), typeof(Editor))]\nclass OrdersController {\n    [GetRoute]\n    public object GetOrders() { return null; }\n}\n",
        ),
        (
            "server/controllers/OrdersController.kt",
            "@RequestRoute(\"/orders\")\n@Allow(Admin::class, Editor::class)\nclass OrdersController {\n    @GetRoute\n    fun getOrders(): Any? { return null }\n}\n",
        ),
        (
            "server/controllers/OrdersController.py",
            "@request_route(\"/orders\")\n@allow(Admin, Editor)\nclass OrdersController:\n    @get_route\n    def get_orders(self):\n        return []\n",
        ),
        (
            "server/controllers/OrdersController.rs",
            "#[RequestRoute(\"/orders\")]\n#[Allow(Admin, Editor)]\nstruct OrdersController {}\nimpl OrdersController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get_orders(&self) -> String { String::new() }\n}\n",
        ),
    ] {
        let found = routes(file, source);
        assert_eq!(found[0].access, roles(&["Admin", "Editor"]), "{file}");
    }
}

#[test]
fn a_groovy_controller_writes_role_classes() {
    let found = routes(
        "server/controllers/OrdersController.groovy",
        "@RequestRoute('/orders')\n@Allow([Admin.class, Editor.class])\nclass OrdersController {\n    @GetRoute\n    Object getOrders() { return null }\n}\n",
    );
    assert_eq!(found[0].access, roles(&["Admin", "Editor"]));
}

#[test]
fn a_groovy_controller_writes_bare_role_names() {
    let found = routes(
        "server/controllers/OrdersController.groovy",
        "@RequestRoute('/orders')\n@Allow(Admin)\nclass OrdersController {\n    @GetRoute\n    Object getOrders() { return null }\n}\n",
    );
    assert_eq!(found[0].access, roles(&["Admin"]));
}

#[test]
fn a_groovy_controller_reads_every_access_rule() {
    let found = routes(
        "server/controllers/OrdersController.groovy",
        "@RequestRoute('/orders')\n@SignedIn\nclass OrdersController {\n    @GetRoute\n    @Public\n    Object getOrders() { return null }\n    @PostRoute\n    Object postOrder() { return null }\n    @DeleteRoute\n    @Allow(Admin.class)\n    Object deleteOrders() { return null }\n}\n",
    );
    let access: Vec<RouteAccess> = found.into_iter().map(|route| route.access).collect();
    assert_eq!(
        access,
        vec![RouteAccess::Public, RouteAccess::SignedIn, roles(&["Admin"])]
    );
}

#[test]
fn an_fsharp_controller_writes_role_types() {
    let found = fsharp_routes(
        "server/controllers/OrdersController.fs",
        "[<RequestRoute(\"/orders\")>]\n[<Allow(typeof<Admin>, typeof<Editor>)>]\ntype OrdersController() =\n    [<GetRoute>]\n    member this.GetOrders() : string = \"\"\n",
    );
    assert_eq!(found[0].access, roles(&["Admin", "Editor"]));
}

#[test]
fn an_fsharp_controller_reads_every_access_rule() {
    let found = fsharp_routes(
        "server/controllers/OrdersController.fs",
        "[<RequestRoute(\"/orders\")>]\n[<SignedIn>]\ntype OrdersController() =\n    [<GetRoute>]\n    [<Public>]\n    member this.GetOrders() : string = \"\"\n\n    [<PostRoute>]\n    member this.PostOrder() : string = \"\"\n\n    [<DeleteRoute>]\n    [<Allow(typeof<Admin>)>]\n    member this.DeleteOrders() : string = \"\"\n",
    );
    let access: Vec<RouteAccess> = found.into_iter().map(|route| route.access).collect();
    assert_eq!(
        access,
        vec![RouteAccess::Public, RouteAccess::SignedIn, roles(&["Admin"])]
    );
}

#[test]
fn an_fsharp_identity_provider_has_one_identify_method() {
    let found = fsharp_routes(
        "server/controllers/AppIdentityProvider.fs",
        "[<IdentityProvider>]\ntype AppIdentityProvider() =\n    member this.Identify([<FromHeader(\"x-test-user\")>] user: string) : string = user\n",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].method, "Identify");
    assert_eq!(found[0].verb, "GET");
}

#[test]
fn a_groovy_identity_provider_has_one_identify_method() {
    let found = routes(
        "server/controllers/AppIdentityProvider.groovy",
        "@IdentityProvider\nclass AppIdentityProvider {\n    String identify(@FromHeader('x-test-user') String user) { return user }\n}\n",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].method, "identify");
    assert_eq!(found[0].verb, "GET");
}

#[test]
fn a_rust_allow_attribute_is_not_an_access_rule() {
    let found = routes(
        "server/controllers/OrdersController.rs",
        "#[RequestRoute(\"/orders\")]\n#[Public]\nstruct OrdersController {}\nimpl OrdersController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get_orders(&self) -> String { String::new() }\n}\n#[allow(dead_code)]\nfn unused() {}\n",
    );
    assert_eq!(found[0].access, RouteAccess::Public);
}

#[test]
fn wrong_access_rules_are_refused_with_advice() {
    for (source, expected) in [
        (
            "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  @Public\n  @SignedIn\n  getOrders() { return [] }\n}\n",
            "A route has one access rule",
        ),
        (
            "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  @Public(Admin)\n  getOrders() { return [] }\n}\n",
            "@Public and @SignedIn take no arguments",
        ),
        (
            "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  @Allow(\"Admin\")\n  getOrders() { return [] }\n}\n",
            "@Allow takes one or more role types",
        ),
        (
            "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @Public\n  helper() { return [] }\n  @GetRoute\n  getOrders() { return [] }\n}\n",
            "go on the controller class or on a route method",
        ),
    ] {
        let message = refusal("server/controllers/OrdersController.js", source);
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn an_identity_provider_has_one_identify_method() {
    let found = routes(
        "server/controllers/AppIdentityProvider.js",
        "@IdentityProvider\nclass AppIdentityProvider {\n  identify(request) { return null }\n}\n",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].method, "identify");
    assert_eq!(found[0].verb, "GET");
    assert!(found[0].takes_request);
    let missing = refusal(
        "server/controllers/AppIdentityProvider.js",
        "@IdentityProvider\nclass AppIdentityProvider {\n  who(request) { return null }\n}\n",
    );
    assert!(
        missing.contains("exactly one method named identify"),
        "{missing}"
    );
    let public = refusal(
        "server/controllers/AppIdentityProvider.js",
        "@IdentityProvider\n@Public\nclass AppIdentityProvider {\n  identify(request) { return null }\n}\n",
    );
    assert!(public.contains("is not a route"), "{public}");
}
