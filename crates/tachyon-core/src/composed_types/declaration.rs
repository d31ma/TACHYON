pub(super) struct Declaration {
    path: String,
    member: String,
}

impl Declaration {
    const DEFAULT_MEMBER: &'static str = "default";

    pub(super) fn new(path: String, member: String) -> Self {
        Self { path, member }
    }

    pub(super) fn lines(&self, name: &str) -> String {
        let module = self.module();
        let target = format!("import({module}).{}", self.member);
        format!("  type {name} = {target};\n  const {name}: typeof {target};\n")
    }

    pub(super) fn test_unit_lines(&self, name: &str) -> String {
        let module = self.module();
        let export = self.export_words();
        format!(
            "\ndeclare module {module} {{\n  {export} interface {name} extends BehaviorTestUnit {{}}\n}}\n"
        )
    }

    fn module(&self) -> String {
        let module = format!("../../../{}", self.path);
        serde_json::to_string(&module).unwrap_or_default()
    }

    fn export_words(&self) -> &'static str {
        if self.member == Self::DEFAULT_MEMBER {
            return "export default";
        }
        "export"
    }
}
