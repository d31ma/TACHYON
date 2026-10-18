use super::config::config_failure;
use super::host::quoted_string_escape;
use crate::Failure;
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct NativeOutgoingHosts {
    hosts: Vec<String>,
}

impl NativeOutgoingHosts {
    const SETTING: &'static str = "tac.outgoing.hosts in client/manifest.json";
    const HOSTS_POINTER: &'static str = "/hosts";
    const MESSAGE: &'static str = "Manifest tac.outgoing.hosts must list host names without scheme or port, such as \"api.example.com\", or \"*.example.com\" for its subdomains. Native hosts write this list into the page's connect-src rule.";

    pub(super) fn declared(outgoing: Option<&Value>) -> Result<Self, Failure> {
        let listed = outgoing.and_then(|member| member.pointer(Self::HOSTS_POINTER));
        let entries = listed
            .map_or(Some(&[][..]), |value| value.as_array().map(Vec::as_slice))
            .ok_or_else(|| config_failure(Self::MESSAGE))?;
        let hosts = entries
            .iter()
            .map(Self::host_entry)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { hosts })
    }

    #[cfg(test)]
    pub(super) fn listed(hosts: &[&str]) -> Self {
        Self {
            hosts: hosts.iter().map(|host| host.to_ascii_lowercase()).collect(),
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }

    pub(super) fn connect_sources(&self) -> String {
        crate::safety::DeclaredHosts::named(&self.hosts, Self::SETTING).connect_sources()
    }

    pub(super) fn java_array(&self) -> String {
        let quoted: Vec<String> = self
            .hosts
            .iter()
            .map(|host| format!("\"{}\"", quoted_string_escape(host)))
            .collect();
        format!("{{{}}}", quoted.join(", "))
    }

    fn host_entry(entry: &Value) -> Result<String, Failure> {
        entry
            .as_str()
            .filter(|name| crate::safety::DeclaredHosts::is_name_entry(name))
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| config_failure(Self::MESSAGE))
    }
}
