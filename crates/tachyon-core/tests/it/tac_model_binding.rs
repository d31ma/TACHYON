#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;
use tachyon_core::{BuildOptions, WebCompiler};

struct ShopPage;

impl ShopPage {
    const PAGE: (&str, &str) = ("client/pages/tac.html", "<shop-view hydrate=\"load\" />\n");
    const VIEW: (&str, &str) = (
        "client/components/shop/view/tac.html",
        "<p>{shop.product.name} by {shop.product.maker.name}</p>\n<loop :for=\"item of shop.products\">\n  <button on:click=\"shop.choose(item)\">{item.name} from {item.maker.name}</button>\n</loop>\n<p>{shop.picked}</p>\n",
    );
    const DART: [(&str, &str); 3] = [
        (
            "client/models/Maker.dart",
            "class Maker {\n  final String name;\n\n  const Maker(this.name);\n}\n",
        ),
        (
            "client/models/Product.dart",
            "import 'Maker.dart';\n\nclass Product {\n  final String name;\n  final Maker maker;\n\n  const Product(this.name, this.maker);\n}\n",
        ),
        (
            "client/services/ShopService.dart",
            "import 'models/Maker.dart';\nimport 'models/Product.dart';\n\n@Service\nclass ShopService {\n  String picked = '';\n  Product product = const Product('Anvil', Maker('Acme'));\n  List<Product> products = [const Product('Rope', Maker('Knots'))];\n\n  void choose(Product product) => picked = product.maker.name;\n}\n",
        ),
    ];
    const RUST: [(&str, &str); 3] = [
        (
            "client/models/Maker.rs",
            "#[derive(Clone, Debug)]\npub struct Maker {\n    pub name: String,\n}\n",
        ),
        (
            "client/models/Product.rs",
            "use crate::models::Maker;\n\n#[derive(Clone, Debug)]\npub struct Product {\n    pub name: String,\n    pub maker: Maker,\n}\n",
        ),
        (
            "client/services/ShopService.rs",
            "use crate::models::{Maker, Product};\n\n#[Service]\npub struct ShopService {\n    pub picked: String,\n    pub product: Product,\n    pub products: Vec<Product>,\n}\n\nimpl ShopService {\n    pub fn new() -> Self {\n        let product = Product { name: String::from(\"Anvil\"), maker: Maker { name: String::from(\"Acme\") } };\n        Self { picked: String::new(), products: vec![product.clone()], product }\n    }\n\n    pub fn choose(&mut self, product: Product) {\n        self.picked = product.maker.name;\n    }\n}\n",
        ),
    ];
    const CSHARP: [(&str, &str); 3] = [
        (
            "client/models/Maker.cs",
            "public record Maker(string Name);\n",
        ),
        (
            "client/models/Product.cs",
            "public record Product(string Name, Maker Maker);\n",
        ),
        (
            "client/services/ShopService.cs",
            "using Models;\n\n[Service]\npublic class ShopService\n{\n    public string Picked = \"\";\n    public Product Product = new Product(\"Anvil\", new Maker(\"Acme\"));\n    public List<Product> Products = new List<Product> { new Product(\"Rope\", new Maker(\"Knots\")) };\n\n    public void Choose(Product product) => Picked = product.Maker.Name;\n}\n",
        ),
    ];
    const FSHARP: [(&str, &str); 3] = [
        ("client/models/Maker.fs", "type Maker = { Name: string }\n"),
        (
            "client/models/Product.fs",
            "type Product = { Name: string; Maker: Maker }\n",
        ),
        (
            "client/services/ShopService.fs",
            "open Models\n\n[<Service>]\ntype ShopService() =\n    member val Picked : string = \"\" with get, set\n\n    member val Product : Product = { Name = \"Anvil\"; Maker = { Name = \"Acme\" } } with get, set\n\n    member val Products : Product list = [ { Name = \"Rope\"; Maker = { Name = \"Knots\" } } ] with get, set\n\n    member this.Choose(product: Product) : unit =\n        this.Picked <- product.Maker.Name\n",
        ),
    ];
}

struct SnippetPage;

impl SnippetPage {
    const PAGE: (&str, &str) = (
        "client/pages/tac.html",
        "<snippet-list :files=\"data.snippet.files\" :owner=\"data.snippet.owner\" />\n",
    );
    const DATA: (&str, &str) = (
        "client/shared/data/snippet.json",
        "{ \"files\": [\"index.js\"], \"owner\": \"Ada\" }\n",
    );
    const VIEW: (&str, &str) = (
        "client/components/snippet/list/tac.html",
        "<p>{snippet.owner}</p>\n<loop :for=\"file of snippet.files\">\n  <p>{file}</p>\n</loop>\n",
    );
    const FSHARP: (&str, &str) = (
        "client/services/SnippetService.fs",
        "[<Service>]\n[<NeedsAll>]\ntype SnippetService(files: string list, owner: string) =\n    member this.Files : string list = files\n\n    member this.Owner : string = owner\n",
    );
    const REQUIRED: &str = "\"required\":[\"files\",\"owner\"]";
}

fn runs(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn rust_web_ready() -> bool {
    runs(
        "rustc",
        &[
            "--print",
            "target-libdir",
            "--target",
            "wasm32-unknown-unknown",
        ],
    )
    .is_some_and(|folder| Path::new(folder.trim()).exists())
}

fn dotnet_web_ready() -> bool {
    runs("dotnet", &["workload", "list"]).is_some_and(|listed| listed.contains("wasm-tools"))
}

fn build(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in files {
        let target = root.path().join(relative);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
        std::fs::write(target, source).expect("source");
    }
    let options = BuildOptions {
        output_directory: root.path().join("dist"),
        ..BuildOptions::default()
    };
    WebCompiler::build(root.path(), &options).unwrap_or_else(|failure| panic!("{failure}"));
    root
}

fn shop(language: &[(&str, &str); 3]) -> tempfile::TempDir {
    build(&[
        ShopPage::PAGE,
        ShopPage::VIEW,
        language[0],
        language[1],
        language[2],
    ])
}

fn page_plan(root: &Path) -> String {
    std::fs::read_to_string(root.join("dist/index.html")).expect("page")
}

#[test]
fn a_dart_service_lists_its_model_and_list_of_models_fields_for_the_view() {
    if runs("dart", &["--version"]).is_none() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = shop(&ShopPage::DART);
    let module = std::fs::read_to_string(root.path().join("dist/.tachyon/services/ShopService.js"))
        .expect("the Dart service module");
    assert!(
        module.contains("const fields = [\"picked\",\"product\",\"products\"]"),
        "{module}"
    );
}

#[test]
fn a_rust_service_with_model_fields_builds_for_the_browser() {
    if !rust_web_ready() {
        eprintln!("skipping: the wasm32 Rust target is unavailable");
        return;
    }
    let root = shop(&ShopPage::RUST);
    assert!(page_plan(root.path()).contains("shop-view"));
}

#[test]
fn a_csharp_service_with_model_fields_builds_for_the_browser() {
    if !dotnet_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = shop(&ShopPage::CSHARP);
    assert!(page_plan(root.path()).contains("shop-view"));
}

#[test]
fn an_fsharp_service_with_model_fields_builds_for_the_browser() {
    if !dotnet_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = shop(&ShopPage::FSHARP);
    assert!(page_plan(root.path()).contains("shop-view"));
}

#[test]
fn an_fsharp_needs_all_service_takes_its_constructor_from_the_component_props() {
    if !dotnet_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload is unavailable");
        return;
    }
    let root = build(&[
        SnippetPage::PAGE,
        SnippetPage::DATA,
        SnippetPage::VIEW,
        SnippetPage::FSHARP,
    ]);
    let plan = page_plan(root.path());
    assert!(plan.contains(SnippetPage::REQUIRED), "{plan}");
}
