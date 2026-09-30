#![allow(clippy::expect_used)]
use std::path::{Path, PathBuf};
use std::process::Command;
use tachyon_core::{BuildOptions, WebCompiler};

struct TacMapping;

#[cfg(test)]
impl TacMapping {
    fn service(language: &str) -> &'static str {
        match language {
            "java" => {
                "import models.Product;\nimport models.Detail;\nimport models.Summary;\n@Service\nclass CatalogService {\n@Needs\nProductConverter converter;\npublic Summary read() {\nDetail detail = new Detail(\"detail\", \"inner\");\nreturn converter.map(converter.copy(new Product(7, \"Anvil\", detail, java.util.List.of(detail), \"hidden\")));\n}\n}\n"
            }
            "kt" => {
                "import models.Product\nimport models.Detail\nimport models.Summary\n@Service\nclass CatalogService {\n@Needs\nval converter: ProductConverter\nfun read(): Summary {\nval detail = Detail(\"detail\", \"inner\")\nreturn converter.map(converter.copy(Product(7, \"Anvil\", detail, listOf(detail), \"hidden\")))\n}\n}\n"
            }
            "cs" => {
                "using Models;\n[Service]\npublic class CatalogService {\n[Needs]\nProductConverter _converter;\npublic Summary Read() {\nvar detail = new Detail(\"detail\", \"inner\");\nreturn _converter.map(_converter.copy(new Product(7, \"Anvil\", detail, new[] {detail}, \"hidden\")));\n}\n}\n"
            }
            "fs" => {
                "open Models\n[<Service>]\n[<NeedsAll>]\ntype CatalogService(converter: ProductConverter) =\n    member this.Read() : Summary =\n        let detail : Detail = { name = \"detail\"; secret = \"hidden\" }\n        let source : Product = { id = 7; name = \"Anvil\"; detail = detail; items = [detail]; secret = \"hidden\" }\n        converter.map(converter.copy(source))\n"
            }
            _ => Self::other_service(language),
        }
    }

    fn other_service(language: &str) -> &'static str {
        match language {
            "go" => {
                "// @Service\ntype CatalogService struct {\n// @Needs\nconverter *ProductConverter\n}\nfunc (service *CatalogService) Read() (Summary, error) { return service.readMapped() }\nfunc (service *CatalogService) readMapped() (Summary, error) {\ndetail := Detail{Name: \"detail\", Secret: \"hidden\"}\nproduct, failure := service.converter.Copy(Product{ID: 7, Name: \"Anvil\", Detail: detail, Items: []Detail{detail}, Secret: \"hidden\"})\nif failure != nil { return Summary{}, failure }\nreturn service.converter.Map(product)\n}\n"
            }
            "rs" => {
                "use crate::models::{Product, Detail, Summary};\n#[Service]\n#[derive(Default)]\nstruct CatalogService {\n#[Needs]\nconverter: std::sync::Arc<ProductConverter>,\n}\nimpl CatalogService {\npub fn read(&self) -> Summary { self.read_mapped().expect(\"mapping\") }\nfn read_mapped(&self) -> BehaviorResult<Summary> {\nlet detail = Detail{name: \"detail\".into(), secret: \"hidden\".into()};\nlet source = self.converter.copy(Product{id:7, name:\"Anvil\".into(), detail:detail.clone(), items:vec![detail], secret:\"hidden\".into()})?;\nself.converter.map(source)\n}\n}\n"
            }
            "dart" => {
                "import \"models/Product.dart\";\nimport \"models/Detail.dart\";\nimport \"models/Summary.dart\";\n@Service\nclass CatalogService {\n@Needs\nfinal ProductConverter converter;\nSummary read() {\nconst detail = Detail(\"detail\", \"inner\");\nreturn converter.map(converter.copy(Product(7, \"Anvil\", detail, [detail], \"hidden\")));\n}\n}\n"
            }
            _ => panic!("unknown language"),
        }
    }

    fn converter(language: &str, source: &str) -> String {
        let (original, replacement) = match language {
            "java" | "cs" | "dart" => (
                "Summary map(Product product);",
                "Summary map(Product product);\nProduct copy(Product product);",
            ),
            "kt" => (
                "fun map(product: Product): Summary",
                "fun map(product: Product): Summary\nfun copy(product: Product): Product",
            ),
            "go" => (
                "Map(product Product) (Summary, error)",
                "Map(product Product) (Summary, error)\nCopy(product Product) (Product, error)",
            ),
            "rs" => (
                "fn map(&self, product: Product) -> BehaviorResult<Summary>;",
                "fn map(&self, product: Product) -> BehaviorResult<Summary>;\nfn copy(&self, product: Product) -> BehaviorResult<Product>;",
            ),
            "fs" => (
                "abstract map: product: Product -> Summary",
                "abstract map: product: Product -> Summary\n    abstract copy: product: Product -> Product",
            ),
            _ => panic!("unknown language"),
        };
        source.replace(original, replacement)
    }

    fn project(root: &Path, language: &str) {
        let (product, detail, summary, short, converter, _) = super::mapping_sources(language);
        for (name, source) in [
            ("Product", product),
            ("Detail", detail),
            ("Summary", summary),
            ("ShortDetail", short),
        ] {
            super::write(root, &format!("client/models/{name}.{language}"), source);
        }
        let converter = Self::converter(language, converter);
        super::write(
            root,
            &format!("client/converters/ProductConverter.{language}"),
            &converter,
        );
        super::write(
            root,
            &format!("client/services/CatalogService.{language}"),
            Self::service(language),
        );
        super::write(root, "client/pages/tac.html", "<catalog-panel />");
        super::write(
            root,
            "client/components/catalog/panel/tac.html",
            "<button on:click=\"catalog.read()\">Map</button>",
        );
    }

    fn output(root: &Path) -> PathBuf {
        root.join("dist/.tachyon/services")
    }

    fn execute(root: &Path) -> serde_json::Value {
        let output = Self::output(root);
        let driver = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
const { default: CatalogService } = await import('./CatalogService.js')
const service = new CatalogService({})
const result = await service.read()
console.log(JSON.stringify(result))
process.exit(0)
";
        std::fs::write(output.join("package.json"), "{\"type\":\"module\"}").expect("package");
        std::fs::write(output.join("drive.mjs"), driver).expect("driver");
        let result = Command::new("node")
            .arg("drive.mjs")
            .current_dir(&output)
            .output()
            .expect("Node");
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&result.stdout).expect("JSON")
    }

    fn save_fixture(root: &Path, language: &str) {
        let Some(output) = std::env::var_os("TAC_MAPPING_FIXTURES") else {
            return;
        };
        let output = PathBuf::from(output).join(language);
        for folder in [
            "models",
            "converters",
            "services",
            "pages",
            "components/catalog/panel",
        ] {
            let source = root.join("client").join(folder);
            let destination = output.join("client").join(folder);
            std::fs::create_dir_all(&destination).expect("evidence folder");
            for entry in std::fs::read_dir(source).expect("fixture folder").flatten() {
                if entry.path().is_file() {
                    std::fs::copy(entry.path(), destination.join(entry.file_name()))
                        .expect("evidence source");
                }
            }
        }
    }

    async fn assert(language: &str) {
        let root = tempfile::tempdir().expect("project");
        Self::project(root.path(), language);
        Self::save_fixture(root.path(), language);
        let options = BuildOptions {
            output_directory: root.path().join("dist"),
            incremental: false,
            ..BuildOptions::default()
        };
        WebCompiler::build_async(root.path(), &options)
            .await
            .unwrap_or_else(|failure| panic!("{language}: {failure}"));
        let actual = Self::execute(root.path());
        assert_eq!(
            actual,
            serde_json::json!({"id":7,"name":"Anvil","kind":"product","detail":{"name":"detail"},"items":[{"name":"detail"}]}),
            "{language}"
        );
    }
}

#[tokio::test]
async fn tac_implicit_mapping_java_compiles_and_executes() {
    TacMapping::assert("java").await;
}

#[tokio::test]
async fn tac_implicit_mapping_kt_compiles_and_executes() {
    TacMapping::assert("kt").await;
}

#[tokio::test]
async fn tac_implicit_mapping_cs_compiles_and_executes() {
    TacMapping::assert("cs").await;
}

#[tokio::test]
async fn tac_implicit_mapping_fs_compiles_and_executes() {
    TacMapping::assert("fs").await;
}

#[tokio::test]
async fn tac_implicit_mapping_go_compiles_and_executes() {
    TacMapping::assert("go").await;
}

#[tokio::test]
async fn tac_implicit_mapping_rs_compiles_and_executes() {
    TacMapping::assert("rs").await;
}

#[tokio::test]
async fn tac_implicit_mapping_dart_compiles_and_executes() {
    TacMapping::assert("dart").await;
}
