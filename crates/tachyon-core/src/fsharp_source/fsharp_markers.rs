use super::{FsharpLexer, FsharpType};
use crate::annotation_rules::AnnotationRules;
use std::ops::Range;

pub(crate) struct FsharpMarkers;

impl FsharpMarkers {
    const OPEN: &'static str = "[<";
    const CLOSE: &'static str = ">]";

    pub(crate) fn strip(source: &str) -> String {
        let code = FsharpLexer::code_mask(source).code;
        Self::without_groups(source, Self::groups(source, &code))
    }

    fn without_groups(source: &str, groups: Vec<Range<usize>>) -> String {
        let mut stripped = source.to_owned();
        for group in groups.into_iter().rev() {
            let replacement = Self::kept(&source[group.clone()]);
            stripped.replace_range(group, &replacement);
        }
        stripped
    }

    fn groups(source: &str, code: &[bool]) -> Vec<Range<usize>> {
        let mut groups = Vec::new();
        for (start, _) in source.match_indices(Self::OPEN) {
            groups.extend(Self::group_at(source, code, start));
        }
        groups
    }

    fn group_at(source: &str, code: &[bool], start: usize) -> Option<Range<usize>> {
        code.get(start).copied().filter(|visible| *visible)?;
        let length = source[start..].find(Self::CLOSE)?;
        let end = start + length + Self::CLOSE.len();
        let line_break = source[start..end].contains('\n');
        (!line_break).then_some(start..end)
    }

    fn kept(group: &str) -> String {
        let inside = &group[Self::OPEN.len()..group.len() - Self::CLOSE.len()];
        let kept: Vec<&str> = FsharpType::split_top_level(inside, ";")
            .into_iter()
            .filter(|item| !Self::is_tachyon(item))
            .collect();
        let written = Self::written(&kept);
        format!("{written:<width$}", width = group.len())
    }

    fn written(kept: &[&str]) -> String {
        if kept.is_empty() {
            return String::new();
        }
        format!("{}{}{}", Self::OPEN, kept.join("; "), Self::CLOSE)
    }

    fn is_tachyon(item: &str) -> bool {
        let name = item.split(['(', ' ']).next().unwrap_or(item);
        AnnotationRules::find(name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpMarkers;

    #[test]
    fn tachyon_attributes_are_blanked_and_others_stay() {
        let source = "[<Service>]\ntype A() =\n    [<GetRoute; Obsolete>]\n    member this.Get([<FromBody>] order: Order) = \"[<Service>]\"\n";
        let stripped = FsharpMarkers::strip(source);
        assert_eq!(
            stripped,
            "           \ntype A() =\n    [<Obsolete>]          \n    member this.Get(             order: Order) = \"[<Service>]\"\n"
        );
    }
}
