pub(super) struct TestUnitDeclaration {
    path: String,
    member: String,
}

impl TestUnitDeclaration {
    const DEFAULT_MEMBER: &'static str = "default";

    pub(super) fn new(path: String, member: String) -> Self {
        Self { path, member }
    }

    pub(super) fn lines(&self, name: &str) -> String {
        let module = format!("../../../{}", self.path);
        let module = serde_json::to_string(&module).unwrap_or_default();
        let export = self.export_words();
        format!(
            "\ndeclare module {module} {{\n  {export} interface {name} extends BehaviorTestUnit {{}}\n}}\n"
        )
    }

    fn export_words(&self) -> &'static str {
        if self.member == Self::DEFAULT_MEMBER {
            return "export default";
        }
        "export"
    }
}
