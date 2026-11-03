use super::LuaConverterProviders;
use crate::Failure;
use crate::lua_source::{LuaFields, LuaView};
use crate::tac_ruby::RubyHeldStreams;
use std::path::Path;

pub(super) struct LuaHeldStreams<'a> {
    snapshot_root: &'a Path,
    relative: &'a Path,
}

impl<'a> LuaHeldStreams<'a> {
    const REALM: &'static str = "tac";
    const CLIENT: &'static str = "client";
    const NO_TABLE: &'static str = "";

    pub(super) const fn new(snapshot_root: &'a Path, relative: &'a Path) -> Self {
        Self {
            snapshot_root,
            relative,
        }
    }

    pub(super) fn describe(
        &self,
        assembled: &LuaView,
        (class, source): (&str, &str),
    ) -> Result<serde_json::Value, Failure> {
        let needed: Vec<String> = LuaFields::without_values(source, class)
            .into_iter()
            .filter(|(_, _, marked)| *marked)
            .map(|(_, type_name, _)| type_name)
            .collect();
        crate::events::extract(assembled.path(), assembled.text(), Self::REALM)
            .map_err(|failure| assembled.failure(&failure))?
            .map(|module| self.held(module, class, &needed))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    pub(super) fn clients(
        streams: &serde_json::Value,
        (class, source): (&str, &str),
    ) -> Vec<(String, String)> {
        let held: Vec<&str> = streams
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|stream| stream["client"].as_str())
            .collect();
        LuaFields::without_values(source, class)
            .into_iter()
            .filter(|(_, name, marked)| [*marked, held.contains(&name.as_str())] == [true, true])
            .map(|(field, name, _)| (field, name))
            .collect()
    }

    pub(super) fn table(class: &str, clients: &[(String, String)]) -> String {
        let entries = LuaConverterProviders::entries(clients);
        let table = format!("{class}.__tachyonClients = {{ {entries} }}\n");
        [Self::NO_TABLE, table.as_str()][usize::from(!clients.is_empty())].to_owned()
    }

    fn held(
        &self,
        mut module: crate::events::EventModule,
        class: &str,
        needed: &[String],
    ) -> Result<serde_json::Value, Failure> {
        crate::events::describe(&mut module, self.relative, self.snapshot_root, Self::CLIENT)?;
        RubyHeldStreams::lua(&module, class).held(needed, self.relative)
    }
}
