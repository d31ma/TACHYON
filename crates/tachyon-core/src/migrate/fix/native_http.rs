use super::edits::TextEdit;
use super::scan::SourceText;

struct RequestParameter {
    open: usize,
    close: usize,
    name_at: usize,
    name: String,
}

#[derive(Default)]
pub(super) struct NativeHttpPlan {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) manual: Option<(String, String)>,
}

pub(super) struct NativeHttpFix<'t, 'a> {
    text: &'t SourceText<'a>,
    extension: &'t str,
}

impl<'t, 'a> NativeHttpFix<'t, 'a> {
    const REQUEST: &'static str = "YonRequest";
    const ANSWERS: [&'static str; 3] = ["YonResponse", "YonJSON", "YonEmpty"];
    const LANGUAGES: [&'static str; 7] = ["php", "java", "kt", "cs", "rs", "dart", "go"];
    const MODIFIERS: [&'static str; 4] = ["final", "mut", "var", "const"];

    pub(super) fn plan(
        extension: &'t str,
        text: &'t SourceText<'a>,
        only_notes: bool,
    ) -> NativeHttpPlan {
        let mut plan = NativeHttpPlan::default();
        if !Self::LANGUAGES.contains(&extension) {
            return plan;
        }
        let fix = Self { text, extension };
        let parameters = fix.parameters();
        let mut still_used = fix.answers_used();
        for parameter in &parameters {
            if only_notes || fix.is_read(parameter, &parameters) {
                still_used = true;
                continue;
            }
            plan.edits.push(fix.removal(parameter));
            plan.notes.push(format!(
                "Removed the unused YonRequest parameter '{}' on line {}: the method does not read the request.",
                parameter.name,
                text.line_number(parameter.name_at)
            ));
        }
        if still_used || fix.other_requests(&parameters) {
            plan.manual = Some(fix.manual_step());
        }
        plan
    }

    fn answers_used(&self) -> bool {
        Self::ANSWERS
            .iter()
            .any(|name| !self.text.code_words(name).is_empty())
    }

    fn other_requests(&self, parameters: &[RequestParameter]) -> bool {
        self.text
            .code_words(Self::REQUEST)
            .iter()
            .any(|at| !parameters.iter().any(|parameter| parameter.contains(*at)))
    }

    fn manual_step(&self) -> (String, String) {
        Self::manual_step_for(self.extension)
    }

    pub(super) fn manual_step_for(extension: &str) -> (String, String) {
        let problem = String::from(
            "The file still uses YonRequest or YonResponse, which Tachyon removed (ADR 0062).",
        );
        let action = format!(
            "Read the request as {} and answer with {}.",
            crate::convention::NativeHttp::request(extension),
            crate::convention::NativeHttp::response(extension)
        );
        (problem, action)
    }

    fn parameters(&self) -> Vec<RequestParameter> {
        let mut found = Vec::new();
        for at in self.text.code_words(Self::REQUEST) {
            if let Some(parameter) = self.parameter_at(at) {
                found.push(parameter);
            }
        }
        found
    }

    fn parameter_at(&self, at: usize) -> Option<RequestParameter> {
        let open = self.boundary_before(at)?;
        let close = self.boundary_after(at)?;
        let (name_at, name) = self.parameter_name(open + 1, close)?;
        Some(RequestParameter {
            open,
            close,
            name_at,
            name,
        })
    }

    fn boundary_before(&self, at: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let mut index = at;
        while index > 0 {
            index -= 1;
            if !self.text.is_code(index) {
                continue;
            }
            match self.text.bytes[index] {
                b')' | b'>' | b']' => depth += 1,
                b'(' | b',' if depth == 0 => return Some(index),
                b'(' | b'<' | b'[' => depth = depth.saturating_sub(1),
                b'{' | b'}' | b';' | b'=' => return None,
                _ => {}
            }
        }
        None
    }

    fn boundary_after(&self, at: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for index in at..self.text.len() {
            if !self.text.is_code(index) {
                continue;
            }
            match self.text.bytes[index] {
                b'(' | b'<' | b'[' => depth += 1,
                b')' | b',' if depth == 0 => return Some(index),
                b')' | b'>' | b']' => depth = depth.saturating_sub(1),
                b'{' | b'}' | b';' | b'=' => return None,
                _ => {}
            }
        }
        None
    }

    fn parameter_name(&self, start: usize, end: usize) -> Option<(usize, String)> {
        let mut at = start;
        while at < end {
            if !self.text.starts_word(at) {
                at += 1;
                continue;
            }
            let word_end = self.text.word_end(at);
            let word = &self.text.source[at..word_end];
            if word != Self::REQUEST && !Self::MODIFIERS.contains(&word) {
                return Some((at, String::from(word)));
            }
            at = word_end;
        }
        None
    }

    fn is_read(&self, parameter: &RequestParameter, parameters: &[RequestParameter]) -> bool {
        self.text
            .code_words(&parameter.name)
            .iter()
            .any(|at| !parameters.iter().any(|other| other.contains(*at)))
    }

    fn removal(&self, parameter: &RequestParameter) -> TextEdit {
        let bytes = self.text.bytes;
        if bytes[parameter.close] == b',' {
            let mut end = parameter.close + 1;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            return TextEdit::delete(parameter.open + 1, end);
        }
        if bytes[parameter.open] == b',' {
            return TextEdit::delete(parameter.open, parameter.close);
        }
        TextEdit::delete(parameter.open + 1, parameter.close)
    }
}

impl RequestParameter {
    fn contains(&self, at: usize) -> bool {
        self.open < at && at < self.close
    }
}
