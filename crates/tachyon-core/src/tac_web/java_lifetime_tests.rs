#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use crate::template::ServiceRegistry;
use serde_json::json;
use std::path::Path;

struct JavaLifetimeFixtures;

impl JavaLifetimeFixtures {
    const FILES: [(&str, &str); 4] = [
        (
            "client/services/TallyService.java",
            "@Service\n@Lifetime({\"value\":\"prototype\"})\nclass TallyService {\n    int count = 0;\n\n    \
             int add(int by) {\n        count += by;\n        return count;\n    }\n}\n",
        ),
        (
            "client/services/BoardService.java",
            "@Service\nclass BoardService {\n    int count = 0;\n\n    \
             int add(int by) {\n        count += by;\n        return count;\n    }\n}\n",
        ),
        (
            "client/services/ScoreService.java",
            "@Service\nclass ScoreService {\n    @Needs\n    private final BoardService board;\n\n    \
             int raise() {\n        return board.add(10);\n    }\n}\n",
        ),
        (
            "client/services/GreetingService.java",
            "@Service\n@NeedsAll\nclass GreetingService {\n    String name;\n    int limit;\n    int greeted = 0;\n\n    \
             String greet() {\n        greeted += 1;\n        return name + \" may take \" + limit;\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const modules = {}\n\
         for (const name of ['Tally', 'Board', 'Score', 'Greeting']) modules[name] = (await import(`./${name}Service.js`)).default\n\
         const open = (name, properties) => { const owner = new modules[name](properties); owner.__tachyonStarted(); return owner }\n\
         const firstTally = open('Tally', {}), secondTally = open('Tally', {})\n\
         firstTally.add(2)\n\
         const tallies = [firstTally.count, secondTally.count]\n\
         const board = open('Board', {}), score = open('Score', {})\n\
         board.add(3)\n\
         const raised = score.raise()\n\
         const boards = [board.count, raised]\n\
         const ann = open('Greeting', { name: 'Ann', limit: 2 }), bo = open('Greeting', { name: 'Bo', limit: 5 })\n\
         const greetings = [ann.greet(), bo.greet(), ann.greet()]\n\
         const greeted = [ann.greeted, bo.greeted]\n\
         for (const owner of [firstTally, secondTally, score, board, ann, bo]) owner.__tachyonDispose()\n\
         console.log(JSON.stringify({ tallies, boards, greetings, greeted }))\n\
         process.exit(0)\n";

    async fn publish(root: &Path) {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = write(root, relative, source);
            let class = relative
                .trim_start_matches("client/services/")
                .trim_end_matches(".java");
            services.push((
                Self::instance(class),
                String::from(class),
                WebLanguage::Java,
                path,
            ));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
    }

    fn instance(class: &str) -> String {
        class.trim_end_matches("Service").to_ascii_lowercase()
    }

    fn page_scopes(root: &Path) -> Vec<bool> {
        let registry = ServiceRegistry::discover(root).expect("registry");
        ["tally", "board", "score", "greeting"]
            .map(|instance| registry.get(instance).expect(instance).is_prototype())
            .to_vec()
    }
}

#[tokio::test]
async fn a_java_lifetime_and_needs_all_service_gives_each_component_its_own_object() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaLifetimeFixtures::publish(root.path()).await;
    assert_eq!(
        JavaLifetimeFixtures::page_scopes(root.path()),
        [true, false, false, true]
    );
    let report = run_driver(root.path(), JavaLifetimeFixtures::DRIVER);

    assert_eq!(report["tallies"], json!([2, 0]), "{report}");
    assert_eq!(report["boards"], json!([13, 13]), "{report}");
    assert_eq!(
        report["greetings"],
        json!(["Ann may take 2", "Bo may take 5", "Ann may take 2"]),
        "{report}"
    );
    assert_eq!(report["greeted"], json!([2, 1]), "{report}");
}
