use super::host::{quoted_string_escape, xml_escape};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeLinks {
    #[serde(default)]
    schemes: Vec<String>,
    #[serde(default)]
    hosts: Vec<String>,
}

impl NativeLinks {
    const MOST_ENTRIES: usize = 16;
    const MOST_NAME_LENGTH: usize = 64;
    const MOST_HOST_LENGTH: usize = 253;
    const SCHEME_MARKS: &'static [u8] = b"+-.";
    const HOST_SEPARATOR: char = '.';
    const WILDCARD: char = '*';
    const RESERVED_SCHEMES: [&'static str; 18] = [
        "about",
        "blob",
        "content",
        "data",
        "file",
        "ftp",
        "http",
        "https",
        "intent",
        "javascript",
        "mailto",
        "sms",
        "tachyon",
        "tachyon-app",
        "tel",
        "ws",
        "wss",
        "x-scheme-handler",
    ];
    const PROBLEM: &'static str = "Manifest tac.application.links takes schemes, such as [\"myapp\"]: lowercase URL schemes of at most 64 characters that start with a letter and use letters, digits, '+', '-' or '.', other than http, https, mailto, file and the schemes Tachyon uses itself; and hosts, such as [\"example.com\"]: lowercase host names with a dot, without scheme, port, path or wildcard. Each list holds at most 16 different entries.";

    pub(super) fn problem(&self) -> Option<&'static str> {
        let valid = [
            Self::fits(&self.schemes),
            Self::fits(&self.hosts),
            self.schemes.iter().map(String::as_str).all(Self::is_scheme),
            self.hosts.iter().map(String::as_str).all(Self::is_host),
        ] == [true; 4];
        (!valid).then_some(Self::PROBLEM)
    }

    #[cfg(test)]
    pub(super) fn declared(schemes: &[&str], hosts: &[&str]) -> Self {
        Self {
            schemes: schemes.iter().map(|scheme| String::from(*scheme)).collect(),
            hosts: hosts.iter().map(|host| String::from(*host)).collect(),
        }
    }

    #[cfg(test)]
    pub(super) fn schemes(&self) -> &[String] {
        &self.schemes
    }

    pub(super) fn hosts(&self) -> &[String] {
        &self.hosts
    }

    pub(super) fn quoted_schemes(&self, open: &str, close: &str) -> String {
        Self::quoted(&self.schemes, open, close)
    }

    pub(super) fn quoted_hosts(&self, open: &str, close: &str) -> String {
        Self::quoted(&self.hosts, open, close)
    }

    pub(super) fn url_types_plist(&self, application_id: &str) -> String {
        let schemes = self
            .schemes
            .iter()
            .map(|scheme| format!("<string>{}</string>", xml_escape(scheme)))
            .collect::<Vec<_>>()
            .concat();
        let entry = format!(
            "\n<key>CFBundleURLTypes</key><array><dict><key>CFBundleURLName</key><string>{}</string>\
             <key>CFBundleURLSchemes</key><array>{schemes}</array></dict></array>",
            xml_escape(application_id)
        );
        Self::unless_empty(&self.schemes, entry)
    }

    pub(super) fn android_intent_filters(&self) -> String {
        let schemes = self
            .schemes
            .iter()
            .map(|scheme| {
                format!(
                    "\n      <intent-filter>\n        <action android:name=\"android.intent.action.VIEW\"/>\n        <category android:name=\"android.intent.category.DEFAULT\"/>\n        <category android:name=\"android.intent.category.BROWSABLE\"/>\n        <data android:scheme=\"{}\"/>\n      </intent-filter>",
                    xml_escape(scheme)
                )
            })
            .collect::<Vec<_>>()
            .concat();
        let hosts = self
            .hosts
            .iter()
            .map(|host| format!("\n        <data android:host=\"{}\"/>", xml_escape(host)))
            .collect::<Vec<_>>()
            .concat();
        let verified = format!(
            "\n      <intent-filter android:autoVerify=\"true\">\n        <action android:name=\"android.intent.action.VIEW\"/>\n        <category android:name=\"android.intent.category.DEFAULT\"/>\n        <category android:name=\"android.intent.category.BROWSABLE\"/>\n        <data android:scheme=\"https\"/>{hosts}\n      </intent-filter>"
        );
        schemes + &Self::unless_empty(&self.hosts, verified)
    }

    pub(super) fn desktop_mime_types(&self) -> String {
        let handlers = self
            .schemes
            .iter()
            .map(|scheme| format!("x-scheme-handler/{scheme};"))
            .collect::<Vec<_>>()
            .concat();
        Self::unless_empty(&self.schemes, format!("MimeType={handlers}\n"))
    }

    pub(super) fn msix_extensions(&self) -> String {
        let protocols = Self::each(&self.schemes, |scheme| {
            format!(
                "\n        <uap:Extension Category=\"windows.protocol\"><uap:Protocol Name=\"{scheme}\"/></uap:Extension>"
            )
        });
        let hosts = Self::each(&self.hosts, |host| {
            format!("\n            <uap3:Host Name=\"{host}\"/>")
        });
        let handlers = Self::unless_empty(
            &self.hosts,
            format!(
                "\n        <uap3:Extension Category=\"windows.appUriHandler\">\n          <uap3:AppUriHandler>{hosts}\n          </uap3:AppUriHandler>\n        </uap3:Extension>"
            ),
        );
        Self::unless_empty(
            &[self.schemes.as_slice(), self.hosts.as_slice()].concat(),
            format!("\n      <Extensions>{protocols}{handlers}\n      </Extensions>"),
        )
    }

    pub(super) fn desktop_exec_argument(&self) -> &'static str {
        [" %u", ""][usize::from(self.schemes.is_empty())]
    }

    fn each(names: &[String], entry: impl Fn(&str) -> String) -> String {
        names
            .iter()
            .map(|name| entry(&xml_escape(name)))
            .collect::<Vec<_>>()
            .concat()
    }

    fn unless_empty(names: &[String], text: String) -> String {
        if names.is_empty() {
            return String::new();
        }
        text
    }

    fn quoted(names: &[String], open: &str, close: &str) -> String {
        let quoted: Vec<String> = names
            .iter()
            .map(|name| format!("\"{}\"", quoted_string_escape(name)))
            .collect();
        format!("{open}{}{close}", quoted.join(", "))
    }

    fn fits(names: &[String]) -> bool {
        let distinct = names.iter().collect::<BTreeSet<_>>().len() == names.len();
        [names.len() <= Self::MOST_ENTRIES, distinct] == [true, true]
    }

    fn is_scheme(scheme: &str) -> bool {
        let starts = scheme.starts_with(|first: char| first.is_ascii_lowercase());
        let characters = scheme.bytes().all(Self::is_scheme_byte);
        let fits = scheme.len() <= Self::MOST_NAME_LENGTH;
        let free = !Self::RESERVED_SCHEMES.contains(&scheme);
        [starts, characters, fits, free] == [true; 4]
    }

    fn is_scheme_byte(byte: u8) -> bool {
        [
            byte.is_ascii_lowercase(),
            byte.is_ascii_digit(),
            Self::SCHEME_MARKS.contains(&byte),
        ]
        .contains(&true)
    }

    fn is_host(host: &str) -> bool {
        let named = crate::safety::DeclaredHosts::is_name_entry(host);
        let lowercase = !host.bytes().any(|byte| byte.is_ascii_uppercase());
        let exact = !host.starts_with(Self::WILDCARD);
        let dotted = host.contains(Self::HOST_SEPARATOR);
        let fits = host.len() <= Self::MOST_HOST_LENGTH;
        [named, lowercase, exact, dotted, fits] == [true; 5]
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeLinks;

    fn parsed(source: &str) -> NativeLinks {
        serde_json::from_str(source).expect("links")
    }

    #[test]
    fn schemes_and_hosts_in_their_documented_form_are_accepted() {
        let links = parsed(
            r#"{"schemes":["myapp","my-app.v2+x"],"hosts":["example.com","app.example.co.uk"]}"#,
        );
        assert_eq!(links.problem(), None);
        assert_eq!(links.schemes(), ["myapp", "my-app.v2+x"]);
        assert_eq!(links.hosts(), ["example.com", "app.example.co.uk"]);
        assert_eq!(NativeLinks::default().problem(), None);
    }

    #[test]
    fn schemes_tachyon_or_the_web_already_owns_are_refused() {
        for scheme in [
            "http",
            "https",
            "mailto",
            "file",
            "tachyon",
            "tachyon-app",
            "javascript",
        ] {
            assert!(
                NativeLinks::declared(&[scheme], &[]).problem().is_some(),
                "{scheme}"
            );
        }
    }

    #[test]
    fn malformed_or_repeated_entries_are_refused() {
        let schemes = ["", "MyApp", "1app", "my app", "my_app", "my:app", "é"];
        for scheme in schemes {
            assert!(
                NativeLinks::declared(&[scheme], &[]).problem().is_some(),
                "{scheme}"
            );
        }
        let hosts = [
            "",
            "Example.com",
            "localhost",
            "*.example.com",
            "https://example.com",
            "example.com:443",
            "example.com/path",
            "-example.com",
            "user@example.com",
        ];
        for host in hosts {
            assert!(
                NativeLinks::declared(&[], &[host]).problem().is_some(),
                "{host}"
            );
        }
        assert!(
            NativeLinks::declared(&["myapp", "myapp"], &[])
                .problem()
                .is_some()
        );
        let many: Vec<String> = (0..17).map(|index| format!("app{index}")).collect();
        let borrowed: Vec<&str> = many.iter().map(String::as_str).collect();
        assert!(NativeLinks::declared(&borrowed, &[]).problem().is_some());
        assert!(serde_json::from_str::<NativeLinks>(r#"{"paths":["/"]}"#).is_err());
    }

    #[test]
    fn apple_android_and_linux_register_each_listed_scheme() {
        let links = NativeLinks::declared(&["myapp", "other"], &["example.com"]);
        let plist = links.url_types_plist("com.example.app");
        assert!(plist.contains("<key>CFBundleURLName</key><string>com.example.app</string>"));
        assert!(plist.contains("<array><string>myapp</string><string>other</string></array>"));
        let filters = links.android_intent_filters();
        assert!(filters.contains("<data android:scheme=\"myapp\"/>"));
        assert!(filters.contains("<data android:scheme=\"other\"/>"));
        assert!(filters.contains("<intent-filter android:autoVerify=\"true\">"));
        assert!(filters.contains("<data android:host=\"example.com\"/>"));
        assert_eq!(
            links.desktop_mime_types(),
            "MimeType=x-scheme-handler/myapp;x-scheme-handler/other;\n"
        );
        assert_eq!(links.desktop_exec_argument(), " %u");
        let extensions = links.msix_extensions();
        assert!(extensions.contains(
            "<uap:Extension Category=\"windows.protocol\"><uap:Protocol Name=\"other\"/></uap:Extension>"
        ));
        assert!(extensions.contains(
            "<uap3:Extension Category=\"windows.appUriHandler\">\n          <uap3:AppUriHandler>\n            <uap3:Host Name=\"example.com\"/>\n          </uap3:AppUriHandler>"
        ));
        assert_eq!(links.quoted_schemes("[", "]"), r#"["myapp", "other"]"#);
        assert_eq!(links.quoted_hosts("{", "}"), r#"{"example.com"}"#);
    }

    #[test]
    fn an_app_without_links_registers_nothing() {
        let links = NativeLinks::default();
        assert_eq!(links.url_types_plist("com.example.app"), "");
        assert_eq!(links.android_intent_filters(), "");
        assert_eq!(links.desktop_mime_types(), "");
        assert_eq!(links.desktop_exec_argument(), "");
        assert_eq!(links.msix_extensions(), "");
        assert_eq!(links.quoted_schemes("[", "]"), "[]");
        let hosts_only = NativeLinks::declared(&[], &["example.com"]);
        assert!(hosts_only.android_intent_filters().contains("autoVerify"));
        assert_eq!(hosts_only.url_types_plist("com.example.app"), "");
        let handlers = hosts_only.msix_extensions();
        assert!(handlers.starts_with(
            "\n      <Extensions>\n        <uap3:Extension Category=\"windows.appUriHandler\">"
        ));
        assert!(!handlers.contains("windows.protocol"));
    }
}
