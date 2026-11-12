use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::fylo_root_lock::RootHolder;
use super::{ManagedStorage, fylo_store_with_lease};

fn fylo_has_invocation_mode() -> bool {
    let directory = tempfile::tempdir().expect("probe root");
    let output = Command::new("fylo")
        .args(["exec", "--invocation", "--request", r#"{"op":"handshake"}"#])
        .args([
            "--deadline-ms",
            "5000",
            "--cleanup-reserve-ms",
            "0",
            "--root",
        ])
        .arg(directory.path())
        .output();
    let answer = output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    let available = answer.is_ok_and(|answer| answer.contains("EBADREQUEST"));
    assert!(
        available || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
        "a fylo with exec --invocation (FYLO ADR 0028) is required on this runner"
    );
    available
}

fn canonical_root(directory: &tempfile::TempDir) -> PathBuf {
    std::fs::canonicalize(directory.path()).expect("canonical root")
}

async fn call_store(root: &Path, deadline_factor: u32) -> ManagedStorage {
    fylo_store_with_lease(root, "call", deadline_factor)
        .await
        .unwrap_or_else(|error| panic!("{error}"))
}

fn document(command: &str, fields: &Value) -> Value {
    let mut request = json!({"operation":"document","command":command,"collection":"hd-events"});
    for (key, value) in fields.as_object().expect("fields") {
        request[key] = value.clone();
    }
    request
}

async fn run(store: &ManagedStorage, command: &Value) -> Value {
    store
        .execute(command)
        .await
        .unwrap_or_else(|error| panic!("{error}"))
}

async fn create_events(store: &ManagedStorage) {
    run(store, &document("createCollection", &json!({}))).await;
}

async fn document_count(store: &ManagedStorage) -> usize {
    let page = run(store, &document("scroll", &json!({"size": 100}))).await;
    page["content"].as_array().map_or(0, Vec::len)
}

async fn insert_many(store: &ManagedStorage, writer: &str, count: usize) -> Vec<Value> {
    let inserts = (0..count).map(|number| {
        let value = json!({"writer": writer, "number": number});
        let command = document("insert", &json!({ "value": value }));
        async move { store.execute(&command).await }
    });
    let answers = futures_util::future::join_all(inserts).await;
    answers
        .into_iter()
        .map(|answer| answer.unwrap_or_else(|error| panic!("{error}")))
        .collect()
}

fn invocation(root: &Path, request: &Value) -> Value {
    let output = Command::new("fylo")
        .args(["exec", "--invocation", "--request", &request.to_string()])
        .args([
            "--deadline-ms",
            "5000",
            "--cleanup-reserve-ms",
            "200",
            "--root",
        ])
        .arg(root)
        .output()
        .expect("fylo invocation");
    serde_json::from_slice(&output.stdout).expect("fylo answer")
}

#[tokio::test]
async fn two_call_lease_stores_on_one_root_take_turns_without_a_root_lock_failure() {
    if !fylo_has_invocation_mode() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let first = call_store(&root, 6).await;
    let second = call_store(&root, 6).await;
    create_events(&first).await;
    for _ in 0..2 {
        let (left, right) = tokio::join!(
            insert_many(&first, "first", 12),
            insert_many(&second, "second", 12)
        );
        assert_eq!(left.len() + right.len(), 24);
    }
    assert_eq!(document_count(&first).await, 48);
    assert_eq!(document_count(&second).await, 48);
}

#[tokio::test]
async fn a_call_waits_for_a_root_held_by_a_fylo_loop_and_then_fails_with_the_root_lock_code() {
    if !fylo_has_invocation_mode() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let holder = RootHolder::take(&root);
    let store = call_store(&root, 1).await;
    let started = Instant::now();
    let insert = document("insert", &json!({"value": {"n": 1}}));
    let failure = store
        .execute(&insert)
        .await
        .expect_err("a held root fails")
        .to_string();
    let waited = started.elapsed();
    assert!(
        failure.contains("TY2160") && failure.contains("FYLO operation failed (EROOTLOCKED)."),
        "{failure}"
    );
    assert!(waited >= Duration::from_secs(4), "{waited:?}");
    holder.release();
    assert_eq!(document_count(&store).await, 0);
}

#[tokio::test]
async fn a_root_released_inside_the_deadline_lets_the_waiting_call_answer() {
    if !fylo_has_invocation_mode() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let holder = RootHolder::take(&root);
    let store = call_store(&root, 1).await;
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        holder.release();
    });
    let answer = store
        .execute(&document("collectionExists", &json!({})))
        .await;
    releaser.join().expect("releaser");
    assert_eq!(answer.expect("released root answers"), json!(true));
}

#[tokio::test]
async fn an_insert_carries_a_tachyon_identifier_and_a_repeated_insert_is_stored_once() {
    if !fylo_has_invocation_mode() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = call_store(&root, 6).await;
    create_events(&store).await;
    let saved = run(&store, &document("insert", &json!({"value": {"n": 1}}))).await;
    let id = saved["id"].as_str().expect("id").to_owned();
    assert!(crate::ttid_created_at(&id).is_some(), "{id}");
    let repeated = json!({"op":"putData","collection":"hd-events","id":id,"data":{"n":1}});
    let answer = invocation(&root, &repeated);
    assert_eq!(answer["ok"], json!(false), "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("already exists")),
        "{answer}"
    );
    assert_eq!(document_count(&store).await, 1);
    let found = run(&store, &document("findById", &json!({ "id": id }))).await;
    assert_eq!(found["n"], json!(1));
}

#[tokio::test]
async fn every_document_and_queue_command_answers_in_call_mode() {
    if !fylo_has_invocation_mode() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = call_store(&root, 6).await;
    create_events(&store).await;
    let saved = run(&store, &document("insert", &json!({"value": {"n": 1}}))).await;
    let id = saved["id"].clone();
    let patched = run(
        &store,
        &document("patch", &json!({"id": id, "value": {"n": 2}})),
    )
    .await;
    assert_eq!(patched["n"], json!(2));
    let deleted = run(&store, &document("deleteById", &json!({ "id": id }))).await;
    assert_eq!(deleted, json!(true));
    let published =
        json!({"operation":"queue","command":"publish","topic":"app.render","payload":{"page":1}});
    assert!(run(&store, &published).await.is_string());
    let claim =
        json!({"operation":"queue","command":"claim","topic":"app.render","group":"renderers"});
    let deliveries = run(&store, &claim).await;
    assert_eq!(deliveries[0]["payload"], json!({"page": 1}));
    let ack = json!({"operation":"queue","command":"ack","topic":"app.render","group":"renderers","id":deliveries[0]["id"],"receipt":deliveries[0]["receipt"]});
    assert_eq!(run(&store, &ack).await, json!(true));
    let products =
        json!({"operation":"document","command":"createCollection","collection":"products"});
    run(&store, &products).await;
    let insert = json!({"operation":"sql","sql":"INSERT INTO products (name) VALUES ($1)","parameters":["lamp"],"mode":"execute"});
    assert_eq!(run(&store, &insert).await["affectedRows"], json!(1));
    let select = json!({"operation":"sql","sql":"SELECT * FROM products WHERE name = $1","parameters":["lamp"],"mode":"rows"});
    assert_eq!(run(&store, &select).await[0]["name"], json!("lamp"));
    let dropped = run(&store, &document("dropCollection", &json!({}))).await;
    assert_eq!(dropped, json!(true));
}

#[tokio::test]
async fn an_unknown_lease_is_refused_before_fylo_starts() {
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let refused = fylo_store_with_lease(&root, "request", 1).await;
    let failure = refused.expect_err("unknown lease").to_string();
    assert!(
        failure.contains("\"server\" (the default) or \"call\""),
        "{failure}"
    );
}

#[tokio::test]
#[ignore = "latency benchmark; run with --run-ignored only"]
async fn fylo_latency_per_operation_in_each_lease_mode() {
    if !fylo_has_invocation_mode() {
        return;
    }
    for lease in ["server", "call"] {
        let directory = tempfile::tempdir().expect("root");
        let root = canonical_root(&directory);
        let store = fylo_store_with_lease(&root, lease, 6)
            .await
            .unwrap_or_else(|error| panic!("{error}"));
        create_events(&store).await;
        let mut inserts = Vec::new();
        let mut reads = Vec::new();
        for number in 0..40 {
            let started = Instant::now();
            let saved = run(
                &store,
                &document("insert", &json!({"value": {"n": number}})),
            )
            .await;
            inserts.push(started.elapsed());
            let started = Instant::now();
            run(&store, &document("findById", &json!({"id": saved["id"]}))).await;
            reads.push(started.elapsed());
        }
        println!(
            "fylo lease {lease} on {}: insert {}, findById {}",
            std::env::consts::OS,
            percentiles(inserts),
            percentiles(reads)
        );
    }
}

fn percentiles(mut samples: Vec<Duration>) -> String {
    samples.sort();
    let at = |share: usize| samples[(samples.len() - 1) * share / 100].as_secs_f64() * 1000.0;
    format!("p50 {:.1} ms / p95 {:.1} ms", at(50), at(95))
}
