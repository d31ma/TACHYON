#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

struct KotlinBrowserHttpProject;

impl KotlinBrowserHttpProject {
    const EXAMPLE: &'static str =
        "../../website/content/docs/calling-the-server/browser-http-kotlin";
    const FOLDERS: [&'static str; 3] = ["client/models", "client/remotes", "client/setups"];
    const SERVICE: &'static str = "client/services/GreetingService.kt";

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
        let service = (
            String::from("GreetingService"),
            String::from("GreetingService"),
            root.path().join(Self::SERVICE),
        );
        let sources =
            super::KotlinWebServices::sources(root.path(), &[service]).expect("Kotlin web program");
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
