#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

struct SharedUtilityDiscovery;

impl SharedUtilityDiscovery {
    const TAC_EXTENSIONS: [&'static str; 13] = [
        "js", "ts", "dart", "rs", "go", "rb", "swift", "kt", "java", "cs", "fs", "lua", "py",
    ];

    fn composed(folder: &str, extension: &str) -> (Vec<PathBuf>, PathBuf) {
        let root = tempfile::tempdir().expect("project");
        let entry = PathBuf::from(format!("client/services/ReachService.{extension}"));
        let utility = PathBuf::from(format!("{folder}/TextUtils.{extension}"));
        for relative in [&entry, &utility] {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, "").expect("source");
        }
        let directory =
            super::open_entry_root(root.path(), &entry, extension, "tac").expect("entry root");
        let paths = super::discover_paths(&directory, &entry, extension, false, false)
            .expect("composed paths");
        (paths, utility)
    }
}

#[test]
fn every_tac_language_composes_a_utility_from_a_nested_shared_utils_folder() {
    for extension in SharedUtilityDiscovery::TAC_EXTENSIONS {
        let (paths, utility) =
            SharedUtilityDiscovery::composed("client/shared/utils/nested", extension);
        assert!(paths.contains(&utility), "{extension}: {paths:?}");
    }
}

#[test]
fn no_tac_language_composes_a_utility_from_the_removed_client_utils_folder() {
    for extension in SharedUtilityDiscovery::TAC_EXTENSIONS {
        let (paths, utility) = SharedUtilityDiscovery::composed("client/utils/nested", extension);
        assert!(!paths.contains(&utility), "{extension}: {paths:?}");
        assert!(
            paths.iter().all(|path| !path.starts_with(Path::new("client/utils"))),
            "{extension}: {paths:?}"
        );
    }
}
