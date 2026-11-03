#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};
use std::process::Command;

struct ShopProject {
    extension: &'static str,
    language: WebLanguage,
    maker: &'static str,
    product: &'static str,
    service: &'static str,
}

impl ShopProject {
    const RUST: Self = Self {
        extension: "rs",
        language: WebLanguage::Rust,
        maker: "#[derive(Clone, Debug)]\npub struct Maker {\n    pub name: String,\n}\n",
        product: "use crate::models::Maker;\n\n#[derive(Clone, Debug)]\npub struct Product {\n    pub name: String,\n    pub maker: Maker,\n}\n",
        service: "use crate::models::{Maker, Product};\n\n#[Service]\npub struct ShopService {\n    pub picked: String,\n    pub product: Product,\n    pub products: Vec<Product>,\n}\n\n\
             impl ShopService {\n    pub fn new() -> Self {\n        let made = |name: &str, maker: &str| Product { name: String::from(name), maker: Maker { name: String::from(maker) } };\n        \
             Self { picked: String::new(), product: made(\"Anvil\", \"Acme\"), products: vec![made(\"Rope\", \"Knots\"), made(\"Hook\", \"Iron\")] }\n    }\n\n    \
             pub fn choose(&mut self, product: Product) {\n        self.picked = product.maker.name;\n    }\n}\n",
    };

    const CSHARP: Self = Self {
        extension: "cs",
        language: WebLanguage::CSharp,
        maker: "public record Maker(string Name);\n",
        product: "public record Product(string Name, Maker Maker);\n",
        service: "using Models;\n\n[Service]\npublic class ShopService\n{\n    public string Picked = \"\";\n    public Product Product = new Product(\"Anvil\", new Maker(\"Acme\"));\n    \
             public List<Product> Products = new List<Product> { new Product(\"Rope\", new Maker(\"Knots\")), new Product(\"Hook\", new Maker(\"Iron\")) };\n\n    \
             public void Choose(Product product) => Picked = product.Maker.Name;\n}\n",
    };

    const FSHARP: Self = Self {
        extension: "fs",
        language: WebLanguage::CSharp,
        maker: "type Maker = { Name: string }\n",
        product: "type Product = { Name: string; Maker: Maker }\n",
        service: "open Models\n\n[<Service>]\ntype ShopService() =\n    member val Picked : string = \"\" with get, set\n\n    \
             member val Product : Product = { Name = \"Anvil\"; Maker = { Name = \"Acme\" } } with get, set\n\n    \
             member val Products : Product list = [ { Name = \"Rope\"; Maker = { Name = \"Knots\" } }; { Name = \"Hook\"; Maker = { Name = \"Iron\" } } ] with get, set\n\n    \
             member this.Choose(product: Product) : unit =\n        this.Picked <- product.Maker.Name\n",
    };

    const RUBY: Self = Self {
        extension: "rb",
        language: WebLanguage::Ruby,
        maker: "# @!attribute [r] name\n#   @return [String]\nMaker = Data.define(:name)\n",
        product: "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] maker\n#   @return [Maker]\nProduct = Data.define(:name, :maker)\n",
        service: "#[Service]\nclass ShopService\n  # @return [String]\n  attr_reader :picked\n  # @return [Product]\n  attr_accessor :product\n  # @return [Array<Product>]\n  attr_reader :products\n\n  \
             def initialize\n    @picked = \"\"\n    @product = Product.new(name: \"Anvil\", maker: Maker.new(name: \"Acme\"))\n    \
             @products = [Product.new(name: \"Rope\", maker: Maker.new(name: \"Knots\")), Product.new(name: \"Hook\", maker: Maker.new(name: \"Iron\"))]\n  end\n\n  \
             # @param product [Product]\n  # @return [void]\n  def choose(product)\n    @picked = product.maker.name\n  end\nend\n",
    };

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: ShopService } = await import('./ShopService.js')\n\
         const shop = new ShopService({})\n\
         const read = { product: shop.product, products: shop.products }\n\
         shop.choose(shop.products[0])\n\
         shop.product = shop.products[1]\n\
         console.log(JSON.stringify({ read, picked: shop.picked, after: shop.product }))\n\
         process.exit(0)\n";

    fn files(&self) -> [(String, &'static str); 3] {
        let extension = self.extension;
        [
            (format!("client/models/Maker.{extension}"), self.maker),
            (format!("client/models/Product.{extension}"), self.product),
            (
                format!("client/services/ShopService.{extension}"),
                self.service,
            ),
        ]
    }

    async fn report(&self) -> Option<serde_json::Value> {
        let service = TacService {
            language: self.language,
            instance: "shop",
            class: "ShopService",
            files: self.files().to_vec(),
        };
        service.report(Self::DRIVER).await
    }
}

struct TacService {
    language: WebLanguage,
    instance: &'static str,
    class: &'static str,
    files: Vec<(String, &'static str)>,
}

impl TacService {
    const FSHARP_PROPS: &str = "[<Service>]\n[<NeedsAll>]\ntype SnippetService(files: string list, owner: string) =\n    member this.Files : string list = files\n\n    member this.Owner : string = owner\n";
    const CSHARP_PROPS: &str = "[Service]\n[NeedsAll]\npublic class PagerService\n{\n    public string Path;\n    public int Size;\n    public int Turns = 0;\n\n    \
             public string Describe()\n    {\n        Turns += 1;\n        return Path + \":\" + Size + \":\" + Turns;\n    }\n}\n";
    const CSHARP_PROPS_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: PagerService } = await import('./PagerService.js')\n\
         const docs = new PagerService({ path: '/docs', size: 20 })\n\
         const shop = new PagerService({ path: '/shop', size: 5 })\n\
         const described = [await docs.describe(), await docs.describe(), await shop.describe()]\n\
         console.log(JSON.stringify({ docs: [docs.path, docs.size], shop: [shop.path, shop.size], described }))\n\
         process.exit(0)\n";
    const PROPS_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: SnippetService } = await import('./SnippetService.js')\n\
         const snippet = new SnippetService({ files: ['index.js', 'style.css'], owner: 'Ada' })\n\
         console.log(JSON.stringify({ files: snippet.files, owner: snippet.owner }))\n\
         process.exit(0)\n";

    fn ready(&self) -> bool {
        let node = NodeDriver::available();
        let tool = match self.language {
            WebLanguage::Ruby => crate::tac_ruby::RubyRuntimeBuild::unavailable_reason().is_none(),
            WebLanguage::Rust => Command::new("rustc")
                .args(["--print", "target-libdir", "--target"])
                .arg(crate::native::RustCompanion::WEB_TARGET)
                .output()
                .is_ok_and(|output| output.status.success()),
            _ => Command::new("dotnet")
                .args(["workload", "list"])
                .output()
                .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools")),
        };
        node && tool
    }

    async fn report(&self, driver: &str) -> Option<serde_json::Value> {
        if !self.ready() {
            eprintln!(
                "skipping: node or the web toolchain for {} is unavailable",
                self.class
            );
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        let mut source = PathBuf::new();
        for (relative, text) in &self.files {
            source = write(root.path(), relative, text);
        }
        let services = [(
            String::from(self.instance),
            String::from(self.class),
            self.language,
            source,
        )];
        let built = build_all(
            root.path(),
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Release,
        )
        .await
        .expect("the service builds");
        for (name, bytes) in built.files {
            write(root.path(), &name, "");
            std::fs::write(root.path().join(name), bytes).expect("publish");
        }
        Some(drive(root.path(), driver))
    }
}

fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
}

fn drive(root: &Path, script: &str) -> serde_json::Value {
    std::fs::write(root.join("drive.mjs"), script).expect("driver");
    let output =
        NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root)).expect("node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
}

fn assert_models_bind(report: &serde_json::Value) {
    let made =
        |name: &str, maker: &str| serde_json::json!({ "name": name, "maker": { "name": maker } });
    assert_eq!(report["read"]["product"], made("Anvil", "Acme"), "{report}");
    assert_eq!(
        report["read"]["products"],
        serde_json::json!([made("Rope", "Knots"), made("Hook", "Iron")]),
        "{report}"
    );
    assert_eq!(report["picked"], "Knots", "{report}");
    assert_eq!(report["after"], made("Hook", "Iron"), "{report}");
}

#[tokio::test]
async fn a_rust_service_binds_a_model_a_list_of_models_and_a_nested_model() {
    if let Some(report) = ShopProject::RUST.report().await {
        assert_models_bind(&report);
    }
}

#[tokio::test]
async fn a_csharp_service_binds_a_model_a_list_of_models_and_a_nested_model() {
    if let Some(report) = ShopProject::CSHARP.report().await {
        assert_models_bind(&report);
    }
}

#[tokio::test]
async fn an_fsharp_service_binds_a_model_a_list_of_models_and_a_nested_model() {
    if let Some(report) = ShopProject::FSHARP.report().await {
        assert_models_bind(&report);
    }
}

#[tokio::test]
async fn an_fsharp_needs_all_service_is_built_from_its_component_props() {
    let service = TacService {
        language: WebLanguage::CSharp,
        instance: "snippet",
        class: "SnippetService",
        files: vec![(
            String::from("client/services/SnippetService.fs"),
            TacService::FSHARP_PROPS,
        )],
    };
    if let Some(report) = service.report(TacService::PROPS_DRIVER).await {
        assert_eq!(
            report["files"],
            serde_json::json!(["index.js", "style.css"]),
            "{report}"
        );
        assert_eq!(report["owner"], "Ada", "{report}");
    }
}

#[tokio::test]
async fn a_csharp_needs_all_service_keeps_the_props_of_each_owner() {
    let service = TacService {
        language: WebLanguage::CSharp,
        instance: "pager",
        class: "PagerService",
        files: vec![(
            String::from("client/services/PagerService.cs"),
            TacService::CSHARP_PROPS,
        )],
    };
    if let Some(report) = service.report(TacService::CSHARP_PROPS_DRIVER).await {
        assert_eq!(report["docs"], serde_json::json!(["/docs", 20]), "{report}");
        assert_eq!(report["shop"], serde_json::json!(["/shop", 5]), "{report}");
        assert_eq!(
            report["described"],
            serde_json::json!(["/docs:20:1", "/docs:20:2", "/shop:5:1"]),
            "{report}"
        );
    }
}

#[tokio::test]
async fn a_ruby_handler_takes_a_nested_model_argument_and_a_model_field_takes_a_model() {
    if let Some(report) = ShopProject::RUBY.report().await {
        assert_models_bind(&report);
    }
}
