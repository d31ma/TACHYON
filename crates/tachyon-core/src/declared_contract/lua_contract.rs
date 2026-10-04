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

    pub(crate) fn lower(path: &Path, source: &'a str, environment: &str) -> Option<String> {
        let view = LuaView::of(path, source).filter(|_| environment == Self::YON)?;
        let contract = Self { source, view };
        contract.store().map(|store| contract.lowered(&store))
    }

    fn store(&self) -> Option<ContractDeclaration> {
        let form = ContractForm::of(Self::SHADOW_LANGUAGE)?;
        let shadow = self.view.text();
        let mask = crate::lexical::code_mask(Self::SHADOW_LANGUAGE, shadow);
        let code = HttpClientSource::masked(shadow, &mask.code);
        ContractDeclaration::find(form, shadow, &code)
            .into_iter()
            .find(|declaration| declaration.role.names_entity)
    }

    fn lowered(&self, store: &ContractDeclaration) -> String {
        let marker_line = self.view.source_offset(store.marker_at);
        let mut output = self.source.to_owned();
        output.push_str(&Self::context(store));
        output.insert_str(marker_line, Self::PROVIDER);
        output
    }

    fn context(store: &ContractDeclaration) -> String {
        let setup = store.setup();
        let partition = store.adapter();
        let members = ContextMembers {
            language: Self::LANGUAGE,
            tac: false,
            provided: false,
            needs_context: true,
            owner: &store.name,
            receiver: store.role.noun,
            setup: &setup,
            partition: &partition,
            indent: "",
        };
        members.parts_for(store.role.marker).1
    }
}
