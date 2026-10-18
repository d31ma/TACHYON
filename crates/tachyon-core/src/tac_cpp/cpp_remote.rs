use super::{
    CppAttribute, CppClass, CppFailure, CppMember, CppMemberKind, CppParameter, CppStreamRequest,
};
use crate::Failure;
use crate::annotation_rules::Realm;
use std::path::Path;

pub(crate) struct CppRemote<'a> {
    class: &'a CppClass,
    base: String,
    adapter: String,
    call: &'static str,
}

impl<'a> CppRemote<'a> {
    const VERBS: [(&'static str, &'static str); 5] = [
        ("GetRequest", "Get"),
        ("PostRequest", "Post"),
        ("PutRequest", "Put"),
        ("PatchRequest", "Patch"),
        ("DeleteRequest", "Delete"),
    ];
    const TASK_PREFIXES: [&'static str; 2] = ["tachyon::Task<", "Task<"];
    const SEGMENT: &'static str = "tachyon::http::TacBrowserRequest::segment";
    const GATEWAY: &'static str = "Gateway";
    const ONE_REQUEST: [&'static str; 2] = [
        "Each method of a C++ [[HTTPClient]] class has one request marker ([[GetRequest(\"/path\")]], [[PostRequest]], [[PutRequest]], [[PatchRequest]] or [[DeleteRequest]]), returns Task<Answer> and has no body: Tachyon writes it. A [[StreamRequest]] or [[SocketRequest]] method returns void instead",
        "Each method of a Yon C++ [[HTTPClient]] class has one request marker ([[GetRequest(\"/path\")]], [[PostRequest]], [[PutRequest]], [[PatchRequest]] or [[DeleteRequest]]) or a [[Gateway({...})]] marker, returns its answer and has no body: Tachyon writes it. Yon C++ runs synchronously, so the answer is not a Task, for example `Order find([[PathVariable]] int id) const;`",
    ];
    const CALLS: [&'static str; 2] = [
        "    co_return co_await tachyon::http::TacRemoteCall::answer<{answer}>({adapter}, std::move(request));\n",
        "    return ::tachyon::yon::YonRemote::answer<{answer}>(tachyonBehaviors, {adapter}, std::move(request));\n",
    ];
    const PATH_FORM: &'static str = "A C++ request path is one string that starts with /, for example [[GetRequest(\"/orders/{id}\")]]";

    pub(crate) fn check(path: &Path, source: &str, class: &'a CppClass) -> Result<(), Failure> {
        Self::check_members(path, class)?;
        CppStreamRequest::check(path, source, class)
    }

    pub(crate) fn base(class: &CppClass) -> String {
        Self::written_path(
            class
                .markers
                .iter()
                .find(|marker| marker.name == "HTTPClient"),
        )
    }

    fn check_members(path: &Path, class: &CppClass) -> Result<(), Failure> {
        let yon = Realm::of(path) == Some(Realm::Yon);
        for member in Self::requests(class) {
            Self::check_member(path, member, yon)?;
        }
        Ok(())
    }

    pub(crate) fn requests(class: &CppClass) -> impl Iterator<Item = &CppMember> {
        class
            .members
            .iter()
            .filter(|member| !CppStreamRequest::is_stream(member))
            .filter(|member| !member.marked(Self::GATEWAY))
    }

    pub(crate) fn definitions(class: &'a CppClass) -> String {
        Self::written(class, Self::CALLS[0])
    }

    pub(crate) fn yon_definitions(class: &'a CppClass) -> String {
        Self::written(class, Self::CALLS[1])
    }

    pub(crate) fn adapter(class: &CppClass) -> String {
        crate::behavior::HttpClientSource::adapter_name(&class.name)
    }

    fn written(class: &'a CppClass, call: &'static str) -> String {
        let remote = Self {
            class,
            base: Self::base(class),
            adapter: Self::adapter(class),
            call,
        };
        remote.every_definition()
    }

    fn check_member(path: &Path, member: &CppMember, yon: bool) -> Result<(), Failure> {
        let verbs = Self::VERBS
            .iter()
            .filter(|(marker, _)| member.marked(marker))
            .count();
        let fits = [
            member.kind == CppMemberKind::Method,
            verbs == 1,
            member.is_task() != yon,
        ] == [true, true, true];
        let refused = Self::ONE_REQUEST[usize::from(yon)];
        fits.then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, member.name_at, refused))?;
        let marker = Self::request_marker(member);
        let written = marker.map_or("", |marker| marker.arguments.as_str());
        let valid = serde_json::from_str::<String>(written).is_ok_and(|path| path.starts_with('/'));
        valid
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, member.name_at, Self::PATH_FORM))
    }

    fn request_marker(member: &CppMember) -> Option<&CppAttribute> {
        member
            .markers
            .iter()
            .find(|marker| Self::VERBS.iter().any(|(name, _)| marker.name == *name))
    }

    fn written_path(marker: Option<&CppAttribute>) -> String {
        let arguments = marker.map_or("", |marker| marker.arguments.as_str());
        serde_json::from_str::<String>(arguments).unwrap_or_default()
    }

    fn every_definition(&self) -> String {
        let mut text = String::new();
        for member in Self::requests(self.class) {
            text.push_str(&self.definition(member));
        }
        text
    }

    fn definition(&self, member: &CppMember) -> String {
        let marker = Self::request_marker(member);
        let verb = Self::VERBS
            .iter()
            .find(|(name, _)| marker.is_some_and(|marker| marker.name == *name))
            .map_or("Get", |(_, verb)| verb);
        let path = format!("{}{}", self.base, Self::written_path(marker));
        let answer = Self::answer_type(member);
        let parameters = Self::parameter_list(&member.parameters);
        let constant = [" ", " const "][usize::from(member.is_const)];
        let target = Self::path_expression(&path, &member.parameters);
        let parts = Self::request_parts(&member.parameters);
        let guards = super::CppChecks::guards(member);
        let adapter = Self::literal(&self.adapter);
        let call = self
            .call
            .replace("{answer}", &answer)
            .replace("{adapter}", &adapter);
        format!(
            "inline {} {}::{}({parameters}){constant}{{{guards}\n    tachyon::http::HttpRequestMessage request(tachyon::http::HttpMethod::{verb}, {target});\n{parts}{call}}}\n",
            member.type_text, self.class.name, member.name
        )
    }

    pub(crate) fn answer_type(member: &CppMember) -> String {
        let plain = member.plain_type();
        let inner = Self::TASK_PREFIXES
            .iter()
            .find_map(|prefix| plain.strip_prefix(prefix))
            .unwrap_or(&plain);
        inner.strip_suffix('>').unwrap_or(inner).trim().to_owned()
    }

    pub(crate) fn parameter_list(parameters: &[CppParameter]) -> String {
        let written: Vec<String> = parameters
            .iter()
            .map(|parameter| format!("{} {}", parameter.type_text, parameter.name))
            .collect();
        written.join(", ")
    }

    fn path_expression(path: &str, parameters: &[CppParameter]) -> String {
        let mut pieces = vec![String::from("std::string()")];
        let mut rest = path;
        while let Some(open) = rest.find('{') {
            let close = rest[open..]
                .find('}')
                .map_or(rest.len(), |close| open + close);
            pieces.push(Self::literal(&rest[..open]));
            let variable = Self::path_variable(parameters, &rest[open + 1..close]);
            pieces.push(format!("{}({variable})", Self::SEGMENT));
            rest = rest.get(close + 1..).unwrap_or_default();
        }
        pieces.push(Self::literal(rest));
        pieces.join(" + ")
    }

    fn path_variable<'p>(parameters: &'p [CppParameter], written: &'p str) -> &'p str {
        parameters
            .iter()
            .find(|parameter| {
                Self::marker(parameter, "PathVariable")
                    .is_some_and(|marker| Self::named(marker, &parameter.name) == written)
            })
            .map_or(written, |parameter| parameter.name.as_str())
    }

    pub(crate) fn literal(text: &str) -> String {
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn request_parts(parameters: &[CppParameter]) -> String {
        let mut text = String::new();
        for parameter in parameters {
            text.push_str(&Self::request_part(parameter));
        }
        text
    }

    fn request_part(parameter: &CppParameter) -> String {
        format!(
            "{}{}{}",
            Self::body_part(parameter),
            Self::query_part(parameter),
            Self::header_part(parameter)
        )
    }

    fn marker<'p>(parameter: &'p CppParameter, wanted: &str) -> Option<&'p CppAttribute> {
        parameter
            .markers
            .iter()
            .find(|marker| marker.name == wanted)
    }

    fn body_part(parameter: &CppParameter) -> String {
        Self::marker(parameter, "RequestBody").map_or_else(String::new, |_| {
            format!(
                "    request.content = tachyon::http::JsonContent::create({});\n",
                parameter.name
            )
        })
    }

    fn query_part(parameter: &CppParameter) -> String {
        Self::marker(parameter, "RequestParam").map_or_else(String::new, |marker| {
            let key = Self::literal(&Self::named(marker, &parameter.name));
            format!(
                "    tachyon::http::TacBrowserRequest::addQuery(request, {key}, {});\n",
                parameter.name
            )
        })
    }

    fn header_part(parameter: &CppParameter) -> String {
        Self::marker(parameter, "RequestHeader").map_or_else(String::new, |marker| {
            let key = Self::literal(&Self::named(marker, &parameter.name));
            format!(
                "    request.headers.add({key}, tachyon::http::TacBrowserRequest::text({}));\n",
                parameter.name
            )
        })
    }

    fn named(marker: &CppAttribute, fallback: &str) -> String {
        serde_json::from_str::<String>(&marker.arguments).unwrap_or_else(|_| fallback.to_owned())
    }
}
