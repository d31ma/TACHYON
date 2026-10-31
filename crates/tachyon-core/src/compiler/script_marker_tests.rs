use super::script_named_needs_tests::ScriptNamedNeeds;
use super::{BuildOptions, WebCompiler};
use crate::one_test_per_case_tests::one_test_per_case;

struct ScriptMarkers;

impl ScriptMarkers {
    const LANGUAGES: [&'static str; 2] = ["js", "ts"];

    const CONVERTERS: [(&'static str, &'static str); 9] = [
        ("client/pages/tac.html", "<catalog-panel />"),
        (
            "client/components/catalog/panel/tac.html",
            "<p><button on:click=\"catalog.plain()\">plain</button><button on:click=\"catalog.typed()\">typed</button></p>",
        ),
        (
            "client/models/Detail.ts",
            "export interface Detail { name: string; secret: string }\n",
        ),
        (
            "client/models/Product.ts",
            "import type { Detail } from '/models/Detail.ts'\nexport interface Product { id: number; name: string; detail: Detail; items: Detail[]; secret: string }\n",
        ),
        (
            "client/models/ShortDetail.ts",
            "export interface ShortDetail { name: string }\n",
        ),
        (
            "client/models/Summary.ts",
            "import type { ShortDetail } from '/models/ShortDetail.ts'\nexport interface Summary { id: number; name: string; detail: ShortDetail; items: ShortDetail[]; kind: string }\n",
        ),
        (
            "client/converters/ProductConverter.ts",
            "import type { Product } from '/models/Product.ts'\nimport type { Summary } from '/models/Summary.ts'\n\n@Converter\nexport default class ProductConverter {\n  @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\n  async map(product: Product): Promise<Summary | void> {}\n\n  async copy(product: Product): Promise<Product | void> {}\n}\n",
        ),
        (
            "client/converters/CatalogConverter.ts",
            "@Converter\nexport default class CatalogConverter {\n  @Convert({\"fields\":[{\"source\":\"detail.id\",\"target\":\"productId\"},{\"source\":\"name\",\"target\":\"title\"},{\"source\":\"missing.deep\",\"target\":\"absent\"},{\"target\":\"kind\",\"constant\":\"product\"}]})\n  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonObject | void> {}\n}\n",
        ),
        (
            "client/services/CatalogService.ts",
            "import type { Product } from '/models/Product.ts'\nimport type { Summary } from '/models/Summary.ts'\n\n@Service\nexport default class CatalogService {\n  \
             private static readonly ID = 7;\n  private static readonly NAME = 'Anvil';\n  @Needs\n  productConverter!: ProductConverter;\n  @Needs\n  catalogConverter!: CatalogConverter;\n\n  \
             async typed(): Promise<Summary | void> {\n    const detail = { name: 'detail', secret: 'detail-private' };\n    \
             const product = await this.productConverter.copy({ id: CatalogService.ID, name: CatalogService.NAME, detail, items: [detail], secret: 'product-private' });\n    \
             return this.summaryOf(product);\n  }\n\n  \
             async plain(): Promise<BehaviorJsonObject | void> {\n    return this.catalogConverter.toSummary({ detail: { id: CatalogService.ID }, name: CatalogService.NAME, extra: 'dropped' });\n  }\n\n  \
             private async summaryOf(product: Product | void): Promise<Summary | void> {\n    return product ? this.productConverter.map(product) : undefined;\n  }\n}\n",
        ),
    ];

    const CONVERTER_DRIVER: &'static str = "const module = await import('./.tachyon/services/CatalogService.js')\n\
         const catalog = await module.__tachyon_create_owner__({}, new AbortController().signal, undefined)\n\
         console.log(JSON.stringify({ typed: await catalog.typed(), plain: await catalog.plain() }))\n";

    const LABEL_PAGE: (&'static str, &'static str) = (
        "client/pages/tac.html",
        "<label-panel :title=\"'A'\" :start=\"1\" />",
    );

    const TYPESCRIPT_LABEL: [(&'static str, &'static str); 3] = [
        (
            "client/components/label/panel/tac.html",
            "<p>{label.shown()} {label.clicks}</p>",
        ),
        (
            "client/delegates/TallyDelegate.ts",
            "@Delegate\nexport default class TallyDelegate {\n  start(): number {\n    return 3;\n  }\n}\n",
        ),
        (
            "client/services/LabelService.ts",
            "@Service\n@NeedsAll\nexport default class LabelService {\n  title!: string;\n  start!: number;\n  tallyDelegate!: TallyDelegate;\n  clicks = 0;\n\n  \
             shown(): string {\n    return `${this.title}:${this.start + this.tallyDelegate.start() + this.clicks}`;\n  }\n}\n",
        ),
    ];

    const LABEL_DRIVER: &'static str = "const module = await import('./.tachyon/services/LabelService.js')\n\
         const open = (properties) => module.__tachyon_create_owner__(properties, new AbortController().signal, undefined)\n\
         const first = await open({ title: 'A', start: 1, clicks: 9 })\n\
         const second = await open({ title: 'B', start: 10 })\n\
         console.log(JSON.stringify({ shown: [first.shown(), second.shown()], required: module.__tachyon_required_args__ }))\n";

    const JAVASCRIPT_LABEL: [(&'static str, &'static str); 3] = [
        (
            "client/components/label/panel/tac.html",
            "<p>{label.shown()} {label.clicks}</p>",
        ),
        (
            "client/delegates/TallyDelegate.js",
            "@Delegate\nexport default class TallyDelegate {\n  start() {\n    return 3;\n  }\n}\n",
        ),
        (
            "client/services/LabelService.js",
            "@Service\n@NeedsAll\nexport default class LabelService {\n  /** @type {string} */\n  title;\n  /** @type {number} */\n  start;\n  \
             /** @type {TallyDelegate} */\n  tallyDelegate;\n  clicks = 0;\n\n  \
             /** @returns {string} */\n  shown() {\n    return `${this.title}:${this.start + this.tallyDelegate.start() + this.clicks}`;\n  }\n}\n",
        ),
    ];

    const UNFILLED_PAGE: (&'static str, &'static str) =
        ("client/pages/tac.html", "<label-panel :title=\"'A'\" />");

    fn label(
        page: (&'static str, &'static str),
        extension: &str,
    ) -> Vec<(&'static str, &'static str)> {
        let files = match extension {
            "js" => Self::JAVASCRIPT_LABEL,
            "ts" => Self::TYPESCRIPT_LABEL,
            _ => panic!("no NeedsAll label uses .{extension}"),
        };
        [page].into_iter().chain(files).collect()
    }
}

async fn needs_all_service_takes_its_properties_and_its_helper_in(extension: &str) {
    let project = ScriptMarkers::label(ScriptMarkers::LABEL_PAGE, extension);
    let report = ScriptNamedNeeds::run(&project, ScriptMarkers::LABEL_DRIVER).await;
    assert_eq!(
        report["shown"],
        serde_json::json!(["A:4", "B:13"]),
        "{report}"
    );
    assert_eq!(
        report["required"],
        serde_json::json!(["title", "start", "tallyDelegate"]),
        "{report}"
    );
}

async fn needs_all_field_that_the_page_leaves_out_fails_the_build_in(extension: &str) {
    let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
    for (relative, source) in ScriptMarkers::label(ScriptMarkers::UNFILLED_PAGE, extension) {
        ScriptNamedNeeds::write(root.path(), relative, source);
    }
    let failure = WebCompiler::build_async(root.path(), &BuildOptions::default())
        .await
        .err()
        .unwrap_or_else(|| panic!("a page that leaves out a NeedsAll field must not build"));
    assert!(
        failure
            .to_string()
            .contains("LabelService needs `start`; pass :start=\"...\" to <label-panel>"),
        "{failure}"
    );
}

#[tokio::test]
async fn a_typescript_service_needs_converters_and_runs_their_mappings() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report =
        ScriptNamedNeeds::run(&ScriptMarkers::CONVERTERS, ScriptMarkers::CONVERTER_DRIVER).await;
    assert_eq!(
        report["typed"],
        serde_json::json!({"id": 7, "name": "Anvil", "detail": {"name": "detail"}, "items": [{"name": "detail"}], "kind": "product"}),
        "{report}"
    );
    assert_eq!(
        report["plain"],
        serde_json::json!({"productId": 7, "title": "Anvil", "absent": null, "kind": "product"}),
        "{report}"
    );
}

one_test_per_case!(
    a_script_needs_all_service_takes_its_properties_and_its_helper,
    async needs_all_service_takes_its_properties_and_its_helper_in,
    all ScriptMarkers::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_needs_all_field_that_the_page_leaves_out_fails_the_build,
    async needs_all_field_that_the_page_leaves_out_fails_the_build_in,
    all ScriptMarkers::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);
