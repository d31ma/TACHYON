#![allow(clippy::expect_used)]

use super::fsharp_tests::fsharp_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;
use std::process::Command;

struct FsharpNeedsFixtures;

impl FsharpNeedsFixtures {
    const FILES: [(&str, &str); 7] = [
        (
            "client/models/CounterBumped.fs",
            "type CounterBumped = { Count: int }\n",
        ),
        (
            "client/services/CounterService.fs",
            "open Models\n\n[<Service>]\ntype CounterService() =\n    [<Needs>]\n    member val Publisher : ApplicationEventPublisher = Unchecked.defaultof<ApplicationEventPublisher> with get, set\n    member val Count = 0 with get, set\n\n    \
             [<OnStart>]\n    member this.Start() : unit =\n        printfn \"needs: start counter\"\n\n    \
             [<OnStop>]\n    member this.Stop() : unit =\n        printfn \"needs: stop counter\"\n\n    \
             member this.Increment() : int =\n        this.Count <- this.Count + 1\n        this.Publisher.PublishEvent({ CounterBumped.Count = this.Count })\n        this.Count\n",
        ),
        (
            "client/services/TallyService.fs",
            "[<Service>]\ntype TallyService() =\n    [<Needs>]\n    member val Counter : CounterService = Unchecked.defaultof<CounterService> with get, set\n    member val Total = 0 with get, set\n\n    \
             [<OnStart>]\n    member this.Start() : unit =\n        printfn \"needs: start tally after %d\" this.Counter.Count\n\n    \
             [<OnStop>]\n    member this.Stop() : unit =\n        printfn \"needs: stop tally at %d\" this.Counter.Count\n\n    \
             member this.Bump() : int =\n        this.Total <- this.Counter.Increment()\n        this.Total\n",
        ),
        (
            "client/services/WatchService.fs",
            "open Models\n\n[<Service>]\ntype WatchService() =\n    member val Seen = 0 with get, set\n\n    \
             [<OnEvent>]\n    member this.Saw(value: CounterBumped) : unit =\n        this.Seen <- value.Count\n",
        ),
        (
            "client/services/GreeterService.fs",
            "[<Service>]\ntype GreeterService() =\n    abstract Greet : unit -> string\n    default this.Greet() : string = \"hello\"\n",
        ),
        (
            "client/services/LoudGreeterService.fs",
            "[<Service>]\ntype LoudGreeterService() =\n    inherit GreeterService()\n    override this.Greet() : string = \"HI\"\n",
        ),
        (
            "client/services/WelcomeService.fs",
            "[<Service>]\ntype WelcomeService() =\n    [<Needs>]\n    [<Named(\"LoudGreeterService\")>]\n    member val Greeter : GreeterService = Unchecked.defaultof<GreeterService> with get, set\n\n    \
             member this.Welcome() : string =\n        this.Greeter.Greet()\n",
        ),
    ];

    const DRIVER: &str = "const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const modules = {}\n\
         for (const name of ['Counter', 'Tally', 'Watch', 'Welcome']) modules[name] = await import(`./${name}Service.js`)\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         bus.attach({ receive: (value) => events.push(value.count) }, { listeners: [{ method: 'receive', event: 'CounterBumped', order: -100 }] }, {})\n\
         const [created, disposed] = process.argv.slice(2)\n\
         const owners = {}\n\
         const open = async (name) => {\n\
         \x20 const module = modules[name]\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         for (const name of [...created.split(','), 'Watch', 'Welcome']) owners[name] = await open(name)\n\
         const { Counter: counter, Tally: tally, Watch: watch, Welcome: welcome } = owners\n\
         const bumped = [tally.bump(), tally.bump(), counter.increment()]\n\
         const seen = { count: counter.count, total: tally.total, watched: watch.seen }\n\
         const welcomed = welcome.welcome()\n\
         const [first, second] = disposed.split(',')\n\
         owners[first].__tachyonDispose()\n\
         const later = first === 'Counter' ? tally.bump() : counter.increment()\n\
         owners[second].__tachyonDispose()\n\
         const again = await open('Tally')\n\
         const resumed = again.bump()\n\
         again.__tachyonDispose()\n\
         watch.__tachyonDispose()\n\
         welcome.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify({ bumped, seen, welcomed, later, resumed, events, keys: Object.keys(tally), welcomeKeys: Object.keys(welcome), lines: lines.filter((line) => line.startsWith('needs: ')) })}\\n`)\n\
         process.exit(0)\n";

    fn instance(class: &str) -> String {
        let name = class.trim_end_matches("Service");
        let mut letters = name.chars();
        let first = letters.next().map(|letter| letter.to_ascii_lowercase());
        first.into_iter().chain(letters).collect()
    }

    async fn publish(root: &Path) {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(&path, source).expect("source");
            let class = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(".fs"));
            services.extend(class.map(|class| {
                (
                    Self::instance(class),
                    String::from(class),
                    WebLanguage::CSharp,
                    path.clone(),
                )
            }));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Release,
        )
        .await
        .expect("an F# service that needs another F# service builds");
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
        std::fs::write(
            root.join("application-events.mjs"),
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        )
        .expect("event runtime");
        std::fs::write(root.join("drive.mjs"), Self::DRIVER).expect("driver");
    }

    fn drive(root: &Path, created: &str, disposed: &str) -> serde_json::Value {
        let output = Command::new("node")
            .args(["drive.mjs", created, disposed])
            .current_dir(root)
            .output()
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }

    fn assert_shared(root: &Path) {
        for (created, disposed, stopped) in [
            ("Tally,Counter", "Counter,Tally", 4),
            ("Counter,Tally", "Tally,Counter", 3),
        ] {
            let report = Self::drive(root, created, disposed);
            assert_eq!(report["bumped"], serde_json::json!([1, 2, 3]), "{report}");
            assert_eq!(
                report["seen"],
                serde_json::json!({"count": 3, "total": 2, "watched": 3}),
                "{report}"
            );
            assert_eq!(report["welcomed"], "HI", "{report}");
            assert_eq!(report["later"], 4, "{report}");
            assert_eq!(report["resumed"], 5, "{report}");
            assert_eq!(
                report["events"],
                serde_json::json!([1, 2, 3, 4, 5]),
                "{report}"
            );
            assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
            assert_eq!(report["welcomeKeys"], serde_json::json!([]), "{report}");
            assert_eq!(
                report["lines"],
                serde_json::json!([
                    "needs: start counter",
                    "needs: start tally after 0",
                    format!("needs: stop tally at {stopped}"),
                    "needs: start tally after 4",
                    "needs: stop tally at 5",
                    "needs: stop counter",
                ]),
                "{created} then {disposed}: {report}"
            );
        }
    }
}

#[tokio::test]
async fn an_fsharp_service_shares_the_fsharp_service_it_needs_with_the_page() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpNeedsFixtures::publish(root.path()).await;
    FsharpNeedsFixtures::assert_shared(root.path());
}
