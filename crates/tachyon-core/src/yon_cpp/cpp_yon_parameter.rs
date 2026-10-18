use super::{CppYonPath, CppYonRoute};
use crate::Failure;
use crate::route_types::Shape;
use crate::routing::RoutePath;
use crate::tac_cpp::{CppAttribute, CppParameter, CppTypeText};
use crate::yon::plan::{ParameterSource, RouteParameter};

pub(crate) struct CppYonParameter;

impl CppYonParameter {
    const SOURCES: [(&'static str, ParameterSource); 5] = [
        ("FromPath", ParameterSource::Path),
        ("FromQuery", ParameterSource::Query),
        ("FromBody", ParameterSource::Body),
        ("FromHeader", ParameterSource::Header),
        ("FromCookie", ParameterSource::Cookie),
    ];
    const MAX_NAME_BYTES: usize = 256;
    const UNMARKED: &'static str = "Mark each parameter of a C++ route method with [[FromPath]], [[FromQuery]], [[FromBody]], [[FromHeader]] or [[FromCookie]] and give it a name. Only one parameter may be left unmarked, `const tachyon::http::HttpRequestMessage& request`, and it receives the whole request";
    const REQUEST: &'static str = "HttpRequestMessage";
    const DEFAULT: &'static str = "A C++ route parameter cannot have a default value, because the server checks every argument before the method runs and does not see the default. Write std::optional<T> and choose the value in the method, for example page.value_or(1)";
    const BODY: &'static str = "A request has one body, so one parameter at most uses [[FromBody]], and a GET request has no body: use [[PostRoute]] for a method that reads a body";

    pub(crate) fn all(
        route: &CppYonRoute<'_>,
        pattern: &str,
    ) -> Result<Vec<RouteParameter>, Failure> {
        let parameters = route
            .member()
            .parameters
            .iter()
            .map(|parameter| Self::one(route, parameter))
            .collect::<Result<Vec<_>, _>>()?;
        Self::check_body(route, &parameters)?;
        Self::check_requests(route, &parameters)?;
        Self::check_paths(route, pattern, &parameters)?;
        Ok(parameters)
    }

    fn one(route: &CppYonRoute<'_>, parameter: &CppParameter) -> Result<RouteParameter, Failure> {
        let written = parameter.markers.iter().find_map(Self::source_of);
        let request =
            Self::is_request(&parameter.type_text).then_some((None, ParameterSource::Request));
        let (marker, source) = written
            .map(|(marker, source)| (Some(marker), source))
            .or(request)
            .filter(|_| !parameter.name.is_empty())
            .ok_or_else(|| route.failure(Self::UNMARKED))?;
        if parameter.has_default {
            return Err(route.failure(Self::DEFAULT));
        }
        Ok(RouteParameter {
            source,
            name: Self::marked_name(route, marker, &parameter.name)?,
            code_name: parameter.name.clone(),
            type_text: CppTypeText::plain(&parameter.type_text),
            shape: Shape::Any,
        })
    }

    pub(crate) fn is_request(type_text: &str) -> bool {
        let plain = CppTypeText::plain(type_text);
        plain.rsplit("::").next() == Some(Self::REQUEST)
    }

    fn marked_name(
        route: &CppYonRoute<'_>,
        marker: Option<&CppAttribute>,
        code_name: &str,
    ) -> Result<String, Failure> {
        marker.map_or_else(
            || Ok(code_name.to_owned()),
            |marker| Self::name(route, marker, code_name),
        )
    }

    fn check_requests(
        route: &CppYonRoute<'_>,
        parameters: &[RouteParameter],
    ) -> Result<(), Failure> {
        let requests = parameters
            .iter()
            .filter(|parameter| parameter.source == ParameterSource::Request)
            .count();
        (requests <= 1)
            .then_some(())
            .ok_or_else(|| route.failure(Self::UNMARKED))
    }

    fn source_of(marker: &CppAttribute) -> Option<(&CppAttribute, ParameterSource)> {
        let found = Self::SOURCES.iter().find(|(name, _)| *name == marker.name);
        found.map(|(_, source)| (marker, *source))
    }

    fn name(
        route: &CppYonRoute<'_>,
        marker: &CppAttribute,
        code_name: &str,
    ) -> Result<String, Failure> {
        let text = marker.arguments.trim();
        let unnamed = text.is_empty().then(|| code_name.to_owned());
        let message = format!(
            "[[{0}]] takes one name in quotes, for example [[{0}(\"page_size\")]]. Without a name it uses the parameter name",
            marker.name
        );
        unnamed
            .or_else(|| CppYonPath::literal(text).filter(|name| Self::valid_name(name)))
            .ok_or_else(|| route.failure(&message))
    }

    fn valid_name(name: &str) -> bool {
        let sized = (1..=Self::MAX_NAME_BYTES).contains(&name.len());
        let plain = name
            .bytes()
            .all(|byte| [byte.is_ascii_alphanumeric(), b"_-.".contains(&byte)].contains(&true));
        [sized, plain] == [true, true]
    }

    fn check_body(route: &CppYonRoute<'_>, parameters: &[RouteParameter]) -> Result<(), Failure> {
        let bodies = parameters
            .iter()
            .filter(|parameter| parameter.source == ParameterSource::Body)
            .count();
        let allowed = usize::from(route.verb() != "GET");
        (bodies <= allowed)
            .then_some(())
            .ok_or_else(|| route.failure(Self::BODY))
    }

    fn check_paths(
        route: &CppYonRoute<'_>,
        pattern: &str,
        parameters: &[RouteParameter],
    ) -> Result<(), Failure> {
        let segments: Vec<String> = pattern.split('/').map(str::to_owned).collect();
        let missing = parameters
            .iter()
            .filter(|parameter| parameter.source == ParameterSource::Path)
            .find(|parameter| !segments.contains(&format!("_{}", parameter.name)));
        missing.map_or(Ok(()), |parameter| {
            Err(route.failure(&format!(
                "[[FromPath]] means the value comes from the route path, but '{}' has no {{{}}} part. Add {{{}}} to the path or use another marker",
                RoutePath::template(pattern),
                parameter.name,
                parameter.name
            )))
        })
    }
}
