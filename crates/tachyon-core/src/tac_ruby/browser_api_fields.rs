use crate::declared_contract::DeclaredContract;
use crate::web_api_clients::WebApiClients;
use std::path::Path;

pub(crate) struct BrowserApiFields;

impl BrowserApiFields {
    pub(crate) fn injected(path: &Path, source: &str) -> String {
        let browser_apis = WebApiClients::TYPE_NAMES.map(String::from);
        DeclaredContract::injected_with_remotes(path, source, &browser_apis, &[])
            .unwrap_or_else(|| source.to_owned())
    }
}
