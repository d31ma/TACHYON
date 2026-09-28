pub(super) struct Declaration {
    path: String,
    member: String,
}

impl Declaration {
    pub(super) fn new(path: String, member: String) -> Self {
        Self { path, member }
    }

    pub(super) fn lines(&self, name: &str) -> String {
        let module = format!("../../../{}", self.path);
        let module = serde_json::to_string(&module).unwrap_or_default();
        let target = format!("import({module}).{}", self.member);
        format!("  type {name} = {target};\n  const {name}: typeof {target};\n")
    }
}
