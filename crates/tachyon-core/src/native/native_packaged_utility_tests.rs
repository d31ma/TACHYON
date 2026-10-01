#![allow(clippy::expect_used)]

use super::native_page_graph_tests::NativePageGraphCheck;
use std::path::Path;

struct PackagedUtilityCheck;

impl PackagedUtilityCheck {
    const CALLER: &str = r##"
struct PackagedUtilityProbe;

impl PackagedUtilityProbe {
    const INIT: &'static str = r#"{"op":"init","route":"/","service":"label","owner":"00000000-0000-0000-0000-000000000001"}"#;
    const CALL: &'static str = r#"{"op":"call","route":"/","service":"label","owner":"00000000-0000-0000-0000-000000000001","name":"read","args":[]}"#;
    const DISPOSE: &'static str = r#"{"op":"dispose","route":"/","service":"label","owner":"00000000-0000-0000-0000-000000000001"}"#;

    fn run() {
        assert_eq!(Self::exchange(Self::INIT), r#"{"value":{"fields":[],"methods":["read"],"listeners":[]}}"#);
        assert_eq!(Self::exchange(Self::CALL), r#"{"value":"Ready"}"#);
        assert_eq!(Self::exchange(Self::DISPOSE), r#"{"value":null}"#);
    }

    fn exchange(request: &str) -> String {
        let input = std::ffi::CString::new(request).expect("bounded request");
        let pointer = unsafe { tac_native_invoke(input.as_ptr()) };
        assert!(!pointer.is_null());
        let answer = unsafe { std::ffi::CStr::from_ptr(pointer) }.to_string_lossy().into_owned();
        unsafe { tac_native_free(pointer) };
        answer
    }
}

fn main() {
    PackagedUtilityProbe::run();
}
"##;

    async fn execute(root: &Path) {
        let source = root.join("dist-checkpoint07/macos/project/companion.rs");
        let original = std::fs::read(&source).expect("actual packaged companion");
        let temporary = tempfile::tempdir().expect("isolated probe");
        let copy = temporary.path().join("companion.rs");
        std::fs::write(&copy, &original).expect("copy actual emitted source");
        NativePageGraphCheck::execute(&copy, Self::CALLER).await;
        assert_eq!(std::fs::read(&source).expect("retained source"), original);
    }
}

#[tokio::test]
#[ignore = "requires the three actual checkpoint07 packages; set TAC_PACKAGED_RUST_UTILITY_ROOT"]
async fn packaged_rust_utilities_execute_the_actual_generated_native_bridge() {
    let root = std::env::var_os("TAC_PACKAGED_RUST_UTILITY_ROOT")
        .expect("explicit copied monitor reproduction root");
    let root = Path::new(&root);
    for project in [
        root.to_path_buf(),
        root.join("explicit-import"),
        root.join("constant-repro"),
    ] {
        PackagedUtilityCheck::execute(&project).await;
    }
}
