mod lua_body;
mod lua_lexer;
mod lua_line;
mod lua_lowering;
mod lua_models;
mod lua_shadow;
mod lua_style;
mod lua_test_unit;
mod lua_types;
mod lua_view;
mod lua_yon;
mod shadow_lines;
mod shadow_writer;
mod writer_mode;

#[cfg(test)]
mod tests;

use lua_body::LuaBody;
pub(crate) use lua_lexer::LuaLexer;
use lua_line::LuaLine;
use lua_lowering::LuaLowering;
pub(crate) use lua_models::LuaModels;
pub(crate) use lua_shadow::LuaShadow;
pub(crate) use lua_style::LuaStyle;
use lua_test_unit::LuaTestUnit;
use lua_types::LuaTypes;
pub(crate) use lua_view::LuaView;
pub(crate) use lua_yon::LuaYon;
use shadow_lines::ShadowLines;
use shadow_writer::ShadowWriter;
use writer_mode::WriterMode;

use std::path::Path;

pub(crate) struct LuaSource;

impl LuaSource {
    pub(crate) const EXTENSION: &'static str = "lua";
    pub(crate) const RUNTIME: &'static str = "lua";
    const SYNTAX: &'static str = "rb";

    pub(crate) fn is_lua(source: &Path) -> bool {
        source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(Self::EXTENSION))
    }

    pub(crate) fn syntax(extension: &str) -> &str {
        if extension == Self::EXTENSION {
            return Self::SYNTAX;
        }
        extension
    }
}
