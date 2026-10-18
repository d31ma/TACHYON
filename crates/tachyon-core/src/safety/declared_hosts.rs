use super::HostLabel;
use cap_std::fs::Dir;
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DeclaredHosts {
    hosts: Vec<String>,
    setting: &'static str,
    yon_origin: Option<String>,
}

impl DeclaredHosts {
    const MANIFEST: &'static str = "client/manifest.json";
    const PUBLISHED_MANIFEST: &'static str = "manifest.webmanifest";
    const APPLICATION: &'static str = "server/application.json";
    const TAC_POINTER: &'static str = "/tac/outgoing/hosts";
    const YON_POINTER: &'static str = "/outgoing/hosts";
    const TAC_SETTING: &'static str = "tac.outgoing.hosts in client/manifest.json";
    const YON_SETTING: &'static str =
        "outgoing.hosts in server/application.json (yon.outgoing.hosts)";
    const ANY_SUBDOMAIN: &'static str = "*.";
    const SCHEMES: [&'static str; 4] = ["https", "wss", "http", "ws"];
    const YON_ORIGIN_POINTER: &'static str = "/tac/yon/origin";

    pub(crate) fn tac(project: &Dir) -> Self {
        Self::read(
            project,
            Self::MANIFEST,
            Self::TAC_POINTER,
            Self::TAC_SETTING,
        )
    }

    pub(crate) fn published(bundle: &std::path::Path) -> Self {
        let directory = Dir::open_ambient_dir(bundle, cap_std::ambient_authority());
        directory.map_or_else(
            |_| Self::read_nothing(Self::TAC_SETTING),
            |directory| {
                Self::read(
                    &directory,
                    Self::PUBLISHED_MANIFEST,
                    Self::TAC_POINTER,
                    Self::TAC_SETTING,
                )
            },
        )
    }

    pub(crate) fn yon(project: &Dir) -> Self {
        Self::read(
            project,
            Self::APPLICATION,
            Self::YON_POINTER,
            Self::YON_SETTING,
        )
    }

    pub(crate) fn named(hosts: &[String], setting: &'static str) -> Self {
        Self {
            hosts: hosts.iter().map(|host| host.to_ascii_lowercase()).collect(),
            setting,
            yon_origin: None,
        }
    }

    pub(crate) fn with_yon_origin(self, yon_origin: Option<String>) -> Self {
        Self { yon_origin, ..self }
    }

    pub(crate) fn is_name_entry(entry: &str) -> bool {
        let name = entry.strip_prefix(Self::ANY_SUBDOMAIN).unwrap_or(entry);
        HostLabel::is_name(name)
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn listed(hosts: &[&str], setting: &'static str) -> Self {
        Self {
            hosts: hosts.iter().map(|host| host.to_ascii_lowercase()).collect(),
            setting,
            yon_origin: None,
        }
    }

    pub(crate) fn allows(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.hosts
            .iter()
            .any(|declared| Self::matches(declared, &host))
    }

    pub(crate) const fn setting(&self) -> &'static str {
        self.setting
    }

    pub(crate) fn connect_sources(&self) -> String {
        let sources: Vec<String> = self
            .hosts
            .iter()
            .flat_map(|host| Self::sources_for(host))
            .chain(
                self.yon_origin
                    .iter()
                    .flat_map(|origin| Self::yon_sources(origin)),
            )
            .collect();
        sources.join(" ")
    }

    fn read(project: &Dir, file: &str, pointer: &str, setting: &'static str) -> Self {
        let document = Self::stored_document(project, file).unwrap_or_default();
        Self {
            hosts: Self::stored_hosts(&document, pointer).unwrap_or_default(),
            setting,
            yon_origin: Self::stored_yon_origin(&document),
        }
    }

    fn read_nothing(setting: &'static str) -> Self {
        Self {
            hosts: Vec::new(),
            setting,
            yon_origin: None,
        }
    }

    fn stored_document(project: &Dir, file: &str) -> Option<Value> {
        let bytes = project.read(file).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn stored_hosts(document: &Value, pointer: &str) -> Option<Vec<String>> {
        let listed = document.pointer(pointer)?.as_array()?;
        Some(listed.iter().filter_map(Self::host_name).collect())
    }

    fn stored_yon_origin(document: &Value) -> Option<String> {
        let text = document.pointer(Self::YON_ORIGIN_POINTER)?.as_str()?;
        let address = url::Url::parse(text).ok()?;
        let origin = address.origin();
        origin.is_tuple().then(|| origin.ascii_serialization())
    }

    fn yon_sources(origin: &str) -> Vec<String> {
        let socket = origin
            .strip_prefix(Self::SCHEMES[0])
            .map(|rest| format!("{}{rest}", Self::SCHEMES[1]))
            .or_else(|| {
                let rest = origin.strip_prefix(Self::SCHEMES[2])?;
                Some(format!("{}{rest}", Self::SCHEMES[3]))
            });
        std::iter::once(origin.to_owned()).chain(socket).collect()
    }

    fn host_name(value: &Value) -> Option<String> {
        value.as_str().map(str::to_ascii_lowercase)
    }

    fn matches(declared: &str, host: &str) -> bool {
        let parent = declared.strip_prefix(Self::ANY_SUBDOMAIN);
        parent.map_or(declared == host, |parent| Self::below(host, parent))
    }

    fn below(host: &str, parent: &str) -> bool {
        host.strip_suffix(parent)
            .is_some_and(|name| name.ends_with('.'))
    }

    fn sources_for(host: &str) -> Vec<String> {
        Self::SCHEMES
            .iter()
            .map(|scheme| format!("{scheme}://{host}"))
            .collect()
    }
}
