#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResourceErrorCode {
    BudgetSpent,
    Busy,
    Conflict,
    NotFound,
    TooLarge,
    TransactionRefused,
    Unavailable,
    Failed,
}

impl ResourceErrorCode {
    const FYLO_CODES: [(&'static str, Self); 33] = [
        ("EDEADLINE", Self::BudgetSpent),
        ("EROOTLOCKED", Self::Busy),
        ("EBUSY", Self::Busy),
        ("ETXNBUSY", Self::Busy),
        ("EDOCEXISTS", Self::Conflict),
        ("EENGINE_CONCURRENT_WRITE", Self::Conflict),
        ("ENATIVE_CONCURRENT_WRITE", Self::Conflict),
        ("EINVALIDDOCID", Self::NotFound),
        ("ENATIVE_NOT_FOUND", Self::NotFound),
        ("ENOTFOUND", Self::NotFound),
        ("ECOLLECTION_NOT_FOUND", Self::NotFound),
        ("ENATIVE_UNSAFE_PATH", Self::NotFound),
        ("ETXNLIMIT", Self::TooLarge),
        ("EFRAME_REQUEST_TOO_LARGE", Self::TooLarge),
        ("EFRAME_RESPONSE_TOO_LARGE", Self::TooLarge),
        ("EQUERY_INPUT_SIZE", Self::TooLarge),
        ("EQUERY_OUTPUT_SIZE", Self::TooLarge),
        ("EQUERY_TERM_SIZE", Self::TooLarge),
        ("EQUERYITEMTOOLARGE", Self::TooLarge),
        ("EQUERYSNAPSHOTTOOLARGE", Self::TooLarge),
        ("EINDEX_KEY_SIZE", Self::TooLarge),
        ("EINDEX_SNAPSHOT_SIZE", Self::TooLarge),
        ("ENATIVE_FILE_SIZE", Self::TooLarge),
        ("EFORMAT_SIZE", Self::TooLarge),
        ("EFORMAT_DEPTH", Self::TooLarge),
        ("EFORMAT_NODES", Self::TooLarge),
        ("ETXNSCOPE", Self::TransactionRefused),
        ("ETXNLAYOUT", Self::TransactionRefused),
        ("ETXNUNKNOWN", Self::TransactionRefused),
        ("ETXNEXPIRED", Self::TransactionRefused),
        ("EPROTOCOL", Self::Unavailable),
        ("EROOTCLOSED", Self::Unavailable),
        ("EROOTLEASELOST", Self::Unavailable),
    ];

    #[must_use]
    pub(crate) const fn text(self) -> &'static str {
        Self::name(self)
    }

    #[must_use]
    pub(crate) fn of_fylo(fylo_code: &str) -> Self {
        Self::fylo_entry(fylo_code).map_or(Self::Failed, |(_, code)| code)
    }

    const fn name(code: Self) -> &'static str {
        match code {
            Self::BudgetSpent => "budget_spent",
            Self::Busy => "busy",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::TooLarge => "too_large",
            Self::TransactionRefused => "transaction_refused",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }

    fn fylo_entry(fylo_code: &str) -> Option<(&'static str, Self)> {
        Self::FYLO_CODES
            .into_iter()
            .find(|(name, _)| *name == fylo_code)
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceErrorCode;

    #[test]
    fn each_code_has_its_stable_text() {
        let texts = [
            ResourceErrorCode::BudgetSpent,
            ResourceErrorCode::Busy,
            ResourceErrorCode::Conflict,
            ResourceErrorCode::NotFound,
            ResourceErrorCode::TooLarge,
            ResourceErrorCode::TransactionRefused,
            ResourceErrorCode::Unavailable,
            ResourceErrorCode::Failed,
        ]
        .map(ResourceErrorCode::text);
        assert_eq!(
            texts,
            [
                "budget_spent",
                "busy",
                "conflict",
                "not_found",
                "too_large",
                "transaction_refused",
                "unavailable",
                "failed",
            ]
        );
    }

    #[test]
    fn fylo_codes_map_to_their_resource_codes() {
        let cases = [
            ("EDEADLINE", ResourceErrorCode::BudgetSpent),
            ("EROOTLOCKED", ResourceErrorCode::Busy),
            ("ETXNBUSY", ResourceErrorCode::Busy),
            ("EDOCEXISTS", ResourceErrorCode::Conflict),
            ("ENATIVE_CONCURRENT_WRITE", ResourceErrorCode::Conflict),
            ("ENOTFOUND", ResourceErrorCode::NotFound),
            ("ECOLLECTION_NOT_FOUND", ResourceErrorCode::NotFound),
            ("ENATIVE_NOT_FOUND", ResourceErrorCode::NotFound),
            ("ETXNLIMIT", ResourceErrorCode::TooLarge),
            ("EFRAME_REQUEST_TOO_LARGE", ResourceErrorCode::TooLarge),
            ("ETXNSCOPE", ResourceErrorCode::TransactionRefused),
            ("ETXNLAYOUT", ResourceErrorCode::TransactionRefused),
            ("ETXNUNKNOWN", ResourceErrorCode::TransactionRefused),
            ("ETXNEXPIRED", ResourceErrorCode::TransactionRefused),
            ("EPROTOCOL", ResourceErrorCode::Unavailable),
            ("ESCHEMA", ResourceErrorCode::Failed),
            ("EUNKNOWN", ResourceErrorCode::Failed),
            ("", ResourceErrorCode::Failed),
        ];
        let mapped = cases.map(|(fylo_code, _)| ResourceErrorCode::of_fylo(fylo_code));
        assert_eq!(mapped, cases.map(|(_, code)| code));
    }
}
