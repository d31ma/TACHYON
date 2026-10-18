use super::{AttributeLine, FsharpConstructorProps, FsharpView, LineKind, TypeHeader};
use crate::Failure;
use crate::helper_new_call::HelperNewCall;
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
    const NEEDS: &'static str = "Needs";
    const PROPERTY: &'static str = "member val ";
    const HELPER_SUFFIX: &'static str = "Helper";
    const LIMIT: &'static str = "An F# Tac Helper needs an owner: a caller-owned `new` call outside its own class, or a primary constructor parameter or [<Needs>] property of the class that needs it, which makes a new Helper for that class. A Helper has an ordinary primary constructor, its [<Needs>] properties name other Helpers, and it takes no NeedsAll, Lifetime, OnStart, OnStop or OnEvent.";

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
        for line in code.split_inclusive('\n') {
            Self::record_class(
                &mut classes,
                line.trim_end_matches(['\r', '\n']),
                offset,
                code.len(),
            );
            offset += line.len();
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
        let lines: Vec<&str> = self.code[range.clone()].split_inclusive('\n').collect();
        for (at, line) in lines.iter().enumerate() {
            let text = line.trim_end_matches(['\r', '\n']);
            let next = lines.get(at + 1).copied().unwrap_or("");
            self.require_member_marker(text, next, offset)?;
            self.require_primary_only(text, offset)?;
            offset += line.len();
        }
        Ok(())
    }

    fn require_member_marker(&self, line: &str, next: &str, at: usize) -> Result<(), Failure> {
        let refused = AttributeLine::raw(line.trim()).iter().any(|marker| {
            [
                Self::forbidden(&marker.name),
                Self::needed_helper(marker, next),
            ] == [true, false]
        });
        if [line.starts_with(char::is_whitespace), refused] == [true, true] {
            return Err(self.failure(at + line.len() - line.trim_start().len()));
        }
        Ok(())
    }

    fn needed_helper(marker: &AttributeLine, next: &str) -> bool {
        let property = next
            .trim()
            .strip_prefix(Self::PROPERTY)
            .and_then(|declared| declared.split_once(':'))
            .and_then(|(_, rest)| rest.split_once('='))
            .map(|(type_name, _)| type_name.trim());
        [
            Self::marker_name(&marker.name) == Self::NEEDS,
            property.is_some_and(|type_name| type_name.ends_with(Self::HELPER_SUFFIX)),
        ] == [true, true]
    }

    fn require_primary_only(&self, line: &str, at: usize) -> Result<(), Failure> {
        let secondary = line
            .trim_start()
            .strip_prefix("new")
            .is_some_and(|rest| rest.trim_start().starts_with('('));
        secondary.then(|| self.failure(at)).map_or(Ok(()), Err)
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
        !range.contains(&at)
            && HelperNewCall::matches(&self.code[..at], &self.code[at + name.len()..])
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
