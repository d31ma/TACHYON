use super::Choice;
use crate::Failure;
use crate::failure::diagnostic;

pub(crate) struct ChoiceError;

impl ChoiceError {
    const CODE: u16 = 1408;

    pub(crate) fn unknown<T: Choice>(value: &str, topic: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            format!("'{value}' is not a {topic}."),
            Some(format!("Choose from: {}.", T::flag_names())),
            None,
        ))
    }

    pub(crate) fn conflict(message: String, help: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(help)),
            None,
        ))
    }
}
