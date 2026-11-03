use super::StaticFiles;
use crate::node_driver::NodeDriver;
use serde_json::json;

struct DevelopmentMark;

impl DevelopmentMark {
    const DOCUMENT: &'static str =
        "<!doctype html><html><head></head><body class=\"page\"><main></main></body></html>";
    const READ_BODY: &'static str = "import { readFileSync } from 'node:fs'\n\
         const body = readFileSync('document.html', 'utf8').match(/<body([^>]*)>/)[1]\n\
         const dataset = {}\n\
         for (const [, name, value] of body.matchAll(/\\sdata-([a-z0-9-]+)(?:=\"([^\"]*)\")?/g)) {\n\
         \x20 dataset[name.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase())] = value ?? ''\n\
         }\n\
         globalThis.location = { pathname: '/docs/' }\n\
         globalThis.document = { body: { dataset } }\n";
    const REPORT: &'static str = "console.log(JSON.stringify(pageContext()))\n";

    fn page_context() -> &'static str {
        let client = include_str!("../template/tac-client.js");
        let start = client.find("const pageContext = ").expect("pageContext");
        let length = client[start..]
            .find("const evaluate = ")
            .expect("pageContext end");
        &client[start..start + length]
    }

    fn read(document: &str) -> serde_json::Value {
        let root = tempfile::tempdir().expect("fixture");
        std::fs::write(root.path().join("document.html"), document).expect("document");
        let program = [Self::READ_BODY, Self::page_context(), Self::REPORT].concat();
        std::fs::write(root.path().join("read.mjs"), program).expect("program");
        let mut command = NodeDriver::command();
        command.arg("read.mjs").current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node read.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("page context")
    }
}

#[test]
fn a_view_reads_page_development_only_in_a_document_the_development_server_marked() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let marked = StaticFiles::inject_live_reload(DevelopmentMark::DOCUMENT, "<script></script>");
    assert_eq!(
        DevelopmentMark::read(&marked),
        json!({"path": "/docs", "development": true})
    );
    assert_eq!(
        DevelopmentMark::read(DevelopmentMark::DOCUMENT),
        json!({"path": "/docs", "development": false})
    );
}
