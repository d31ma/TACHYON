use super::contract_role::ContractRole;
use crate::behavior::HttpClientSource;
use serde_json::Value;

pub(crate) struct FsharpDeclaration {
    pub(crate) role: ContractRole,
    pub(crate) marker: usize,
    pub(crate) header: usize,
    pub(crate) name: String,
    pub(crate) options: Value,
    pub(crate) body: Vec<usize>,
    pub(crate) repository: Option<(usize, String)>,
}

impl FsharpDeclaration {
    const SETUP_SUFFIX: &'static str = "Setup";
    const SETUP_KEY: &'static str = "setup";
    const ADAPTER_KEY: &'static str = "adapter";

    pub(crate) fn options_of(marker: &str) -> Value {
        let open = marker.find('{');
        let close = marker.rfind('}');
        open.zip(close)
            .filter(|(open, close)| close > open)
            .and_then(|(open, close)| serde_json::from_str(&marker[open..=close]).ok())
            .unwrap_or(Value::Null)
    }

    pub(crate) fn setup(&self) -> String {
        let stem = self
            .name
            .strip_suffix(self.role.suffix)
            .unwrap_or(&self.name);
        self.option(Self::SETUP_KEY)
            .unwrap_or_else(|| format!("{stem}{}", Self::SETUP_SUFFIX))
    }

    pub(crate) fn partition(&self) -> String {
        self.option(Self::ADAPTER_KEY)
            .unwrap_or_else(|| HttpClientSource::adapter_name(&self.name))
    }

    fn option(&self, key: &str) -> Option<String> {
        self.options
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}
