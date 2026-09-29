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
    const GATEWAY: &'static str = "gateway";
    const CONSTRUCTORS: [&'static str; 4] =
        ["constructor", "__construct", "initialize", "__init__"];

    pub(crate) fn sends_request(&self) -> bool {
        self.markers
            .iter()
            .any(|marker| Self::REQUESTS.contains(&marker.as_str()))
    }

    pub(crate) fn is_declared_call(&self) -> bool {
        self.sends_request() || self.markers.iter().any(|marker| marker == Self::GATEWAY)
    }

    pub(crate) fn has_code(&self, code: &str) -> bool {
        self.expression
            || self
                .body
                .as_ref()
                .is_some_and(|body| !Self::is_blank(code, body))
    }

    pub(crate) fn has_block(&self) -> bool {
        self.body.is_some() || self.expression
    }

    pub(crate) fn is_constructor(&self, code: &str) -> bool {
        self.method && Self::CONSTRUCTORS.contains(&self.name(code).as_str())
    }

    pub(crate) fn name(&self, code: &str) -> String {
        let text = &code[self.start..self.end];
        let before = text.split('(').next().unwrap_or(text);
        let before = before.split(['=', ':']).next().unwrap_or(before);
        before
            .split_whitespace()
            .last()
            .unwrap_or("")
            .trim_start_matches(['$', '*', '&'])
            .to_owned()
    }

    fn is_blank(code: &str, body: &Range<usize>) -> bool {
        let text = code[body.clone()].trim();
        text.is_empty() || text == "..." || text == "pass"
    }
}
