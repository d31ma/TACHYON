use super::page_tests::PythonPage;

struct PythonNamedPage;

impl PythonNamedPage {
    const HELPERS: [(&'static str, &'static str); 2] = [
        (
            "client/helpers/GreeterHelper.py",
            "@helper\nclass GreeterHelper:\n    def greet(self) -> str:\n        return 'hi'\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.py",
            "@helper\nclass LoudGreeterHelper:\n    def greet(self) -> str:\n        return 'HI'\n",
        ),
    ];

    const SERVICE: &'static str = "from typing import Annotated\n\n@service\nclass WelcomeService:\n    greeter: Annotated[GreeterHelper, needs, named(\"LoudGreeterHelper\")]\n    plain: Annotated[GreeterHelper, needs]\n\n    \
         def welcome(self) -> str:\n        return self.greeter.greet()\n\n    \
         def hello(self) -> str:\n        return self.plain.greet()\n";

    const DRIVER: &'static str = "const { default: WelcomeService } = await import('./WelcomeService.js')\n\
         const welcome = new WelcomeService({})\n\
         const greeted = [await welcome.welcome(), await welcome.hello()]\n\
         console.log(JSON.stringify({ greeted }))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_python_needs_field_gets_the_helper_that_named_names() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    for (relative, text) in PythonNamedPage::HELPERS {
        PythonPage::write(root.path(), relative, text);
    }
    PythonPage::publish(
        root.path(),
        &[("welcome", "WelcomeService", PythonNamedPage::SERVICE)],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonNamedPage::DRIVER);
    assert_eq!(
        report["greeted"],
        serde_json::json!(["HI", "hi"]),
        "{report}"
    );
}
