use super::CppYonController;
use crate::Failure;
use crate::behavior::{GatewayDeclaration, WrittenGateway};
use crate::tac_cpp::{CppClass, CppFailure, CppMember, CppRemote, CppService, CppTypeText};
use std::path::Path;

pub(crate) struct CppYonGateway;

impl CppYonGateway {
    const MARKER: &'static str = "Gateway";
    const OUTSIDE: &'static str = "A Yon C++ [[Gateway]] method goes in an [[HTTPClient]] class in server/remotes, as a signature without a body, for example `[[Gateway({\"class\":\"server/services/ReportService.py\"})]] tachyon::json::JsonNode summarize(tachyon::json::JsonNode period) const;`";
    const COMBINED: &'static str = "Gateway cannot be combined with another method annotation";

    pub(crate) fn declarations(service: &CppService) -> Result<Vec<GatewayDeclaration>, Failure> {
        Self::refuse_outside(&service.path, service.class(), service.is_remote())?;
        Self::members(service.class())
            .map(|member| Self::declaration(service, member))
            .collect()
    }

    pub(crate) fn refuse_outside(
        path: &Path,
        class: &CppClass,
        remote: bool,
    ) -> Result<(), Failure> {
        let misplaced = Self::members(class).next().filter(|_| !remote);
        misplaced.map_or(Ok(()), |member| {
            Err(CppFailure::coded(
                CppYonController::CODE,
                path,
                member.name_at,
                Self::OUTSIDE,
            ))
        })
    }

    pub(crate) fn definitions(class: &CppClass) -> String {
        Self::members(class)
            .map(|member| Self::definition(class, member))
            .collect()
    }

    fn members(class: &CppClass) -> impl Iterator<Item = &CppMember> {
        class
            .members
            .iter()
            .filter(|member| member.marked(Self::MARKER))
    }

    fn declaration(
        service: &CppService,
        member: &CppMember,
    ) -> Result<GatewayDeclaration, Failure> {
        let (arguments, at) = member
            .markers
            .iter()
            .find(|marker| marker.name == Self::MARKER)
            .map_or(("", member.name_at), |marker| {
                (marker.arguments.as_str(), marker.at)
            });
        Self::check_alone(&service.path, member, at)?;
        let parameter_types: Vec<String> = member
            .parameters
            .iter()
            .map(|parameter| CppTypeText::plain(&parameter.type_text))
            .collect();
        let result_type = member.plain_type();
        WrittenGateway {
            owner: &service.class().name,
            name: &member.name,
            arguments,
            at,
            parameter_types: parameter_types.iter().map(String::as_str).collect(),
            result_type: &result_type,
            has_body: service.source.as_bytes().get(member.terminator) == Some(&b'{'),
        }
        .declared(&service.path, &service.source)
    }

    fn check_alone(path: &Path, member: &CppMember, at: usize) -> Result<(), Failure> {
        let combined = member
            .markers
            .iter()
            .any(|marker| marker.name != Self::MARKER);
        (!combined)
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, at, Self::COMBINED))
    }

    fn definition(class: &CppClass, member: &CppMember) -> String {
        let arguments: Vec<&str> = member
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        let constant = [" ", " const "][usize::from(member.is_const)];
        format!(
            "inline {} {class}::{name}({parameters}){constant}{{\n    return ::tachyon::yon::YonGateway::call(\"{class}.{name}\", {{{arguments}}});\n}}\n",
            member.type_text,
            class = class.name,
            name = member.name,
            parameters = CppRemote::parameter_list(&member.parameters),
            arguments = arguments.join(", ")
        )
    }
}
