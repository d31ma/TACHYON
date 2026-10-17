use super::context_members::ContextMembers;
use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use crate::behavior::HttpClientSource;
use crate::lua_source::LuaView;
use std::path::Path;

pub(crate) struct LuaContract<'a> {
    source: &'a str,
    view: LuaView,
}

impl<'a> LuaContract<'a> {
    const YON: &'static str = "yon";
    const LANGUAGE: &'static str = "lua";
    const SHADOW_LANGUAGE: &'static str = "rb";
    const PROVIDER: &'static str = "-- @Store\n";
    const CLIENT: &'static str = "HTTPClient";

    pub(crate) fn lower(path: &Path, source: &'a str, environment: &str) -> Option<String> {
        let view = LuaView::of(path, source).filter(|_| environment == Self::YON)?;
        let contract = Self { source, view };
        contract
            .contract()
            .and_then(|found| contract.lowered(&found))
    }

    fn contract(&self) -> Option<(ContractDeclaration, bool)> {
        let form = ContractForm::of(Self::SHADOW_LANGUAGE)?;
        let shadow = self.view.text();
        let mask = crate::lexical::code_mask(Self::SHADOW_LANGUAGE, shadow);
        let code = HttpClientSource::masked(shadow, &mask.code);
        let declaration = ContractDeclaration::find(form, shadow, &code)
            .into_iter()
            .find(|declaration| {
                [declaration.role.names_entity, Self::is_client(declaration)].contains(&true)
            })?;
        let members = declaration.members(form, &code);
        let needs_context = declaration.role.needs_context(&members);
        Some((declaration, needs_context))
    }

    fn is_client(declaration: &ContractDeclaration) -> bool {
        declaration.role.marker == Self::CLIENT
    }

    fn lowered(
        &self,
        (declaration, needs_context): &(ContractDeclaration, bool),
    ) -> Option<String> {
        let context = [String::new(), Self::context(declaration)];
        let provider = ["", Self::PROVIDER][usize::from(declaration.role.names_entity)];
        let marker_line = self.view.source_offset(declaration.marker_at);
        let mut output = self.source.to_owned();
        output.push_str(&context[usize::from(*needs_context)]);
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
