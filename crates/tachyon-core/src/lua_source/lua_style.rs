use super::LuaView;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct LuaStyle;

impl LuaStyle {
    const CODE: u16 = 1357;
    const HELP: &'static str = "A Lua class file holds one global class table, `Name = {}`, with `-- @Role` comment markers and LuaCATS types above it, and its methods as `function Name:method(...)` (ADR 0085).";

    pub(crate) fn validate(path: &Path, source: &str) -> Result<(), Failure> {
        let Some(view) = LuaView::of(path, source) else {
            return Ok(());
        };
        let Some((line, message)) = view.shadow().first_problem() else {
            return Ok(());
        };
        let start = view.line_start(line);
        let end = source[start..].find('\n').map_or(source.len(), |length| start + length);
        Err(Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(view.source_name(), start, end.max(start + 1).min(source.len())),
        )))
    }
}
