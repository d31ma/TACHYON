pub(crate) struct SpeculationRules;

impl SpeculationRules {
    const OPENING: &'static str = r#"<script type="speculationrules" data-tachyon-runtime>"#;
    const RULES: &'static str =
        "\n{\"prefetch\":[{\"where\":{\"href_matches\":\"/*\"},\"eagerness\":\"moderate\"}]}\n";
    const CLOSING: &'static str = "</script>";

    pub(crate) fn tag() -> String {
        format!("{}{}{}", Self::OPENING, Self::RULES, Self::CLOSING)
    }

    pub(crate) fn script_source() -> String {
        super::ContentSecurityPolicy::inline_script_hash(Self::RULES)
    }
}

#[cfg(test)]
mod tests {
    use super::SpeculationRules;

    #[test]
    fn the_policy_allows_the_speculation_rules_by_the_hash_of_their_text() {
        let tag = SpeculationRules::tag();
        assert!(tag.contains(r#""prefetch""#), "{tag}");
        assert_eq!(
            SpeculationRules::script_source(),
            "'sha256-ceXDNKX1eA4/ZmjUnT2JKnlHG4WruVHbkdRcBqOjQUg='"
        );
    }
}
