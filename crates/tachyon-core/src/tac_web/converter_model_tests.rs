#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

struct ConverterProject;

impl ConverterProject {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
const { default: CatalogService } = await import('./CatalogService.js')
const catalog = new CatalogService({})
catalog.load()
console.log(JSON.stringify({ summary: catalog.summary, copied: catalog.copied }))
process.exit(0)
";
    const RUBY: [(&'static str, &'static str); 7] = [
        (
            "client/models/Detail.rb",
            "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] secret\n#   @return [String]\nDetail = Data.define(:name, :secret)\n",
        ),
        (
            "client/models/ShortDetail.rb",
            "# @!attribute [r] name\n#   @return [String]\nShortDetail = Data.define(:name)\n",
        ),
        (
            "client/models/Product.rb",
            "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] detail\n#   @return [Detail]\n# @!attribute [r] items\n#   @return [Array<Detail>]\n# @!attribute [r] secret\n#   @return [String]\nProduct = Data.define(:id, :name, :detail, :items, :secret)\n",
        ),
        (
            "client/models/Summary.rb",
            "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] detail\n#   @return [ShortDetail]\n# @!attribute [r] items\n#   @return [Array<ShortDetail>]\n# @!attribute [r] kind\n#   @return [String]\nSummary = Data.define(:id, :name, :detail, :items, :kind)\n",
        ),
        (
            "client/models/Projection.rb",
            "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] detail\n#   @return [ShortDetail]\n# @!attribute [r] items\n#   @return [Array<ShortDetail>]\nProjection = Data.define(:id, :name, :detail, :items)\n",
        ),
        (
            "client/converters/ProductConverter.rb",
            r#"#[Converter]
class ProductConverter
  # @param product [Product]
  # @return [Summary]
  #[Convert({"fields":[{"target":"name","constant":"fixed"},{"target":"kind","constant":"public"}]})]
  def map(product)
  end

  # @param product [Product]
  # @return [Projection]
  def copy(product)
  end
end
"#,
        ),
        (
            "client/services/CatalogService.rb",
            r#"#[Service]
class CatalogService
  # @return [Summary]
  attr_reader :summary
  # @return [Projection]
  attr_reader :copied

  # @return [void]
  def load
    detail = Detail.new(name: "detail", secret: "hidden")
    product = Product.new(id: 7, name: "Anvil", detail: detail, items: [detail], secret: "hidden")
    converter = ProductConverter.new
    @summary = converter.map(product)
    @copied = converter.copy(product)
    raise "A converter must return its declared model" unless @summary.is_a?(Summary) && @copied.is_a?(Projection)
    raise "Nested values must be projected models" unless @summary.detail.is_a?(ShortDetail) && @summary.items.first.is_a?(ShortDetail)
  end
end
"#,
        ),
    ];
    const SWIFT: [(&'static str, &'static str); 7] = [
        (
            "client/models/Detail.swift",
            "struct Detail: Codable {\n    let name: String\n    let secret: String\n}\n",
        ),
        (
            "client/models/ShortDetail.swift",
            "struct ShortDetail: Codable {\n    let name: String\n}\n",
        ),
        (
            "client/models/Product.swift",
            "struct Product: Codable {\n    let id: Int\n    let name: String\n    let detail: Detail\n    let items: [Detail]\n    let secret: String\n}\n",
        ),
        (
            "client/models/Summary.swift",
            "struct Summary: Codable {\n    let id: Int\n    let name: String\n    let detail: ShortDetail\n    let items: [ShortDetail]\n    let kind: String\n}\n",
        ),
        (
            "client/models/Projection.swift",
            "struct Projection: Codable {\n    let id: Int\n    let name: String\n    let detail: ShortDetail\n    let items: [ShortDetail]\n}\n",
        ),
        (
            "client/converters/ProductConverter.swift",
            r#"@Converter
protocol ProductConverter {
    @Convert({"fields":[{"target":"name","constant":"fixed"},{"target":"kind","constant":"public"}]})
    func map(_ product: Product) throws -> Summary

    func copy(_ product: Product) throws -> Projection
}
"#,
        ),
        (
            "client/services/CatalogService.swift",
            r#"@Service
final class CatalogService {
    var summary: Summary? = nil
    var copied: Projection? = nil

    func load() throws {
        let detail = Detail(name: "detail", secret: "hidden")
        let product = Product(id: 7, name: "Anvil", detail: detail, items: [detail], secret: "hidden")
        let converter = ProductConverter()
        summary = try converter.map(product)
        copied = try converter.copy(product)
    }
}
"#,
        ),
    ];
    const JAVA: [(&'static str, &'static str); 7] = [
        (
            "client/models/Detail.java",
            "record Detail(String name, String secret) {}\n",
        ),
        (
            "client/models/ShortDetail.java",
            "record ShortDetail(String name) {}\n",
        ),
        (
            "client/models/Product.java",
            "import models.Detail;\nrecord Product(int id, String name, Detail detail, java.util.List<Detail> items, String secret) {}\n",
        ),
        (
            "client/models/Summary.java",
            "import models.ShortDetail;\nrecord Summary(int id, String name, ShortDetail detail, java.util.List<ShortDetail> items, String kind) {}\n",
        ),
        (
            "client/models/Projection.java",
            "import models.ShortDetail;\nrecord Projection(int id, String name, ShortDetail detail, java.util.List<ShortDetail> items) {}\n",
        ),
        (
            "client/converters/ProductConverter.java",
            r#"import models.Product;
import models.Projection;
import models.Summary;
@Converter
interface ProductConverter {
    @Convert({"fields":[{"target":"name","constant":"fixed"},{"target":"kind","constant":"public"}]})
    Summary map(Product product);

    Projection copy(Product product);
}
"#,
        ),
        (
            "client/services/CatalogService.java",
            r#"import models.Detail;
import models.Product;
import models.Projection;
import models.Summary;
@Service
class CatalogService {
    @Needs
    ProductConverter productConverter;
    Summary summary = null;
    Projection copied = null;

    void load() {
        Detail detail = new Detail("detail", "hidden");
        Product product = new Product(7, "Anvil", detail, java.util.List.of(detail), "hidden");
        summary = productConverter.map(product);
        copied = productConverter.copy(product);
    }
}
"#,
        ),
    ];

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    fn ready(language: WebLanguage) -> bool {
        match language {
            WebLanguage::Swift => crate::native::SwiftWebServices::sdk_installed(),
            WebLanguage::Java => super::java_tests::java_web_ready(),
            _ => Command::new("node")
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success()),
        }
    }

    fn write_sources(root: &Path, files: &[(&str, &str)]) {
        for (relative, source) in files {
            Self::write(root, relative, source);
        }
    }

    async fn report(language: WebLanguage, files: &[(&str, &str)]) -> Value {
        let root = tempfile::tempdir().expect("project");
        Self::write_sources(root.path(), files);
        Self::build(root.path(), language, files.last().expect("service").0).await;
        Self::drive(root.path())
    }

    async fn build(root: &Path, language: WebLanguage, source: &str) {
        let services = [(
            String::from("catalog"),
            String::from("CatalogService"),
            language,
            root.join(source),
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Release,
        )
        .await
        .expect("typed converter builds");
        Self::publish(root, built.files);
    }

    fn publish(root: &Path, files: Vec<(String, Vec<u8>)>) {
        for (name, bytes) in files {
            Self::write(root, &name, "");
            std::fs::write(root.join(name), bytes).expect("artifact");
        }
    }

    fn drive(root: &Path) -> Value {
        Self::write(root, "drive.mjs", Self::DRIVER);
        let output = Command::new("node")
            .arg("drive.mjs")
            .current_dir(root)
            .output()
            .expect("node driver");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default())
            .expect("typed projection report")
    }

    fn check(report: &Value) {
        let copied =
            json!({"id":7,"name":"Anvil","detail":{"name":"detail"},"items":[{"name":"detail"}]});
        let summary = json!({"id":7,"name":"fixed","detail":{"name":"detail"},"items":[{"name":"detail"}],"kind":"public"});
        assert_eq!(
            report["copied"], copied,
            "unannotated implicit projection: {report}"
        );
        assert_eq!(
            report["summary"], summary,
            "explicit override and nested list projection: {report}"
        );
    }
}

#[tokio::test]
async fn a_ruby_tac_converter_returns_models_with_overrides_nested_values_and_lists() {
    if !ConverterProject::ready(WebLanguage::Ruby) {
        eprintln!("skipping: node is unavailable");
        return;
    }
    ConverterProject::check(
        &ConverterProject::report(WebLanguage::Ruby, &ConverterProject::RUBY).await,
    );
}

#[tokio::test]
async fn a_swift_tac_converter_executes_with_overrides_nested_values_and_lists() {
    if !ConverterProject::ready(WebLanguage::Swift) {
        eprintln!("skipping: Swift WebAssembly SDK is unavailable");
        return;
    }
    ConverterProject::check(
        &ConverterProject::report(WebLanguage::Swift, &ConverterProject::SWIFT).await,
    );
}

#[tokio::test]
async fn a_java_web_service_converter_returns_records_with_overrides_nested_values_and_lists() {
    if !ConverterProject::ready(WebLanguage::Java) {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    ConverterProject::check(
        &ConverterProject::report(WebLanguage::Java, &ConverterProject::JAVA).await,
    );
}
