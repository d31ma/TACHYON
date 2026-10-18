use super::{CppYon, CppYonController};
use crate::schedule::Schedule;
use std::path::Path;

struct Roles;

impl Roles {
    const CONTROLLER_PATH: &'static str = "server/controllers/LifecycleController.hpp";
    const CONTROLLER: &'static str = r#"#pragma once
#include "../services/AuditService.hpp"
#include "../services/CheckoutService.hpp"

class [[RequestRoute("/lifecycle")]] [[Public]] LifecycleController {
    [[Needs]] AuditService& audit;
    [[Needs]] CheckoutService& checkout;

public:
    [[GetRoute]]
    Lifecycle getLifecycle() {
        return Lifecycle{audit.log, audit.settled, audit.audited, checkout.total()};
    }
};
"#;
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "server/models/PlacedOrder.hpp",
            "#pragma once\n#include <string>\n\nstruct PlacedOrder {\n    std::string status;\n    long long total = 0;\n};\n",
        ),
        (
            "server/models/SettledOrder.hpp",
            "#pragma once\n\nstruct SettledOrder {\n    long long total = 0;\n};\n",
        ),
        (
            "server/models/Lifecycle.hpp",
            "#pragma once\n#include <string>\n#include <vector>\n\nstruct Lifecycle {\n    std::vector<std::string> log;\n    long long settled = 0;\n    long long audited = 0;\n    int fee = 0;\n};\n",
        ),
        (
            "server/services/AuditService.hpp",
            r#"#pragma once
#include <string>
#include <vector>

class [[Service]] AuditService {
public:
    std::vector<std::string> log;
    long long settled = 0;
    long long audited = 0;

private:
    [[OnEvent({"id":"audit-log"})]] [[Order(-10)]]
    void record(const PlacedOrder& order) { log.push_back(order.status); }

    [[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]] [[Order(10)]]
    SettledOrder settle(const PlacedOrder& order) { return SettledOrder{order.total}; }

    [[OnEvent]]
    void saveSettled(const SettledOrder& order) { settled = order.total; }

    [[OnEvent]] [[InBackground]]
    void audit(const SettledOrder& order) {
        audited = order.total;
        log.push_back("audited");
    }

    [[OnEvent({"condition":"value.status == 'refunded'"})]]
    void refunded(const PlacedOrder& order) { log.push_back("refund " + order.status); }
};
"#,
        ),
        (
            "server/services/CheckoutService.hpp",
            r#"#pragma once

class [[Service, Description("Places the opening order")]] CheckoutService {
    ApplicationEventPublisher applicationEventPublisher;
    [[Needs]] [[Named("FeeHelper")]] FeeHelper& fees;

public:
    int total() const { return fees.fee(); }

    [[OnStart]]
    void start() { applicationEventPublisher.publishEvent(PlacedOrder{"paid", 25}); }

    [[OnStop]]
    void stop() {}
};
"#,
        ),
        (
            "server/helpers/FeeHelper.hpp",
            "#pragma once\n\nclass [[Helper]] [[NeedsAll]] FeeHelper {\n    RateHelper& rates;\n\npublic:\n    int fee() const { return rates.rate() + 1; }\n};\n",
        ),
        (
            "server/helpers/RateHelper.hpp",
            "#pragma once\n\nclass [[Helper]] RateHelper {\npublic:\n    int rate() const { return 2; }\n};\n",
        ),
    ];

    fn project(changes: &[(&str, &str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            let edited = changes
                .iter()
                .filter(|(file, _, _)| *file == relative)
                .fold(source.to_owned(), |text, (_, from, to)| {
                    assert!(text.contains(from), "{relative} has {from}");
                    text.replace(from, to)
                });
            Self::write(root.path(), relative, &edited);
        }
        root
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, source).expect("file");
    }

    fn lower(root: &Path, controller: &str) -> Result<crate::yon::Lowered, crate::Failure> {
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), controller, root)
    }

    fn refusal(changes: &[(&str, &str, &str)]) -> String {
        let root = Self::project(changes);
        Self::lower(root.path(), Self::CONTROLLER)
            .err()
            .expect("the project is refused")
            .to_string()
    }

    fn compiler_available() -> bool {
        std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn run(program: &str, frame: &str) -> (serde_json::Value, String) {
        let folder = tempfile::tempdir().expect("build folder");
        let source = folder.path().join("program.cpp");
        let binary = folder.path().join("program");
        std::fs::write(&source, program).expect("program");
        let built = std::process::Command::new("clang++")
            .args(["-std=c++20", "-O0", "-o"])
            .arg(&binary)
            .arg(&source)
            .output()
            .expect("clang++");
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let mut child = std::process::Command::new(&binary)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("program runs");
        std::io::Write::write_all(&mut child.stdin.take().expect("stdin"), frame.as_bytes())
            .expect("frame");
        let output = child.wait_with_output().expect("answer");
        let answer = serde_json::from_slice(&output.stdout).expect("JSON answer");
        (answer, String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

#[test]
fn a_controller_receives_its_services_and_helpers_in_dependency_order() {
    let root = Roles::project(&[]);
    let program = Roles::lower(root.path(), Roles::CONTROLLER)
        .expect("lowered")
        .source;
    let order: Vec<usize> = [
        "class [[Service]] AuditService",
        "class [[Helper]] RateHelper",
        "class [[Helper]] [[NeedsAll]] FeeHelper",
        "CheckoutService {",
        "LifecycleController {",
    ]
    .iter()
    .map(|class| {
        let blanked = class.replace("[[Helper]]", "          ");
        let blanked = blanked.replace("[[NeedsAll]]", "            ");
        let blanked = blanked.replace("[[Service]]", "           ");
        program.find(&blanked).expect(class)
    })
    .collect();
    assert!(order.is_sorted(), "{order:?}");
    assert!(program.contains("template <>\n::AuditService& YonNeeds::of<::AuditService>();"));
    assert!(
        program.contains("AuditService& audit = ::tachyon::yon::YonNeeds::of<AuditService>();")
    );
    assert!(program.contains("RateHelper& rates = ::tachyon::yon::YonNeeds::of<RateHelper>();"));
    assert!(program.contains("FeeHelper& fees = ::tachyon::yon::YonNeeds::of<FeeHelper>();"));
    assert!(program.contains("made.start = [&owner] { owner.start(); };"));
    assert!(program.contains("made.stop = [&owner] { owner.stop(); };"));
    assert!(program.contains(r#""events":["PlacedOrder"]"#));
    assert!(program.contains("&::AuditService::settle"));
    assert!(program.contains(r#""returns":"SettledOrder""#));
    assert!(!program.contains("[[OnEvent"));
}

#[test]
fn a_transient_helper_is_made_for_each_field() {
    let root = Roles::project(&[(
        "server/helpers/RateHelper.hpp",
        "class [[Helper]] RateHelper",
        "class [[Helper, Lifetime(\"transient\")]] RateHelper",
    )]);
    let refused = Roles::lower(root.path(), Roles::CONTROLLER)
        .err()
        .expect("events refuse a transient class")
        .to_string();
    assert!(
        refused.contains("no class it works with may be"),
        "{refused}"
    );
    let quiet = Roles::CONTROLLER.replace("[[Needs]] CheckoutService& checkout;", "");
    let quiet = quiet.replace(
        "audit.log, audit.settled, audit.audited, checkout.total()",
        "",
    );
    let quiet = quiet.replace(
        "[[Needs]] AuditService& audit;",
        "[[Needs]] FeeHelper& fees;",
    );
    let program = Roles::lower(root.path(), &quiet).expect("lowered").source;
    assert!(program.contains("return YonContainer::fresh<::RateHelper>("));
    assert!(program.contains("static ::FeeHelper* slot = nullptr;"));
}

#[test]
fn the_provider_graph_refuses_what_cannot_be_built() {
    let cases = [
        (
            "server/services/CheckoutService.hpp",
            "[[Named(\"FeeHelper\")]]",
            "[[Named(\"PriceHelper\")]]",
            "there is no PriceHelper.hpp",
        ),
        (
            "server/helpers/RateHelper.hpp",
            "class [[Helper]] RateHelper",
            "class [[Helper, Lifetime(\"request\")]] RateHelper",
            "captures a request-scoped object through RateHelper",
        ),
        (
            "server/helpers/RateHelper.hpp",
            "class [[Helper]] RateHelper",
            "class [[Helper, Lifetime({\"value\":\"prototype\"})]] RateHelper",
            "takes one quoted scope",
        ),
        (
            "server/services/AuditService.hpp",
            "void saveSettled(const SettledOrder& order)",
            "Task<void> saveSettled(const SettledOrder& order)",
            "returns no Task",
        ),
        (
            "server/services/CheckoutService.hpp",
            "void start()",
            "void start(int times)",
            "takes no parameters",
        ),
        (
            "server/services/CheckoutService.hpp",
            "[[Needs]] [[Named(\"FeeHelper\")]] FeeHelper& fees;",
            "[[Needs]] [[Named(\"FeeHelper\")]] FeeHelper fees;",
            "is a reference with no initial value",
        ),
    ];
    for (file, from, to, expected) in cases {
        let failure = Roles::refusal(&[(file, from, to)]);
        assert!(failure.contains(expected), "{expected}: {failure}");
    }
}

#[test]
fn a_worker_is_a_handler_with_a_post_method_and_its_own_schedule() {
    let path = "server/workers/beat.hpp";
    let worker = "#pragma once\n\nclass [[Schedule({\"fixedDelaySeconds\":30,\"initialDelaySeconds\":0})]] Handler {\n    [[Needs]] RateHelper& rates;\n\npublic:\n    int POST() { return rates.rate(); }\n};\n";
    let read = CppYonController::read(Path::new(path), worker).expect("worker");
    assert_eq!(read.routes[0].verb, "POST");
    assert_eq!(read.routes[0].pattern, None);
    let schedule = Schedule::declared(path, worker).expect("schedule");
    assert_eq!(
        schedule,
        Some(Schedule {
            every_seconds: 30,
            initial_seconds: 0
        })
    );
    let wrong = CppYonController::read(Path::new(path), &worker.replace("POST()", "post()"))
        .err()
        .expect("a worker needs POST")
        .to_string();
    assert!(wrong.contains("class Handler"), "{wrong}");
    let root = Roles::project(&[]);
    let lowered = CppYon::lower(Path::new(path), worker, root.path()).expect("lowered");
    assert!(lowered.routes.is_empty());
    assert!(lowered.source.contains("if (verb == \"POST\") {"));
    if Roles::compiler_available() {
        let (answer, _) = Roles::run(&lowered.source, r#"{"method":"POST","route":"/"}"#);
        assert_eq!(answer["body"], "2", "{answer}");
    }
}

#[test]
fn lifecycle_hooks_and_listeners_run_around_the_route() {
    if !Roles::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = Roles::project(&[]);
    let program = Roles::lower(root.path(), Roles::CONTROLLER)
        .expect("lowered")
        .source;
    let (answer, errors) = Roles::run(&program, r#"{"method":"GET","route":"/lifecycle"}"#);
    assert_eq!(answer["status"], 200, "{answer} {errors}");
    let body: serde_json::Value =
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("body JSON");
    assert_eq!(
        body,
        serde_json::json!({"log": ["paid", "audited"], "settled": 25, "audited": 25, "fee": 3})
    );
}
