use super::NativeWebApis;
use crate::Failure;
use std::path::Path;

struct RustKnownModelPathsFixture;

impl RustKnownModelPathsFixture {
    const PATH: &'static str = "client/services/PlaceService.rs";
    const MODEL_PATH: &'static str = "client/models/ConnectivityChanged.rs";
    const MODEL: &'static str = "#[derive(Default)]\npub struct ConnectivityChanged {\n pub cache: std::collections::BTreeMap<i32, bool>,\n}\n";
    const SOURCE: &'static str = "#[Service]\n#[derive(Default)]\nstruct PlaceService {}\nimpl PlaceService {\n fn read(&self) -> i64 {\n  let changed: crate::models::ConnectivityChanged = crate::models::ConnectivityChanged::default();\n  changed.cache.len() as i64\n }\n}\n";

    fn root() -> Result<tempfile::TempDir, String> {
        tempfile::tempdir().map_err(|error| error.to_string())
    }

    fn model(root: &Path, model: &str) -> Result<(), String> {
        let path = root.join(Self::MODEL_PATH);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .map_err(|error| error.to_string())?;
        std::fs::write(path, model).map_err(|error| error.to_string())
    }

    fn check(root: &Path, authored: &str) -> Result<(), Failure> {
        NativeWebApis::refuse_with_models(Path::new(Self::PATH), authored, root)
    }

    fn refused(root: &Path, authored: &str) -> Result<Failure, String> {
        Self::check(root, authored)
            .err()
            .ok_or_else(|| "expected native API refusal".to_owned())
    }

    fn located(failure: &Failure, authored: &str) {
        let name = "ConnectivityChanged";
        let start = authored
            .find(name)
            .unwrap_or_else(|| panic!("missing authored name"));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1317);
        assert_eq!(diagnostic.spans[0].file, Self::PATH);
        assert_eq!(diagnostic.spans[0].start, start as u64);
        assert_eq!(diagnostic.spans[0].end, (start + name.len()) as u64);
    }
}

#[test]
fn captured_qualified_rust_model_local_paths_do_not_become_browser_events() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    RustKnownModelPathsFixture::check(root.path(), RustKnownModelPathsFixture::SOURCE)
        .map_err(|error| error.to_string())
}

#[test]
fn captured_absolute_qualified_rust_model_paths_keep_their_own_identity() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let authored =
        RustKnownModelPathsFixture::SOURCE.replace("crate::models::", "::crate::models::");
    RustKnownModelPathsFixture::check(root.path(), &authored).map_err(|error| error.to_string())
}

#[test]
fn captured_model_name_does_not_exempt_a_bare_browser_event() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let authored = "let changed: ConnectivityChanged;";
    let failure = RustKnownModelPathsFixture::refused(root.path(), authored)?;
    RustKnownModelPathsFixture::located(&failure, authored);
    Ok(())
}

#[test]
fn captured_model_name_does_not_exempt_the_crate_root_browser_event() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let authored = "let changed: crate::ConnectivityChanged;";
    let failure = RustKnownModelPathsFixture::refused(root.path(), authored)?;
    RustKnownModelPathsFixture::located(&failure, authored);
    Ok(())
}

#[test]
fn qualified_unknown_model_name_cannot_exempt_a_browser_event() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    let authored = "let changed: crate::models::ConnectivityChanged;";
    let failure = RustKnownModelPathsFixture::refused(root.path(), authored)?;
    RustKnownModelPathsFixture::located(&failure, authored);
    Ok(())
}

#[test]
fn captured_model_name_does_not_exempt_an_unrelated_qualified_module() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let authored = "let changed: other::crate::models::ConnectivityChanged;";
    let failure = RustKnownModelPathsFixture::refused(root.path(), authored)?;
    RustKnownModelPathsFixture::located(&failure, authored);
    Ok(())
}

#[test]
fn captured_model_name_does_not_exempt_a_longer_crate_identifier() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let authored = "let changed: xcrate::models::ConnectivityChanged;";
    let failure = RustKnownModelPathsFixture::refused(root.path(), authored)?;
    RustKnownModelPathsFixture::located(&failure, authored);
    Ok(())
}

#[test]
fn known_model_paths_do_not_exempt_real_clipboard_dependencies() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(root.path(), RustKnownModelPathsFixture::MODEL)?;
    let failure = RustKnownModelPathsFixture::refused(root.path(), "let clipboard: Clipboard;")?;
    assert_eq!(failure.diagnostics()[0].code.number(), 1317);
    assert!(failure.to_string().contains("Clipboard"));
    Ok(())
}

#[test]
fn model_path_mentions_in_strings_and_comments_remain_ignored() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::check(
        root.path(),
        "// crate::models::ConnectivityChanged\nlet text = \"ConnectivityChanged\";",
    )
    .map_err(|error| error.to_string())
}

#[test]
fn invalid_model_role_still_fails_before_a_known_name_exemption() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(
        root.path(),
        "#[Service]\npub struct ConnectivityChanged {}\n",
    )?;
    let failure =
        RustKnownModelPathsFixture::refused(root.path(), RustKnownModelPathsFixture::SOURCE)?;
    assert_ne!(failure.diagnostics()[0].code.number(), 1317);
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        RustKnownModelPathsFixture::MODEL_PATH
    );
    Ok(())
}

#[test]
fn default_native_api_refusal_does_not_infer_a_project_model_exemption() -> Result<(), String> {
    let failure = NativeWebApis::refuse(
        Path::new(RustKnownModelPathsFixture::PATH),
        RustKnownModelPathsFixture::SOURCE,
    )
    .err()
    .ok_or_else(|| "default guard unexpectedly exempted a model path".to_owned())?;
    assert_eq!(failure.diagnostics()[0].code.number(), 1317);
    Ok(())
}

#[test]
fn other_languages_do_not_inspect_unrelated_rust_models() -> Result<(), String> {
    let root = RustKnownModelPathsFixture::root()?;
    RustKnownModelPathsFixture::model(
        root.path(),
        "#[Service]\npub struct ConnectivityChanged {}\n",
    )?;
    let failure = NativeWebApis::refuse_with_models(
        Path::new("client/services/PlaceService.cs"),
        "ConnectivityChanged changed;",
        root.path(),
    )
    .err()
    .ok_or_else(|| "C# page event unexpectedly accepted".to_owned())?;
    assert_eq!(failure.diagnostics()[0].code.number(), 1317);
    Ok(())
}
