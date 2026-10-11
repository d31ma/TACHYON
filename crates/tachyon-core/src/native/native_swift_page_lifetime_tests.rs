#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

struct NativeSwiftPageLifetimeCheck;

impl NativeSwiftPageLifetimeCheck {
    const COUNTER: &str = r#"@Service
final class CounterService {
    var count: Int = 0
    private let filePicker: FilePicker
    func increment() -> Int {
        count += 1
        return count
    }
    func identity() -> Int { ObjectIdentifier(self).hashValue }
    func available() -> Bool { filePicker.available() }
    @OnStart
    func started() { tachyonProbe("counter.start") }
    @OnStop
    func stopped() { tachyonProbe("counter.stop") }
}
"#;
    const TALLY: &str = r#"@Service
@Lifetime({"value":"prototype"})
final class TallyService {
    var count: Int = 0
    private let filePicker: FilePicker
    func increment() -> Int {
        count += 1
        return count
    }
    func identity() -> Int { ObjectIdentifier(self).hashValue }
    @OnStart
    func started() { tachyonProbe("tally.start") }
    @OnStop
    func stopped() { tachyonProbe(filePicker.available() ? "tally.stop" : "tally.stop.revoked") }
}
"#;
    const LABEL: &str = r#"@Service
@Lifetime({"value":"singleton"})
final class LabelService {
    var label: String = "label"
    @OnStart
    func started() { tachyonProbe("label.start") }
    @OnStop
    func stopped() { tachyonProbe("label.stop") }
}
"#;
    const BROKEN: &str = r#"import Foundation

@Service
final class BrokenService {
    var label: String = "broken"
    @OnStart
    func started() throws {
        tachyonProbe("broken.start")
        throw NSError(domain: "private startup detail", code: 7)
    }
    @OnStop
    func stopped() { tachyonProbe("broken.stop") }
}
"#;
    const CALLER: &str = r#"import Foundation

nonisolated(unsafe) var tachyonProbeTape: [String] = []
nonisolated(unsafe) var tachyonCapabilityOwners: [String] = []

func tachyonProbe(_ event: String) { tachyonProbeTape.append(event) }

@main struct Caller {
    static let one = "00000000-0000-4000-8000-000000000001"
    static let two = "00000000-0000-4000-8000-000000000002"
    static let three = "00000000-0000-4000-8000-000000000003"

    static func request(_ service: String, _ owner: String, _ operation: String, _ name: String = "", route: String = "/") -> String {
        tacNativeInvoke("{\"route\":\"" + route + "\",\"service\":\"" + service + "\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\",\"name\":\"" + name + "\",\"args\":[]}")
    }

    static func pageExit() { _ = tacNativeInvoke("{\"op\":\"host-dispose\"}") }

    static func events(_ prefix: String) -> [String] { tachyonProbeTape.filter { $0.hasPrefix(prefix) } }

    static func count(_ event: String) -> Int { tachyonProbeTape.filter { $0 == event }.count }

    static func started(_ answer: String) {
        precondition(!answer.contains("error"), answer)
    }

    static func sharesIdentityUntilPageExit() {
        started(request("counter", one, "init"))
        started(request("counter", two, "init"))
        precondition(request("counter", one, "call", "identity") == request("counter", two, "call", "identity"), "two owners of a page singleton must share one instance")
        _ = request("counter", one, "call", "increment")
        precondition(request("counter", two, "get", "count") == "{\"value\":1}", "state written by one owner must be read by the other")
        precondition(count("counter.start") == 1, "page singleton startup must run only once")
        _ = request("counter", one, "dispose")
        precondition(request("counter", two, "call", "available") == "{\"value\":true}", "disposing the first component must not revoke the shared provider capability")
        precondition(tachyonCapabilityOwners.allSatisfy { $0.hasPrefix("00000000-0000-4000-a000-") }, "a page singleton must own its capability under a provider identity")
        _ = request("counter", two, "dispose")
        precondition(count("counter.stop") == 0, "singleton survives zero component owners")
        started(request("counter", two, "init"))
        precondition(request("counter", two, "get", "count") == "{\"value\":1}", "page singleton must survive zero temporary component leases")
        started(request("counter", three, "init", route: "/other"))
        precondition(request("counter", three, "call", "identity", route: "/other") != request("counter", two, "call", "identity"), "each page route owns its own singleton")
        pageExit()
        precondition(count("counter.stop") == 2, "page disposal must stop each singleton once")
        pageExit()
        precondition(count("counter.stop") == 2, "repeated page disposal must not stop twice")
        started(request("counter", two, "init"))
        precondition(request("counter", two, "get", "count") == "{\"value\":0}", "page exit must release the singleton")
        precondition(count("counter.start") == 3)
        pageExit()
    }

    static func isolatesPrototypes() {
        started(request("tally", one, "init"))
        started(request("tally", two, "init"))
        precondition(request("tally", one, "call", "identity") != request("tally", two, "call", "identity"), "a prototype must give each owner its own instance")
        _ = request("tally", one, "call", "increment")
        precondition(request("tally", two, "get", "count") == "{\"value\":0}", "prototype state must stay with its owner")
        precondition(count("tally.start") == 2)
        _ = request("tally", one, "dispose")
        precondition(events("tally.stop") == ["tally.stop"], "owner disposal must stop its prototype while its capability is still live")
        _ = request("tally", one, "dispose")
        precondition(count("tally.stop") == 1, "owner disposal must not stop twice")
        _ = request("tally", two, "dispose")
        precondition(count("tally.stop") == 2)
        pageExit()
        precondition(count("tally.stop") == 2, "page disposal must not stop a disposed prototype again")
    }

    static func rollsBackFailedStartup() {
        started(request("counter", two, "init"))
        let failed = request("broken", one, "init")
        precondition(failed.contains("error"), failed)
        precondition(!failed.contains("private startup detail"), failed)
        precondition(events("broken.") == ["broken.start", "broken.stop"], "a failed startup must stop the new singleton")
        precondition(request("counter", two, "get", "count") == "{\"value\":0}", "a failed owner must preserve the preexisting page singleton")
        precondition(request("broken", one, "get", "label").contains("error"), "a failed owner must not stay active")
        let retried = request("broken", three, "init")
        precondition(retried.contains("error"), retried)
        precondition(events("broken.") == ["broken.start", "broken.stop", "broken.start", "broken.stop"], "a failed singleton must not stay on the page")
        pageExit()
        precondition(count("broken.stop") == 2, "a rolled-back singleton must not stop again at page exit")
        precondition(count("counter.stop") == 1)
    }

    static func stopsInReverseCreationOrder() {
        started(request("counter", one, "init"))
        started(request("tally", two, "init"))
        started(request("label", three, "init"))
        _ = request("counter", one, "dispose")
        _ = request("label", three, "dispose")
        precondition(tachyonProbeTape.filter { $0.hasSuffix(".stop") }.isEmpty, "owner disposal must leave page singletons running")
        pageExit()
        precondition(tachyonProbeTape.filter { $0.hasSuffix(".stop") } == ["label.stop", "tally.stop", "counter.stop"], "page exit must stop services in reverse creation order")
        pageExit()
        precondition(tachyonProbeTape.filter { $0.hasSuffix(".stop") }.count == 3, "cleanup must happen exactly once")
    }

    static func refusesProviderOwners() {
        let reserved = request("counter", "00000000-0000-4000-A000-000000000001", "init")
        precondition(reserved.contains("TY_NATIVE_OWNER"), reserved)
        precondition(count("counter.start") == 0, "a reserved provider owner must not construct a service")
    }

    static func scenario(_ run: () -> Void) {
        tachyonProbeTape = []
        tachyonCapabilityOwners = []
        run()
        pageExit()
    }

    static func main() {
        NativeCapabilities.call = { operation, payload in
            if operation == "filePicker.available" {
                let request = (try? JSONSerialization.jsonObject(with: Data(payload.utf8))) as? [String: Any]
                tachyonCapabilityOwners.append(request?["owner"] as? String ?? "")
                return "{\"status\":\"available\"}"
            }
            return "{\"status\":\"cancelled\"}"
        }
        scenario(sharesIdentityUntilPageExit)
        scenario(isolatesPrototypes)
        scenario(rollsBackFailedStartup)
        scenario(stopsInReverseCreationOrder)
        scenario(refusesProviderOwners)
        print("native swift page singleton identity, lifetime, ordering and capability ownership passed")
    }
}
"#;

    fn stage(root: &Path) -> PathBuf {
        let services = root.join("client/services");
        std::fs::create_dir_all(&services).expect("services");
        let inputs = [
            ("counter", "CounterService", Self::COUNTER),
            ("tally", "TallyService", Self::TALLY),
            ("label", "LabelService", Self::LABEL),
            ("broken", "BrokenService", Self::BROKEN),
        ]
        .into_iter()
        .map(|(service, class, text)| {
            let source = services.join(format!("{class}.swift"));
            std::fs::write(&source, text).expect("service");
            NativeCompanionInput {
                snapshot_root: root.to_path_buf(),
                language: NativeCompanion::Swift,
                source,
                service: service.to_owned(),
                class: class.to_owned(),
                web_assembly: false,
                application_provider: false,
            }
        })
        .collect::<Vec<_>>();
        super::macos::stage_swift_companion(&inputs, root)
            .expect("stage")
            .expect("companion")
    }

    #[cfg(target_os = "macos")]
    async fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        let json = root.path().join("json.swift");
        std::fs::write(&json, super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST).expect("json");
        let caller = root.path().join("caller.swift");
        std::fs::write(&caller, Self::CALLER).expect("caller");
        let program = root.path().join("caller");
        super::host::run_tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                &staged.to_string_lossy(),
                &json.to_string_lossy(),
                &caller.to_string_lossy(),
                "-o",
                &program.to_string_lossy(),
            ],
        )
        .await
        .expect("compile");
        super::host::run_tool(&program.to_string_lossy(), &[])
            .await
            .expect("execute")
    }
}

#[test]
fn native_swift_dispatch_declares_each_service_lifetime() {
    let root = tempfile::tempdir().expect("project");
    let staged = NativeSwiftPageLifetimeCheck::stage(root.path());
    let source = std::fs::read_to_string(staged).expect("staged companion");
    let namespaces = ["counter", "tally", "label", "broken"]
        .into_iter()
        .enumerate()
        .map(|(index, service)| {
            format!("case \"{service}\": return TachyonService{index}.tacPrototype")
        })
        .collect::<Vec<_>>();
    assert!(
        namespaces.iter().all(|line| source.contains(line)),
        "{source}"
    );
    assert_eq!(
        source.matches("static let tacPrototype = true").count(),
        1,
        "only the explicit prototype is per activation"
    );
    assert_eq!(source.matches("static let tacPrototype = false").count(), 3);
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn native_swift_page_singletons_follow_the_documented_service_lifetime() {
    assert!(
        NativeSwiftPageLifetimeCheck::run()
            .await
            .contains("capability ownership passed")
    );
}
