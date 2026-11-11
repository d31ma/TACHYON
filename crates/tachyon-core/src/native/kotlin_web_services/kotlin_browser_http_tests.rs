#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

struct KotlinBrowserHttpProject;

impl KotlinBrowserHttpProject {
    const EXAMPLE: &'static str =
        "../../website/content/docs/calling-the-server/browser-http-kotlin";
    const FOLDERS: [&'static str; 3] = ["client/models", "client/remotes", "client/setups"];
    const SERVICE: &'static str = "client/services/GreetingService.kt";
    const ERROR_BODY_EXAMPLE: &'static str =
        "../../website/content/docs/calling-the-server/http-error-bodies";
    const ERROR_BODY_SERVICE: &'static str = "client/services/OrderService.kt";
    const KOTLIN_FILES: [&'static str; 5] = [
        "client/models/OrderInput.kt",
        "client/models/Problem.kt",
        "client/models/ProblemEntry.kt",
        "client/remotes/OrderClient.kt",
        "client/setups/OrderSetup.kt",
    ];

    fn program() -> String {
        let root = tempfile::tempdir().expect("project");
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::EXAMPLE);
        for relative in Self::FOLDERS
            .iter()
            .map(Path::new)
            .chain([Path::new(Self::SERVICE)])
        {
            Self::copy(&example.join(relative), &root.path().join(relative));
        }
        Self::sources(root.path(), "GreetingService", Self::SERVICE)
    }

    fn error_body_program() -> String {
        let root = tempfile::tempdir().expect("project");
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::ERROR_BODY_EXAMPLE);
        for relative in Self::KOTLIN_FILES
            .into_iter()
            .chain([Self::ERROR_BODY_SERVICE])
        {
            Self::copy(&example.join(relative), &root.path().join(relative));
        }
        Self::sources(root.path(), "OrderService", Self::ERROR_BODY_SERVICE)
    }

    fn sources(root: &Path, name: &str, relative: &str) -> String {
        let service = (String::from(name), String::from(name), root.join(relative));
        let sources =
            super::KotlinWebServices::sources(root, &[service]).expect("Kotlin web program");
        sources
            .into_iter()
            .find(|(path, _)| path == &PathBuf::from(super::KotlinWebServices::PROGRAM_SOURCE))
            .map(|(_, text)| text)
            .expect("services.kt")
    }

    fn copy(from: &Path, to: &Path) {
        if from.is_dir() {
            for entry in std::fs::read_dir(from).expect("example folder") {
                let entry = entry.expect("example entry");
                Self::copy(&entry.path(), &to.join(entry.file_name()));
            }
            return;
        }
        std::fs::create_dir_all(to.parent().expect("parent")).expect("folder");
        std::fs::copy(from, to).expect("example file");
    }
}

#[test]
fn kotlin_suspend_clients_and_services_reach_the_browser_transport() {
    let program = KotlinBrowserHttpProject::program();
    for expected in [
        "this.behaviors.invokeAwait(\"GreetingClient\", \"read\"",
        "this.behaviors.mutateAwait(\"GreetingClient\", \"read\"",
        "tacRead<Greeting>(it)",
        "private val greetingClient: GreetingClient = GreetingClient()",
        "\"ask\" to TacTask({ tacThrownCode(it) }) { tacInstance.ask() }",
        "private suspend fun behaviorDefaultExchange(request: Map<String, Any?>): Any? = BehaviorBrowserHttp.send(request)",
        "fun tacHttpSettle(ticket: Int, answer: String)",
    ] {
        assert!(program.contains(expected), "missing {expected}");
    }
    assert!(program.starts_with("import kotlin.coroutines.startCoroutine\n"));
}

#[test]
fn kotlin_clients_read_an_error_status_and_body_from_the_browser_transport() {
    let program = KotlinBrowserHttpProject::error_body_program();
    for expected in [
        "tacRead<OrderInput>(it)",
        "inline fun <reified T> Behaviors.errorBody(error: Throwable?): T? = errorBody(error) { tacRead<T>(it) }",
        "Behaviors.errorBody<Problem>(error)",
        "(error[\"status\"] as? Number)?.toInt() ?: 0, error[\"body\"])",
    ] {
        assert!(program.contains(expected), "missing {expected}");
    }
}
