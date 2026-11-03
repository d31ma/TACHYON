use super::name_advice::NameAdvice;
use crate::application::NameRefusal;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RefusedName {
    pub(super) line_start: usize,
    pub(super) line_end: usize,
    pub(super) name_start: usize,
    pub(super) name: String,
    pub(super) line: usize,
    pub(super) advice: NameAdvice,
}

impl RefusedName {
    pub(super) fn read(raw: &str, line_start: usize, line: usize) -> Option<Self> {
        let content = raw.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        let (written, _) = Self::entry(trimmed)?;
        let name = written.trim_end();
        let refusal = NameRefusal::of(name)?;
        Some(Self {
            line_start,
            line_end: line_start + content.len(),
            name_start: line_start + content.len() - trimmed.len(),
            name: String::from(name),
            line,
            advice: NameAdvice::of(name, refusal),
        })
    }

    pub(super) fn name_end(&self) -> usize {
        self.name_start + self.name.len()
    }

    fn entry(trimmed: &str) -> Option<(&str, &str)> {
        (!trimmed.starts_with('#')).then_some(())?;
        trimmed.split_once('=')
    }
}
