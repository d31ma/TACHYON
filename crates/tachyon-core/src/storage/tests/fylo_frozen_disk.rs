use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{Value, json};

use super::{ManagedStorage, fylo_is_available, fylo_url};

struct FrozenDisk {
    _directory: tempfile::TempDir,
    mount: PathBuf,
    frozen: bool,
}

impl FrozenDisk {
    fn create() -> Self {
        let directory = tempfile::tempdir().expect("disk folder");
        let image = directory.path().join("disk.img");
        let mount = directory.path().join("mounted");
        std::fs::create_dir(&mount).expect("mount point");
        assert!(
            run("truncate", &["-s", "128M", &text(&image)]),
            "image file"
        );
        assert!(sudo(&["mkfs.ext4", "-q", "-F", &text(&image)]), "mkfs.ext4");
        assert!(
            sudo(&["mount", "-o", "loop", &text(&image), &text(&mount)]),
            "loop mount"
        );
        let owner = format!("{}:{}", id("-u"), id("-g"));
        assert!(sudo(&["chown", &owner, &text(&mount)]), "chown");
        Self {
            _directory: directory,
            mount,
            frozen: false,
        }
    }

    fn root(&self) -> PathBuf {
        let root = self.mount.join("root");
        std::fs::create_dir(&root).expect("root");
        std::fs::canonicalize(root).expect("canonical root")
    }

    fn freeze(&mut self) {
        assert!(sudo(&["fsfreeze", "-f", &text(&self.mount)]), "fsfreeze -f");
        self.frozen = true;
    }

    fn thaw_later(&mut self, after: Duration) -> std::thread::JoinHandle<bool> {
        self.frozen = false;
        let mount = text(&self.mount);
        std::thread::spawn(move || {
            std::thread::sleep(after);
            sudo(&["fsfreeze", "-u", &mount])
        })
    }
}

impl Drop for FrozenDisk {
    fn drop(&mut self) {
        if self.frozen {
            sudo(&["fsfreeze", "-u", &text(&self.mount)]);
        }
        sudo(&["umount", "-l", &text(&self.mount)]);
    }
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

fn run(program: &str, arguments: &[&str]) -> bool {
    Command::new(program)
        .args(arguments)
        .status()
        .is_ok_and(|status| status.success())
}

fn sudo(arguments: &[&str]) -> bool {
    Command::new("sudo")
        .arg("-n")
        .args(arguments)
        .status()
        .is_ok_and(|status| status.success())
}

fn id(option: &str) -> String {
    let output = Command::new("id").arg(option).output().expect("id");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn fylo_process_ids(root: &Path) -> Vec<String> {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(format!("fylo exec --loop --root {}", root.display()))
        .output()
        .expect("pgrep");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn process_state(process_id: &str) -> String {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{process_id}/stat")) else {
        return String::from("gone");
    };
    let after_name = stat.rsplit_once(") ").map_or("", |(_, rest)| rest);
    after_name.split(' ').next().unwrap_or_default().to_owned()
}

fn document(command: &str, extra: &Value) -> Value {
    let mut request = json!({"operation":"document","command":command,"collection":"hd-events"});
    for (key, value) in extra.as_object().expect("fields") {
        request[key] = value.clone();
    }
    request
}

#[tokio::test]
#[ignore = "Linux only: needs passwordless sudo, a loop device, mkfs.ext4 and fsfreeze; run with --run-ignored only in the privileged Linux CI container"]
async fn a_fylo_frozen_in_a_disk_write_is_waited_for_and_no_second_fylo_starts() {
    assert!(fylo_is_available(), "fylo binary is required");
    let mut disk = FrozenDisk::create();
    let root = disk.root();
    let datasource = json!({"driver":"fylo","url":fylo_url(&root),"timeout":2000});
    let store =
        ManagedStorage::connect_with_deadline_factor(&json!({"yon":{"datasource":datasource}}), 1)
            .await
            .expect("store");
    store
        .execute(&document("createCollection", &json!({})))
        .await
        .expect("collection");
    let insert = document("insert", &json!({"value": {"n": 1}}));
    store.execute(&insert).await.expect("first insert");
    let exists = document("collectionExists", &json!({}));

    disk.freeze();
    let failure = store
        .execute(&insert)
        .await
        .expect_err("a write on a frozen disk passes the deadline");
    assert!(
        failure.diagnostics()[0]
            .message
            .contains("deadline exceeded during FYLO putData"),
        "{failure}"
    );
    let stuck = fylo_process_ids(&root);
    assert_eq!(stuck.len(), 1, "{stuck:?}");
    assert_eq!(
        process_state(&stuck[0]),
        "D",
        "killed but still in the disk write"
    );

    let failure = store
        .execute(&exists)
        .await
        .expect_err("the next call waits for the stuck fylo and passes its own deadline")
        .to_string();
    assert!(failure.contains("deadline exceeded"), "{failure}");
    assert!(!failure.contains("EROOTLOCKED"), "{failure}");
    assert_eq!(
        fylo_process_ids(&root),
        stuck,
        "no second fylo while the first is alive"
    );

    let thaw = disk.thaw_later(Duration::from_millis(500));
    let answer = store.execute(&exists).await;
    assert!(thaw.join().expect("thaw thread"), "fsfreeze -u");
    assert_eq!(
        answer.expect("the call after the thaw answers"),
        json!(true)
    );
    assert_eq!(
        process_state(&stuck[0]),
        "gone",
        "the stuck fylo has exited"
    );
    for _ in 0..3 {
        store
            .execute(&insert)
            .await
            .expect("no EROOTLOCKED after the thaw");
    }
    store.close().await;
}
