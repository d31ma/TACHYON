#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaBrowserHttpProject;

impl JavaBrowserHttpProject {
    const EXAMPLE: &'static str = "../../website/content/docs/calling-the-server/browser-http-java";
    const FOLDERS: [&'static str; 3] = ["client/models", "client/remotes", "client/setups"];
    const SERVICE: &'static str = "client/services/GreetingService.java";

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
            String::from("greeting"),
            String::from("GreetingService"),
            root.path().join(Self::SERVICE),
        );
        let sources =
            super::JavaWebServices::sources(root.path(), &[service]).expect("Java web program");
        let mut program = String::new();
        for (path, text) in sources {
            program.push_str("// ");
            program.push_str(&path.to_string_lossy().replace('\\', "/"));
            program.push('\n');
            program.push_str(&text);
        }
        program
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
fn java_clients_and_services_reach_the_browser_transport() {
    let program = JavaBrowserHttpProject::program();
    for expected in [
        "return Behaviors.answer(this.behaviors.invoke(\"GreetingClient\", \"read\"",
        "__ty_answer -> TacModels.readGreeting(__ty_answer));",
        "this.behaviors.mutate(\"GreetingClient\", \"read\"",
        "private final GreetingClient greetingClient = new GreetingClient();",
        "GreetingClient(Behaviors.Context context) {",
        "Object value = BehaviorBuiltins.exchange(java.util.Collections.unmodifiableMap(browser), execution);",
        "Object answer = TacJsonReader.read(TacHttpTickets.exchange(Behaviors.encode(request)));",
        "static final class Failure extends TacFailure {",
        "return TacTasks.run(request);",
        "public static void tacHttpSettle(int ticket, String answer) {",
        "org.teavm.runtime.Fiber.start(() -> finish(task, TacBridge.handle(request)), true);",
        "private static native String await(int ticket, TacTask task, String request);",
    ] {
        assert!(program.contains(expected), "missing {expected}");
    }
    for name in [
        "TacTasks.java",
        "TacHttpTickets.java",
        "TacFailure.java",
        "TacTask.java",
    ] {
        let marker = format!("// tachyon/{name}");
        assert!(program.contains(&marker), "missing {marker}");
    }
}
