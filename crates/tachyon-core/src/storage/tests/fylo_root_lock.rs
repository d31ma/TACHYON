use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{fylo_is_available, fylo_store};

pub(super) struct RootHolder {
    child: Child,
    _output: BufReader<ChildStdout>,
}

impl RootHolder {
    pub(super) fn take(root: &Path) -> Self {
        let mut child = Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("second fylo loop");
        let mut output = BufReader::new(child.stdout.take().expect("fylo output"));
        let request = json!({"op": "createCollection", "collection": "hd-events"});
        let input = child.stdin.as_mut().expect("fylo input");
        writeln!(input, "{request}").expect("fylo request");
        let mut answer = String::new();
        output.read_line(&mut answer).expect("fylo answer");
        let answer: Value = serde_json::from_str(&answer).expect("fylo frame");
        assert_eq!(answer["ok"], json!(true), "{answer}");
        Self {
            child,
            _output: output,
        }
    }

    pub(super) fn release(mut self) {
        self.child.kill().expect("stop second fylo loop");
        self.child.wait().expect("reap second fylo loop");
    }
}

impl Drop for RootHolder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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

fn exists_command() -> Value {
    json!({"operation":"document","command":"collectionExists","collection":"hd-events"})
}

#[tokio::test]
async fn a_root_held_by_another_fylo_loop_waits_and_then_fails_as_before() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let holder = RootHolder::take(&root);
    let store = fylo_store(&root).await;
    let started = Instant::now();
    let answer = store.execute(&exists_command()).await;
    let waited = started.elapsed();
    let failure = answer.expect_err("a held root fails").to_string();
    assert!(
        failure.contains("TY2160") && failure.contains("FYLO operation failed (EROOTLOCKED)."),
        "{failure}"
    );
    assert!(waited >= Duration::from_secs(2), "{waited:?}");
    holder.release();
}

#[tokio::test]
async fn a_root_released_inside_the_wait_answers_the_call() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let holder = RootHolder::take(&root);
    let store = fylo_store(&root).await;
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        holder.release();
    });
    let answer = store.execute(&exists_command()).await;
    releaser.join().expect("releaser");
    assert_eq!(answer.expect("released root answers"), json!(true));
}

#[tokio::test]
async fn a_fylo_error_other_than_a_held_root_fails_without_waiting() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let store = fylo_store(&root).await;
    let command =
        json!({"operation":"document","command":"insert","collection":"hd-events","value":{"n":1}});
    let failure = store
        .execute(&command)
        .await
        .expect_err("no collection")
        .to_string();
    assert!(
        failure.contains("TY2160") && !failure.contains("EROOTLOCKED"),
        "{failure}"
    );
}

#[cfg(unix)]
fn fylo_process_id(root: &Path) -> String {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(format!("fylo exec --loop --root {}", root.display()))
        .output()
        .expect("pgrep");
    let ids = String::from_utf8_lossy(&output.stdout);
    let id = ids.lines().next().expect("one fylo process for the root");
    id.trim().to_owned()
}

#[cfg(unix)]
fn signal(process_id: &str, signal: &str) -> bool {
    Command::new("kill")
        .args([signal, process_id])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
#[tokio::test]
async fn a_fylo_stalled_past_the_deadline_is_stopped_before_the_next_call_starts_another() {
    if !fylo_runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = canonical_root(&directory);
    let datasource =
        json!({"driver":"fylo","url":super::fylo_url(&root),"operationTimeoutMs":1000});
    let store = super::ManagedStorage::connect_with_deadline_factor(
        &json!({"yon":{"datasource":datasource}}),
        1,
    )
    .await
    .expect("store");
    let create =
        json!({"operation":"document","command":"createCollection","collection":"hd-events"});
    store.execute(&create).await.expect("collection");
    let stalled = fylo_process_id(&root);
    assert!(signal(&stalled, "-STOP"));
    let insert =
        json!({"operation":"document","command":"insert","collection":"hd-events","value":{"n":1}});
    let failure = store
        .execute(&insert)
        .await
        .expect_err("a stalled fylo passes the deadline")
        .to_string();
    assert!(
        failure.contains("TY2160") && failure.contains("deadline exceeded during FYLO putData"),
        "{failure}"
    );
    for _ in 0..3 {
        let answer = store.execute(&exists_command()).await;
        assert_eq!(answer.expect("no EROOTLOCKED after the stall"), json!(true));
    }
    assert!(!signal(&stalled, "-0"), "the stalled fylo has exited");
    assert_ne!(fylo_process_id(&root), stalled);
}
