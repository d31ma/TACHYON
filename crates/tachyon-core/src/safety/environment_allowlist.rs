use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EnvironmentAllowlist {
    names: BTreeSet<String>,
    prefixes: BTreeSet<String>,
}

impl EnvironmentAllowlist {
    const WILDCARD: char = '*';
    const RESERVED_PREFIXES: [&'static str; 3] = ["TAC_", "TACHYON_", "YON_"];
    const RESERVED_NAMES: [&'static str; 5] = [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "NODE_USE_ENV_PROXY",
    ];

    pub(crate) fn parse(entries: &[&str]) -> Result<Self, String> {
        let mut allowlist = Self::default();
        for entry in entries {
            allowlist.add(entry)?;
        }
        Ok(allowlist)
    }

    pub(crate) const fn names(&self) -> &BTreeSet<String> {
        &self.names
    }

    pub(crate) const fn prefixes(&self) -> &BTreeSet<String> {
        &self.prefixes
    }

    pub(crate) fn is_name(text: &str) -> bool {
        let mut bytes = text.bytes();
        let starts_well = bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
        starts_well && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }

    pub(crate) fn is_reserved(name: &str) -> bool {
        let name = name.to_ascii_uppercase();
        let is_reserved_name = Self::RESERVED_NAMES.contains(&name.as_str());
        is_reserved_name
            || Self::RESERVED_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
    }

    fn add(&mut self, entry: &str) -> Result<(), String> {
        let Some(prefix) = entry.strip_suffix(Self::WILDCARD) else {
            Self::check(entry, entry, Self::is_reserved(entry))?;
            self.names.insert(entry.to_owned());
            return Ok(());
        };
        if prefix.is_empty() {
            return Err(Self::everything_message());
        }
        Self::check(entry, prefix, Self::covers_reserved(prefix))?;
        self.prefixes.insert(prefix.to_owned());
        Ok(())
    }

    fn check(entry: &str, name: &str, reserved: bool) -> Result<(), String> {
        if !Self::is_name(name) {
            return Err(Self::malformed_message(entry));
        }
        if reserved {
            return Err(Self::reserved_message(entry));
        }
        Ok(())
    }

    fn covers_reserved(prefix: &str) -> bool {
        let prefix = prefix.to_ascii_uppercase();
        let covers_reserved_entry = Self::RESERVED_NAMES
            .iter()
            .chain(Self::RESERVED_PREFIXES.iter())
            .any(|reserved| reserved.starts_with(&prefix));
        covers_reserved_entry || Self::is_reserved(&prefix)
    }

    fn everything_message() -> String {
        String::from(
            "yon.server.environment cannot contain \"*\": it would pass every variable of ty's environment to handlers. List names or prefixes, such as \"HOME\" or \"HD_*\".",
        )
    }

    fn malformed_message(entry: &str) -> String {
        format!(
            "yon.server.environment entry \"{entry}\" is not a variable name or a prefix pattern. Use ASCII letters, digits and underscores, beginning with a letter or underscore, and an optional \"*\" at the end, such as \"HOME\" or \"HD_*\"."
        )
    }

    fn reserved_message(entry: &str) -> String {
        format!(
            "yon.server.environment entry \"{entry}\" would pass ty's own variables to handlers: TAC_*, TACHYON_*, YON_* and the proxy settings of the outgoing request guard. List only the application's own variables."
        )
    }
}
