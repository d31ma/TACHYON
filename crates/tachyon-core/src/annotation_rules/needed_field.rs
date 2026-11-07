use super::AnnotationRules;
use crate::fsharp_source::{FsharpSource, FsharpView};
use crate::lua_source::LuaView;
use crate::tac_cpp::{CppFile, CppMemberKind, CppSource};
use std::path::Path;

pub(crate) struct NeededField {
    pub(crate) name: String,
    pub(crate) at: usize,
}

impl NeededField {
    const NEEDS: &'static str = "Needs";

    pub(super) fn through_view(path: &Path, source: &str) -> Option<Vec<Self>> {
        let cpp = CppSource::is_cpp(path).then(|| Self::in_cpp(path, source));
        cpp.or_else(|| Self::in_fsharp(path, source))
            .or_else(|| Self::in_lua(path, source))
    }

    fn in_cpp(path: &Path, source: &str) -> Vec<Self> {
        let members = CppFile::read(path, source)
            .map(|file| file.class.members)
            .unwrap_or_default();
        members
            .into_iter()
            .filter(|member| member.is_instance(CppMemberKind::Field))
            .filter(|member| member.marked(Self::NEEDS))
            .map(|member| Self {
                name: member.name,
                at: member.name_at,
            })
            .collect()
    }

    fn in_fsharp(path: &Path, source: &str) -> Option<Vec<Self>> {
        let view = FsharpSource::is_fsharp(path).then(|| FsharpView::unmoved(path, source))?;
        let fields = AnnotationRules::needed_fields(view.path(), view.text());
        Some(
            fields
                .into_iter()
                .map(|field| Self {
                    at: view.source_at(field.at),
                    name: field.name,
                })
                .collect(),
        )
    }

    fn in_lua(path: &Path, source: &str) -> Option<Vec<Self>> {
        let view = LuaView::of(path, source)?;
        let fields = AnnotationRules::needed_fields(view.path(), view.text());
        Some(
            fields
                .into_iter()
                .map(|field| {
                    let line = view.source_offset(field.at);
                    let column = source[line..].find(&field.name).unwrap_or_default();
                    Self {
                        at: line + column,
                        name: field.name,
                    }
                })
                .collect(),
        )
    }
}
