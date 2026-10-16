use super::{CppYonAccess, CppYonController, CppYonParameter, CppYonPath};
use crate::Failure;
use crate::routing::RoutePath;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure, CppMember};
use crate::yon::plan::Route;
use crate::yon::route_access::RouteAccess;
use std::path::Path;

pub(crate) struct CppYonRoute<'a> {
    path: &'a Path,
    member: &'a CppMember,
    marker: &'a CppAttribute,
    verb: &'static str,
}

impl<'a> CppYonRoute<'a> {
    const VERBS: [(&'static str, &'static str); 5] = [
        ("GetRoute", "GET"),
        ("PostRoute", "POST"),
        ("PutRoute", "PUT"),
        ("PatchRoute", "PATCH"),
        ("DeleteRoute", "DELETE"),
    ];

    pub(crate) fn all(
        path: &Path,
        class: &CppClass,
        base: &str,
        class_access: &RouteAccess,
    ) -> Result<Vec<Route>, Failure> {
        class
            .members
            .iter()
            .filter_map(|member| CppYonRoute::marked(path, member))
            .map(|route| route.route(base, class_access))
            .collect()
    }

    pub(crate) fn verb(&self) -> &'static str {
        self.verb
    }

    fn marked(path: &'a Path, member: &'a CppMember) -> Option<Self> {
        let (marker, verb) = member.markers.iter().find_map(Self::verb_of)?;
        Some(Self {
            path,
            member,
            marker,
            verb,
        })
    }

    fn verb_of(marker: &CppAttribute) -> Option<(&CppAttribute, &'static str)> {
        let found = Self::VERBS.iter().find(|(name, _)| *name == marker.name);
        found.map(|(_, verb)| (marker, *verb))
    }

    fn route(&self, base: &str, class_access: &RouteAccess) -> Result<Route, Failure> {
        self.check_static()?;
        self.check_name()?;
        let own = CppYonPath::of(self.path, self.marker)?;
        let pattern = RoutePath::pattern(base, &own).map_err(|message| self.failure(&message))?;
        let parameters = CppYonParameter::all(self, &pattern)?;
        let access = CppYonAccess::read(&self.member.markers).or_class(class_access);
        Ok(Route {
            verb: self.verb.to_owned(),
            method: self.member.name.clone(),
            pattern: Some(pattern),
            parameters,
            returns: self.member.plain_type(),
            access,
            ..Route::default()
        })
    }

    pub(crate) fn failure(&self, message: &str) -> Failure {
        CppFailure::coded(
            CppYonController::CODE,
            self.path,
            self.member.name_at,
            message,
        )
    }

    pub(crate) fn member(&self) -> &CppMember {
        self.member
    }

    fn check_static(&self) -> Result<(), Failure> {
        let message = format!(
            "{} is static, but a C++ route method is an instance method: Tachyon makes one controller object for each request. Remove static",
            self.member.name
        );
        (!self.member.is_static)
            .then_some(())
            .ok_or_else(|| self.failure(&message))
    }

    fn check_name(&self) -> Result<(), Failure> {
        let verb = self.verb.to_ascii_lowercase();
        let named = self.member.name.to_ascii_lowercase().starts_with(&verb);
        let message = format!(
            "[[{}]] marks a method whose name starts with '{verb}', for example {verb}Orders: {} does not",
            self.marker.name, self.member.name
        );
        named.then_some(()).ok_or_else(|| self.failure(&message))
    }
}
