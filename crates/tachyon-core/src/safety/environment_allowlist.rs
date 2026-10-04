use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EnvironmentAllowlist {
    names: BTreeSet<String>,
    prefixes: BTreeSet<String>,
}

impl EnvironmentAllowlist {
    const WILDCARD: char = '*';
    const EVERYTHING: &'static str = "*";
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
        entries.iter().try_for_each(|entry| allowlist.add(entry))?;
        Ok(allowlist)
    }

    pub(crate) const fn names(&self) -> &BTreeSet<String> {
        &self.names
    }

    pub(crate) const fn prefixes(&self) -> &BTreeSet<String> {
        &self.prefixes
    }

    pub(crate) fn is_name(text: &str) -> bool {
        let starts_well = text.bytes().next().is_some_and(Self::starts_name);
        let continues_well = text.bytes().all(Self::continues_name);
        [starts_well, continues_well] == [true, true]
    }

    pub(crate) fn is_reserved(name: &str) -> bool {
        let name = name.to_ascii_uppercase();
        let is_reserved_name = Self::RESERVED_NAMES.contains(&name.as_str());
        let has_reserved_prefix = Self::RESERVED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix));
        [is_reserved_name, has_reserved_prefix].contains(&true)
    }

    fn add(&mut self, entry: &str) -> Result<(), String> {
        let prefix = entry.strip_suffix(Self::WILDCARD);
        let refusal = prefix.map_or_else(
            || Self::name_refusal(entry),
            |prefix| Self::prefix_refusal(entry, prefix),
        );
        refusal.map_or(Ok(()), Err)?;
        let kept = prefix.map_or(&mut self.names, |_| &mut self.prefixes);
        kept.insert(prefix.unwrap_or(entry).to_owned());
        Ok(())
    }

    fn name_refusal(entry: &str) -> Option<String> {
        Self::refusal(entry, Self::is_name(entry), Self::is_reserved(entry))
    }

    fn prefix_refusal(entry: &str, prefix: &str) -> Option<String> {
        Self::refusal(entry, Self::is_name(prefix), Self::covers_reserved(prefix))
    }

    fn refusal(entry: &str, is_name: bool, is_reserved: bool) -> Option<String> {
        [
            (entry == Self::EVERYTHING).then(Self::everything_message),
            (!is_name).then(|| Self::malformed_message(entry)),
            is_reserved.then(|| Self::reserved_message(entry)),
        ]
        .into_iter()
        .flatten()
        .next()
    }

    fn covers_reserved(prefix: &str) -> bool {
        let prefix = prefix.to_ascii_uppercase();
        let covers_reserved_entry = Self::RESERVED_NAMES
            .iter()
            .chain(Self::RESERVED_PREFIXES.iter())
            .any(|reserved| reserved.starts_with(&prefix));
        [covers_reserved_entry, Self::is_reserved(&prefix)].contains(&true)
    }

    fn starts_name(byte: u8) -> bool {
        [byte.is_ascii_alphabetic(), byte == b'_'].contains(&true)
    }

    fn continues_name(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
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
