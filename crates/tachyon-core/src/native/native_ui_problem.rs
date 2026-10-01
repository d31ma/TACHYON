use crate::{Failure, failure::diagnostic};

pub(super) struct NativeUiProblem;

impl NativeUiProblem {
    pub(super) fn require(valid: bool, message: &str) -> Result<(), Failure> {
        Self::checked(valid, message)
    }

    fn checked(valid: bool, message: &str) -> Result<(), Failure> {
        if !valid {
            return Err(Self::failure(message));
        }
        Ok(())
    }

    pub(super) fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(1605, message, None, None))
    }

    pub(super) fn io(error: &std::io::Error) -> Failure {
        Self::failure(&format!("Cannot capture native UI generation: {error}"))
    }
}
