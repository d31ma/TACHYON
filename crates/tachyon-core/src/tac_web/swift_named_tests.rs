#![allow(clippy::expect_used)]

use super::swift_tests::swift_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;

struct SwiftNamedNeeds;

impl SwiftNamedNeeds {
    const FILES: [(&str, &str); 4] = [
        (
            "client/helpers/GreeterHelper.swift",
            "@Helper\nfinal class GreeterHelper {\n    func greet() -> String { \"hi\" }\n}\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.swift",
            "@Helper\nfinal class LoudGreeterHelper {\n    func greet() -> String { \"HI\" }\n}\n",
        ),
        (
            "client/services/WelcomeService.swift",
            "@Service\nfinal class WelcomeService {\n    @Needs\n    @Named(\"LoudGreeterHelper\")\n    private let greeter: LoudGreeterHelper\n\n    \
             func welcome() -> String { greeter.greet() }\n}\n",
        ),
        (
            "client/services/HelloService.swift",
            "@Service\nfinal class HelloService {\n    @Needs\n    private let greeter: GreeterHelper\n\n    \
             func hello() -> String { greeter.greet() }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: WelcomeService } = await import('./WelcomeService.js')\n\
         const { default: HelloService } = await import('./HelloService.js')\n\
         const welcome = new WelcomeService({}), hello = new HelloService({})\n\
         welcome.__tachyonStarted(); hello.__tachyonStarted()\n\
         const report = { welcomed: welcome.welcome(), greeted: hello.hello() }\n\
         welcome.__tachyonDispose(); hello.__tachyonDispose()\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, text).expect("file");
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text);
        }
        let services =
            [("welcome", "WelcomeService"), ("hello", "HelloService")].map(|(instance, class)| {
                (
                    String::from(instance),
                    String::from(class),
                    WebLanguage::Swift,
                    root.join(format!("client/services/{class}.swift")),
                )
            });
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, bytes).expect("publish");
        }
    }

    fn drive(root: &Path) -> serde_json::Value {
        std::fs::write(root.join("drive.mjs"), Self::DRIVER).expect("driver");
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }
}

#[tokio::test]
async fn a_swift_named_needs_field_gets_the_helper_it_names() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    SwiftNamedNeeds::publish(root.path()).await;
    let report = SwiftNamedNeeds::drive(root.path());
    assert_eq!(
        report,
        serde_json::json!({"welcomed": "HI", "greeted": "hi"}),
        "{report}"
    );
}
