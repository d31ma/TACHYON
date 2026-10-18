use super::CppYon;
use std::path::Path;

struct Policies;

impl Policies {
    const CONTROLLER_PATH: &'static str = "server/controllers/CatalogController.hpp";
    const FRAME: &'static str = r#"{"method":"GET","route":"/catalog"}"#;
    const CONTROLLER: &'static str = r#"#pragma once
#include "../services/CatalogService.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] CatalogService& catalog;
    [[Needs]] ProductStore& products;

public:
    [[GetRoute]]
    Report getReport() {
        Report report;
        report.first = catalog.price("tea", "a");
        report.again = catalog.price("tea", "b");
        report.reads = catalog.reads;
        catalog.forget("tea");
        report.forgotten = catalog.price("tea", "c");
        report.updated = catalog.write("tea", 99);
        report.kept = catalog.price("tea", "d");
        report.steady = catalog.steady();
        report.once = catalog.once(true);
        report.page = catalog.paged(tachyon::json::JsonNode::parse("{\"page\":2}")).getValue<double>();
        catalog.add(ProductEntity{"one", "active", 3});
        catalog.add(ProductEntity{"two", "active", 5});
        catalog.add(ProductEntity{"three", "hidden", 7});
        try {
            catalog.addThenRefuse(ProductEntity{"four", "active", 1});
        } catch (const std::exception&) {
        }
        report.active = static_cast<int>(products.findByStatus("active").size());
        report.count = static_cast<int>(products.countByStatus("active"));
        report.found = products.findById("two").has_value();
        report.cheap = static_cast<int>(products.cheaperThan(6).size());
        report.removed = static_cast<int>(products.deleteById("three"));
        report.all = static_cast<int>(products.findAll().size());
        return report;
    }
};
"#;
    const FILES: [(&'static str, &'static str); 5] = [
        (
            "server/models/Report.hpp",
            "#pragma once\n\nstruct Report {\n    int first = 0;\n    int again = 0;\n    int reads = 0;\n    int forgotten = 0;\n    int updated = 0;\n    int kept = 0;\n    int steady = 0;\n    std::string once;\n    double page = 0;\n    int active = 0;\n    int count = 0;\n    bool found = false;\n    int cheap = 0;\n    int removed = 0;\n    int all = 0;\n};\n",
        ),
        (
            "server/entities/ProductEntity.hpp",
            "#pragma once\n#include <string>\n\nstruct [[Entity({\"adapter\":\"data\"}), Table(\"products\")]] ProductEntity {\n    [[Id]] std::string id;\n    std::string status;\n    int price = 0;\n};\n",
        ),
        (
            "server/stores/ProductStore.hpp",
            r#"#pragma once
#include <vector>

class [[Store(ProductEntity, {"fields":["id","status","price"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]] ProductStore : public tachyon::Repository<ProductEntity, std::string> {
public:
    std::vector<ProductEntity> findByStatus(const std::string& status);
    long long countByStatus(const std::string& status);
    [[Query({"operation":"find","where":[{"field":"price","operator":"lt","argument":0}],"orderBy":[]})]]
    std::vector<ProductEntity> cheaperThan(int price);
};
"#,
        ),
        (
            "server/setups/CatalogSetup.hpp",
            r#"#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        BehaviorAdapters adapters;
        adapters.caches["prices"] = BehaviorMemory::cache();
        adapters.entities["data"] = BehaviorMemory::entities();
        adapters.authorize = [](const BehaviorCacheCall& call, const BehaviorExecution& execution) {
            return call.owner == "CatalogService" && execution.partition == "catalog";
        };
        adapters.authorizeEntity = [](const tachyon::json::JsonNode& command, const BehaviorExecution&) {
            return command["table"].getValue<std::string>() == "products";
        };
        return adapters;
    }
};
"#,
        ),
        (
            "server/services/CatalogService.hpp",
            r#"#pragma once
#include <stdexcept>
#include <string>

class [[Service, CacheSettings({"name":"prices","ttlMs":60000,"maxBytes":4096,"maxEntries":16})]] CatalogService {
    BehaviorContext behaviors;
    [[Needs]] ProductStore& products;

public:
    int reads = 0;
    int attempts = 0;

    [[Cached]]
    int price([[CacheKey]] std::string sku, std::string note) {
        reads += 1;
        return static_cast<int>(sku.size()) * 10 + reads;
    }

    [[UpdateCache({"method":"price"})]]
    int write(std::string sku, int value) {
        return value;
    }

    [[ClearCache({"method":"price"})]]
    void forget(std::string sku) {}

    [[Retry({"maxRetries":3,"delayMs":1,"multiplier":1,"maxDelayMs":5,"jitterMs":0})]]
    int steady() {
        attempts += 1;
        if (attempts < 3) {
            throw std::runtime_error("not yet");
        }
        return attempts;
    }

    [[MaxAtOnce({"limit":1,"policy":"reject"})]]
    std::string once(bool again) {
        if (!again) {
            return "inside";
        }
        try {
            return once(false);
        } catch (const tachyon::TacCodedError& refused) {
            return refused.code();
        }
    }

    tachyon::json::JsonNode paged([[PageDefaults({"defaultSize":5,"maxSize":10,"sort":["id"]})]] tachyon::json::JsonNode page) {
        return page["offset"];
    }

    ProductEntity add(ProductEntity product) {
        return products.save(product);
    }

    [[AllOrNothing({"adapter":"data"})]]
    void addThenRefuse(ProductEntity product) {
        products.save(product);
        throw std::runtime_error("refused");
    }

private:
    [[OnStart]]
    void open() {
        BehaviorOptions options;
        options.partition = "catalog";
        behaviors = Behaviors::create<CatalogSetup>(options);
    }

    [[OnStop]]
    void close() { behaviors.dispose(); }
};
"#,
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

    fn lower(root: &Path) -> Result<crate::yon::Lowered, crate::Failure> {
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), Self::CONTROLLER, root)
    }

    fn refusal(changes: &[(&str, &str, &str)]) -> String {
        let root = Self::project(changes);
        Self::lower(root.path())
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
fn policies_and_stores_run_with_the_adapters_of_their_setup() {
    if !Policies::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = Policies::project(&[]);
    let program = Policies::lower(root.path()).expect("lowered").source;
    let (answer, errors) = Policies::run(&program, Policies::FRAME);
    assert_eq!(answer["status"], 200, "{answer} {errors}");
    let body: serde_json::Value =
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("body JSON");
    assert_eq!(
        body,
        serde_json::json!({
            "first": 31, "again": 31, "reads": 1, "forgotten": 32, "updated": 99, "kept": 99,
            "steady": 3, "once": "capacity", "page": 10,
            "active": 2, "count": 2, "found": true, "cheap": 2, "removed": 1, "all": 2
        })
    );
}

#[test]
fn a_cache_call_that_the_setup_refuses_fails_the_route() {
    if !Policies::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let cases = [
        (
            "execution.partition == \"catalog\"",
            "execution.partition == \"other\"",
            "not authorized",
        ),
        (
            "        adapters.authorize = [](const BehaviorCacheCall& call, const BehaviorExecution& execution) {\n            return call.owner == \"CatalogService\" && execution.partition == \"catalog\";\n        };\n",
            "",
            "adapters.authorize",
        ),
        (
            "adapters.caches[\"prices\"]",
            "adapters.caches[\"other\"]",
            "no cache adapter named 'prices'",
        ),
    ];
    for (from, to, expected) in cases {
        let root = Policies::project(&[("server/setups/CatalogSetup.hpp", from, to)]);
        let program = Policies::lower(root.path()).expect("lowered").source;
        let (answer, errors) = Policies::run(&program, Policies::FRAME);
        assert_eq!(answer["status"], 500, "{answer}");
        assert!(errors.contains(expected), "{to}: {errors}");
    }
}

#[test]
fn a_policy_that_keeps_data_needs_a_behavior_context_and_a_store_needs_its_setup() {
    let failure = Policies::refusal(&[(
        "server/services/CatalogService.hpp",
        "    BehaviorContext behaviors;\n",
        "",
    )]);
    assert!(
        failure.contains("`BehaviorContext behaviors;`"),
        "{failure}"
    );
    let failure = Policies::refusal(&[(
        "server/stores/ProductStore.hpp",
        ",\"setup\":\"CatalogSetup\"",
        "",
    )]);
    assert!(
        failure.contains("there is no server/setups/ProductSetup.hpp"),
        "{failure}"
    );
    let failure = Policies::refusal(&[(
        "server/services/CatalogService.hpp",
        "[[Service, CacheSettings({\"name\":\"prices\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":16})]]",
        "[[Service]]",
    )]);
    assert!(
        failure.contains("belongs in a class marked [[CacheSettings]]"),
        "{failure}"
    );
}
