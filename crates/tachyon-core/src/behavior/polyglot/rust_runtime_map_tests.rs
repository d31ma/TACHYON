#![allow(clippy::expect_used)]

use std::process::Command;

struct RuntimeMaps;

impl RuntimeMaps {
    const PROGRAM: &str = r#"
fn main() {
    let repeated = BehaviorValue::object([
        ("b", BehaviorValue::Number(2.0)),
        ("a", BehaviorValue::Number(1.0)),
        ("b", BehaviorValue::Number(3.0)),
    ]);
    println!("{}", repeated.to_json(4096).expect("object"));
    let owned = BehaviorValue::object(vec![
        (String::from("z"), BehaviorValue::Null),
        (String::from("m"), BehaviorValue::Bool(true)),
    ]);
    println!("{}", owned.to_json(4096).expect("owned object"));
    let feature = BehaviorValue::object([
        ("flag", BehaviorValue::from("on")),
        ("level", BehaviorValue::Number(2.0)),
    ]);
    let api = BehaviorValue::object([("base", BehaviorValue::from("/v1"))]);
    let application = BehaviorValue::object([
        ("yon", BehaviorValue::object([("secret", BehaviorValue::from("hidden"))])),
        ("tac", BehaviorValue::object([("feature", feature), ("api", api)])),
    ]);
    println!("{:?}", application.behavior_property_inputs().expect("inputs"));
    let policy = BehaviorQueryPolicy {
        entity: BehaviorEntityMetadata { name: "Product", adapter: "main", table: "products", id: "id" },
        fields: vec!["id", "name"],
        default_size: 1,
        max_size: 1,
        operation: "findOne",
        predicates: vec![BehaviorQueryPredicate { field: "name", operator: "eq", argument: Some(0) }],
        order_by: Vec::new(),
        page_argument: None,
    };
    let query = behavior_query(&policy, &[BehaviorValue::from("lamp")]).expect("query");
    println!("{}", query.to_json(65_536).expect("query json"));
}
"#;
    const EXPECTED: &str = concat!(
        "{\"a\":1,\"b\":3}\n",
        "{\"m\":true,\"z\":null}\n",
        "{\"TAC_API_BASE\": \"/v1\", \"TAC_FEATURE_FLAG\": \"on\", \"TAC_FEATURE_LEVEL\": \"2\"}\n",
        "{\"kind\":\"find\",\"orderBy\":[],\"page\":{\"offset\":0,\"page\":0,\"size\":1,\"sort\":[]},",
        "\"where\":[{\"field\":\"name\",\"operator\":\"eq\",\"value\":\"lamp\"}]}\n"
    );

    fn executable_name() -> &'static str {
        if cfg!(windows) { "maps.exe" } else { "maps" }
    }

    fn run() -> String {
        let folder = tempfile::tempdir().expect("folder");
        let source = folder.path().join("maps.rs");
        let executable = folder.path().join(Self::executable_name());
        let runtime = super::rust::runtime("map-test", "tac");
        std::fs::write(&source, format!("{runtime}{}", Self::PROGRAM)).expect("source");
        let compiled = Command::new("rustc")
            .args(["--edition", "2024", "-A", "warnings"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let ran = Command::new(&executable).output().expect("program");
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        String::from_utf8(ran.stdout).expect("text")
    }
}

#[test]
fn the_rust_behavior_runtime_keeps_the_last_value_of_a_key_and_orders_keys() {
    assert_eq!(RuntimeMaps::run(), RuntimeMaps::EXPECTED);
}
