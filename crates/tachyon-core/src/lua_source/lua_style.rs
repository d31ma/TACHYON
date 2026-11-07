use super::{LuaShadow, LuaView};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct LuaStyle;

impl LuaStyle {
    const CODE: u16 = 1357;
    const HELP: &'static str = "A Lua class file holds one global class table, `Name = {}`, with `-- @Role` comment markers and LuaCATS types above it, and its methods as `function Name:method(...)` (ADR 0085).";

    pub(crate) fn check_rules(path: &Path, source: &str) -> Result<(), Failure> {
        LuaView::of(path, source).map_or(Ok(()), |_| {
            crate::annotation_rules::AnnotationRules::check(path, source)
        })
    }

    pub(crate) fn validate(path: &Path, source: &str) -> Result<(), Failure> {
        LuaView::of(path, source).map_or(Ok(()), |view| Self::first_problem(&view, source))
    }

    fn first_problem(view: &LuaView, source: &str) -> Result<(), Failure> {
        LuaShadow::of(source)
            .first_problem()
            .map_or(Ok(()), |(line, message)| {
                Err(Self::failure(view, source, line, message))
            })
    }

    fn failure(view: &LuaView, source: &str, line: usize, message: &str) -> Failure {
        let start = view.line_start(line);
        let length = source[start..].lines().next().map_or(0, str::len);
        let end = (start + length.max(1)).min(source.len());
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(view.source_name(), start, end),
        ))
    }
}
