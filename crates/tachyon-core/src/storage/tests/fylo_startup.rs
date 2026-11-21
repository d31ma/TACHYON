use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{ManagedStorage, fylo_is_available, fylo_url};

struct PausedFylo;

impl PausedFylo {
    const PAUSE: Duration = Duration::from_millis(1_500);

    fn process_id(root: &Path) -> String {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(format!("fylo exec --loop --root {}", root.display()))
            .output()
            .expect("pgrep");
        let ids = String::from_utf8_lossy(&output.stdout);
        ids.lines()
            .next()
            .expect("one fylo process for the root")
            .trim()
            .to_owned()
    }

    fn signal(process_id: &str, signal: &str) -> bool {
        Command::new("kill")
            .args([signal, process_id])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn pause_for_a_while(process_id: &str) -> JoinHandle<bool> {
        Self::pause_for(process_id, Self::PAUSE)
    }

    fn pause_for(process_id: &str, pause: Duration) -> JoinHandle<bool> {
        assert!(Self::signal(process_id, "-STOP"), "pause fylo");
        let process_id = process_id.to_owned();
        std::thread::spawn(move || {
            std::thread::sleep(pause);
            Self::signal(&process_id, "-CONT")
        })
    }
}

fn fylo_runs_here() -> bool {
    let available = fylo_is_available();
    assert!(
        available || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
        "fylo binary is required on this runner"
    );
    available
}

fn canonical_root(directory: &tempfile::TempDir) -> PathBuf {
    std::fs::canonicalize(directory.path()).expect("canonical root")
}

async fn store_with_one_second(root: &Path) -> ManagedStorage {
    let datasource = json!({"driver": "fylo", "url": fylo_url(root), "timeout": 1000});
    ManagedStorage::connect_with_deadline_factor(&json!({"yon": {"datasource": datasource}}), 1)
        .await
        .expect("store")
}

fn document(command: &str, extra: &Value) -> Value {
    let mut document =
        json!({"operation": "document", "command": command, "collection": "hd-events"});
    for (key, value) in extra.as_object().expect("extra fields") {
        document[key] = value.clone();
    }
    document
}

#[tokio::test]
async fn a_first_request_slower_than_the_deadline_finishes_and_the_same_fylo_stays() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = store_with_one_second(&root).await;
    let first = PausedFylo::process_id(&root);
    let resumed = PausedFylo::pause_for_a_while(&first);
    let started = Instant::now();
    let created = store
        .execute(&document("createCollection", &json!({})))
        .await;
    let took = started.elapsed();
    assert!(resumed.join().expect("resume thread"), "resume fylo");
    created.expect("the first request after a start gets the start-up deadline");
    assert!(took >= Duration::from_millis(1_400), "{took:?}");
    for count in 0..3 {
        let insert = document("insert", &json!({"value": {"n": count}}));
        store.execute(&insert).await.expect("later calls answer");
    }
    assert_eq!(
        PausedFylo::process_id(&root),
        first,
        "the slow fylo was not stopped and started again"
    );
    store.close().await;
}

#[tokio::test]
async fn a_later_request_to_the_same_collection_keeps_the_normal_deadline() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = store_with_one_second(&root).await;
    store
        .execute(&document("createCollection", &json!({})))
        .await
        .expect("collection");
    let fylo = PausedFylo::process_id(&root);
    let resumed = PausedFylo::pause_for_a_while(&fylo);
    let started = Instant::now();
    let failure = store
        .execute(&document("insert", &json!({"value": {"n": 1}})))
        .await
        .expect_err("a request that is not the first passes its deadline")
        .to_string();
    let took = started.elapsed();
    assert!(resumed.join().expect("resume thread"), "resume fylo");
    assert!(
        failure.contains("deadline exceeded during FYLO putData"),
        "{failure}"
    );
    assert!(took < Duration::from_millis(1_400), "{took:?}");
    let exists = store
        .execute(&document("collectionExists", &json!({})))
        .await;
    assert_eq!(exists.expect("the next call answers"), json!(true));
    store.close().await;
}

async fn insert_then_close(store: &ManagedStorage) -> (Result<Value, crate::Failure>, Duration) {
    let insert = document("insert", &json!({"value": {"n": 1}}));
    let close_soon = async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let started = Instant::now();
        store.close().await;
        started.elapsed()
    };
    tokio::join!(store.execute(&insert), close_soon)
}

async fn stored_documents(root: &Path) -> usize {
    let reader = store_with_one_second(root).await;
    let find = json!({"operation": "document", "command": "scroll", "collection": "hd-events", "size": 100});
    let answer = reader.execute(&find).await.expect("scroll");
    reader.close().await;
    answer["content"].as_array().map_or(0, Vec::len)
}

#[tokio::test]
async fn closing_the_store_stops_an_idle_fylo_at_once() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = store_with_one_second(&root).await;
    store
        .execute(&document("createCollection", &json!({})))
        .await
        .expect("collection");
    let fylo = PausedFylo::process_id(&root);
    let started = Instant::now();
    store.close().await;
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    assert!(!PausedFylo::signal(&fylo, "-0"), "fylo has exited");
}

#[tokio::test]
async fn closing_the_store_during_a_write_lets_the_write_finish() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = store_with_one_second(&root).await;
    store
        .execute(&document("createCollection", &json!({})))
        .await
        .expect("collection");
    let fylo = PausedFylo::process_id(&root);
    let resumed = PausedFylo::pause_for(&fylo, Duration::from_millis(400));
    let (inserted, closing) = insert_then_close(&store).await;
    assert!(resumed.join().expect("resume thread"), "resume fylo");
    inserted.expect("the write in progress answers");
    assert!(closing < Duration::from_secs(1), "{closing:?}");
    assert_eq!(stored_documents(&root).await, 1);
}

#[tokio::test]
async fn closing_the_store_waits_for_a_stuck_fylo_only_a_short_time() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = store_with_one_second(&root).await;
    store
        .execute(&document("createCollection", &json!({})))
        .await
        .expect("collection");
    let fylo = PausedFylo::process_id(&root);
    assert!(PausedFylo::signal(&fylo, "-STOP"), "pause fylo");
    let (_inserted, closing) = insert_then_close(&store).await;
    PausedFylo::signal(&fylo, "-KILL");
    assert!(closing < Duration::from_millis(1_500), "{closing:?}");
}

#[tokio::test]
async fn a_first_write_to_a_second_collection_after_a_start_gets_the_allowance() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let setup = store_with_one_second(&root).await;
    for name in ["people", "local-parts"] {
        let create =
            json!({"operation": "document", "command": "createCollection", "collection": name});
        setup.execute(&create).await.expect("collection");
    }
    setup.close().await;
    let store = store_with_one_second(&root).await;
    let person = json!({"operation": "document", "command": "insert", "collection": "people", "value": {"name": "a"}});
    store.execute(&person).await.expect("first write");
    let fylo = PausedFylo::process_id(&root);
    let resumed = PausedFylo::pause_for_a_while(&fylo);
    let started = Instant::now();
    let part = json!({"operation": "document", "command": "insert", "collection": "local-parts", "value": {"part": "a"}});
    let written = store.execute(&part).await;
    let took = started.elapsed();
    assert!(resumed.join().expect("resume thread"), "resume fylo");
    written.expect("the first write to the second collection gets the start-up allowance");
    assert!(took >= Duration::from_millis(1_400), "{took:?}");
    store.close().await;
}
