use super::LuaView;
use std::path::Path;

pub(super) struct LuaInput<'a> {
    pub(super) source: &'a Path,
    pub(super) contents: &'a str,
    pub(super) view: &'a LuaView,
    pub(super) models_root: Option<&'a Path>,
}
