#![allow(clippy::expect_used)]
use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("fixture");
}

type MappingSources = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

fn mapping_sources(language: &str) -> MappingSources {
    match language {
        "ts" => (
            "import type { Detail } from '/models/Detail.ts'\nexport interface Product { id: number; name: string; detail: Detail; items: Detail[]; secret: string }\n",
            "export interface Detail { name: string; secret: string }\n",
            "import type { ShortDetail } from '/models/ShortDetail.ts'\nexport interface Summary { id: number; name: string; detail: ShortDetail; items: ShortDetail[]; kind: string }\n",
            "export interface ShortDetail { name: string }\n",
            "import type { Product } from '/models/Product.ts'\nimport type { Summary } from '/models/Summary.ts'\n@Converter\nclass ProductConverter {\n@Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nasync map(product: Product): Promise<Summary> {}\n}\n",
            "import type { Summary } from '/models/Summary.ts'\n@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController {\n@Needs\nconverter!: ProductConverter;\n@GetRoute\nasync getCatalog(): Promise<Summary> {\nconst detail = {name: 'detail', secret: 'hidden'};\nreturn this.converter.map({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});\n}\n}\n",
        ),
        "js" => (
            "/** @typedef {Object} Product\n * @property {number} id\n * @property {string} name\n * @property {Detail} detail\n * @property {Detail[]} items\n * @property {string} secret\n */\nexport {};\n",
            "/** @typedef {Object} Detail\n * @property {string} name\n * @property {string} secret\n */\nexport {};\n",
            "/** @typedef {Object} Summary\n * @property {number} id\n * @property {string} name\n * @property {ShortDetail} detail\n * @property {ShortDetail[]} items\n * @property {string} kind\n */\nexport {};\n",
            "/** @typedef {Object} ShortDetail\n * @property {string} name\n */\nexport {};\n",
            "@Converter\nclass ProductConverter {\n/** @param {Product} product\n * @returns {Summary} */\n@Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nasync map(product) {}\n}\n",
            "@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController {\n@Needs\nproductConverter;\n/** @returns {Summary} */\n@GetRoute\nasync getCatalog() {\nconst detail = {name: 'detail', secret: 'hidden'};\nreturn this.productConverter.map({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});\n}\n}\n",
        ),
        "py" => (
            "from dataclasses import dataclass\nfrom models import Detail\n@dataclass\nclass Product:\n    id: int\n    name: str\n    detail: Detail\n    items: list[Detail]\n    secret: str\n",
            "from dataclasses import dataclass\n@dataclass\nclass Detail:\n    name: str\n    secret: str\n",
            "from dataclasses import dataclass\nfrom models import ShortDetail\n@dataclass\nclass Summary:\n    id: int\n    name: str\n    detail: ShortDetail\n    items: list[ShortDetail]\n    kind: str\n",
            "from dataclasses import dataclass\n@dataclass\nclass ShortDetail:\n    name: str\n",
            "from typing import Protocol\nfrom models import Product, Summary\n@Converter\nclass ProductConverter(Protocol):\n    @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\n    def map(self, product: Product) -> Summary: ...\n",
            "from typing import Annotated\nfrom models import Product, Detail, Summary\n@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController:\n    converter: Annotated[ProductConverter, Needs]\n    @GetRoute\n    async def get_catalog(self) -> Summary:\n        detail = Detail('detail', 'hidden')\n        return self.converter.map(Product(7, 'Anvil', detail, [detail], 'hidden'))\n",
        ),
        "go" => (
            "type Product struct {\nID int `json:\"id\"`\nName string `json:\"name\"`\nDetail Detail `json:\"detail\"`\nItems []Detail `json:\"items\"`\nSecret string `json:\"secret\"`\n}\n",
            "type Detail struct {\nName string `json:\"name\"`\nSecret string `json:\"secret\"`\n}\n",
            "type Summary struct {\nID int `json:\"id\"`\nName string `json:\"name\"`\nDetail ShortDetail `json:\"detail\"`\nItems []ShortDetail `json:\"items\"`\nKind string `json:\"kind\"`\n}\n",
            "type ShortDetail struct {\nName string `json:\"name\"`\n}\n",
            "// @Converter\ntype ProductConverter interface {\n// @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nMap(product Product) (Summary, error)\n}\n",
            "// @RequestRoute(\"/catalog\")\n// @Public\ntype CatalogController struct {\n// @Needs\nconverter *ProductConverter\n}\n// @GetRoute\nfunc (controller *CatalogController) GetCatalog() (Summary, error) {\ndetail := Detail{Name: \"detail\", Secret: \"hidden\"}\nvalue, failure := controller.converter.Map(Product{ID: 7, Name: \"Anvil\", Detail: detail, Items: []Detail{detail}, Secret: \"hidden\"})\nif failure != nil { return Summary{}, YonResourceError{} }\nreturn value, nil\n}\n",
        ),
        "lua" => (
            "---@class Product\n---@field id integer\n---@field name string\n---@field detail Detail\n---@field items Detail[]\n---@field secret string\nProduct = {}\n",
            "---@class Detail\n---@field name string\n---@field secret string\nDetail = {}\n",
            "---@class Summary\n---@field id number\n---@field name string\n---@field detail ShortDetail\n---@field items ShortDetail[]\n---@field kind string\nSummary = {}\n",
            "---@class ShortDetail\n---@field name string\nShortDetail = {}\n",
            "-- @Converter\nProductConverter = {}\n---@param product Product\n---@return Summary\n-- @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nfunction ProductConverter:map(product)\nend\n",
            "-- @RequestRoute(\"/catalog\")\n-- @Public\nCatalogController = {\n-- @Needs\n---@type ProductConverter\nconverter = nil,\n}\n---@return Summary\n-- @GetRoute\nfunction CatalogController:get_catalog()\n  local detail = setmetatable({name = \"detail\", secret = \"detail-private\"}, Detail)\n  return self.converter:map(setmetatable({id = 7, name = \"Anvil\", detail = detail, items = {detail}, secret = \"product-private\"}, Product))\nend\n",
        ),
        "rb" => (
            "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] detail\n#   @return [Detail]\n# @!attribute [r] items\n#   @return [Array<Detail>]\n# @!attribute [r] secret\n#   @return [String]\nProduct = Data.define(:id, :name, :detail, :items, :secret)\n",
            "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] secret\n#   @return [String]\nDetail = Data.define(:name, :secret)\n",
            "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] detail\n#   @return [ShortDetail]\n# @!attribute [r] items\n#   @return [Array<ShortDetail>]\n# @!attribute [r] kind\n#   @return [String]\nSummary = Data.define(:id, :name, :detail, :items, :kind)\n",
            "# @!attribute [r] name\n#   @return [String]\nShortDetail = Data.define(:name)\n",
            "#[Converter]\nclass ProductConverter\n  # @param product [Product]\n  # @return [Summary]\n  #[Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})]\n  def map(product)\n  end\nend\n",
            "#[RequestRoute(\"/catalog\")]\n#[Public]\nclass CatalogController\n  #[Needs]\n  # @return [ProductConverter]\n  attr_reader :product_converter\n  # @return [Summary]\n  #[GetRoute]\n  def get_catalog\n    detail = Detail.new(name: \"detail\", secret: \"hidden\")\n    product_converter.map(Product.new(id: 7, name: \"Anvil\", detail: detail, items: [detail], secret: \"hidden\"))\n  end\nend\n",
        ),
        "php" => (
            "<?php\nuse Models\\Detail;\nclass Product {\npublic function __construct(public int $id, public string $name, public Detail $detail,\n/** @var list<Detail> */\npublic array $items, public string $secret) {}\n}\n",
            "<?php\nclass Detail {\npublic function __construct(public string $name, public string $secret) {}\n}\n",
            "<?php\nuse Models\\ShortDetail;\nclass Summary {\npublic function __construct(public int $id, public string $name, public ShortDetail $detail,\n/** @var list<ShortDetail> */\npublic array $items, public string $kind) {}\n}\n",
            "<?php\nclass ShortDetail {\npublic function __construct(public string $name) {}\n}\n",
            "<?php\nuse Models\\Product;\nuse Models\\Summary;\n#[Converter]\ninterface ProductConverter {\n#[Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})]\npublic function map(Product $product): Summary;\n}\n",
            "<?php\nuse Models\\Product;\nuse Models\\Detail;\nuse Models\\Summary;\n#[RequestRoute(\"/catalog\")]\n#[Public]\nclass CatalogController {\n#[Needs]\nprivate ProductConverter $converter;\n#[GetRoute]\npublic function getCatalog(): Summary {\n$detail = new Detail(\"detail\", \"hidden\");\nreturn $this->converter->map(new Product(7, \"Anvil\", $detail, [$detail], \"hidden\"));\n}\n}\n",
        ),
        "dart" => (
            "import 'Detail.dart';\nclass Product {\nfinal int id;\nfinal String name;\nfinal Detail detail;\nfinal List<Detail> items;\nfinal String secret;\nconst Product(this.id, this.name, this.detail, this.items, this.secret);\n}\n",
            "class Detail {\nfinal String name;\nfinal String secret;\nconst Detail(this.name, this.secret);\n}\n",
            "import 'ShortDetail.dart';\nclass Summary {\nfinal int id;\nfinal String name;\nfinal ShortDetail detail;\nfinal List<ShortDetail> items;\nfinal String kind;\nconst Summary(this.id, this.name, this.detail, this.items, this.kind);\n}\n",
            "class ShortDetail {\nfinal String name;\nconst ShortDetail(this.name);\n}\n",
            "import 'models/Product.dart';\nimport 'models/Summary.dart';\n@Converter\nabstract interface class ProductConverter {\n@Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nSummary map(Product product);\n}\n",
            "import 'models/Product.dart';\nimport 'models/Detail.dart';\nimport 'models/Summary.dart';\n@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController {\n@Needs\nfinal ProductConverter converter;\n@GetRoute\nSummary getCatalog() {\nconst detail = Detail('detail', 'hidden');\nreturn converter.map(Product(7, 'Anvil', detail, [detail], 'hidden'));\n}\n}\n",
        ),
        _ => compiled_mapping_sources(language),
    }
}

fn compiled_mapping_sources(language: &str) -> MappingSources {
    match language {
        "java" | "groovy" => (
            "import models.Detail;\nrecord Product(int id, String name, Detail detail, java.util.List<Detail> items, String secret) {}\n",
            "record Detail(String name, String secret) {}\n",
            "import models.ShortDetail;\nrecord Summary(int id, String name, ShortDetail detail, java.util.List<ShortDetail> items, String kind) {}\n",
            "record ShortDetail(String name) {}\n",
            "import models.Product;\nimport models.Summary;\n@Converter\ninterface ProductConverter {\n@Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nSummary map(Product product);\n}\n",
            "import models.Product;\nimport models.Detail;\nimport models.Summary;\n@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController {\n@Needs\nProductConverter converter;\n@GetRoute\nSummary getCatalog() {\nDetail detail = new Detail(\"detail\", \"hidden\");\nreturn converter.map(new Product(7, \"Anvil\", detail, java.util.List.of(detail), \"hidden\"));\n}\n}\n",
        ),
        "kt" => (
            "import models.Detail\ndata class Product(val id: Int, val name: String, val detail: Detail, val items: List<Detail>, val secret: String)\n",
            "data class Detail(val name: String, val secret: String)\n",
            "import models.ShortDetail\ndata class Summary(val id: Int, val name: String, val detail: ShortDetail, val items: List<ShortDetail>, val kind: String)\n",
            "data class ShortDetail(val name: String)\n",
            "import models.Product\nimport models.Summary\n@Converter\ninterface ProductConverter {\n@Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nfun map(product: Product): Summary\n}\n",
            "import models.Product\nimport models.Detail\nimport models.Summary\n@RequestRoute(\"/catalog\")\n@Public\nclass CatalogController {\n@Needs\nval converter: ProductConverter\n@GetRoute\nfun getCatalog(): Summary {\nval detail = Detail(\"detail\", \"hidden\")\nreturn converter.map(Product(7, \"Anvil\", detail, listOf(detail), \"hidden\"))\n}\n}\n",
        ),
        "cs" => (
            "public record Product(int id, string name, Detail detail, Detail[] items, string secret);\n",
            "public record Detail(string name, string secret);\n",
            "public record Summary(int id, string name, ShortDetail detail, ShortDetail[] items, string kind);\n",
            "public record ShortDetail(string name);\n",
            "using Models;\n[Converter]\npublic interface ProductConverter {\n[Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})]\nSummary map(Product product);\n}\n",
            "using Models;\n[RequestRoute(\"/catalog\")]\n[Public]\nclass CatalogController {\n[Needs]\nProductConverter _converter;\n[GetRoute]\npublic Summary GetCatalog() {\nvar detail = new Detail(\"detail\", \"hidden\");\nreturn _converter.map(new Product(7, \"Anvil\", detail, new[] {detail}, \"hidden\"));\n}\n}\n",
        ),
        "rs" => (
            "use crate::models::Detail;\n#[derive(Clone)]\npub struct Product {\npub id: i64,\npub name: String,\npub detail: Detail,\npub items: Vec<Detail>,\npub secret: String,\n}\n",
            "#[derive(Clone)]\npub struct Detail {\npub name: String,\npub secret: String,\n}\n",
            "use crate::models::ShortDetail;\n#[derive(Clone)]\npub struct Summary {\npub id: i64,\npub name: String,\npub detail: ShortDetail,\npub items: Vec<ShortDetail>,\npub kind: String,\n}\n",
            "#[derive(Clone)]\npub struct ShortDetail {\npub name: String,\n}\n",
            "use crate::models::*;\n#[Converter]\ntrait ProductConverter {\n#[Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})]\nfn map(&self, product: Product) -> BehaviorResult<Summary>;\n}\n",
            "use crate::models::{Product, Detail, Summary};\n#[RequestRoute(\"/catalog\")]\n#[Public]\nstruct CatalogController {\n#[Needs]\nconverter: std::sync::Arc<ProductConverter>,\n}\nimpl CatalogController {\n#[GetRoute]\nfn get_catalog(&self) -> Result<Summary, YonResourceError> {\nlet detail = Detail{name: \"detail\".into(), secret: \"hidden\".into()};\nself.converter.map(Product{id:7, name:\"Anvil\".into(), detail:detail.clone(), items:vec![detail], secret:\"hidden\".into()}).map_err(|_| YonResourceError)\n}\n}\n",
        ),
        "fs" => (
            "type Product = { id: int; name: string; detail: Detail; items: Detail list; secret: string }\n",
            "type Detail = { name: string; secret: string }\n",
            "type Summary = { id: int; name: string; detail: ShortDetail; items: ShortDetail list; kind: string }\n",
            "type ShortDetail = { name: string }\n",
            "open Models\n[<Converter>]\ntype ProductConverter =\n    [<Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})>]\n    abstract map: product: Product -> Summary\n",
            "open Models\n[<RequestRoute(\"/catalog\")>]\n[<Public>]\ntype CatalogController(converter: ProductConverter) =\n    [<GetRoute>]\n    member this.GetCatalog() : Summary =\n        let detail : Detail = { name = \"detail\"; secret = \"hidden\" }\n        converter.map({ id = 7; name = \"Anvil\"; detail = detail; items = [detail]; secret = \"hidden\" })\n",
        ),
        _ => panic!("unknown language"),
    }
}

fn project(root: &Path, language: &str) -> String {
    let (product, detail, summary, short, converter, controller) = mapping_sources(language);
    for (name, source) in [
        ("Product", product),
        ("Detail", detail),
        ("Summary", summary),
        ("ShortDetail", short),
    ] {
        write(root, &format!("server/models/{name}.{language}"), source);
    }
    write(
        root,
        &format!("server/converters/ProductConverter.{language}"),
        converter,
    );
    let relative = format!("server/controllers/CatalogController.{language}");
    write(root, &relative, controller);
    relative
}

async fn assert_mapping(language: &str) {
    let directory = tempfile::tempdir().expect("project");
    let relative = project(directory.path(), language);
    let source = HandlerSource::discover(directory.path(), &relative).expect("discovery");
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor");
    let answer = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("implicit-models", "/catalog", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invoke");
    assert!(answer.error.is_none(), "{language}: {:?}", answer.error);
    let actual: serde_json::Value =
        serde_json::from_str(&answer.body.expect("body").data).expect("JSON");
    assert_eq!(
        actual,
        serde_json::json!({"id":7,"name":"Anvil", "kind":"product", "detail":{"name":"detail"}, "items":[{"name":"detail"}]}),
        "{language}"
    );
}

#[tokio::test]
async fn implicit_mapping_typescript_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("ts").await;
}

#[tokio::test]
async fn implicit_mapping_python_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("py").await;
}

#[tokio::test]
async fn implicit_mapping_go_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("go").await;
}

#[tokio::test]
async fn implicit_mapping_java_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("java").await;
}

#[tokio::test]
async fn implicit_mapping_groovy_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("groovy").await;
}

#[tokio::test]
async fn implicit_mapping_kt_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("kt").await;
}

#[tokio::test]
async fn implicit_mapping_cs_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("cs").await;
}

#[tokio::test]
async fn implicit_mapping_rs_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("rs").await;
}

#[tokio::test]
async fn implicit_mapping_fs_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("fs").await;
}

#[tokio::test]
async fn implicit_mapping_rb_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("rb").await;
}

#[tokio::test]
async fn implicit_mapping_php_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("php").await;
}

#[tokio::test]
async fn implicit_mapping_dart_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("dart").await;
}

#[tokio::test]
async fn implicit_mapping_js_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("js").await;
}

#[tokio::test]
async fn implicit_mapping_lua_models_execute_with_overrides_and_nested_lists() {
    assert_mapping("lua").await;
}

#[cfg(test)]
mod tac_mapping;

struct LuaMappingDiagnostics;

impl LuaMappingDiagnostics {
    fn refuses_converter(source: &str, expected: &str) {
        let root = tempfile::tempdir().expect("project");
        let relative = project(root.path(), "lua");
        write(
            root.path(),
            "server/converters/ProductConverter.lua",
            source,
        );
        let failure = HandlerSource::discover(root.path(), &relative)
            .expect_err("invalid converter implementation");
        let message = failure.to_string();
        assert!(message.contains(expected), "{message}");
    }

    fn refuses(summary: &str, expected: &str) {
        let root = tempfile::tempdir().expect("project");
        let relative = project(root.path(), "lua");
        write(root.path(), "server/models/Summary.lua", summary);
        let failure =
            HandlerSource::discover(root.path(), &relative).expect_err("invalid converter model");
        let message = failure.to_string();
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn lua_mapping_names_missing_target_fields() {
    LuaMappingDiagnostics::refuses(
        "---@class Summary\n---@field absent string\n---@field kind string\nSummary = {}\n",
        "Summary has fields that Product does not have: 'absent'",
    );
}

#[test]
fn lua_mapping_refuses_incompatible_target_fields() {
    LuaMappingDiagnostics::refuses(
        "---@class Summary\n---@field id boolean\n---@field kind string\nSummary = {}\n",
        "Summary.id is true or false, but Product.id is a whole number",
    );
}

#[test]
fn lua_mapping_refuses_authored_converter_bodies() {
    LuaMappingDiagnostics::refuses_converter(
        "-- @Converter\nProductConverter = {}\n---@param product Product\n---@return Summary\nfunction ProductConverter:map(product)\n  return product\nend\n",
        "A Converter has no implementation",
    );
}

#[test]
fn lua_mapping_refuses_authored_converter_fields() {
    LuaMappingDiagnostics::refuses_converter(
        "-- @Converter\nProductConverter = {\n---@type string\nstate = \"hidden\",\n}\n---@param product Product\n---@return Summary\nfunction ProductConverter:map(product)\nend\n",
        "A Converter has no implementation",
    );
}
