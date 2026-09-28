#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::name_picker::NamePicker;
use super::parameter::Parameter;
use std::ops::Range;

pub(super) struct ParameterList;

impl ParameterList {
    const RECEIVERS: [&'static str; 3] = ["self", "cls", "this"];
    const GROUPED: [&'static str; 2] = ["{", "["];
    const DART: &'static str = "dart";
    const REQUEST_MARKERS: [&'static str; 5] = [
        "frompath",
        "fromquery",
        "frombody",
        "fromheader",
        "fromcookie",
    ];

    pub(super) fn read(class: &ClassFile, inside: Range<usize>) -> Vec<Parameter> {
        Self::parts(class, inside)
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(|part| Self::parameter(class, &part))
            .collect()
    }

    fn parts(class: &ClassFile, inside: Range<usize>) -> Vec<Range<usize>> {
        let parts = NamePicker::split_at_commas(class, inside);
        parts
            .into_iter()
            .flat_map(|part| Self::dart_named(class, part))
            .collect()
    }

    fn dart_named(class: &ClassFile, part: Range<usize>) -> Vec<Range<usize>> {
        let grouped = Self::GROUPED.contains(&class.words.text(part.start));
        if grouped & (class.extension == Self::DART) {
            return NamePicker::split_at_commas(class, class.words.inside(part.start));
        }
        vec![part]
    }

    fn parameter(class: &ClassFile, part: &Range<usize>) -> Parameter {
        let name_at = NamePicker::parameter_name(class, part.clone());
        let framework = part
            .clone()
            .any(|at| Self::is_request_marker(class.words.text(at)));
        let receiver = name_at.is_some_and(|at| Self::RECEIVERS.contains(&class.words.text(at)));
        let named = part
            .clone()
            .any(|at| class.words.is_name(at) | Self::GROUPED.contains(&class.words.text(at)));
        Parameter {
            name_at: name_at.filter(|_| !receiver),
            counted: named & !(framework | receiver),
        }
    }

    fn is_request_marker(word: &str) -> bool {
        let normalized = word.replace('_', "").to_ascii_lowercase();
        Self::REQUEST_MARKERS.contains(&normalized.as_str())
    }
}
