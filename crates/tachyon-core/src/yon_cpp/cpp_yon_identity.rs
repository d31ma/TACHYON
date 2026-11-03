use super::{CppYonController, CppYonRoute};
use crate::Failure;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure, CppMember, CppMemberKind};
use crate::yon::plan::Route;
use crate::yon::route_access::RouteAccess;
use std::path::Path;

pub(crate) struct CppYonIdentity;

impl CppYonIdentity {
    const MARKER: &'static str = "IdentityProvider";
    const ROUTE: &'static str = "/.tachyon/identity";
    const METHOD: &'static str = "identify";
    const ONE_METHOD: &'static str = "A C++ [[IdentityProvider]] class has exactly one method named identify. It takes the request, `const tachyon::http::HttpRequestMessage& request`, or one [[FromHeader]], [[FromCookie]] or [[FromQuery]] value, and returns the signed-in user with a list of role names, such as std::optional<Identity>, or no value when nobody is signed in";

    pub(crate) fn marker(class: &CppClass) -> Option<&CppAttribute> {
        class
            .markers
            .iter()
            .find(|marker| marker.name == Self::MARKER)
    }

    pub(crate) fn routes(
        path: &Path,
        class: &CppClass,
        marker: &CppAttribute,
    ) -> Result<Vec<Route>, Failure> {
        let methods: Vec<&CppMember> = class
            .members
            .iter()
            .filter(|member| member.name == Self::METHOD)
            .collect();
        let member = Self::one_method(path, class, &methods)?;
        let route = CppYonRoute::identity(path, member, marker);
        Ok(vec![
            route.fixed(Self::ROUTE.to_owned(), RouteAccess::Missing)?,
        ])
    }

    fn one_method<'a>(
        path: &Path,
        class: &CppClass,
        methods: &[&'a CppMember],
    ) -> Result<&'a CppMember, Failure> {
        let single = methods.first().copied().filter(|member| {
            [methods.len() == 1, member.kind == CppMemberKind::Method] == [true, true]
        });
        single.ok_or_else(|| {
            CppFailure::coded(
                CppYonController::CODE,
                path,
                class.name_at,
                Self::ONE_METHOD,
            )
        })
    }
}
