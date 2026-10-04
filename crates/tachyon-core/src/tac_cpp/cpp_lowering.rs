use super::{CppAttribute, CppClass, CppFile, CppMarkerCheck, CppRemote, CppService};
use std::ops::Range;

pub(crate) struct CppLowering {
    text: String,
    insertions: Vec<(usize, String)>,
}

impl CppLowering {
    const REMOVED_DIRECTIVES: [&'static str; 3] =
        ["pragma once", "include \"", "include <tachyon/"];

    pub(crate) fn model(file: &CppFile, source: &str) -> String {
        Self::blanked(file, source).finished()
    }

    pub(crate) fn service(service: &CppService) -> String {
        let mut lowering = Self::blanked(&service.file, &service.source);
        let class = service.class();
        let friend = format!(" friend class {};", service.binding_name());
        lowering.insertions.push((class.body.start + 1, friend));
        lowering.fill_needs(class);
        let remote = Self::remote_definitions(service);
        format!("{}{remote}", lowering.finished())
    }

    fn remote_definitions(service: &CppService) -> String {
        if !service.is_remote() {
            return String::new();
        }
        format!("\n{}", CppRemote::definitions(service.class()))
    }

    fn fill_needs(&mut self, class: &CppClass) {
        for member in class.members.iter().filter(|member| member.marked("Needs")) {
            let provided = format!(" = ::tachyon::TacNeeds::of<{}>()", member.plain_type());
            self.insertions.push((member.terminator, provided));
        }
    }

    fn blanked(file: &CppFile, source: &str) -> Self {
        let mut text = source.to_owned();
        Self::blank_directives(&mut text, file, source);
        Self::blank_markers(&mut text, &file.class);
        Self {
            text,
            insertions: Vec::new(),
        }
    }

    fn blank_directives(text: &mut String, file: &CppFile, source: &str) {
        let removed = file
            .directives
            .iter()
            .filter(|range| Self::is_removed(&source[(*range).clone()]));
        for range in removed {
            Self::blank(text, range);
        }
    }

    fn blank_markers(text: &mut String, class: &CppClass) {
        let markers = class
            .every_marker()
            .into_iter()
            .filter(|marker| Self::is_tachyon(marker));
        for marker in markers {
            Self::blank(text, &marker.specifier);
        }
    }

    fn is_removed(directive: &str) -> bool {
        let squeezed = directive
            .trim_start_matches('#')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        Self::REMOVED_DIRECTIVES
            .iter()
            .any(|removed| squeezed.contains(removed))
    }

    fn is_tachyon(marker: &CppAttribute) -> bool {
        let standard = CppMarkerCheck::is_standard(&marker.name);
        let known = crate::annotation_rules::AnnotationRules::find(&marker.name).is_some();
        [standard, known] == [false, true]
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
