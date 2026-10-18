use super::{CppYonController, CppYonParameter};
use crate::Failure;
use crate::route_types::Shape;
use crate::tac_cpp::{CppClass, CppFailure, CppFile, CppMember, CppMemberKind, CppParameter};
use crate::yon::plan::{ParameterSource, Route, RouteParameter};
use std::path::Path;

pub(crate) struct CppYonMiddleware;

impl CppYonMiddleware {
    const FILE: &'static str = "middleware.hpp";
    const CLASS: &'static str = "Middleware";
    const PHASES: [(&'static str, &'static str); 2] = [
        ("before", "middleware.before"),
        ("after", "middleware.after"),
    ];
    const ANSWERS: [&'static str; 2] = ["HttpResponseMessage", "HttpResponseMessage>"];
    const OPTIONAL: &'static str = "std::optional<";
    const VOID: &'static str = "void";
    const SHAPE: &'static str = "C++ root middleware is one `class Middleware` in middleware.hpp with a `before(const tachyon::http::HttpRequestMessage& request)` method, an `after(const tachyon::http::HttpRequestMessage& request)` method, or both. The request parameter may be left out";
    const MARKED: &'static str = "C++ root middleware takes no markers: Tachyon calls before(request) before every route and after(request) after it. Who may call a route is decided by the route's access marker and the one [[IdentityProvider]]";
    const PHASE: &'static str = "A C++ middleware phase returns `std::optional<tachyon::http::HttpResponseMessage>`, `tachyon::http::HttpResponseMessage` or `void`, and declares one method of its name. Returning nothing, or a 204, lets the request pass and adds its headers; any other status answers instead of the route";

    pub(crate) fn is_middleware(path: &Path) -> bool {
        path == Path::new(Self::FILE)
    }

    pub(crate) fn field(path: &Path) -> &'static str {
        ["method", "operation"][usize::from(Self::is_middleware(path))]
    }

    pub(crate) fn unrouted(path: &Path) -> &'static str {
        [
            "tachyon::yon::YonDelivery::unrouted()",
            "tachyon::yon::YonAnswer::empty(204)",
        ][usize::from(Self::is_middleware(path))]
    }

    pub(crate) fn read(path: &Path, source: &str) -> Result<CppYonController, Failure> {
        let file = CppFile::read(path, source)?;
        Self::check_class(path, &file.class)?;
        let routes = Self::PHASES
            .iter()
            .map(|phase| Self::phase(path, &file.class, *phase))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CppYonController {
            file,
            source: source.to_owned(),
            path: path.to_path_buf(),
            routes: routes.into_iter().flatten().collect(),
        })
    }

    fn check_class(path: &Path, class: &CppClass) -> Result<(), Failure> {
        let problem = [
            (class.name == Self::CLASS, Self::SHAPE),
            (class.every_marker().is_empty(), Self::MARKED),
        ]
        .into_iter()
        .find(|(passed, _)| !passed);
        problem.map_or(Ok(()), |(_, message)| {
            Err(CppFailure::coded(
                CppYonController::CODE,
                path,
                class.name_at,
                message,
            ))
        })
    }

    fn phase(
        path: &Path,
        class: &CppClass,
        (method, operation): (&str, &str),
    ) -> Result<Option<Route>, Failure> {
        let written: Vec<&CppMember> = class
            .members
            .iter()
            .filter(|member| {
                [member.kind == CppMemberKind::Method, member.name == method] == [true, true]
            })
            .collect();
        let once = written.len() < 2;
        written
            .first()
            .map(|member| Self::route(path, member, operation, once))
            .transpose()
    }

    fn route(
        path: &Path,
        member: &CppMember,
        operation: &str,
        once: bool,
    ) -> Result<Route, Failure> {
        let returns = member.plain_type();
        let answers = [once, Self::answers(&returns)] == [true, true];
        let refused = [
            (answers, Self::PHASE),
            (Self::takes_request(member), Self::SHAPE),
        ]
        .into_iter()
        .find(|(passed, _)| !passed);
        refused.map_or(Ok(()), |(_, message)| {
            Err(CppFailure::coded(
                CppYonController::CODE,
                path,
                member.name_at,
                message,
            ))
        })?;
        Ok(Route {
            verb: String::from(operation),
            method: member.name.clone(),
            returns,
            parameters: member.parameters.iter().map(Self::request).collect(),
            ..Route::default()
        })
    }

    fn answers(returns: &str) -> bool {
        let compact: String = returns.split_whitespace().collect();
        let optional = compact.starts_with(Self::OPTIONAL);
        let answer = Self::ANSWERS[usize::from(optional)];
        [compact == Self::VOID, compact.ends_with(answer)].contains(&true)
    }

    fn takes_request(member: &CppMember) -> bool {
        let requests = member
            .parameters
            .iter()
            .filter(|parameter| CppYonParameter::is_request(&parameter.type_text))
            .count();
        [member.parameters.len() == requests, requests < 2] == [true, true]
    }

    fn request(parameter: &CppParameter) -> RouteParameter {
        RouteParameter {
            source: ParameterSource::Request,
            name: parameter.name.clone(),
            code_name: parameter.name.clone(),
            type_text: parameter.type_text.clone(),
            shape: Shape::Any,
        }
    }
}
