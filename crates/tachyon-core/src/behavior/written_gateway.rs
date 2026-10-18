use super::gateway::{GatewayDeclaration, GatewayMethod, policy};
use super::{Input, Marker};
use crate::Failure;
use std::path::Path;

pub(crate) struct WrittenGateway<'a> {
    pub(crate) owner: &'a str,
    pub(crate) name: &'a str,
    pub(crate) arguments: &'a str,
    pub(crate) at: usize,
    pub(crate) parameter_types: Vec<&'a str>,
    pub(crate) result_type: &'a str,
    pub(crate) has_body: bool,
}

impl WrittenGateway<'_> {
    const CPP: &'static str = "hpp";
    const UNREAD: &'static str = "Gateway takes either class with an optional method, or command";

    pub(crate) fn declared(
        &self,
        path: &Path,
        source: &str,
    ) -> Result<GatewayDeclaration, Failure> {
        let input = Input {
            path,
            source,
            code: source.to_owned(),
        };
        let marker = Marker {
            name: String::from("Gateway"),
            arguments: self.arguments.to_owned(),
            span: self.at..self.at + 1,
            target: self.at,
            parameter: false,
        };
        let method = GatewayMethod {
            owner: self.owner,
            name: self.name,
            language: Self::CPP,
            parameter_types: self.parameter_types.clone(),
            result_type: self.result_type,
            has_body: self.has_body,
        };
        let written = policy(&input, &marker, &method)?;
        GatewayDeclaration::from_policy(&written)
            .ok_or_else(|| input.failure(self.at, Self::UNREAD))
    }
}
