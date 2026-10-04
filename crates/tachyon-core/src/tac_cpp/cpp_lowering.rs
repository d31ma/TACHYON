use super::{CppAttribute, CppFile, CppMarkerCheck, CppService};
use std::ops::Range;

pub(crate) struct CppLowering {
    text: String,
    insertions: Vec<(usize, String)>,
}

impl CppLowering {
    const REMOVED_DIRECTIVES: [&'static str; 3] =
        ["pragma once", "include \"", "include <tachyon/"];

    pub(crate) fn model(file: &CppFile, source: &str) -> String {
        let lowering = Self::blanked(file, source);
        lowering.finished()
    }

    pub(crate) fn service(service: &CppService) -> String {
        let mut lowering = Self::blanked(&service.file, &service.source);
        let class = service.class();
        lowering.insertions.push((
            class.body.start + 1,
            format!(" friend class {};", service.binding_name()),
        ));
        for member in class.members.iter().filter(|member| member.marked("Needs")) {
            let provided = format!(" = ::tachyon::TacNeeds::of<{}>()", member.plain_type());
            lowering.insertions.push((member.terminator, provided));
        }
        lowering.finished()
    }

    fn blanked(file: &CppFile, source: &str) -> Self {
        let mut text = source.to_owned();
        for range in &file.directives {
            let directive = Self::squeezed(&source[range.clone()]);
            if Self::REMOVED_DIRECTIVES
                .iter()
                .any(|removed| directive.contains(removed))
            {
                Self::blank(&mut text, range);
            }
        }
        let class = &file.class;
        let markers = class
            .every_marker()
            .into_iter()
            .filter(|marker| Self::is_tachyon(marker));
        for marker in markers {
            Self::blank(&mut text, &marker.specifier);
        }
        Self {
            text,
            insertions: Vec::new(),
        }
    }

    fn is_tachyon(marker: &CppAttribute) -> bool {
        let standard = CppMarkerCheck::is_standard(&marker.name);
        !standard && crate::annotation_rules::AnnotationRules::find(&marker.name).is_some()
    }

    fn squeezed(directive: &str) -> String {
        directive
            .trim_start_matches('#')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn blank(text: &mut String, range: &Range<usize>) {
        let spaces: String = text[range.clone()]
            .chars()
            .map(Self::blank_character)
            .collect();
        text.replace_range(range.clone(), &spaces);
    }

    fn blank_character(character: char) -> String {
        if character == '\n' {
            return String::from("\n");
        }
        " ".repeat(character.len_utf8())
    }

    fn finished(mut self) -> String {
        self.insertions
            .sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        for (at, inserted) in &self.insertions {
            self.text.insert_str(*at, inserted);
        }
        self.text
    }
}
