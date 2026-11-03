use super::host::{quoted_string_escape, write};
use crate::Failure;
use std::path::Path;

pub(super) struct AndroidYonReach {
    origin: Option<url::Url>,
}

impl AndroidYonReach {
    const LOCAL_SCHEME: &'static str = "http";
    const ORIGIN_PLACEHOLDER: &'static str = "__YON_ORIGIN__";
    const KOTLIN_TRANSPORT: &'static str = include_str!("android_yon_http.kt");
    const KOTLIN_TASK_IMPORT: &'static str = "import kotlin.coroutines.startCoroutine\n";
    const NETWORK_SECURITY_FILE: &'static str = "app/src/main/res/xml/tachyon_network_security.xml";
    const NETWORK_SECURITY_ATTRIBUTE: &'static str =
        "\n      android:networkSecurityConfig=\"@xml/tachyon_network_security\"";

    pub(super) fn declared(origin: Option<&str>) -> Self {
        Self {
            origin: origin.and_then(|origin| url::Url::parse(origin).ok()),
        }
    }

    pub(super) fn java_origin(&self) -> String {
        quoted_string_escape(&self.origin_text())
    }

    pub(super) fn kotlin_companion(&self, package: &str, authored: &str) -> String {
        format!(
            "package {package}\n\n{}{}",
            Self::KOTLIN_TASK_IMPORT,
            self.linked(authored)
        )
    }

    pub(super) fn linked(&self, authored: &str) -> String {
        let origin = quoted_string_escape(&self.origin_text()).replace('$', "\\$");
        let transport = Self::KOTLIN_TRANSPORT.replace(Self::ORIGIN_PLACEHOLDER, &origin);
        crate::behavior::KotlinBuiltins::natively(authored, &transport)
    }

    pub(super) fn manifest_attribute(&self) -> &'static str {
        self.local_host()
            .map_or("", |_| Self::NETWORK_SECURITY_ATTRIBUTE)
    }

    pub(super) fn network_security(&self) -> Option<String> {
        self.local_host().map(|host| {
            format!(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<network-security-config>\n  <base-config cleartextTrafficPermitted=\"false\"/>\n  <domain-config cleartextTrafficPermitted=\"true\">\n    <domain includeSubdomains=\"false\">{}</domain>\n  </domain-config>\n</network-security-config>\n",
                super::host::xml_escape(&host)
            )
        })
    }

    pub(super) fn write_network_security(&self, project: &Path) -> Result<(), Failure> {
        self.network_security()
            .map(|policy| {
                write(
                    &project.join(Self::NETWORK_SECURITY_FILE),
                    policy.as_bytes(),
                )
            })
            .transpose()
            .map(|_| ())
    }

    fn origin_text(&self) -> String {
        self.origin
            .as_ref()
            .map(|origin| origin.origin().ascii_serialization())
            .unwrap_or_default()
    }

    fn local_host(&self) -> Option<String> {
        self.origin
            .as_ref()
            .filter(|origin| origin.scheme() == Self::LOCAL_SCHEME)
            .and_then(|origin| origin.host_str())
            .map(|host| {
                host.trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_owned()
            })
    }
}
