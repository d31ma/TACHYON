pub(super) struct ModelDeclaration {
    path: String,
}

impl ModelDeclaration {
    pub(super) fn new(path: String) -> Self {
        Self { path }
    }

    pub(super) fn line(&self, name: &str) -> String {
        let module = format!("../../../{}", self.path);
        let module = serde_json::to_string(&module).unwrap_or_default();
        format!("  type {name} = import({module}).{name};\n")
    }
}
