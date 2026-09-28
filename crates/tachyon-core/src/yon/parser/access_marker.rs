use super::{Annotation, Declaration, Input, declared_type};
use crate::Failure;
use crate::behavior::literal::class_reference;
use crate::yon::route_access::RouteAccess;

pub(super) struct AccessMarker;

impl AccessMarker {
    const PUBLIC: &'static str = "public";
    const SIGNED_IN: &'static str = "signedin";
    const ALLOW: &'static str = "allow";
    const NAMES: [&'static str; 3] = [Self::PUBLIC, Self::SIGNED_IN, Self::ALLOW];
    const LIST_EDGES: [char; 4] = ['{', '}', '[', ']'];
    const ONE_RULE: &'static str = "A route has one access rule. Keep one of @Public, @SignedIn or @Allow(Role): @Public means anyone may call the route, @SignedIn means any signed-in user may call it, and @Allow(Role) means only users with one of these roles may call it";
    const NO_ARGUMENTS: &'static str = "@Public and @SignedIn take no arguments. To name the roles that may call the route, write @Allow(Role), for example @Allow(Admin)";
    const ROLE_ADVICE: &'static str = "@Allow takes one or more role types from server/roles/, for example @Allow(Admin) or @Allow(Admin, Editor). Java writes Admin.class, Kotlin and PHP Admin::class, and C# typeof(Admin)";
    pub(super) const METHOD_ADVICE: &'static str = "@Public, @SignedIn and @Allow go on the controller class or on a route method, together with @GetRoute, @PostRoute, @PutRoute, @PatchRoute, @DeleteRoute or @StreamRoute";

    pub(super) fn is_name(name: &str) -> bool {
        Self::NAMES.contains(&name)
    }

    pub(super) fn only_access(declaration: &Declaration) -> bool {
        Self::markers(declaration).len() == declaration.annotations.len()
    }

    pub(super) fn read(
        input: &Input<'_>,
        declaration: &Declaration,
    ) -> Result<RouteAccess, Failure> {
        let markers = Self::markers(declaration);
        Self::refuse_second(input, &markers)?;
        Self::refuse_bare_arguments(input, &markers)?;
        let roles = Self::roles(input, &markers)?;
        Ok(Self::access(Self::kind(&markers), roles))
    }

    pub(super) fn class(
        input: &Input<'_>,
        declarations: &[Declaration],
        controller: &str,
    ) -> Result<RouteAccess, Failure> {
        let declaration = Self::class_declaration(input, declarations, controller);
        declaration.map_or(Ok(RouteAccess::Missing), |declaration| {
            Self::read(input, declaration)
        })
    }

    fn class_declaration<'a>(
        input: &Input<'_>,
        declarations: &'a [Declaration],
        controller: &str,
    ) -> Option<&'a Declaration> {
        declarations.iter().find(|declaration| {
            declared_type(input, declaration).is_some_and(|(name, _)| name == controller)
        })
    }

    fn markers(declaration: &Declaration) -> Vec<&Annotation> {
        declaration
            .annotations
            .iter()
            .filter(|annotation| Self::is_name(&annotation.name))
            .collect()
    }

    fn refuse_second(input: &Input<'_>, markers: &[&Annotation]) -> Result<(), Failure> {
        match markers.get(1) {
            Some(second) => Err(input.failure(second.span.start, Self::ONE_RULE)),
            None => Ok(()),
        }
    }

    fn refuse_bare_arguments(input: &Input<'_>, markers: &[&Annotation]) -> Result<(), Failure> {
        let marker = markers
            .iter()
            .find(|marker| Self::has_bare_arguments(marker));
        marker.map_or(Ok(()), |marker| {
            Err(input.failure(marker.span.start, Self::NO_ARGUMENTS))
        })
    }

    fn has_bare_arguments(marker: &Annotation) -> bool {
        marker.name != Self::ALLOW && !marker.arguments.is_empty()
    }

    fn roles(input: &Input<'_>, markers: &[&Annotation]) -> Result<Vec<String>, Failure> {
        let allow = markers.iter().find(|marker| marker.name == Self::ALLOW);
        allow.map_or(Ok(Vec::new()), |marker| Self::role_names(input, marker))
    }

    fn role_names(input: &Input<'_>, marker: &Annotation) -> Result<Vec<String>, Failure> {
        let texts = Self::role_texts(&marker.arguments);
        texts
            .iter()
            .map(|text| Self::role_name(input, marker, text))
            .collect()
    }

    fn role_name(input: &Input<'_>, marker: &Annotation, text: &str) -> Result<String, Failure> {
        class_reference(input.extension, text)
            .map(str::to_owned)
            .ok_or_else(|| input.failure(marker.span.start, Self::ROLE_ADVICE))
    }

    fn role_texts(arguments: &str) -> Vec<&str> {
        let list = arguments.trim().trim_matches(Self::LIST_EDGES);
        list.split(',').map(str::trim).collect()
    }

    fn kind<'a>(markers: &[&'a Annotation]) -> &'a str {
        markers.first().map_or("", |marker| marker.name.as_str())
    }

    fn access(kind: &str, roles: Vec<String>) -> RouteAccess {
        match kind {
            Self::PUBLIC => RouteAccess::Public,
            Self::SIGNED_IN => RouteAccess::SignedIn,
            Self::ALLOW => RouteAccess::Allow(roles),
            _ => RouteAccess::Missing,
        }
    }
}
