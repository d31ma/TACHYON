use super::{CppYonController, CppYonParameter, CppYonStatus};
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppMember, CppMemberKind, CppTypeText};
use crate::yon::plan::ErrorResponse;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonErrorMethod {
    pub(crate) owner: String,
    pub(crate) method: String,
    pub(crate) error_type: String,
    pub(crate) takes_request: bool,
    pub(crate) status: u16,
    pub(crate) returns: String,
    pub(crate) path: PathBuf,
    pub(crate) at: usize,
}

impl CppYonErrorMethod {
    const MARKER: &'static str = "HandlesError";
    const DEFAULT_STATUS: u16 = 500;
    const ERROR_STATUSES: std::ops::RangeInclusive<u16> = 400..=599;
    const STATUS: &'static str = "[[ResponseStatus]] on a [[HandlesError]] method takes an error status from 400 to 599, for example [[ResponseStatus(404)]]";
    const SHAPE: &'static str = "A [[HandlesError]] method is an instance method that takes the error and, optionally, the request: `Problem missing(const OrderNotFound& error)` or `Problem missing(const OrderNotFound& error, const tachyon::http::HttpRequestMessage& request)`. The type of its first parameter is the error that it handles";
    const RETURNS: &'static str =
        "A [[HandlesError]] method returns a model or a tachyon::http::HttpResponseMessage";

    pub(crate) fn all(
        path: &Path,
        owner: &str,
        members: &[CppMember],
    ) -> Result<Vec<Self>, Failure> {
        members
            .iter()
            .filter(|member| member.marked(Self::MARKER))
            .map(|member| Self::read(path, owner, member))
            .collect()
    }

    pub(crate) fn response(&self) -> ErrorResponse {
        ErrorResponse {
            error_type: self.error_type.clone(),
            status: self.status,
            returns: self.returns.clone(),
            documentation: String::new(),
            return_shape: crate::route_types::Shape::Any,
        }
    }

    fn read(path: &Path, owner: &str, member: &CppMember) -> Result<Self, Failure> {
        let failure = |message: &str| {
            CppFailure::coded(CppYonController::CODE, path, member.name_at, message)
        };
        let shaped = [
            member.is_instance(CppMemberKind::Method),
            (1..=2).contains(&member.parameters.len()),
            member
                .parameters
                .get(1)
                .is_none_or(|request| CppYonParameter::is_request(&request.type_text)),
        ] == [true, true, true];
        shaped.then_some(()).ok_or_else(|| failure(Self::SHAPE))?;
        let returns = member.plain_type();
        (returns != "void")
            .then_some(())
            .ok_or_else(|| failure(Self::RETURNS))?;
        let status = CppYonStatus::read(path, member, Self::ERROR_STATUSES, Self::STATUS)?;
        Ok(Self {
            owner: owner.to_owned(),
            method: member.name.clone(),
            error_type: CppTypeText::plain(&member.parameters[0].type_text)
                .trim_start_matches("::")
                .to_owned(),
            takes_request: member.parameters.len() == 2,
            status: status.unwrap_or(Self::DEFAULT_STATUS),
            returns,
            path: path.to_path_buf(),
            at: member.name_at,
        })
    }
}
