use super::visibility_off::VisibilityOff;
use super::{FileMove, MigrationFix, MigrationFixPlan};
use crate::HandlerSource;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct PlainHandler {
    extension: &'static str,
    source: &'static str,
    expected: &'static str,
}

impl PlainHandler {
    const ALL: [Self; 8] = [
        Self {
            extension: "js",
            source: "// @ts-check\n\nexport class Handler {\n  static GET(request) {\n    return { id: request.path }\n  }\n\n  static POST() {\n    return Handler.saved()\n  }\n\n  static saved() {\n    return { saved: true }\n  }\n}\n",
            expected: "// @ts-check\n\n@RequestRoute(\"/orders/{id}\")\n@Public\nexport class OrdersIdController {\n  @GetRoute\n  get(request) {\n    return { id: request.path }\n  }\n\n  @PostRoute\n  post() {\n    return OrdersIdController.saved()\n  }\n\n  static saved() {\n    return { saved: true }\n  }\n}\n",
        },
        Self {
            extension: "py",
            source: "class Handler:\n    __label = \"order\"\n\n    @staticmethod\n    def GET(request):\n        return {\"label\": Handler.__label}\n\n    @staticmethod\n    def DELETE():\n        return {}\n",
            expected: "@request_route(\"/orders/{id}\")\n@public\nclass OrdersIdController:\n    __label = \"order\"\n\n    @get_route\n    def get(self, request):\n        return {\"label\": OrdersIdController.__label}\n\n    @delete_route\n    def delete(self):\n        return {}\n",
        },
        Self {
            extension: "php",
            source: "<?php\nclass Handler {\n    public static function GET($request) {\n        return ['id' => 1];\n    }\n}\n",
            expected: "<?php\n#[RequestRoute(\"/orders/{id}\")]\n#[Public]\nclass OrdersIdController {\n    #[GetRoute]\n    public function get() {\n        return ['id' => 1];\n    }\n}\n",
        },
        Self {
            extension: "java",
            source: "public class Handler {\n    public static Object GET(YonRequest request) {\n        return \"ok\";\n    }\n}\n",
            expected: "@RequestRoute(\"/orders/{id}\")\n@Public\npublic class OrdersIdController {\n    @GetRoute\n    public Object get() {\n        return \"ok\";\n    }\n}\n",
        },
        Self {
            extension: "kt",
            source: "object Handler {\n    @JvmStatic\n    fun GET(request: YonRequest): Any? {\n        return mapOf(\"id\" to 1)\n    }\n}\n",
            expected: "@RequestRoute(\"/orders/{id}\")\n@Public\nclass OrdersIdController {\n    @GetRoute\n    fun get(): Any? {\n        return mapOf(\"id\" to 1)\n    }\n}\n",
        },
        Self {
            extension: "cs",
            source: "static class Handler {\n    public static object GET(YonRequest request) {\n        return new { id = 1 };\n    }\n}\n",
            expected: "[RequestRoute(\"/orders/{id}\")]\n[Public]\nclass OrdersIdController {\n    [GetRoute]\n    public object Get() {\n        return new { id = 1 };\n    }\n}\n",
        },
        Self {
            extension: "dart",
            source: "class Handler {\n  static Object? GET(YonRequest request) {\n    return {'id': 1};\n  }\n}\n",
            expected: "@RequestRoute(\"/orders/{id}\")\n@Public\nclass OrdersIdController {\n  @GetRoute\n  Object? get() {\n    return {'id': 1};\n  }\n}\n",
        },
        Self {
            extension: "go",
            source: "type Handler struct{}\n\nfunc (Handler) GET(request *YonRequest) any {\n\treturn map[string]any{\"id\": 1}\n}\n",
            expected: "// @RequestRoute(\"/orders/{id}\")\n// @Public\ntype OrdersIdController struct{}\n\n// @GetRoute\nfunc (OrdersIdController) Get() any {\n\treturn map[string]any{\"id\": 1}\n}\n",
        },
    ];
}

fn project() -> TempDir {
    let project = tempfile::tempdir().expect("project");
    VisibilityOff::write(project.path());
    project
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, contents).expect("file");
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).expect("file")
}

fn plan(root: &Path) -> MigrationFixPlan {
    MigrationFix::plan(root).expect("plan")
}

fn assert_routes(root: &Path, relative: &str, expected: &[(&str, &str)]) {
    let source = HandlerSource::discover(root, relative)
        .unwrap_or_else(|failure| panic!("{relative}: {failure}"));
    let routes: Vec<(String, String)> = source
        .routes()
        .iter()
        .map(|route| {
            (
                route.verb.clone(),
                route.pattern.clone().unwrap_or_default(),
            )
        })
        .collect();
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(verb, pattern)| (String::from(*verb), String::from(*pattern)))
        .collect();
    assert_eq!(routes, expected, "{relative}");
}

#[test]
fn a_plain_handler_becomes_a_controller_in_every_language_that_has_one() {
    for handler in PlainHandler::ALL {
        let project = project();
        let from = format!("server/routes/orders/_id/yon.{}", handler.extension);
        let to = format!(
            "server/controllers/OrdersIdController.{}",
            handler.extension
        );
        write(project.path(), &from, handler.source);
        let plan = plan(project.path());
        let text = plan.to_text(false);
        assert!(text.contains(&format!("move     {from} -> {to}")), "{text}");
        plan.write(project.path()).expect("write");
        assert_eq!(read(project.path(), &to), handler.expected, "{text}");
        assert!(!project.path().join("server/routes").exists(), "{text}");
        let verbs: &[(&str, &str)] = match handler.extension {
            "js" => &[("GET", "/orders/_id"), ("POST", "/orders/_id")],
            "py" => &[("GET", "/orders/_id"), ("DELETE", "/orders/_id")],
            _ => &[("GET", "/orders/_id")],
        };
        assert_routes(project.path(), &to, verbs);
    }
}

#[test]
fn old_controllers_move_and_empty_route_files_are_deleted() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/routes/products/yon.js",
        "// the controllers answer\n",
    );
    write(
        root,
        "server/routes/products/controllers/ProductsController.js",
        "@Controller\nclass ProductsController {\n  GET() {\n    return [{ id: 1 }]\n  }\n\n  HEAD() {\n    return {}\n  }\n}\n",
    );
    write(
        root,
        "server/routes/guides/controllers/GuideController.rs",
        "#[yon::controller]\nstruct GuideController {}\nimpl GuideController {\n    fn new() -> Self { Self {} }\n    fn GET(&self) -> Json {\n        Json::from(1)\n    }\n}\n",
    );
    let plan = plan(root);
    let text = plan.to_text(false);
    assert_eq!(plan.delete_count(), 1, "{text}");
    assert!(
        text.contains("delete   server/routes/products/yon.js"),
        "{text}"
    );
    assert!(
        text.contains("manual   server/controllers/ProductsController.js:7"),
        "{text}"
    );
    assert!(text.contains("Delete the HEAD method."), "{text}");
    plan.write(root).expect("write");
    assert_eq!(
        read(root, "server/controllers/ProductsController.js"),
        "@RequestRoute(\"/products\")\n@Public\nclass ProductsController {\n  @GetRoute\n  get() {\n    return [{ id: 1 }]\n  }\n\n  HEAD() {\n    return {}\n  }\n}\n"
    );
    assert_eq!(
        read(root, "server/controllers/GuideController.rs"),
        "#[RequestRoute(\"/guides\")]\n#[Public]\nstruct GuideController {}\nimpl GuideController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get(&self) -> Json {\n        Json::from(1)\n    }\n}\n"
    );
    assert!(!root.join("server/routes").exists());
}

#[test]
fn routes_the_fix_cannot_convert_safely_are_manual_steps() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/routes/cart/yon.rs",
        "struct Handler;\nimpl Handler {\n    fn GET(_request: &YonRequest) -> YonResponse {\n        YonResponse::empty(204)\n    }\n}\n",
    );
    write(
        root,
        "server/routes/basket/yon.js",
        "export class Handler {\n  static GET() { return this.items() }\n  static items() { return [] }\n}\n",
    );
    write(root, "server/routes/basket/OPTIONS.schema.json", "{}\n");
    write(root, "server/routes/basket/helper.cpp", "int x;\n");
    write(
        root,
        "server/routes/Bad_Name/yon.js",
        "export class Handler {\n  static GET() { return {} }\n}\n",
    );
    write(
        root,
        "server/routes/orders/yon.js",
        "export class Handler {\n  static GET() { return {} }\n}\n",
    );
    write(
        root,
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/x\")\nclass OrdersController {\n  @GetRoute\n  get() { return {} }\n}\n",
    );
    let plan = plan(root);
    let text = plan.to_text(false);
    assert_eq!(plan.move_count(), 0, "{text}");
    for expected in [
        "manual   server/routes/cart/yon.rs",
        "A Rust controller needs a struct",
        "manual   server/routes/basket/yon.js",
        "use 'this'",
        "manual   server/routes/basket/OPTIONS.schema.json",
        "Tachyon no longer reads OPTIONS.schema.json.",
        "manual   server/routes/basket/helper.cpp",
        "manual   server/routes/Bad_Name/yon.js",
        "manual   server/routes/orders/yon.js",
        "'server/controllers/OrdersController.js' already exists",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert!(
        !text.contains("inside server/routes, which Tachyon no longer reads"),
        "{text}"
    );
}

#[test]
fn a_moved_file_that_uses_relative_paths_gets_a_manual_step() {
    let project = project();
    write(
        project.path(),
        "server/routes/language/yon.py",
        "import os\n\nBASE = os.path.join(os.path.dirname(__file__), \"../data\")\n\n\nclass Handler:\n    @staticmethod\n    def GET(request):\n        return {\"base\": BASE}\n",
    );
    let text = plan(project.path()).to_text(false);
    assert!(
        text.contains("manual   server/controllers/LanguageController.py:3"),
        "{text}"
    );
    assert!(text.contains("it uses a relative path"), "{text}");
}

#[test]
fn the_write_guard_moves_and_deletes_only_route_files() {
    assert!(MigrationFix::is_allowed_move(&FileMove {
        from: String::from("server/routes/orders/yon.js"),
        to: String::from("server/controllers/OrdersController.js"),
    }));
    assert!(MigrationFix::is_allowed_move(&FileMove {
        from: String::from("server/routes/orders/controllers/ShopController.ts"),
        to: String::from("server/controllers/ShopController.ts"),
    }));
    for (from, to) in [
        (
            "server/routes/orders/yon.js",
            "server/controllers/ShopController.js",
        ),
        (
            "server/routes/orders/yon.js",
            "server/services/OrdersController.js",
        ),
        (
            "server/services/OrdersService.js",
            "server/controllers/OrdersService.js",
        ),
        (
            "server/routes/../yon.js",
            "server/controllers/Controller.js",
        ),
        (
            "server/routes/orders/notes.txt",
            "server/controllers/notes.txt",
        ),
    ] {
        let file_move = FileMove {
            from: String::from(from),
            to: String::from(to),
        };
        assert!(!MigrationFix::is_allowed_move(&file_move), "{from} -> {to}");
    }
    let project = project();
    write(project.path(), "server/services/OrdersService.js", "\n");
    let plan = MigrationFixPlan {
        deletes: vec![String::from("server/services/OrdersService.js")],
        ..MigrationFixPlan::default()
    };
    let failure = plan.write(project.path()).expect_err("not a route file");
    assert!(
        failure
            .to_string()
            .contains("is not a file that the fix may delete"),
        "{failure}"
    );
    write(project.path(), "server/routes/orders/yon.js", "\n");
    let stale = super::MigrationFix::plan(project.path()).expect("plan");
    write(
        project.path(),
        "server/routes/orders/yon.js",
        "export class Handler {}\n",
    );
    let failure = stale.write(project.path()).expect_err("stale delete");
    assert!(
        failure
            .to_string()
            .contains("changed after the fix was planned")
    );
    assert!(project.path().join("server/routes/orders/yon.js").exists());
}

struct OldClient {
    relative: &'static str,
    source: &'static str,
    expected: &'static str,
}

impl OldClient {
    const ALL: [Self; 5] = [
        Self {
            relative: "client/clients/CatalogClient.js",
            source: "@Client({\"cache\":\"no-store\"})\nclass CatalogClient {\n  @WebCall({\"client\":\"catalog\",\"method\":\"GET\",\"path\":\"/products\",\"query\":{\"page\":0},\"headers\":{\"X-Tenant\":1}})\n  async list(page, tenant) {}\n\n  @WebCall({\"client\":\"catalog\",\"method\":\"POST\",\"path\":\"/products\",\"body\":0})\n  async add(product) {}\n}\n",
            expected: "@HTTPClient\nclass CatalogClient {\n  @GetRequest(\"/products\")\n  async list(@RequestParam(\"page\") page, @RequestHeader(\"X-Tenant\") tenant) {}\n\n  @PostRequest(\"/products\")\n  async add(@RequestBody product) {}\n}\n",
        },
        Self {
            relative: "server/clients/ProductsClient.py",
            source: "@client\nclass ProductsClient:\n    @web_call({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/products\",\"query\":{\"page\":0}})\n    async def list(self, page):\n        pass\n",
            expected: "from typing import Annotated\n@http_client({\"adapter\":\"api\"})\nclass ProductsClient:\n    @get_request(\"/products\")\n    async def list(self, page: Annotated[object, request_param(\"page\")]):\n        pass\n",
        },
        Self {
            relative: "server/services/CatalogService.java",
            source: "@Service\nclass CatalogService {\n    @WebCall({\"client\":\"catalog\",\"method\":\"GET\",\"path\":\"/products\"})\n    Object products() { return null; }\n}\n",
            expected: "@Service\n@HTTPClient\nclass CatalogService {\n    @GetRequest(\"/products\")\n    Object products() { return null; }\n}\n",
        },
        Self {
            relative: "server/clients/CatalogClient.go",
            source: "#[Client]\ntype CatalogClient struct{}\n\n#[WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/products\",\"query\":{\"page\":0}})]\nfunc (client *CatalogClient) List(page int) (any, error) {\n\treturn nil, nil\n}\n",
            expected: "// @HTTPClient({\"adapter\":\"api\"})\ntype CatalogClient struct{}\n\n// @GetRequest(\"/products\")\nfunc (client *CatalogClient) List(\n// @RequestParam(\"page\")\npage int) (any, error) {\n\treturn nil, nil\n}\n",
        },
        Self {
            relative: "server/clients/CatalogClient.rs",
            source: "#[Client]\nstruct CatalogClient {}\nimpl CatalogClient {\n    #[WebCall({\"client\":\"catalog\",\"method\":\"DELETE\",\"path\":\"/products\",\"body\":0})]\n    async fn remove(&self, product: Json) -> Json { Json::Null }\n}\n",
            expected: "#[HTTPClient]\nstruct CatalogClient {}\nimpl CatalogClient {\n    #[DeleteRequest(\"/products\")]\n    async fn remove(&self, #[RequestBody] product: Json) -> Json { Json::Null }\n}\n",
        },
    ];
}

#[test]
fn old_clients_and_web_calls_become_http_clients() {
    for client in OldClient::ALL {
        let project = project();
        write(project.path(), client.relative, client.source);
        let plan = plan(project.path());
        let text = plan.to_text(false);
        plan.write(project.path()).expect("write");
        assert_eq!(
            read(
                project.path(),
                &client.relative.replace("/clients/", "/remotes/")
            ),
            client.expected,
            "{text}"
        );
        let rewritten =
            crate::behavior::HttpClientSource::rewrite(Path::new(client.relative), client.expected);
        assert!(
            matches!(rewritten, Ok(Some(_))),
            "{}: {rewritten:?}",
            client.relative
        );
    }
}

#[test]
fn web_calls_the_fix_cannot_convert_are_manual_steps() {
    let project = project();
    write(
        project.path(),
        "client/clients/MixedClient.js",
        "@Client\nclass MixedClient {\n  @WebCall({\"client\":\"a\",\"method\":\"GET\",\"path\":\"/a\"})\n  async a() {}\n\n  @WebCall({\"client\":\"b\",\"method\":\"GET\",\"path\":\"/b\"})\n  async b() {}\n}\n",
    );
    write(
        project.path(),
        "client/clients/HeadClient.js",
        "@Client\nclass HeadClient {\n  @WebCall({\"client\":\"head\",\"method\":\"HEAD\",\"path\":\"/a\"})\n  async a() {}\n}\n",
    );
    let plan = plan(project.path());
    let text = plan.to_text(false);
    assert_eq!(plan.change_count(), 0, "{text}");
    assert!(text.contains("more than one WebCall client name"), "{text}");
    assert!(
        text.contains("a GET, POST, PUT, PATCH or DELETE method"),
        "{text}"
    );
}

#[test]
fn spring_names_that_were_never_ours_are_renamed() {
    let project = project();
    write(
        project.path(),
        "server/clients/OrdersClient.py",
        "@http_exchange(\"/api/orders\")\nclass OrdersClient:\n    @get_exchange(\"/{id}\")\n    async def one(self, @path_variable id):\n        pass\n",
    );
    plan(project.path()).write(project.path()).expect("write");
    assert_eq!(
        read(project.path(), "server/remotes/OrdersClient.py"),
        "from typing import Annotated\n@http_client(\"/api/orders\")\nclass OrdersClient:\n    @get_request(\"/{id}\")\n    async def one(self, id: Annotated[object, path_variable]):\n        pass\n"
    );
    plan(project.path())
        .write(project.path())
        .expect("write again");
    let contract = read(project.path(), "server/remotes/OrdersClient.py");
    assert!(
        contract.starts_with("from typing import Annotated, Protocol\n@http_client"),
        "{contract}"
    );
    assert!(
        contract.contains("class OrdersClient(Protocol):"),
        "{contract}"
    );
    assert!(contract.contains("path_variable]): ...\n"), "{contract}");
}

#[test]
fn a_destination_added_after_planning_preserves_every_source() {
    let project = project();
    let source = "class Handler {\n    static GET() { return {} }\n}\n";
    write(project.path(), "server/routes/cart/yon.js", source);
    let plan = plan(project.path());
    write(
        project.path(),
        "server/controllers/CartController.js",
        "occupied\n",
    );
    assert!(plan.write(project.path()).is_err());
    assert_eq!(read(project.path(), "server/routes/cart/yon.js"), source);
    assert_eq!(
        read(project.path(), "server/controllers/CartController.js"),
        "occupied\n"
    );
}

#[test]
fn an_unreadable_route_is_never_treated_as_an_empty_marker() {
    let project = project();
    write(project.path(), "server/routes/cart/yon.js", "");
    let path = project.path().join("server/routes/cart/yon.js");
    fs::write(&path, [0xff, 0xfe]).expect("bytes");
    let plan = plan(project.path());
    assert_eq!(plan.delete_count(), 0);
    assert_eq!(plan.manual_step_count(), 1);
    plan.write(project.path()).expect("no changes");
    assert_eq!(fs::read(path).expect("bytes"), [0xff, 0xfe]);
}

#[test]
fn controller_destinations_cannot_differ_only_by_case() {
    let project = project();
    for (folder, name) in [("one", "CartController"), ("two", "cartController")] {
        write(
            project.path(),
            &format!("server/routes/{folder}/controllers/{name}.js"),
            &format!("@Controller\nclass {name} {{\nGET() {{ return '{folder}' }}\n}}\n"),
        );
    }
    let plan = plan(project.path());
    assert_eq!(plan.move_count(), 1);
    assert_eq!(plan.manual_step_count(), 1);
    plan.write(project.path()).expect("safe move");
    assert!(
        project
            .path()
            .join("server/routes/two/controllers/cartController.js")
            .exists()
    );
    assert!(read(project.path(), "server/controllers/CartController.js").contains("'one'"));
}

#[test]
fn an_unreadable_old_controller_reports_a_manual_step() {
    let project = project();
    let relative = "server/routes/cart/controllers/CartController.js";
    write(project.path(), relative, "");
    fs::write(project.path().join(relative), [0xff, 0xfe]).expect("bytes");
    let plan = plan(project.path());
    assert_eq!(plan.move_count(), 0);
    assert_eq!(plan.manual_step_count(), 1);
    plan.write(project.path()).expect("no changes");
    assert_eq!(
        fs::read(project.path().join(relative)).expect("bytes"),
        [0xff, 0xfe]
    );
}

#[cfg(unix)]
#[test]
fn a_linked_destination_folder_is_refused_before_any_write() {
    let outside = project();
    let project = project();
    let source = "class Handler {\n    static GET() { return {} }\n}\n";
    write(project.path(), "server/routes/cart/yon.js", source);
    let plan = plan(project.path());
    std::os::unix::fs::symlink(outside.path(), project.path().join("server/controllers"))
        .expect("link");
    assert!(plan.write(project.path()).is_err());
    assert_eq!(read(project.path(), "server/routes/cart/yon.js"), source);
    assert!(!outside.path().join("CartController.js").exists());
}

#[test]
fn needs_all_field_lists_are_removed() {
    let project = project();
    write(
        project.path(),
        "server/services/MoneyService.ts",
        "@Service()\n@NeedsAll('right', 'left')\nexport class MoneyService {\n  right!: string\n  left!: string\n}\n",
    );
    plan(project.path()).write(project.path()).expect("write");
    assert_eq!(
        read(project.path(), "server/services/MoneyService.ts"),
        "@Service()\n@NeedsAll()\nexport class MoneyService {\n  right!: string\n  left!: string\n}\n"
    );
}

#[test]
fn nullable_fields_that_tachyon_fills_become_non_null() {
    let project = project();
    write(
        project.path(),
        "client/services/PagerService.dart",
        "@Service\n@NeedsAll\nclass PagerService {\n  final String? path;\n  final Map<String, Object?>? documents;\n  String? note;\n}\n",
    );
    write(
        project.path(),
        "server/services/OrderService.ts",
        "@Service\nexport class OrderService {\n  @Needs\n  store?: OrderStore\n  label: string | null = null\n}\n",
    );
    plan(project.path()).write(project.path()).expect("write");
    assert_eq!(
        read(project.path(), "client/services/PagerService.dart"),
        "@Service\n@NeedsAll\nclass PagerService {\n  late final String path;\n  late final Map<String, Object?> documents;\n  String? note;\n}\n"
    );
    assert_eq!(
        read(project.path(), "server/services/OrderService.ts"),
        "@Service\nexport class OrderService {\n  @Needs\n  store!: OrderStore\n  label: string | null = null\n}\n"
    );
}
