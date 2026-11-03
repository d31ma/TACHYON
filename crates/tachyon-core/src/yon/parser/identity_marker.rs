use super::access_marker::AccessMarker;
use super::routes::{MappedRoutes, identity_route};
use super::{Declaration, Input, ManagedType, named_methods};
use crate::Failure;
use crate::yon::plan::Route;

pub(super) struct IdentityMarker;

impl IdentityMarker {
    pub(super) const CLASS: &'static str = "identityprovider";
    pub(super) const ROUTE: &'static str = "/.tachyon/identity";
    const METHOD_NAMES: [&'static str; 2] = ["identify", "Identify"];
    const ONE_METHOD: &'static str = "An @IdentityProvider class has exactly one method named identify (Identify in C# and Go). It takes the request and returns the signed-in user with a list of role names, or no value when nobody is signed in";
    const NOT_A_ROUTE: &'static str = "An @IdentityProvider class is not a route, so it takes no @Public, @SignedIn or @Allow. Tachyon calls it before every route marked @SignedIn or @Allow";

    pub(super) fn uses(declarations: &[Declaration]) -> bool {
        declarations.iter().any(Self::declares_provider)
    }

    pub(super) fn route(
        input: &Input<'_>,
        declarations: &[Declaration],
        managed: &ManagedType,
        found: &mut MappedRoutes,
    ) -> Result<Route, Failure> {
        Self::refuse_access(input, declarations, &managed.provider.name)?;
        let methods = named_methods(input, managed, &Self::METHOD_NAMES)?;
        let start = Self::one_method(input, &methods)?;
        identity_route(input, start, Self::ROUTE, found)
    }

    fn declares_provider(declaration: &Declaration) -> bool {
        declaration
            .annotations
            .iter()
            .any(|annotation| annotation.name == Self::CLASS)
    }

    fn refuse_access(
        input: &Input<'_>,
        declarations: &[Declaration],
        provider: &str,
    ) -> Result<(), Failure> {
        let access = AccessMarker::class(input, declarations, provider)?;
        if access.is_missing() {
            return Ok(());
        }
        Err(input.failure(0, Self::NOT_A_ROUTE))
    }

    fn one_method(input: &Input<'_>, methods: &[usize]) -> Result<usize, Failure> {
        match methods {
            [start] => Ok(*start),
            _ => Err(input.failure(0, Self::ONE_METHOD)),
        }
    }
}
