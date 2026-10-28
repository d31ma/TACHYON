use super::browser_service_peers::BrowserServicePeers;
use std::fmt::Write as _;

pub(super) struct Declaration {
    path: String,
    member: String,
}

impl Declaration {
    const DEFAULT_MEMBER: &'static str = "default";

    pub(super) fn new(path: String, member: String) -> Self {
        Self { path, member }
    }

    pub(super) fn is_browser_service(&self, name: &str) -> bool {
        self.browser_service(name)
    }

    fn browser_service(&self, name: &str) -> bool {
        let path = std::path::Path::new(&self.path);
        self.member == Self::DEFAULT_MEMBER
            && path.starts_with("client/services")
            && path.file_stem().and_then(|stem| stem.to_str()) == Some(name)
    }

    pub(super) fn is_selected_browser_service(
        &self,
        name: &str,
        peers: &BrowserServicePeers,
    ) -> bool {
        self.selected_browser_service(name, peers)
    }

    fn selected_browser_service(&self, name: &str, peers: &BrowserServicePeers) -> bool {
        self.member == Self::DEFAULT_MEMBER && peers.matches(name, &self.path)
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

    pub(super) fn base_lines(&self, name: &str, service: &str, methods: &[String]) -> String {
        let module = self.module();
        let export = self.export_words();
        let mut signatures = String::new();
        for method in methods {
            let member = format!("{service}[\"{method}\"]");
            let _ = writeln!(
                signatures,
                "    {method}(...args: Parameters<{member}>): ReturnType<{member}>;"
            );
        }
        format!(
            "\ndeclare module {module} {{\n  {export} interface {name} {{\n{signatures}  }}\n}}\n"
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
