mod lua_behavior_edits;
mod lua_body;
mod lua_events;
mod lua_input;
mod lua_lexer;
mod lua_line;
mod lua_lowering;
mod lua_models;
mod lua_resources;
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

use lua_behavior_edits::LuaBehaviorEdits;
use lua_body::LuaBody;
use lua_events::LuaEvents;
use lua_input::LuaInput;
pub(crate) use lua_lexer::LuaLexer;
use lua_line::LuaLine;
use lua_lowering::LuaLowering;
pub(crate) use lua_models::LuaModels;
use lua_resources::LuaResources;
pub(crate) use lua_shadow::LuaShadow;
pub(crate) use lua_style::LuaStyle;
pub(crate) use lua_test_unit::LuaTestUnit;
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
        Self::syntax_of(extension)
    }

    pub(crate) fn long_string(text: &str) -> String {
        let equals = "=".repeat(Self::closing_level(text));
        format!("[{equals}[{text}]{equals}]")
    }

    fn closing_level(text: &str) -> usize {
        let probe = format!("{text}]");
        (0..=text.len())
            .find(|level| !probe.contains(&format!("]{}]", "=".repeat(*level))))
            .unwrap_or_default()
    }

    fn syntax_of(extension: &str) -> &str {
        if extension == Self::EXTENSION {
            return Self::SYNTAX;
        }
        extension
    }
}
