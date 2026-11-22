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
    const CODES_FROM_FYLO: [(Self, &'static [&'static str]); 7] = [
        (Self::BudgetSpent, &["EDEADLINE"]),
        (Self::Busy, &["EROOTLOCKED", "EBUSY", "ETXNBUSY"]),
        (
            Self::Conflict,
            &[
                "EDOCEXISTS",
                "EENGINE_CONCURRENT_WRITE",
                "ENATIVE_CONCURRENT_WRITE",
            ],
        ),
        (
            Self::NotFound,
            &[
                "EINVALIDDOCID",
                "ENATIVE_NOT_FOUND",
                "ENOTFOUND",
                "ECOLLECTION_NOT_FOUND",
                "ENATIVE_UNSAFE_PATH",
            ],
        ),
        (
            Self::TooLarge,
            &[
                "ETXNLIMIT",
                "EFRAME_REQUEST_TOO_LARGE",
                "EFRAME_RESPONSE_TOO_LARGE",
                "EQUERY_INPUT_SIZE",
                "EQUERY_OUTPUT_SIZE",
                "EQUERY_TERM_SIZE",
                "EQUERYITEMTOOLARGE",
                "EQUERYSNAPSHOTTOOLARGE",
                "EINDEX_KEY_SIZE",
                "EINDEX_SNAPSHOT_SIZE",
                "ENATIVE_FILE_SIZE",
                "EFORMAT_SIZE",
                "EFORMAT_DEPTH",
                "EFORMAT_NODES",
            ],
        ),
        (
            Self::TransactionRefused,
            &["ETXNSCOPE", "ETXNLAYOUT", "ETXNUNKNOWN", "ETXNEXPIRED"],
        ),
        (
            Self::Unavailable,
            &["EPROTOCOL", "EROOTCLOSED", "EROOTLEASELOST"],
        ),
    ];

    #[must_use]
    pub(crate) const fn text(self) -> &'static str {
        Self::name(self)
    }

    #[must_use]
    pub(crate) fn of_fylo(fylo_code: &str) -> Self {
        Self::fylo_group(fylo_code).unwrap_or(Self::Failed)
    }

    #[must_use]
    pub(crate) fn of_sql(error: &sqlx::Error) -> Self {
        Self::unique_violation(error).unwrap_or(Self::Failed)
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

    fn fylo_group(fylo_code: &str) -> Option<Self> {
        Self::CODES_FROM_FYLO
            .into_iter()
            .find_map(|(code, names)| names.contains(&fylo_code).then_some(code))
    }

    fn unique_violation(error: &sqlx::Error) -> Option<Self> {
        let database = error.as_database_error()?;
        database.is_unique_violation().then_some(Self::Conflict)
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
