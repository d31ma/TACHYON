use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};

use super::{execute, fylo_is_available, fylo_url, is_rejected, sqlite};
use crate::storage::ManagedStorage;

fn fylo_runs_here() -> bool {
    if fylo_is_available() {
        return true;
    }
    assert!(
        std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
        "fylo binary is required on this runner"
    );
    false
}

fn canonical_root(directory: &tempfile::TempDir) -> PathBuf {
    std::fs::canonicalize(directory.path()).expect("canonical root")
}

async fn connect(root: &Path) -> ManagedStorage {
    let url = fylo_url(root);
    ManagedStorage::connect(&json!({"yon":{"datasource":{"driver":"fylo","url":url}}}))
        .await
        .unwrap_or_else(|error| panic!("{error}"))
}

fn document(command: &str, fields: Value) -> Value {
    let mut request = json!({"operation":"document","command":command,"collection":"hd-events"});
    let Value::Object(fields) = fields else {
        panic!("fields");
    };
    for (key, value) in fields {
        request[&key] = value;
    }
    request
}

fn queue(command: &str, fields: Value) -> Value {
    let mut request = json!({"operation":"queue","command":command,"topic":"app.render"});
    let Value::Object(fields) = fields else {
        panic!("fields");
    };
    for (key, value) in fields {
        request[&key] = value;
    }
    request
}

fn fylo_request(root: &Path, request: &Value) -> Value {
    let output = std::process::Command::new("fylo")
        .args(["exec", "--request", &request.to_string(), "--root"])
        .arg(root)
        .output()
        .expect("fylo request");
    let answer: Value = serde_json::from_slice(&output.stdout).expect("fylo answer");
    assert_eq!(answer["ok"], json!(true), "{answer}");
    answer["result"].clone()
}

fn base36(mut number: u128) -> String {
    let digits = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut text = Vec::new();
    while number > 0 {
        let digit = usize::try_from(number % 36).expect("digit");
        text.push(digits[digit]);
        number /= 36;
    }
    text.reverse();
    String::from_utf8(text).expect("ascii")
}

fn write_many_documents(root: &Path, count: u128) {
    fylo_request(
        root,
        &json!({"op":"createCollection","collection":"hd-events"}),
    );
    let first = fylo_request(
        root,
        &json!({"op":"putData","collection":"hd-events","data":{"n":0}}),
    );
    let start = u128::from_str_radix(first.as_str().expect("TTID"), 36).expect("base 36");
    for index in 1..count {
        let id = base36(start + index * 7);
        let shard = root
            .join(".collections/hd-events/docs")
            .join(&id[id.len() - 1..]);
        std::fs::create_dir_all(&shard).expect("shard");
        std::fs::write(
            shard.join(format!("{id}.json")),
            format!("{{\"n\":{index}}}"),
        )
        .expect("document");
    }
    let rebuilt = fylo_request(
        root,
        &json!({"op":"rebuildCollection","collection":"hd-events"}),
    );
    assert_eq!(rebuilt["indexedDocs"], json!(count));
}

#[test]
fn fylo_collection_names_allow_hyphens_and_refuse_unsafe_names() {
    use crate::storage::fylo::CollectionName;
    for allowed in ["hd-events", "hd-target-groups", "products", "a1", "x-9"] {
        assert!(CollectionName::checked(allowed).is_ok(), "{allowed}");
    }
    let refused = [
        "",
        "a",
        "-events",
        "events-",
        "../etc",
        "a/b",
        "a\\b",
        ".hidden",
        "hd_events",
        "HD-events",
        "queue",
        "sql",
        "hd events",
        "hd-événements",
    ];
    for name in refused {
        assert!(CollectionName::checked(name).is_err(), "{name}");
    }
    assert!(CollectionName::checked(&"a".repeat(65)).is_err());
}

#[tokio::test]
async fn document_and_queue_commands_need_the_fylo_datasource() {
    let store = sqlite().await;
    assert!(is_rejected(&store, &document("findById", json!({"id":"4WCVU2P9BO0"}))).await);
    assert!(is_rejected(&store, &queue("publish", json!({"payload":{"x":1}}))).await);
}

#[tokio::test]
async fn fylo_documents_round_trip_in_a_hyphenated_collection() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = connect(&canonical_root(&directory)).await;
    let exists = document("collectionExists", json!({}));
    assert_eq!(execute(&store, exists.clone()).await, json!(false));
    assert_eq!(
        execute(&store, document("createCollection", json!({}))).await,
        json!(true)
    );
    assert_eq!(
        execute(&store, document("createCollection", json!({}))).await,
        json!(true)
    );
    assert_eq!(execute(&store, exists.clone()).await, json!(true));
    let saved = execute(
        &store,
        document("insert", json!({"value":{"kind":"deploy","n":1}})),
    )
    .await;
    let id = saved["id"].as_str().expect("TTID").to_owned();
    let found = execute(&store, document("findById", json!({"id":id}))).await;
    assert_eq!(found, json!({"id":id,"kind":"deploy","n":1}));
    let patch = document("patch", json!({"id":id,"value":{"n":2,"note":null}}));
    let patched = execute(&store, patch).await;
    assert_eq!(patched, json!({"id":id,"kind":"deploy","n":2,"note":null}));
    let renaming = document("patch", json!({"id":id,"value":{"id":"4WCVU2P9BO0","n":3}}));
    assert!(is_rejected(&store, &renaming).await);
    let assigned = document("insert", json!({"value":{"id":"4WCVU2P9BO0","kind":"x"}}));
    assert!(is_rejected(&store, &assigned).await);
    let delete = document("deleteById", json!({"id":id}));
    assert_eq!(execute(&store, delete.clone()).await, json!(true));
    assert_eq!(execute(&store, delete).await, json!(false));
    assert_eq!(
        execute(&store, document("findById", json!({"id":id}))).await,
        Value::Null
    );
    let absent_patch = document("patch", json!({"id":id,"value":{"n":4}}));
    assert_eq!(execute(&store, absent_patch).await, Value::Null);
    let drop = document("dropCollection", json!({}));
    assert_eq!(execute(&store, drop.clone()).await, json!(true));
    assert_eq!(execute(&store, drop).await, json!(false));
    assert_eq!(execute(&store, exists).await, json!(false));
    let unsafe_name =
        json!({"operation":"document","command":"createCollection","collection":"../etc"});
    assert!(is_rejected(&store, &unsafe_name).await);
    store.close().await;
}

#[tokio::test]
async fn fylo_stores_accept_a_hyphenated_table() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = connect(&canonical_root(&directory)).await;
    execute(&store, document("createCollection", json!({}))).await;
    let save = json!({"operation":"save","table":"hd-events","entity":"Event","idField":"id","value":{"kind":"a"}});
    let saved = execute(&store, save).await;
    assert!(saved["id"].is_string(), "{saved}");
    store.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_fylo_patches_never_lose_an_update() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = Arc::new(connect(&canonical_root(&directory)).await);
    execute(&store, document("createCollection", json!({}))).await;
    let saved = execute(
        &store,
        document("insert", json!({"value":{"kind":"counter"}})),
    )
    .await;
    let id = saved["id"].as_str().expect("TTID").to_owned();
    let mut tasks = Vec::new();
    for index in 0..12 {
        let store = Arc::clone(&store);
        let patch = document(
            "patch",
            json!({"id":id,"value":{format!("field{index}"):index}}),
        );
        tasks.push(tokio::spawn(async move { execute(&store, patch).await }));
    }
    for task in tasks {
        task.await.expect("patch task");
    }
    let found = execute(&store, document("findById", json!({"id":id}))).await;
    for index in 0..12 {
        assert_eq!(found[format!("field{index}")], json!(index), "{found}");
    }
    assert_eq!(found["kind"], json!("counter"));
    store.close().await;
}

#[tokio::test]
async fn fylo_scroll_pages_past_ten_thousand_documents_with_a_cursor() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    write_many_documents(&root, 10_050);
    let store = connect(&root).await;
    let mut cursor = Value::Null;
    let mut seen = Vec::new();
    loop {
        let page = execute(
            &store,
            document("scroll", json!({"size":100,"cursor":cursor})),
        )
        .await;
        let content = page["content"].as_array().expect("content");
        assert!(content.len() <= 100);
        for row in content {
            seen.push(row["id"].as_str().expect("id").to_owned());
        }
        cursor = page["nextCursor"].clone();
        if cursor.is_null() {
            break;
        }
    }
    assert_eq!(seen.len(), 10_050);
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "stable id order"
    );
    let filtered = execute(
        &store,
        document("scroll", json!({"size":5,"where":{"n":10_049}})),
    )
    .await;
    assert_eq!(filtered["content"][0]["n"], json!(10_049));
    assert_eq!(filtered["nextCursor"], Value::Null);
    let forged = document("scroll", json!({"size":5,"cursor":"fqc1.1.1"}));
    assert!(is_rejected(&store, &forged).await);
    store.close().await;
}

#[tokio::test]
async fn fylo_queue_publishes_claims_and_settles_deliveries() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = connect(&canonical_root(&directory)).await;
    let publish = queue(
        "publish",
        json!({"payload":{"app":"web"},"key":"render:web"}),
    );
    let first = execute(&store, publish.clone()).await;
    assert_eq!(execute(&store, publish).await, first, "idempotent publish");
    let claim = queue(
        "claim",
        json!({"group":"renderers","maxMessages":5,"leaseMs":30_000}),
    );
    let claimed = execute(&store, claim.clone()).await;
    assert_eq!(claimed.as_array().map(Vec::len), Some(1));
    let delivery = &claimed[0];
    assert_eq!(delivery["payload"], json!({"app":"web"}));
    assert_eq!(delivery["attempt"], json!(1));
    let settle = json!({"group":"renderers","id":delivery["id"],"receipt":delivery["receipt"]});
    assert_eq!(
        execute(&store, queue("ack", settle.clone())).await,
        json!(true)
    );
    assert_eq!(
        execute(&store, queue("ack", settle)).await,
        json!(true),
        "an acknowledgement retry with the same receipt is idempotent"
    );
    let forged = json!({"group":"renderers","id":delivery["id"],"receipt":"00ff"});
    assert!(
        is_rejected(&store, &queue("ack", forged)).await,
        "a wrong receipt"
    );
    execute(&store, queue("publish", json!({"payload":{"app":"api"}}))).await;
    let retried = execute(&store, claim.clone()).await;
    let nack = json!({"group":"renderers","id":retried[0]["id"],"receipt":retried[0]["receipt"],"delayMs":0});
    assert_eq!(execute(&store, queue("nack", nack)).await, json!(true));
    let again = execute(&store, claim).await;
    assert_eq!(again[0]["attempt"], json!(2));
    assert_eq!(again[0]["payload"], json!({"app":"api"}));
    let bad_topic = json!({"operation":"queue","command":"publish","topic":"../x","payload":1});
    assert!(is_rejected(&store, &bad_topic).await);
    store.close().await;
}
