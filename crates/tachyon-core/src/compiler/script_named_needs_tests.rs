use super::{BuildOptions, WebCompiler};
use std::path::Path;

pub(super) struct ScriptNamedNeeds;

impl ScriptNamedNeeds {
    const PAGE: [(&'static str, &'static str); 2] = [
        ("client/pages/tac.html", "<greeting-panel />"),
        (
            "client/components/greeting/panel/tac.html",
            "<p>{welcome.welcome()} {hello.hello()}</p>",
        ),
    ];

    const TYPESCRIPT: [(&'static str, &'static str); 4] = [
        (
            "client/helpers/GreeterHelper.ts",
            "@Helper\nexport default class GreeterHelper {\n  greet(): string {\n    return 'hi';\n  }\n}\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.ts",
            "@Helper\nexport default class LoudGreeterHelper {\n  greet(): string {\n    return 'HI';\n  }\n}\n",
        ),
        (
            "client/services/WelcomeService.ts",
            "@Service\nexport default class WelcomeService {\n  @Needs\n  @Named(\"LoudGreeterHelper\")\n  greeter!: GreeterHelper;\n\n  welcome(): string {\n    return this.greeter.greet();\n  }\n}\n",
        ),
        (
            "client/services/HelloService.ts",
            "@Service\nexport default class HelloService {\n  @Needs\n  greeterHelper!: GreeterHelper;\n\n  hello(): string {\n    return this.greeterHelper.greet();\n  }\n}\n",
        ),
    ];

    const JAVASCRIPT: [(&'static str, &'static str); 4] = [
        (
            "client/helpers/GreeterHelper.js",
            "@Helper\nexport default class GreeterHelper {\n  greet() {\n    return 'hi';\n  }\n}\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.js",
            "@Helper\nexport default class LoudGreeterHelper {\n  greet() {\n    return 'HI';\n  }\n}\n",
        ),
        (
            "client/services/WelcomeService.js",
            "@Service\nexport default class WelcomeService {\n  /** @type {GreeterHelper} */\n  @Needs\n  @Named(\"LoudGreeterHelper\")\n  greeter;\n\n  welcome() {\n    return this.greeter.greet();\n  }\n}\n",
        ),
        (
            "client/services/HelloService.js",
            "@Service\nexport default class HelloService {\n  /** @type {GreeterHelper} */\n  @Needs\n  greeterHelper;\n\n  hello() {\n    return this.greeterHelper.greet();\n  }\n}\n",
        ),
    ];

    const DRIVER: &'static str = "const open = async (name) => {\n\
         \x20 const module = await import(`./.tachyon/services/${name}Service.js`)\n\
         \x20 return module.__tachyon_create_owner__({}, new AbortController().signal, undefined)\n\
         }\n\
         const welcome = await open('Welcome'), hello = await open('Hello')\n\
         console.log(JSON.stringify({ greeted: [welcome.welcome(), hello.hello()] }))\n";

    pub(super) fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("parent")))
            .unwrap_or_else(|failure| panic!("source directory: {failure}"));
        std::fs::write(path, source).unwrap_or_else(|failure| panic!("source: {failure}"));
    }

    async fn report(files: &[(&str, &str)]) -> serde_json::Value {
        let page = Self::PAGE.iter().chain(files).copied().collect::<Vec<_>>();
        Self::run(&page, Self::DRIVER).await
    }

    pub(super) async fn run(files: &[(&str, &str)], driver: &str) -> serde_json::Value {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        for (relative, source) in files {
            Self::write(root.path(), relative, source);
        }
        let result = WebCompiler::build_async(root.path(), &BuildOptions::default())
            .await
            .unwrap_or_else(|failure| panic!("the script services build: {failure}"));
        let output = result.output_directory();
        Self::write(output, "package.json", "{\"type\":\"module\"}");
        Self::write(output, "drive.mjs", driver);
        let mut command = crate::node_driver::NodeDriver::async_command();
        command.arg("drive.mjs").current_dir(output);
        let captured = crate::node_driver::NodeDriver::run_within(
            &mut command,
            std::time::Duration::from_secs(30),
        )
        .await
        .unwrap_or_else(|failure| panic!("supervised emitted module execution: {failure}"));
        let printed = String::from_utf8_lossy(&captured.stdout);
        assert!(
            captured.status.success(),
            "{printed}{}",
            String::from_utf8_lossy(&captured.stderr)
        );
        let last = printed.lines().last().unwrap_or_default();
        serde_json::from_str(last).unwrap_or_else(|failure| panic!("{failure}: {printed}"))
    }
}

#[tokio::test]
async fn a_typescript_needs_field_gets_the_helper_that_named_names() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ScriptNamedNeeds::report(&ScriptNamedNeeds::TYPESCRIPT).await;
    assert_eq!(
        report["greeted"],
        serde_json::json!(["HI", "hi"]),
        "{report}"
    );
}

#[tokio::test]
async fn a_javascript_needs_field_gets_the_helper_that_named_names() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ScriptNamedNeeds::report(&ScriptNamedNeeds::JAVASCRIPT).await;
    assert_eq!(
        report["greeted"],
        serde_json::json!(["HI", "hi"]),
        "{report}"
    );
}
