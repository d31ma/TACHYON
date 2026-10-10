use crate::compiler::tests::build_default;
use std::path::Path;

struct PageHeadFixture;

impl PageHeadFixture {
    const BLOCKING_SCRIPT: &'static str =
        r#"<script src="/shared/scripts/ThemeDefault.js" data-tachyon-runtime></script>"#;
    const RUNTIME_SCRIPT: &'static str =
        r#"<script type="module" src="/.tachyon/tac-client.js" data-tachyon-runtime></script>"#;
    const MODULE_SCRIPT: &'static str =
        r#"<script type="module" src="/shared/site.js" data-tachyon-runtime></script>"#;
    const STYLE_LINK: &'static str =
        r#"<link rel="stylesheet" href="/shared/site.css" data-tachyon-runtime>"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|error| panic!("directory: {error}"));
        std::fs::write(path, contents).unwrap_or_else(|error| panic!("file: {error}"));
    }

    fn home(manifest: &str) -> String {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
        Self::write(root.path(), "client/pages/tac.html", "<page-home />");
        Self::write(
            root.path(),
            "client/components/page/home/tac.html",
            "<main>Home</main>",
        );
        Self::write(root.path(), "client/manifest.json", manifest);
        Self::write(
            root.path(),
            "client/shared/scripts/ThemeDefault.js",
            "document.documentElement.setAttribute('w-theme', 'dark')\n",
        );
        Self::write(root.path(), "client/shared/site.css", "body { margin: 0; }");
        let built = build_default(root.path()).unwrap_or_else(|error| panic!("build: {error}"));
        std::fs::read_to_string(built.output_directory().join("index.html"))
            .unwrap_or_else(|error| panic!("home: {error}"))
    }

    fn position(html: &str, tag: &str) -> usize {
        html.find(tag)
            .unwrap_or_else(|| panic!("{tag} is missing from {html}"))
    }
}

#[test]
fn a_blocking_script_is_a_classic_script_placed_before_the_runtime() {
    let html = PageHeadFixture::home(
        r#"{"tac":{
            "blockingScripts":["/shared/scripts/ThemeDefault.js"],
            "scripts":["/shared/site.js"],
            "styles":["/shared/site.css"]
        }}"#,
    );
    let expected = format!(
        "{}{}",
        PageHeadFixture::BLOCKING_SCRIPT,
        PageHeadFixture::RUNTIME_SCRIPT
    );
    assert!(html.contains(&expected), "{html}");
    let blocking = PageHeadFixture::position(&html, PageHeadFixture::BLOCKING_SCRIPT);
    let runtime = PageHeadFixture::position(&html, PageHeadFixture::RUNTIME_SCRIPT);
    let style = PageHeadFixture::position(&html, PageHeadFixture::STYLE_LINK);
    let module = PageHeadFixture::position(&html, PageHeadFixture::MODULE_SCRIPT);
    let head_end = PageHeadFixture::position(&html, "</head>");
    assert!(blocking < runtime, "{html}");
    assert!(runtime < style, "{html}");
    assert!(style < module, "{html}");
    assert!(module < head_end, "{html}");
    assert_eq!(html.matches("<script src=").count(), 1, "{html}");
}

#[test]
fn blocking_scripts_keep_their_declared_order() {
    let html = PageHeadFixture::home(
        r#"{"tac":{"blockingScripts":["/shared/scripts/ThemeDefault.js","/shared/first.js?v=1&x=2"]}}"#,
    );
    let expected = format!(
        r#"{}<script src="/shared/first.js?v=1&amp;x=2" data-tachyon-runtime></script>{}"#,
        PageHeadFixture::BLOCKING_SCRIPT,
        PageHeadFixture::RUNTIME_SCRIPT
    );
    assert!(html.contains(&expected), "{html}");
}

#[test]
fn scripts_without_the_blocking_option_stay_deferred_modules() {
    let html = PageHeadFixture::home(r#"{"tac":{"scripts":["/shared/site.js"]}}"#);
    let runtime = PageHeadFixture::position(&html, PageHeadFixture::RUNTIME_SCRIPT);
    let module = PageHeadFixture::position(&html, PageHeadFixture::MODULE_SCRIPT);
    assert!(runtime < module, "{html}");
    assert!(!html.contains("<script src="), "{html}");
}
