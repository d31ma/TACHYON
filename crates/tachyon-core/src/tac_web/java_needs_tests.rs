#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaNeedsFixtures;

impl JavaNeedsFixtures {
    const FILES: [(&str, &str); 8] = [
        (
            "client/models/CounterBumped.java",
            "record CounterBumped(int count) {}\n",
        ),
        (
            "client/services/CounterService.java",
            "import models.CounterBumped;\n\n@Service\nclass CounterService {\n    int count = 0;\n    @Needs\n    private final ApplicationEventPublisher publisher;\n\n    \
             @OnStart\n    void start() {\n        System.out.println(\"needs: start counter\");\n    }\n\n    \
             @OnStop\n    void stop() {\n        System.out.println(\"needs: stop counter\");\n    }\n\n    \
             int increment() {\n        count += 1;\n        publisher.publishEvent(new CounterBumped(count));\n        return count;\n    }\n}\n",
        ),
        (
            "client/services/TallyService.java",
            "@Service\nclass TallyService {\n    @Needs\n    private final CounterService counter;\n    int total = 0;\n\n    \
             @OnStart\n    void start() {\n        System.out.println(\"needs: start tally after \" + counter.count);\n    }\n\n    \
             @OnStop\n    void stop() {\n        System.out.println(\"needs: stop tally at \" + counter.count);\n    }\n\n    \
             int bump() {\n        total = counter.increment();\n        return total;\n    }\n}\n",
        ),
        (
            "client/services/WatchService.java",
            "import models.CounterBumped;\n\n@Service\nclass WatchService {\n    int seen = 0;\n\n    \
             @OnEvent\n    void saw(CounterBumped value) {\n        seen = value.count();\n    }\n}\n",
        ),
        (
            "client/services/GreeterService.java",
            "@Service\nclass GreeterService {\n    String greet() {\n        return \"hi\";\n    }\n}\n",
        ),
        (
            "client/services/LoudGreeterService.java",
            "@Service\nclass LoudGreeterService extends GreeterService {\n    @Override\n    String greet() {\n        return \"HI\";\n    }\n}\n",
        ),
        (
            "client/services/WelcomeService.java",
            "@Service\nclass WelcomeService {\n    @Needs\n    @Named(\"LoudGreeterService\")\n    private final GreeterService greeter;\n\n    \
             String welcome() {\n        return greeter.greet() + \"!\";\n    }\n}\n",
        ),
        (
            "client/services/HelloService.java",
            "@Service\nclass HelloService {\n    @Needs\n    private final GreeterService greeter;\n\n    \
             String hello() {\n        return greeter.greet() + \".\";\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const modules = {}\n\
         for (const name of ['Counter', 'Tally', 'Watch', 'Welcome', 'Hello']) modules[name] = await import(`./${name}Service.js`)\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         bus.attach({ receive: (value) => events.push(value.count) }, { listeners: [{ method: 'receive', event: 'CounterBumped', order: -100 }] }, {})\n\
         const open = async (name) => {\n\
         \x20 const module = modules[name]\n\
         \x20 const owner = new module.default({})\n\
         \x20 const plan = module.__tachyon_event_plan__?.types?.[0]\n\
         \x20 if (plan) { bus.attach(owner, plan, { applicationEventPublisher: null }); await bus.initialize(owner) } else owner.__tachyonStarted()\n\
         \x20 return owner\n\
         }\n\
         const [created, disposed] = ['__CREATED__', '__DISPOSED__']\n\
         const owners = {}\n\
         for (const name of [...created.split(','), 'Watch']) owners[name] = await open(name)\n\
         const { Counter: counter, Tally: tally, Watch: watch } = owners\n\
         const bumped = [tally.bump(), tally.bump(), counter.increment()]\n\
         const seen = { count: counter.count, total: tally.total, watched: watch.seen }\n\
         const [first, second] = disposed.split(',')\n\
         owners[first].__tachyonDispose()\n\
         let later = null\n\
         try { later = first === 'Counter' ? tally.bump() : counter.increment() } catch (error) { later = String(error?.message ?? error) }\n\
         owners[second].__tachyonDispose()\n\
         const again = await open('Tally')\n\
         const resumed = again.bump()\n\
         again.__tachyonDispose()\n\
         const welcome = await open('Welcome'), hello = await open('Hello')\n\
         const greeted = [welcome.welcome(), hello.hello()]\n\
         welcome.__tachyonDispose(); hello.__tachyonDispose()\n\
         watch.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify({ bumped, seen, later, resumed, events, greeted, keys: Object.keys(tally), lines: lines.filter((line) => line.startsWith('needs: ')) })}\\n`)\n\
         process.exit(0)\n";

    async fn publish(root: &Path) {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = write(root, relative, source);
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(".java"))
            else {
                continue;
            };
            let instance = Self::instance(class);
            services.push((instance, String::from(class), WebLanguage::Java, path));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .expect("a Java service that needs another Java service builds");
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }

    fn instance(class: &str) -> String {
        let short = class.trim_end_matches("Service");
        let mut characters = short.chars();
        characters.next().map_or_else(String::new, |first| {
            format!("{}{}", first.to_ascii_lowercase(), characters.as_str())
        })
    }

    fn drive(root: &Path, created: &str, disposed: &str) -> serde_json::Value {
        let script = Self::DRIVER
            .replace("__CREATED__", created)
            .replace("__DISPOSED__", disposed);
        run_driver(root, &script)
    }
}

#[tokio::test]
async fn a_java_web_service_shares_the_java_service_it_needs_with_the_page() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaNeedsFixtures::publish(root.path()).await;
    for (created, disposed, stopped) in [
        ("Tally,Counter", "Counter,Tally", 4),
        ("Counter,Tally", "Tally,Counter", 3),
    ] {
        let report = JavaNeedsFixtures::drive(root.path(), created, disposed);
        assert_eq!(report["bumped"], serde_json::json!([1, 2, 3]), "{report}");
        assert_eq!(
            report["seen"],
            serde_json::json!({"count": 3, "total": 2, "watched": 3}),
            "{report}"
        );
        assert_eq!(report["later"], 4, "{report}");
        assert_eq!(report["resumed"], 5, "{report}");
        assert_eq!(
            report["events"],
            serde_json::json!([1, 2, 3, 4, 5]),
            "{report}"
        );
        assert_eq!(report["greeted"], serde_json::json!(["HI!", "hi."]));
        assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
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
