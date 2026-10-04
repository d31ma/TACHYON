#![allow(clippy::expect_used)]

use super::HandlerRuntimePrograms;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

struct InstalledPrograms;

impl InstalledPrograms {
    fn folder(names: &[&str]) -> tempfile::TempDir {
        let folder = tempfile::tempdir().expect("program folder");
        for name in names {
            let file_name = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                (*name).to_owned()
            };
            let path = folder.path().join(file_name);
            std::fs::write(&path, "").expect("program");
            Self::make_runnable(&path);
        }
        folder
    }

    #[cfg(unix)]
    fn make_runnable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
    }

    #[cfg(not(unix))]
    fn make_runnable(_path: &Path) {}

    fn javascript(names: &[&str], configured: Option<&str>) -> PathBuf {
        let folder = Self::folder(names);
        HandlerRuntimePrograms::from_lookup(|name| match name {
            "PATH" => Some(folder.path().as_os_str().to_os_string()),
            "YON_JAVASCRIPT_RUNTIME" => configured.map(OsString::from),
            _ => None,
        })
        .javascript
    }
}

#[test]
fn bun_runs_javascript_when_node_is_missing() {
    assert_eq!(
        InstalledPrograms::javascript(&["bun"], None),
        Path::new("bun")
    );
    assert_eq!(
        InstalledPrograms::javascript(&["bun", "node"], None),
        Path::new("node")
    );
    assert_eq!(InstalledPrograms::javascript(&[], None), Path::new("node"));
    assert_eq!(
        InstalledPrograms::javascript(&["bun"], Some("deno")),
        Path::new("deno")
    );
}
