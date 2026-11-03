use super::CppYonController;
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppMember};
use std::ops::RangeInclusive;
use std::path::Path;

pub(crate) struct CppYonStatus;

impl CppYonStatus {
    const MARKER: &'static str = "ResponseStatus";

    pub(crate) fn read(
        path: &Path,
        member: &CppMember,
        allowed: RangeInclusive<u16>,
        message: &str,
    ) -> Result<Option<u16>, Failure> {
        member
            .markers
            .iter()
            .find(|marker| marker.name == Self::MARKER)
            .map(|marker| {
                marker
                    .arguments
                    .trim()
                    .parse::<u16>()
                    .ok()
                    .filter(|status| allowed.contains(status))
                    .ok_or_else(|| {
                        CppFailure::coded(CppYonController::CODE, path, marker.at, message)
                    })
            })
            .transpose()
    }
}
