use super::{CppYonAccess, CppYonController, CppYonParameter, CppYonPath, CppYonStatus};
use crate::Failure;
use crate::routing::RoutePath;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure, CppMember};
use crate::yon::plan::{ParameterSource, Route};
use crate::yon::route_access::RouteAccess;
use std::path::Path;

pub(crate) struct CppYonRoute<'a> {
    path: &'a Path,
    member: &'a CppMember,
    marker: &'a CppAttribute,
    verb: &'static str,
    prefix: &'static str,
}

impl<'a> CppYonRoute<'a> {
    const VERBS: [(&'static str, &'static str, &'static str); 6] = [
        ("GetRoute", "GET", "get"),
        ("PostRoute", "POST", "post"),
        ("PutRoute", "PUT", "put"),
        ("PatchRoute", "PATCH", "patch"),
        ("DeleteRoute", "DELETE", "delete"),
        ("StreamRoute", "GET", "stream"),
    ];
    const STREAM_MARKER: &'static str = "StreamRoute";
    const GENERATOR: &'static str = "tachyon::Generator<";
    const SHAPES: [&'static str; 2] = [
        "A C++ route method that returns tachyon::Generator<Model> sends a stream, so mark it [[StreamRoute]] and start its name with 'stream'",
        "A C++ [[StreamRoute]] method returns tachyon::Generator<Model> and sends each item with co_yield, for example `tachyon::Generator<Tick> streamTicks() { co_yield Tick{}; }`. A thrown exception ends the stream with an error event",
    ];
    const SUCCESS: std::ops::RangeInclusive<u16> = 200..=299;
    const STATUS: &'static str = "[[ResponseStatus]] takes one success status from 200 to 299, for example [[ResponseStatus(201)]]. It sets the status of a normal answer";

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
        let (marker, verb, prefix) = member.markers.iter().find_map(Self::verb_of)?;
        Some(Self {
            path,
            member,
            marker,
            verb,
            prefix,
        })
    }

    fn verb_of(marker: &CppAttribute) -> Option<(&CppAttribute, &'static str, &'static str)> {
        let found = Self::VERBS.iter().find(|(name, _, _)| *name == marker.name);
        found.map(|(_, verb, prefix)| (marker, *verb, *prefix))
    }

    pub(crate) fn identity(
        path: &'a Path,
        member: &'a CppMember,
        marker: &'a CppAttribute,
    ) -> Self {
        Self {
            path,
            member,
            marker,
            verb: "GET",
            prefix: "get",
        }
    }

    fn route(&self, base: &str, class_access: &RouteAccess) -> Result<Route, Failure> {
        self.check_name()?;
        self.check_stream()?;
        let own = CppYonPath::of(self.path, self.marker)?;
        let pattern = RoutePath::pattern(base, &own).map_err(|message| self.failure(&message))?;
        let access = CppYonAccess::read(&self.member.markers).or_class(class_access);
        self.fixed(pattern, access)
    }

    pub(crate) fn fixed(&self, pattern: String, access: RouteAccess) -> Result<Route, Failure> {
        self.check_static()?;
        let parameters = CppYonParameter::all(self, &pattern)?;
        let status = CppYonStatus::read(self.path, self.member, Self::SUCCESS, Self::STATUS)?;
        Ok(Route {
            verb: self.verb.to_owned(),
            method: self.member.name.clone(),
            takes_request: parameters
                .iter()
                .any(|parameter| parameter.source == ParameterSource::Request),
            streaming: self.streaming(),
            pattern: Some(pattern),
            status,
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

    fn streaming(&self) -> bool {
        self.marker.name == Self::STREAM_MARKER
    }

    fn check_stream(&self) -> Result<(), Failure> {
        let written: String = self.member.plain_type().split_whitespace().collect();
        let generator = written.starts_with(Self::GENERATOR);
        let streaming = self.streaming();
        (generator == streaming)
            .then_some(())
            .ok_or_else(|| self.failure(Self::SHAPES[usize::from(streaming)]))
    }

    fn check_name(&self) -> Result<(), Failure> {
        let verb = self.prefix;
        let named = self.member.name.to_ascii_lowercase().starts_with(verb);
        let message = format!(
            "[[{}]] marks a method whose name starts with '{verb}', for example {verb}Orders: {} does not",
            self.marker.name, self.member.name
        );
        named.then_some(()).ok_or_else(|| self.failure(&message))
    }
}
