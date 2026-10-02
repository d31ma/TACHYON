use super::{AttributeLine, FsharpConstructorProps, FsharpView, LineKind, TypeHeader};
use crate::Failure;
use std::ops::Range;
use std::path::Path;

pub(super) struct FsharpHelperOwnership<'a> {
    path: &'a Path,
    source: &'a str,
    code: String,
    classes: Vec<(String, Range<usize>)>,
}

impl<'a> FsharpHelperOwnership<'a> {
    const FORBIDDEN: [&'static str; 6] = [
        "Needs", "NeedsAll", "Lifetime", "OnStart", "OnStop", "OnEvent",
    ];
    const LIMIT: &'static str = "F# Tac Helpers require explicit caller-owned new construction and an ordinary primary constructor. Helper Needs properties, managed injection, lifetime and lifecycle hooks remain unsupported.";

    pub(super) fn read(path: &'a Path, source: &'a str) -> Result<Self, Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        let classes = Self::classes(&code);
        Ok(Self {
            path,
            source,
            code,
            classes,
        })
    }

    pub(super) fn require_source(&self) -> Result<(), Failure> {
        self.helpers()
            .iter()
            .try_for_each(|(_, range)| self.require_helper(range))
    }

    pub(super) fn require_callers(&self) -> Result<(), Failure> {
        self.helpers()
            .iter()
            .try_for_each(|(name, range)| self.require_caller(name, range))?;
        self.require_dependencies()
    }

    fn classes(code: &str) -> Vec<(String, Range<usize>)> {
        let mut offset = 0;
        let mut classes: Vec<(String, Range<usize>)> = Vec::new();
        for line in code.lines() {
            Self::record_class(&mut classes, line, offset, code.len());
            offset += line.len() + 1;
        }
        classes
    }

    fn record_class(classes: &mut Vec<(String, Range<usize>)>, line: &str, at: usize, end: usize) {
        if !line.starts_with(char::is_whitespace) && LineKind::of(line) == LineKind::Type {
            classes
                .iter_mut()
                .rev()
                .take(1)
                .for_each(|(_, range)| range.end = at);
            classes.push((TypeHeader::read(line, &[], false).name, at..end));
        }
    }

    fn helpers(&self) -> Vec<(&str, &Range<usize>)> {
        self.classes
            .iter()
            .filter(|(_, range)| self.class_markers(range).contains(&String::from("Helper")))
            .map(|(name, range)| (name.as_str(), range))
            .collect()
    }

    fn class_markers(&self, range: &Range<usize>) -> Vec<String> {
        self.code[..range.start]
            .lines()
            .rev()
            .take_while(|line| Self::attribute_prefix(line))
            .flat_map(|line| AttributeLine::raw(line.trim()))
            .map(|attribute| Self::marker_name(&attribute.name).to_owned())
            .collect()
    }

    fn attribute_prefix(line: &str) -> bool {
        line.trim().is_empty() || line.trim().starts_with("[<")
    }

    fn require_helper(&self, range: &Range<usize>) -> Result<(), Failure> {
        self.require_primary(range)?;
        self.require_class_markers(range)?;
        self.require_member_markers(range)
    }

    fn require_primary(&self, range: &Range<usize>) -> Result<(), Failure> {
        let line = self.code[range.clone()].lines().next().unwrap_or("");
        let header = TypeHeader::read(line, &[], false);
        let after = line
            .split_once(header.name.as_str())
            .map_or("", |(_, tail)| tail);
        after
            .trim_start()
            .starts_with('(')
            .then_some(())
            .ok_or_else(|| self.failure(range.start))
    }

    fn require_class_markers(&self, range: &Range<usize>) -> Result<(), Failure> {
        self.class_markers(range)
            .iter()
            .find(|name| Self::forbidden(name))
            .map_or(Ok(()), |name| {
                Err(self.failure(self.marker_offset(name, range)))
            })
    }

    fn require_member_markers(&self, range: &Range<usize>) -> Result<(), Failure> {
        let mut offset = range.start;
        for line in self.code[range.clone()].lines() {
            self.require_member_marker(line, offset)?;
            offset += line.len() + 1;
        }
        Ok(())
    }

    fn require_member_marker(&self, line: &str, at: usize) -> Result<(), Failure> {
        let markers = AttributeLine::raw(line.trim());
        if line.starts_with(char::is_whitespace)
            && markers.iter().any(|marker| Self::forbidden(&marker.name))
        {
            return Err(self.failure(at + line.len() - line.trim_start().len()));
        }
        Ok(())
    }

    fn marker_name(name: &str) -> &str {
        name.rsplit('.').next().unwrap_or(name)
    }

    fn marker_offset(&self, name: &str, range: &Range<usize>) -> usize {
        self.code[..range.start].rfind(name).unwrap_or(range.start)
    }

    fn forbidden(name: &str) -> bool {
        Self::FORBIDDEN.contains(&Self::marker_name(name))
    }

    fn require_caller(&self, name: &str, range: &Range<usize>) -> Result<(), Failure> {
        self.code
            .match_indices(name)
            .any(|(at, _)| self.new_call(name, range, at))
            .then_some(())
            .ok_or_else(|| self.failure(range.start))
    }

    fn new_call(&self, name: &str, range: &Range<usize>, at: usize) -> bool {
        let before = &self.code[..at];
        let after = self.code[at + name.len()..].trim_start();
        !range.contains(&at)
            && before.split_whitespace().next_back() == Some("new")
            && after.starts_with('(')
    }

    fn require_dependencies(&self) -> Result<(), Failure> {
        let view = FsharpView::new(self.path, self.source);
        self.classes
            .iter()
            .try_for_each(|(name, range)| self.require_dependency(&view, name, range.start))
    }

    fn require_dependency(&self, view: &FsharpView, name: &str, at: usize) -> Result<(), Failure> {
        let parameters = FsharpConstructorProps::from_shadow(view.text(), name);
        let helpers = self.helpers();
        let requested = parameters.iter().any(|parameter| {
            helpers
                .iter()
                .any(|(helper, _)| Self::type_mentions(&parameter.type_name, helper))
        });
        requested.then(|| self.failure(at)).map_or(Ok(()), Err)
    }

    fn type_mentions(kind: &str, name: &str) -> bool {
        kind.split(|character: char| !character.is_alphanumeric() && character != '_')
            .any(|word| word == name)
    }

    fn failure(&self, at: usize) -> Failure {
        Failure::one(crate::failure::diagnostic(
            1308,
            Self::LIMIT,
            None,
            crate::failure::source_span(&self.path.to_string_lossy(), at, at + 1),
        ))
    }
}
