#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

struct StagedUnit;

impl StagedUnit {
    fn unit(stage: &Path) -> PathBuf {
        stage.join("unit.mjs")
    }

    fn preload(stage: &Path) -> PathBuf {
        stage.join("preload.mjs")
    }

    fn arguments(program: &str, stage: &Path, project: &Path) -> Vec<String> {
        let unit = Self::unit(stage);
        let preload = Self::preload(stage);
        let command = super::runtime_command(Path::new(program), stage, project, &unit, &preload)
            .expect("the command is built")
            .expect("the runtime is known");
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    fn file_path(specifier: &str) -> PathBuf {
        let url = url::Url::parse(specifier).expect("a module URL");
        assert_eq!(url.scheme(), "file", "{specifier}");
        url.to_file_path().expect("a file path")
    }

    fn imported(entry: &str) -> Vec<PathBuf> {
        entry
            .lines()
            .filter_map(|line| line.strip_prefix("await import(")?.strip_suffix(");"))
            .map(|literal| serde_json::from_str::<String>(literal).expect("a string literal"))
            .map(|specifier| Self::file_path(&specifier))
            .collect()
    }

    fn mapped(map: &serde_json::Value, prefix: &str) -> PathBuf {
        let specifier = map["imports"][prefix].as_str().expect("a mapped prefix");
        assert!(specifier.ends_with('/'), "{specifier}");
        Self::file_path(specifier)
    }
}

#[test]
fn node_imports_the_preload_module_by_its_file_url() {
    let stage = tempfile::tempdir().expect("stage");
    let arguments = StagedUnit::arguments("node", stage.path(), stage.path());
    let preload = arguments
        .iter()
        .skip_while(|argument| *argument != "--import")
        .nth(1)
        .expect("the preload module");
    assert_eq!(
        StagedUnit::file_path(preload),
        StagedUnit::preload(stage.path())
    );
}

#[test]
fn deno_imports_the_unit_and_maps_the_client_folders_by_their_file_urls() {
    let stage = tempfile::tempdir().expect("stage");
    let project = tempfile::tempdir().expect("project");
    let services = project.path().join("client").join("services");
    let utilities = stage.path().join("client").join("shared").join("utils");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::create_dir_all(&utilities).expect("hooked utilities");
    StagedUnit::arguments("deno", stage.path(), project.path());
    let entry = std::fs::read_to_string(stage.path().join("entry.mjs")).expect("entry");
    assert_eq!(
        StagedUnit::imported(&entry),
        [
            StagedUnit::preload(stage.path()),
            StagedUnit::unit(stage.path())
        ]
    );
    let map = std::fs::read_to_string(stage.path().join("imports.json")).expect("map");
    let map: serde_json::Value = serde_json::from_str(&map).expect("an import map");
    assert_eq!(StagedUnit::mapped(&map, "/services/"), services);
    assert_eq!(StagedUnit::mapped(&map, "/shared/utils/"), utilities);
}
