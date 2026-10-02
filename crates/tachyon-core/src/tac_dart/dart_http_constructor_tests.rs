#![allow(clippy::expect_used)]

use super::{Bridge, DartModels};
use std::path::Path;

struct DartHttpConstructorTests;

impl DartHttpConstructorTests {
    fn models() -> DartModels {
        DartModels {
            imports: String::new(),
            code: String::new(),
            names: Vec::new(),
        }
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    fn program(source: &str) -> Result<String, String> {
        Bridge::read(source, "CounterService", &[])?.library(
            source,
            "CounterService",
            &[],
            &Self::models(),
        )
    }

    fn capture_source(field: &str, parameters: &str) -> String {
        format!(
            "class GreetingClient {{\n {field}\n GreetingClient({parameters});\n}}\nclass CounterService {{\n final GreetingClient greetingClient;\n CounterService(this.greetingClient);\n}}\n"
        )
    }
}

#[test]
fn dart_http_constructor_ordinary_tutorial_omits_optional_caller_capture() {
    let root = tempfile::tempdir().expect("project");
    let source = include_str!(
        "../../../../website/content/docs/first-app/call-the-server/client/services/CounterService.dart"
    );
    let relative = Path::new("client/services/CounterService.dart");
    for (relative, source) in [
        ("client/services/CounterService.dart", source),
        (
            "client/remotes/GreetingClient.dart",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/remotes/GreetingClient.dart"
            ),
        ),
        (
            "client/setups/GreetingSetup.dart",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/setups/GreetingSetup.dart"
            ),
        ),
        (
            "client/models/Greeting.dart",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/models/Greeting.dart"
            ),
        ),
    ] {
        DartHttpConstructorTests::write(root.path(), relative, source);
    }
    let prepared = super::prepare(root.path(), relative, source).expect("ordinary preparation");
    let program =
        DartHttpConstructorTests::program(&prepared.source).expect("browser bridge library");
    assert!(
        program.contains("GreetingClient([this._suppliedContext]);"),
        "{program}"
    );
    assert!(
        program.contains("final instance = GreetingClient();"),
        "{program}"
    );
    assert!(
        program.contains("CounterService(make_GreetingClient(graph, properties, publish))"),
        "{program}"
    );
    assert!(
        program.contains("_suppliedContext ?? Behaviors.create(GreetingSetup"),
        "{program}"
    );
    assert!(!program.contains("make_BehaviorContext("), "{program}");
}

#[test]
fn dart_http_constructor_refuses_unrecognized_optional_captures() {
    for (field, parameters) in [
        ("final String? _suppliedContext;", "[this._suppliedContext]"),
        (
            "final BehaviorContext _suppliedContext;",
            "[this._suppliedContext]",
        ),
        ("", "[this._suppliedContext]"),
        ("final BehaviorContext? other;", "[this.other]"),
        (
            "final BehaviorContext? _suppliedContext;",
            "{this._suppliedContext}",
        ),
        (
            "final BehaviorContext? _suppliedContext;",
            "[this._suppliedContext, int extra = 1]",
        ),
    ] {
        let source = DartHttpConstructorTests::capture_source(field, parameters);
        let error =
            DartHttpConstructorTests::program(&source).expect_err("unsupported constructor");
        assert!(error.contains("constructor takes"), "{parameters}: {error}");
    }
}

#[test]
fn dart_http_constructor_keeps_required_capture_validation() {
    let source = DartHttpConstructorTests::capture_source(
        "final BehaviorContext? _suppliedContext;",
        "this._suppliedContext",
    );
    let error =
        DartHttpConstructorTests::program(&source).expect_err("required context must resolve");
    assert!(
        error.contains("collaborator BehaviorContext is not a class"),
        "{error}"
    );
}
