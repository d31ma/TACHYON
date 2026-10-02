#![allow(clippy::expect_used)]

use std::path::Path;

struct HelperCallerFiles;

impl HelperCallerFiles {
    const HELPER: &'static str = "using System;\n[Helper]\nclass StepHelper {\n private readonly string label;\n public StepHelper(string label) { this.label = label; }\n public string Read() { return label; }\n}\n";

    fn write(root: &Path, name: &str, source: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("fixture source");
    }

    fn assemble(service: &str) -> Result<Option<String>, crate::Failure> {
        let root = tempfile::tempdir().expect("captured project");
        let entry = Path::new("client/services/TallyService.cs");
        Self::write(root.path(), entry.to_str().expect("entry"), service);
        Self::write(
            root.path(),
            "client/helpers/deep/StepHelper.cs",
            Self::HELPER,
        );
        let roles = crate::behavior::ServiceRoles::read(entry, service).expect("Service roles");
        super::assemble(root.path(), entry, &roles.source, "tac")
    }
}

#[test]
fn csharp_helper_caller_failure_retains_nested_authored_marker_location() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly string sample = \"new StepHelper()\";\n // new StepHelper(\"comment\");\n}\n";
    let failure =
        HelperCallerFiles::assemble(source).expect_err("strings/comments cannot provide ownership");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1308);
    assert!(
        diagnostic
            .message
            .contains("explicit caller-owned construction")
    );
    assert_eq!(
        diagnostic.spans[0].file,
        "client/helpers/deep/StepHelper.cs"
    );
    assert_eq!(diagnostic.spans[0].start, 14);
    assert_eq!(diagnostic.spans[0].end, 15);
}

#[test]
fn csharp_helper_real_service_caller_is_checked_across_captured_files() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly StepHelper helper = new StepHelper(\"owned\");\n public string Read() { return helper.Read(); }\n}\n";
    let source = HelperCallerFiles::assemble(source)
        .expect("real external caller")
        .expect("captured Helper");
    assert!(source.contains("new StepHelper(\"owned\")"));
    assert!(source.contains("class StepHelper"));
}
