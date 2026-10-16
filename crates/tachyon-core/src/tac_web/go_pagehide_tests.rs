#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct GoPageHide;

impl GoPageHide {
    const FILES: [(&str, &str); 4] = [
        (
            "client/models/LoadFailed.go",
            "type LoadFailed struct {\n\tCode string `json:\"code\"`\n}\n",
        ),
        (
            "client/models/LoadAsked.go",
            "type LoadAsked struct {\n\tCount int `json:\"count\"`\n}\n",
        ),
        (
            "client/remotes/SlowClient.go",
            "// @HTTPClient(\"/api\")\ntype SlowClient interface {\n\t// @GetRequest(\"/slow\")\n\tRead() (BehaviorValue, error)\n}\n",
        ),
        (
            "client/setups/SlowSetup.go",
            "// @Setup\ntype SlowSetup struct{}\n\n\
             func (SlowSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\
             \treturn NewBehaviorConfiguration(), nil\n}\n",
        ),
    ];

    const SERVICES: [(&str, &str, &str); 3] = [
        (
            "wait",
            "WaitService",
            "import \"fmt\"\n\n// @Service\n// @Lifetime({\"value\":\"prototype\"})\ntype WaitService struct {\n\t// @Needs\n\tslow      *SlowClient\n\tpublisher ApplicationEventPublisher\n\tCount     int\n}\n\n\
             // @OnStop\nfunc (wait *WaitService) Stop() {\n\tfmt.Println(\"pagehide: stop wait\")\n}\n\n\
             func (wait *WaitService) Load() string {\n\t_, err := wait.slow.Read()\n\treturn BehaviorErrorCode(err)\n}\n\n\
             func (wait *WaitService) LoadThenFallBack() string {\n\tcode := wait.Load()\n\twait.publisher.PublishEvent(LoadFailed{Code: code}, false)\n\tfmt.Println(\"pagehide: fell back after\", code)\n\treturn code + \" then \" + wait.Load()\n}\n\n\
             // @OnEvent\nfunc (wait *WaitService) Asked(asked LoadAsked) {\n\twait.Count = asked.Count\n\twait.Load()\n}\n",
        ),
        (
            "share",
            "ShareService",
            "import \"fmt\"\n\n// @Service\ntype ShareService struct {\n\t// @Needs\n\tslow      *SlowClient\n\tpublisher ApplicationEventPublisher\n}\n\n\
             func (share *ShareService) LoadThenFallBack() string {\n\t_, err := share.slow.Read()\n\tcode := BehaviorErrorCode(err)\n\tshare.publisher.PublishEvent(LoadFailed{Code: code}, false)\n\tfmt.Println(\"pagehide: shared fell back after\", code)\n\t_, err = share.slow.Read()\n\treturn code + \" then \" + BehaviorErrorCode(err)\n}\n",
        ),
        (
            "other",
            "OtherService",
            "import \"fmt\"\n\n// @Service\ntype OtherService struct {\n\tCount int\n}\n\n\
             // @OnStop\nfunc (other *OtherService) Stop() {\n\tfmt.Println(\"pagehide: stop other\")\n}\n",
        ),
    ];

    const DRIVER: &str = r"import { readFile } from 'node:fs/promises'
import { writeSync } from 'node:fs'
const scenario = process.argv[2]
const phase = (name) => { writeSync(1, `${name}\n`) }
const requests = []
globalThis.location = { href: 'http://127.0.0.1:4000/page', origin: 'http://127.0.0.1:4000' }
globalThis.fetch = async (url, init = {}) => {
  if (String(url).startsWith('file:')) return new Response(await readFile(url))
  const request = { path: new URL(url).pathname, aborted: false }
  requests.push(request)
  return new Promise((_, reject) => {
    init.signal?.addEventListener('abort', () => { request.aborted = true; reject(init.signal.reason) }, { once: true })
  })
}
const lines = []
for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))
await import('./tac-http.mjs')
const { default: ApplicationEvents } = await import('./application-events.mjs')
const { default: WaitService, __tachyon_event_plan__: waitPlan } = await import('./WaitService.js')
const { default: OtherService, __tachyon_event_plan__: otherPlan } = await import('./OtherService.js')
const { default: ShareService, __tachyon_event_plan__: sharePlan } = await import('./ShareService.js')
let failures = 0
const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })
const wait = new WaitService({}), other = new OtherService({}), share = new ShareService({})
bus.attach(wait, waitPlan.types[0], { applicationEventPublisher: null })
bus.attach(other, otherPlan.types[0], { applicationEventPublisher: null })
bus.attach(share, sharePlan.types[0], { applicationEventPublisher: null })
await bus.initialize(wait)
await bus.initialize(other)
await bus.initialize(share)
const settle = (promise) => Promise.resolve(promise).then((value) => ({ value }), (error) => ({ error: String(error?.code || error?.message || error) }))
const calls = []
if (scenario === 'one' || scenario === 'two') calls.push(settle(wait.load()))
if (scenario === 'two') calls.push(settle(wait.load()))
if (scenario === 'fallback' || scenario === 'fallbacks') calls.push(settle(wait.loadThenFallBack()))
if (scenario === 'fallbacks') calls.push(settle(wait.loadThenFallBack()))
if (scenario === 'event') bus.publish('LoadAsked', { count: 1 })
if (scenario === 'shared') calls.push(settle(share.loadThenFallBack()), settle(share.loadThenFallBack()))
await new Promise((resolve) => setTimeout(resolve, 100))
const before = requests.length
phase('disposing')
const began = performance.now()
bus.dispose(share)
bus.dispose(wait)
bus.dispose(other)
const elapsed = Math.round(performance.now() - began)
const during = requests.length
phase('disposed')
await new Promise((resolve) => setTimeout(resolve, 300))
const outcomes = await Promise.race([Promise.all(calls), new Promise((resolve) => setTimeout(() => resolve('unsettled'), 2000))])
phase(`${JSON.stringify({ scenario, before, during, after: requests.length, elapsed, aborted: requests.filter((request) => request.aborted).length, outcomes, failures, lines })}`)
process.exit(0)
";

    fn available(program: &str, argument: &str) -> bool {
        Command::new(program)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    async fn publish(root: &Path, profile: crate::BuildProfile) -> Vec<String> {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text.as_bytes());
        }
        let mut services = Vec::new();
        for (instance, class, text) in Self::SERVICES {
            let path = Self::write(
                root,
                &format!("client/services/{class}.go"),
                text.as_bytes(),
            );
            services.push((
                String::from(instance),
                String::from(class),
                WebLanguage::Go,
                path,
            ));
        }
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
        Self::write(
            root,
            "tac-http.mjs",
            include_str!("../template/tac-http.js").as_bytes(),
        );
        Self::write(root, "drive.mjs", Self::DRIVER.as_bytes());
        built.notes
    }

    fn drive(root: &Path, scenario: &str) -> serde_json::Value {
        let mut child = Command::new("node")
            .args(["drive.mjs", scenario])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node");
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut hung = false;
        while child.try_wait().expect("node status").is_none() {
            if Instant::now() > deadline {
                hung = true;
                let _killed = child.kill();
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let output = child.wait_with_output().expect("node output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let last = stdout.lines().last().unwrap_or_default();
        if hung {
            return serde_json::json!({ "scenario": scenario, "hung": true, "phase": last });
        }
        assert!(
            output.status.success(),
            "{scenario}: {stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(last).expect("report")
    }

    const OUTCOMES: [(&str, &str, u64); 6] = [
        ("one", "[{\"value\":\"CANCELLED\"}]", 1),
        (
            "two",
            "[{\"value\":\"CANCELLED\"},{\"value\":\"CANCELLED\"}]",
            2,
        ),
        ("fallback", "[{\"value\":\"CANCELLED then CANCELLED\"}]", 1),
        (
            "fallbacks",
            "[{\"value\":\"CANCELLED then CANCELLED\"},{\"value\":\"CANCELLED then CANCELLED\"}]",
            2,
        ),
        ("event", "[]", 1),
        (
            "shared",
            "[{\"value\":\"CANCELLED then CANCELLED\"},{\"value\":\"CANCELLED then CANCELLED\"}]",
            2,
        ),
    ];

    fn assert_prompt(root: &Path) {
        let reports: Vec<serde_json::Value> = Self::OUTCOMES
            .iter()
            .map(|(scenario, _, _)| Self::drive(root, scenario))
            .collect();
        for report in &reports {
            eprintln!("pagehide report: {report}");
        }
        for ((_, outcomes, pending), report) in Self::OUTCOMES.into_iter().zip(reports) {
            assert!(
                report.get("hung").is_none(),
                "pagehide never returned: {report}"
            );
            assert!(
                report["elapsed"]
                    .as_u64()
                    .is_some_and(|elapsed| elapsed < 1000),
                "pagehide disposal took too long: {report}"
            );
            for count in ["before", "during", "after", "aborted"] {
                assert_eq!(report[count], pending, "{count}: {report}");
            }
            assert_eq!(
                report["outcomes"],
                serde_json::from_str::<serde_json::Value>(outcomes).expect("outcomes"),
                "{report}"
            );
            assert_eq!(report["failures"], 0, "{report}");
            for (index, line) in ["pagehide: stop wait", "pagehide: stop other"]
                .iter()
                .enumerate()
            {
                assert_eq!(
                    report["lines"][index], *line,
                    "OnStop runs inside the disposal: {report}"
                );
            }
        }
    }
}

#[tokio::test]
async fn a_go_page_disposes_services_with_pending_requests_promptly() {
    if !GoPageHide::available("go", "version") || !GoPageHide::available("node", "--version") {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = GoPageHide::publish(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    GoPageHide::assert_prompt(root.path());
}

#[tokio::test]
async fn a_tinygo_page_disposes_services_with_pending_requests_promptly() {
    let tools = [
        GoPageHide::available("go", "version"),
        GoPageHide::available("tinygo", "version"),
        GoPageHide::available("node", "--version"),
    ];
    if tools.contains(&false) {
        eprintln!("skipping: go, tinygo or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoPageHide::write(
        root.path(),
        "client/manifest.json",
        b"{\"tac\":{\"go\":{\"compiler\":\"tinygo\"}}}",
    );
    let notes = GoPageHide::publish(root.path(), crate::BuildProfile::Release).await;
    assert!(notes[0].starts_with("Go: TinyGo"), "{notes:?}");
    GoPageHide::assert_prompt(root.path());
}
