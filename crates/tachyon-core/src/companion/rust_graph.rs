use super::{
    RustCollaborators, RustConstruction, block_named, blocks, declared_service, rust_owner_table,
    rust_struct_fields, unnamed_service,
};
use crate::Failure;
use crate::unicode_name::UnicodeName;

pub(crate) struct RustGraph;

impl RustGraph {
    pub(crate) fn declares_module(source: &str, name: &str) -> bool {
        blocks(source, "mod")
            .iter()
            .any(|(header, _)| UnicodeName::leading(header.trim()) == name)
    }

    pub(crate) fn type_names(source: &str) -> std::collections::BTreeSet<String> {
        ["struct", "trait", "enum"]
            .iter()
            .flat_map(|keyword| blocks(source, keyword))
            .map(|(header, _)| UnicodeName::leading(header.trim()))
            .collect()
    }

    pub(crate) fn instance(
        source: &str,
        path: &str,
        name: &str,
        services: &std::collections::BTreeSet<String>,
    ) -> Result<(String, std::collections::BTreeSet<String>), Failure> {
        RustConstruction::graph_instance(source, path, name, services)
    }

    pub(crate) fn constructor_inputs(
        source: &str,
        path: &str,
        name: &str,
    ) -> Result<Vec<String>, Failure> {
        RustConstruction::constructor_inputs(source, path, name)
    }

    pub(crate) fn dependencies(
        source: &str,
        path: &str,
        name: &str,
    ) -> Result<std::collections::BTreeSet<String>, Failure> {
        RustConstruction::arc_dependencies(source, path, name)
    }

    pub(crate) fn table(
        source: &str,
        path: &str,
        instance: &str,
        events: (&[String], &str),
    ) -> Result<String, Failure> {
        let table = rust_owner_table(source, path, events.0, events.1, false)?;
        let name =
            declared_service(path, source).ok_or_else(|| unnamed_service(path, None, "struct"))?;
        let (_, members) = table
            .split_once("\nvec![")
            .ok_or_else(|| unnamed_service(path, Some(&name), "struct"))?;
        let members = members
            .strip_suffix("]\n}\n")
            .ok_or_else(|| unnamed_service(path, Some(&name), "struct"))?;
        Ok(format!(
            "pub(crate) fn tac_members_{name}(scope: &NativeServiceScope) -> ::core::result::Result<Vec<(&'static str, TacMember)>, String> {{\nlet instance = {instance};\nOk(vec![{members}])\n}}\n"
        ))
    }

    pub(crate) fn supports_arc(source: &str, path: &str, exposed: bool) -> Result<bool, Failure> {
        let wanted = declared_service(path, source);
        let (name, body) = block_named(source, "struct", wanted.as_deref())
            .ok_or_else(|| unnamed_service(path, wanted.as_deref(), "struct"))?;
        let collaborators = RustCollaborators::declared_beside(&name, source);
        let has_setters = rust_struct_fields(body)
            .iter()
            .any(|member| !collaborators.holds(member.declared_type.as_deref()));
        let masked = crate::annotation_layout::masked_source(std::path::Path::new(path), source)?;
        let mutates = blocks(&masked, "impl")
            .iter()
            .filter(|(header, _)| header.trim() == name)
            .any(|(_, body)| body.contains("&mut self"));
        Ok(![mutates, [has_setters, exposed] == [true; 2]].contains(&true))
    }
}
