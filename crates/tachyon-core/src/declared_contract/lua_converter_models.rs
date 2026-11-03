use super::converter_models::ConverterModels;
use crate::lua_source::LuaView;
use serde_json::{Map, Value};
use std::ops::Range;
use std::path::Path;

pub(super) struct LuaConverterModels;

type SourceEdit = (Range<usize>, String);

impl LuaConverterModels {
    pub(super) fn written(
        path: &Path,
        source: &str,
        models: &Map<String, Value>,
        realm: &str,
    ) -> Result<Option<String>, String> {
        let view = LuaView::joined(path, source);
        let edits = ConverterModels::edits_in(view.path(), view.text(), models, realm)?;
        Ok(Self::applied(source, &view, edits))
    }

    fn applied(source: &str, view: &LuaView, edits: Vec<SourceEdit>) -> Option<String> {
        let changed = !edits.is_empty();
        let mut output = source.to_owned();
        for (range, replacement) in edits.into_iter().rev() {
            let (range, marker) = Self::lua_edit(source, view, range, &replacement);
            output.replace_range(range, &marker);
        }
        changed.then_some(output)
    }

    fn lua_edit(
        source: &str,
        view: &LuaView,
        range: Range<usize>,
        replacement: &str,
    ) -> SourceEdit {
        let start = view.source_offset(range.start);
        if range.is_empty() {
            return (start..start, Self::marker(replacement));
        }
        let line = Self::rewritten_line(view.text(), range, replacement);
        let end = source[start..]
            .find('\n')
            .map_or(source.len(), |at| start + at);
        (start..end, Self::marker(&line))
    }

    fn rewritten_line(source: &str, range: Range<usize>, replacement: &str) -> String {
        let start = source[..range.start].rfind('\n').map_or(0, |at| at + 1);
        let end = source[range.end..]
            .find('\n')
            .map_or(source.len(), |at| range.end + at);
        let mut line = source[start..end].to_owned();
        line.replace_range(range.start - start..range.end - start, replacement);
        line
    }

    fn marker(ruby: &str) -> String {
        let text = ruby.trim().strip_prefix("#[").unwrap_or(ruby.trim());
        let text = text.strip_suffix(']').unwrap_or(text);
        let ending = ruby.strip_suffix('\n').map_or("", |_| "\n");
        format!("-- @{text}{ending}")
    }
}
