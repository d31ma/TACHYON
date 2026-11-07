#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::name_picker::NamePicker;
use std::ops::Range;

pub(super) struct StructFields;

impl StructFields {
    const STRUCT: &'static str = "struct";
    const RUST: &'static str = "rs";
    const GO: &'static str = "go";
    const COMMA: &'static str = ",";

    pub(super) fn read(class: &ClassFile) -> Vec<usize> {
        let rust = Self::bodies(class, Self::RUST)
            .into_iter()
            .flat_map(|body| NamePicker::split_at_commas(class, body))
            .filter_map(|part| {
                NamePicker::field_name(class, &NamePicker::without_markers(class, part))
            });
        let go = Self::bodies(class, Self::GO)
            .into_iter()
            .flat_map(|body| Self::go_lines(class, body))
            .flat_map(|line| Self::go_names(class, line));
        rust.chain(go).collect()
    }

    fn bodies(class: &ClassFile, extension: &str) -> Vec<Range<usize>> {
        let wanted = class.extension == extension;
        (0..class.words.len())
            .filter(|&at| wanted & (class.words.text(at) == Self::STRUCT))
            .map(|at| class.words.inside_of(Self::opening(class, at)))
            .collect()
    }

    fn opening(class: &ClassFile, at: usize) -> Option<usize> {
        let next = [at + 1, at + 2]
            .into_iter()
            .find(|&index| class.words.text(index) == "{");
        next.filter(|&index| class.words.depth(index) == class.words.depth(at))
    }

    fn go_lines(class: &ClassFile, body: Range<usize>) -> Vec<Range<usize>> {
        let depth = class.words.depth(body.start);
        let mut starts: Vec<usize> = body
            .clone()
            .filter(|&at| class.words.line_start(at) & (class.words.depth(at) == depth))
            .collect();
        starts.push(body.end);
        starts.windows(2).map(|pair| pair[0]..pair[1]).collect()
    }

    fn go_names(class: &ClassFile, line: Range<usize>) -> Vec<usize> {
        let names = Self::leading_names(class, &line);
        let after = names.last().map_or(line.start, |&at| at + 1);
        let typed = after < line.end;
        names.into_iter().filter(|_| typed).collect()
    }

    fn leading_names(class: &ClassFile, line: &Range<usize>) -> Vec<usize> {
        let mut names = Vec::new();
        let mut at = line.start;
        while class.words.is_name(at) & (at < line.end) {
            names.push(at);
            let comma = class.words.text(at + 1) == Self::COMMA;
            at = [line.end, at + 2][usize::from(comma)];
        }
        names
    }
}
