use super::{BuiltInCase, IdentityAnswer, ServerResponse};
use crate::yon::built_in_error::BuiltInError;
use crate::yon::route_access::RouteAccess;
use axum::body::Body;
use axum::http::{Response, StatusCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::server) enum AccessVerdict {
    Allowed,
    NoRule,
    NotSignedIn,
    WrongRole,
    Failed,
}

impl AccessVerdict {
    const NO_RULE: &'static str = "This route does not say who may call it.";
    const NOT_SIGNED_IN: &'static str = "Sign in to call this route.";
    const WRONG_ROLE: &'static str = "Your roles do not allow this route.";
    const FAILED: &'static str = "Tachyon could not check who is signed in.";

    pub(in crate::server) fn of(access: &RouteAccess, answer: &IdentityAnswer) -> Self {
        let held = Self::held_roles(answer);
        Self::decide(access, answer, held)
    }

    pub(in crate::server) fn response(self) -> Option<Response<Body>> {
        let (status, message) = self.status_and_message()?;
        let response = ServerResponse::text(status, message);
        Some(Self::tagged(self.built_in_error(), response))
    }

    fn tagged(error: Option<BuiltInError>, response: Response<Body>) -> Response<Body> {
        match error {
            Some(error) => BuiltInCase::plain(error).tag(response),
            None => response,
        }
    }

    const fn built_in_error(self) -> Option<BuiltInError> {
        match self {
            Self::NotSignedIn => Some(BuiltInError::NOT_SIGNED_IN),
            Self::WrongRole => Some(BuiltInError::ROLE_MISSING),
            Self::Failed => Some(BuiltInError::IDENTITY_CHECK_FAILED),
            Self::Allowed | Self::NoRule => None,
        }
    }

    fn held_roles(answer: &IdentityAnswer) -> &[String] {
        match answer {
            IdentityAnswer::SignedIn { roles, .. } => roles,
            _ => &[],
        }
    }

    fn decide(access: &RouteAccess, answer: &IdentityAnswer, held: &[String]) -> Self {
        match (access, answer) {
            (RouteAccess::Public, _) => Self::Allowed,
            (RouteAccess::Missing, _) => Self::NoRule,
            (_, IdentityAnswer::Failed) => Self::Failed,
            (_, IdentityAnswer::SignedIn { .. }) => Self::for_roles(access.roles(), held, access),
            _ => Self::NotSignedIn,
        }
    }

    fn for_roles(wanted: &[String], held: &[String], access: &RouteAccess) -> Self {
        let signed_in_only = *access == RouteAccess::SignedIn;
        let shares = wanted.iter().any(|role| held.contains(role));
        Self::allowed_when([signed_in_only, shares].contains(&true))
    }

    fn allowed_when(allowed: bool) -> Self {
        if allowed {
            return Self::Allowed;
        }
        Self::WrongRole
    }

    fn status_and_message(self) -> Option<(StatusCode, &'static str)> {
        match self {
            Self::Allowed => None,
            Self::NoRule => Some((StatusCode::FORBIDDEN, Self::NO_RULE)),
            Self::NotSignedIn => Some((StatusCode::UNAUTHORIZED, Self::NOT_SIGNED_IN)),
            Self::WrongRole => Some((StatusCode::FORBIDDEN, Self::WRONG_ROLE)),
            Self::Failed => Some((StatusCode::INTERNAL_SERVER_ERROR, Self::FAILED)),
        }
    }
}
