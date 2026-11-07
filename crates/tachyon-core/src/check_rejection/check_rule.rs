#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum CheckRule {
    Checked,
    Required,
    NotEmpty,
    Length,
    Min,
    Max,
}

impl CheckRule {
    const ALL: [Self; 6] = [
        Self::Checked,
        Self::Required,
        Self::NotEmpty,
        Self::Length,
        Self::Min,
        Self::Max,
    ];
    const NAMES: [&'static str; 6] = ["Checked", "Required", "NotEmpty", "Length", "Min", "Max"];

    pub(crate) fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.name() == name)
    }

    pub(crate) fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::CheckRule;

    #[test]
    fn every_rule_reads_back_from_its_annotation_name() {
        for rule in CheckRule::ALL {
            assert_eq!(CheckRule::parse(rule.name()), Some(rule));
        }
        assert_eq!(CheckRule::parse("notBlank"), None);
        assert_eq!(CheckRule::parse("required"), None);
        assert_eq!(CheckRule::parse(""), None);
    }

    #[test]
    fn rules_sort_in_the_order_a_field_is_checked() {
        let mut rules = vec![
            CheckRule::Max,
            CheckRule::Required,
            CheckRule::Length,
            CheckRule::Checked,
            CheckRule::Min,
            CheckRule::NotEmpty,
        ];
        rules.sort();
        assert_eq!(rules, CheckRule::ALL.to_vec());
    }
}
