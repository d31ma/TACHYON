use crate::failure::{diagnostic, source_span};
use crate::handler::HandlerSource;
use crate::routing::RoutePath;
use crate::yon::plan::Route;
use cap_std::fs::Dir;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

#[derive(Debug, Default)]
pub(super) struct AccessCheck {
    identity: Option<HandlerSource>,
    identity_path: String,
    guarded_route: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl AccessCheck {
    const MISSING_CODE: u16 = 1322;
    const IDENTITY_CODE: u16 = 1323;
    const ROLE_FOLDER: &'static str = "server/roles";
    const MISSING_HELP: &'static str = "Tachyon refuses every route by default (ADR 0068). Write @Public when anyone may call the route, @SignedIn when any signed-in user may call it, or @Allow(Role) when only some roles may call it. Put it on the controller class or on the route method; the method wins.";
    const IDENTITY_HELP: &'static str = "Add exactly one class marked @IdentityProvider in server/controllers/<Name>IdentityProvider.<ext>. Its method identify(request) returns the signed-in user and their role names, or no value when nobody is signed in.";
    const ROLE_HELP: &'static str = "A role is a type: add an empty class named after the role in server/roles/<Role>.<ext>, for example server/roles/Admin.ts with `export default class Admin {}`.";

    pub(super) fn add_identity(&mut self, source: HandlerSource, path: String) {
        self.refuse_second_identity(&path);
        self.identity = Some(source);
        self.identity_path = path;
    }

    pub(super) fn check_routes(&mut self, source: &HandlerSource, path: &str, project: &Dir) {
        self.check_each_route(source.routes(), path, project);
    }

    pub(super) fn finish(mut self, diagnostics: &mut Vec<Diagnostic>) -> Option<HandlerSource> {
        self.refuse_guard_without_identity();
        diagnostics.append(&mut self.diagnostics);
        self.identity
    }

    fn check_each_route(&mut self, routes: &[Route], path: &str, project: &Dir) {
        for route in routes {
            self.check_route(route, path, project);
        }
    }

    fn check_route(&mut self, route: &Route, path: &str, project: &Dir) {
        self.refuse_missing_access(route, path);
        self.remember_guarded(route, path);
        self.refuse_unknown_roles(route, path, project);
    }

    fn refuse_second_identity(&mut self, path: &str) {
        if self.identity.is_none() {
            return;
        }
        let message = format!(
            "A project has one identity provider, but both '{}' and '{path}' are marked @IdentityProvider.",
            self.identity_path
        );
        self.push(Self::IDENTITY_CODE, message, Self::IDENTITY_HELP, path);
    }

    fn refuse_missing_access(&mut self, route: &Route, path: &str) {
        if !route.access.is_missing() {
            return;
        }
        let message = format!(
            "The route {} {} in '{path}' does not say who may call it.",
            route.verb,
            Self::template(route)
        );
        self.push(Self::MISSING_CODE, message, Self::MISSING_HELP, path);
    }

    fn remember_guarded(&mut self, route: &Route, path: &str) {
        if route.access.needs_identity() {
            self.guarded_route = Some(format!(
                "{} {} in '{path}'",
                route.verb,
                Self::template(route)
            ));
        }
    }

    fn refuse_unknown_roles(&mut self, route: &Route, path: &str, project: &Dir) {
        for role in route.access.roles() {
            self.refuse_unknown_role(role, path, project);
        }
    }

    fn refuse_unknown_role(&mut self, role: &str, path: &str, project: &Dir) {
        if Self::role_exists(role, project) {
            return;
        }
        let message = format!(
            "@Allow in '{path}' names the role {role}, but server/roles has no type named {role}."
        );
        self.push(Self::MISSING_CODE, message, Self::ROLE_HELP, path);
    }

    fn refuse_guard_without_identity(&mut self) {
        let Some(route) = self.route_without_identity() else {
            return;
        };
        let message = format!(
            "The route {route} needs a signed-in user, but the project has no @IdentityProvider class that says who is signed in."
        );
        self.push_without_file(message);
    }

    fn route_without_identity(&mut self) -> Option<String> {
        let identity_missing = self.identity.is_none();
        self.guarded_route.take().filter(|_| identity_missing)
    }

    fn role_exists(role: &str, project: &Dir) -> bool {
        crate::convention::YonConvention::EXTENSIONS
            .iter()
            .any(|extension| Self::role_file_exists(role, extension, project))
    }

    fn role_file_exists(role: &str, extension: &str, project: &Dir) -> bool {
        let file = Path::new(Self::ROLE_FOLDER).join(format!("{role}.{extension}"));
        project.is_file(file)
    }

    fn template(route: &Route) -> String {
        RoutePath::template(route.pattern.as_deref().unwrap_or("/"))
    }

    fn push(&mut self, code: u16, message: String, help: &str, path: &str) {
        self.diagnostics.push(diagnostic(
            code,
            message,
            Some(String::from(help)),
            source_span(path, 0, path.len()),
        ));
    }

    fn push_without_file(&mut self, message: String) {
        self.diagnostics.push(diagnostic(
            Self::IDENTITY_CODE,
            message,
            Some(String::from(Self::IDENTITY_HELP)),
            None,
        ));
    }
}
