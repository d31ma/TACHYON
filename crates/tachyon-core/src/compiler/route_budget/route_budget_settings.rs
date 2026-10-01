use super::BudgetLimit;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RouteBudgetSettings {
    pub(super) check: bool,
    pub(super) script: BudgetLimit,
    pub(super) web_assembly: BudgetLimit,
    script_configured: bool,
}

impl RouteBudgetSettings {
    const MANIFEST: &'static str = "client/manifest.json";
    const BUDGETS: &'static str = "/tac/budgets";
    const CHECK: &'static str = "check";
    const ON_BY_DEFAULT: bool = true;
    const BYTES_PER_KILOBYTE: u64 = 1024;
    const SCRIPT: BudgetLimit = BudgetLimit {
        name: "script",
        setting: "script",
        extensions: &[".js", ".mjs"],
        bytes: 200,
    };
    const WEB_ASSEMBLY: BudgetLimit = BudgetLimit {
        name: "WebAssembly",
        setting: "webassembly",
        extensions: &[".wasm"],
        bytes: 3584,
    };

    pub(super) fn read(project_root: &Path) -> Self {
        let manifest = Self::manifest(project_root);
        let budgets = manifest.pointer(Self::BUDGETS).unwrap_or(&Value::Null);
        Self {
            check: budgets
                .get(Self::CHECK)
                .and_then(Value::as_bool)
                .unwrap_or(Self::ON_BY_DEFAULT),
            script: Self::limit(budgets, Self::SCRIPT),
            script_configured: budgets.get("script").and_then(Value::as_u64).is_some(),
            web_assembly: Self::limit(budgets, Self::WEB_ASSEMBLY),
        }
    }

    pub(super) fn for_python_route(self, selected: bool) -> Self {
        Self {
            script: self.python_script(selected),
            ..self
        }
    }

    fn python_script(self, selected: bool) -> BudgetLimit {
        if [selected, !self.script_configured].contains(&false) {
            return self.script;
        }
        BudgetLimit {
            bytes: crate::tac_python::RuntimeProfile::SCRIPT_BYTES,
            ..self.script
        }
    }

    fn manifest(project_root: &Path) -> Value {
        std::fs::read(project_root.join(Self::MANIFEST))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null)
    }

    fn limit(budgets: &Value, default: BudgetLimit) -> BudgetLimit {
        let kilobytes = budgets
            .get(default.setting)
            .and_then(Value::as_u64)
            .unwrap_or(default.bytes);
        BudgetLimit {
            bytes: kilobytes.saturating_mul(Self::BYTES_PER_KILOBYTE),
            ..default
        }
    }
}
