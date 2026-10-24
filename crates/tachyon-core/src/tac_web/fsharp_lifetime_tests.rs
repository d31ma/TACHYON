#![allow(clippy::expect_used)]

use super::fsharp_tests::{fsharp_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use crate::template::ServiceRegistry;
use serde_json::json;
use std::path::Path;

struct FsharpLifetimeFixtures;

impl FsharpLifetimeFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/services/TallyService.fs",
            "[<Service>]\n[<Lifetime({\"value\":\"prototype\"})>]\ntype TallyService() =\n    \
             member val Count = 0 with get, set\n\n    \
             member this.Add(by: int) : int =\n        this.Count <- this.Count + by\n        this.Count\n",
        ),
        (
            "client/services/BoardService.fs",
            "[<Service>]\ntype BoardService() =\n    \
             member val Count = 0 with get, set\n\n    \
             member this.Add(by: int) : int =\n        this.Count <- this.Count + by\n        this.Count\n",
        ),
        (
            "client/services/ScoreService.fs",
            "[<Service>]\ntype ScoreService() =\n    [<Needs>]\n    \
             member val Board : BoardService = Unchecked.defaultof<BoardService> with get, set\n\n    \
             member this.Raise() : int =\n        this.Board.Add(10)\n",
        ),
    ];

    const DRIVER: &str = "const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const modules = {}\n\
         for (const name of ['Tally', 'Board', 'Score']) modules[name] = await import(`./${name}Service.js`)\n\
         const open = async (name) => {\n\
         \x20 const module = modules[name]\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const firstTally = await open('Tally'), secondTally = await open('Tally')\n\
         firstTally.add(2)\n\
         const tallies = [firstTally.count, secondTally.count]\n\
         const board = await open('Board'), score = await open('Score')\n\
         const added = board.add(3)\n\
         const raised = score.raise()\n\
         const boards = [added, raised, board.count]\n\
         for (const owner of [firstTally, secondTally, score, board]) owner.__tachyonDispose()\n\
         console.log(JSON.stringify({ tallies, boards }))\n\
         process.exit(0)\n";

    async fn publish(root: &Path) {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = write(root, relative, source);
            let class = relative
                .trim_start_matches("client/services/")
                .trim_end_matches(".fs");
            services.push((
                Self::instance(class),
                String::from(class),
                WebLanguage::CSharp,
                path,
            ));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Release,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
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
        class.trim_end_matches("Service").to_ascii_lowercase()
    }

    fn page_scopes(root: &Path) -> Vec<bool> {
        let registry = ServiceRegistry::discover(root).expect("registry");
        ["tally", "board", "score"]
            .map(|instance| registry.get(instance).expect(instance).is_prototype())
            .to_vec()
    }
}

#[tokio::test]
async fn an_fsharp_prototype_service_gives_each_component_its_own_object() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpLifetimeFixtures::publish(root.path()).await;
    assert_eq!(
        FsharpLifetimeFixtures::page_scopes(root.path()),
        [true, false, false]
    );
    let report = run_driver(root.path(), FsharpLifetimeFixtures::DRIVER);

    assert_eq!(report["tallies"], json!([2, 0]), "{report}");
    assert_eq!(report["boards"], json!([3, 13, 13]), "{report}");
}
