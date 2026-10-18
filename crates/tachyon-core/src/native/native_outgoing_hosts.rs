use super::config::config_failure;
use super::host::quoted_string_escape;
use crate::Failure;
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct NativeOutgoingHosts {
    hosts: Vec<String>,
    yon_origin: Option<String>,
}

impl NativeOutgoingHosts {
    const SETTING: &'static str = "tac.outgoing.hosts in client/manifest.json";
    const SECURE_SCHEME: &'static str = "https";
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
        Ok(Self {
            hosts,
            yon_origin: None,
        })
    }

    #[cfg(test)]
    pub(super) fn listed(hosts: &[&str]) -> Self {
        Self {
            hosts: hosts.iter().map(|host| host.to_ascii_lowercase()).collect(),
            yon_origin: None,
        }
    }

    pub(super) fn with_yon_origin(self, yon_origin: Option<String>) -> Self {
        Self { yon_origin, ..self }
    }

    pub(super) fn is_empty(&self) -> bool {
        [self.hosts.is_empty(), self.yon_origin.is_none()] == [true, true]
    }

    pub(super) fn connect_sources(&self) -> String {
        crate::safety::DeclaredHosts::named(&self.hosts, Self::SETTING)
            .with_yon_origin(self.yon_origin.clone())
            .connect_sources()
    }

    pub(super) fn java_array(&self) -> String {
        let quoted: Vec<String> = self
            .hosts
            .iter()
            .cloned()
            .chain(self.yon_host())
            .map(|host| format!("\"{}\"", quoted_string_escape(&host)))
            .collect();
        format!("{{{}}}", quoted.join(", "))
    }

    fn yon_host(&self) -> Option<String> {
        self.yon_origin
            .as_deref()
            .and_then(|origin| url::Url::parse(origin).ok())
            .filter(|address| address.scheme() == Self::SECURE_SCHEME)
            .and_then(|address| address.host_str().map(str::to_owned))
    }

    fn host_entry(entry: &Value) -> Result<String, Failure> {
        entry
            .as_str()
            .filter(|name| crate::safety::DeclaredHosts::is_name_entry(name))
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| config_failure(Self::MESSAGE))
    }
}
