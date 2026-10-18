use super::context_members::ContextMembers;
use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use crate::behavior::HttpClientSource;
use crate::lua_source::LuaView;
use std::path::Path;

pub(crate) struct LuaContract<'a> {
    source: &'a str,
    view: LuaView,
    tac: bool,
}

impl<'a> LuaContract<'a> {
    const REALMS: [&'static str; 2] = ["yon", "tac"];
    const LANGUAGE: &'static str = "lua";
    const SHADOW_LANGUAGE: &'static str = "rb";
    const PROVIDER: &'static str = "-- @Store\n";
    const CLIENT: &'static str = "HTTPClient";
    const HELD: [&'static str; 2] = ["streamrequest", "socketrequest"];
    const SOCKET: [&'static str; 1] = ["socketrequest"];
    const TAC: &'static str = "tac";
    const SEND: &'static str = "\nfunction {owner}:send(message)\n  return BehaviorBrowserSocket.send(\"{owner}\", message)\nend\n";

    pub(crate) fn lower(path: &Path, source: &'a str, environment: &str) -> Option<String> {
        let view = LuaView::of(path, source).filter(|_| Self::REALMS.contains(&environment))?;
        let contract = Self {
            source,
            view,
            tac: environment == Self::TAC,
        };
        contract
            .contract()
            .and_then(|found| contract.lowered(&found))
    }

    fn contract(&self) -> Option<(ContractDeclaration, bool, bool)> {
        let form = ContractForm::of(Self::SHADOW_LANGUAGE)?;
        let shadow = self.view.text();
        let mask = crate::lexical::code_mask(Self::SHADOW_LANGUAGE, shadow);
        let code = HttpClientSource::masked(shadow, &mask.code);
        let declaration = ContractDeclaration::find(form, shadow, &code)
            .into_iter()
            .find(|declaration| {
                [declaration.role.names_entity, Self::is_client(declaration)].contains(&true)
            })?;
        let (held, exchanged): (Vec<_>, Vec<_>) = declaration
            .members(form, &code)
            .into_iter()
            .partition(|member| member.is_marked_with(&Self::HELD));
        let needs_context = declaration.role.needs_context(&exchanged);
        let socket = held
            .iter()
            .any(|member| member.is_marked_with(&Self::SOCKET));
        Some((
            declaration,
            needs_context,
            [self.tac, socket] == [true, true],
        ))
    }

    fn is_client(declaration: &ContractDeclaration) -> bool {
        declaration.role.marker == Self::CLIENT
    }

    fn lowered(
        &self,
        (declaration, needs_context, socket): &(ContractDeclaration, bool, bool),
    ) -> Option<String> {
        let context = [String::new(), Self::context(declaration)];
        let send = ["", Self::SEND][usize::from(*socket)].replace("{owner}", &declaration.name);
        let provider = ["", Self::PROVIDER][usize::from(declaration.role.names_entity)];
        let marker_line = self.view.source_offset(declaration.marker_at);
        let mut output = self.source.to_owned();
        output.push_str(&context[usize::from(*needs_context)]);
        output.push_str(&send);
        output.insert_str(marker_line, provider);
        (output != self.source).then_some(output)
    }

    fn context(declaration: &ContractDeclaration) -> String {
        let setup = declaration.setup();
        let partition = declaration.adapter();
        let members = ContextMembers {
            language: Self::LANGUAGE,
            tac: false,
            provided: false,
            needs_context: true,
            owner: &declaration.name,
            receiver: declaration.role.noun,
            setup: &setup,
            partition: &partition,
            indent: "",
        };
        members.parts_for(declaration.role.marker).1
    }
}
