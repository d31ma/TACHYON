use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub(crate) struct ContractMember {
    pub(crate) markers: Vec<String>,
    pub(crate) first_marker: Option<usize>,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) terminator: Option<usize>,
    pub(crate) body: Option<Range<usize>>,
    pub(crate) expression: bool,
    pub(crate) method: bool,
}

impl ContractMember {
    const REQUESTS: [&'static str; 7] = [
        "getrequest",
        "postrequest",
        "putrequest",
        "patchrequest",
        "deleterequest",
        "streamrequest",
        "socketrequest",
    ];
    const CONSTRUCTORS: [&'static str; 4] =
        ["constructor", "__construct", "initialize", "__init__"];
    const EMPTY_BODIES: [&'static str; 3] = ["", "...", "pass"];
    const NAME_ENDS: [char; 3] = ['(', '=', ':'];
    const NAME_SIGILS: [char; 3] = ['$', '*', '&'];

    pub(crate) fn sends_request(&self) -> bool {
        Self::marked_with(&self.markers, &Self::REQUESTS)
    }

    pub(crate) fn is_marked_with(&self, names: &[&str]) -> bool {
        Self::marked_with(&self.markers, names)
    }

    pub(crate) fn has_code(&self, code: &str) -> bool {
        let written = self
            .body
            .as_ref()
            .is_some_and(|body| Self::is_written(code, body));
        [self.expression, written].contains(&true)
    }

    pub(crate) fn has_block(&self) -> bool {
        [self.expression, self.body.is_some()].contains(&true)
    }

    pub(crate) fn is_constructor(&self, code: &str) -> bool {
        let named = Self::CONSTRUCTORS.contains(&self.name(code).as_str());
        [self.method, named] == [true, true]
    }

    pub(crate) fn name(&self, code: &str) -> String {
        let text = &code[self.start..self.end];
        let before = text.split(Self::NAME_ENDS).next().unwrap_or(text);
        before
            .split_whitespace()
            .last()
            .unwrap_or("")
            .trim_start_matches(Self::NAME_SIGILS)
            .to_owned()
    }

    fn marked_with(markers: &[String], names: &[&str]) -> bool {
        markers
            .iter()
            .any(|marker| names.contains(&marker.as_str()))
    }

    fn is_written(code: &str, body: &Range<usize>) -> bool {
        !Self::EMPTY_BODIES.contains(&code[body.clone()].trim())
    }
}
