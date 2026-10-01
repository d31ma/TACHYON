pub(super) struct JavaTestContracts;

impl JavaTestContracts {
    pub(super) fn read(source: &str) -> String {
        format!("{}{}", Self::preference(source), Self::publisher(source))
    }

    fn preference(source: &str) -> &'static str {
        if source.contains("PreferenceStrings") {
            return include_str!("../web_apis/PreferenceStrings.java");
        }
        ""
    }

    fn publisher(source: &str) -> &'static str {
        if source.contains("ApplicationEventPublisher") {
            return include_str!("test_contracts/ApplicationEventPublisher.java");
        }
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::JavaTestContracts;

    #[test]
    fn publisher_test_contract_is_staged_only_when_the_unit_uses_it() {
        assert!(JavaTestContracts::read("class CounterService {}").is_empty());
        let source = JavaTestContracts::read("private final ApplicationEventPublisher publisher;");
        assert!(source.contains("class ApplicationEventPublisher"));
        assert!(!source.contains("class PreferenceStrings"));
        assert!(!source.contains("TacEvents"));
        assert!(source.contains("must supply an ApplicationEventPublisher fake"));
    }
}
