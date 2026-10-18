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
    request: &'static str,
}

impl<'t, 'a> NativeHttpFix<'t, 'a> {
    const REQUEST: &'static str = "YonRequest";
    const JSON: &'static str = "YonJson";
    const ANSWERS: [&'static str; 3] = ["YonResponse", "YonJSON", "YonEmpty"];
    const LANGUAGES: [&'static str; 8] = ["php", "java", "kt", "cs", "rs", "dart", "go", "cpp"];
    const MODIFIERS: [&'static str; 4] = ["final", "mut", "var", "const"];
    const OWN_JSON: [(&'static str, &'static str, &'static str); 1] = [(
        "cpp",
        "The file still uses YonJson, the JSON type of the old C++ runner, which Tachyon removed (ADR 0099).",
        "Read the body with a [[FromBody]] parameter of a model struct or tachyon::json::JsonNode, or take `const tachyon::http::HttpRequestMessage& request`. Return a model struct or tachyon::json::JsonNode.",
    )];

    pub(super) fn plan(
        extension: &'t str,
        text: &'t SourceText<'a>,
        only_notes: bool,
    ) -> NativeHttpPlan {
        let request = Self::OWN_JSON
            .iter()
            .find(|(written, _, _)| *written == extension)
            .map_or(Self::REQUEST, |_| Self::JSON);
        let fix = Self {
            text,
            extension,
            request,
        };
        fix.supported_plan(only_notes)
    }

    fn supported_plan(&self, only_notes: bool) -> NativeHttpPlan {
        if !Self::LANGUAGES.contains(&self.extension) {
            return NativeHttpPlan::default();
        }
        self.planned(only_notes)
    }

    fn planned(&self, only_notes: bool) -> NativeHttpPlan {
        let parameters = self.parameters();
        let (kept, unused): (Vec<&RequestParameter>, Vec<&RequestParameter>) = parameters
            .iter()
            .partition(|parameter| self.keeps(parameter, &parameters, only_notes));
        let edits = unused
            .iter()
            .map(|parameter| self.removal(parameter))
            .collect();
        let notes = unused
            .iter()
            .map(|parameter| self.removal_note(parameter))
            .collect();
        let headers = self
            .text
            .source
            .contains(&format!("\"{}.hpp\"", self.request));
        let still_used = [
            self.answers_used(),
            !kept.is_empty(),
            self.other_requests(&parameters),
            headers,
        ];
        let manual = still_used.contains(&true).then(|| self.manual_step());
        NativeHttpPlan {
            edits,
            notes,
            manual,
        }
    }

    fn keeps(
        &self,
        parameter: &RequestParameter,
        parameters: &[RequestParameter],
        only_notes: bool,
    ) -> bool {
        [only_notes, self.is_read(parameter, parameters)].contains(&true)
    }

    fn removal_note(&self, parameter: &RequestParameter) -> String {
        format!(
            "Removed the unused {} parameter '{}' on line {}: the method does not read the request.",
            self.request,
            parameter.name,
            self.text.line_number(parameter.name_at)
        )
    }

    fn answers_used(&self) -> bool {
        Self::ANSWERS
            .iter()
            .any(|name| !self.text.code_words(name).is_empty())
    }

    fn other_requests(&self, parameters: &[RequestParameter]) -> bool {
        self.text
            .code_words(self.request)
            .iter()
            .any(|at| !parameters.iter().any(|parameter| parameter.contains(*at)))
    }

    fn manual_step(&self) -> (String, String) {
        Self::manual_step_for(self.extension)
    }

    pub(super) fn manual_step_for(extension: &str) -> (String, String) {
        let own = Self::OWN_JSON
            .iter()
            .find(|(written, _, _)| *written == extension);
        own.map_or_else(
            || Self::removed_types_step(extension),
            |(_, problem, action)| (String::from(*problem), String::from(*action)),
        )
    }

    fn removed_types_step(extension: &str) -> (String, String) {
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
        self.text
            .code_words(self.request)
            .into_iter()
            .filter_map(|at| self.parameter_at(at))
            .collect()
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
        let source = self.text.source;
        let found = (start..end)
            .filter(|at| self.text.starts_word(*at))
            .map(|at| (at, &source[at..self.text.word_end(at)]))
            .find(|(_, word)| self.is_parameter_name(word));
        found.map(|(at, word)| (at, String::from(word)))
    }

    fn is_parameter_name(&self, word: &str) -> bool {
        [word == self.request, Self::MODIFIERS.contains(&word)] == [false, false]
    }

    fn is_read(&self, parameter: &RequestParameter, parameters: &[RequestParameter]) -> bool {
        self.text
            .code_words(&parameter.name)
            .iter()
            .any(|at| !parameters.iter().any(|other| other.contains(*at)))
    }

    fn removal(&self, parameter: &RequestParameter) -> TextEdit {
        let bytes = self.text.bytes;
        match (bytes[parameter.open], bytes[parameter.close]) {
            (_, b',') => {
                TextEdit::delete(parameter.open + 1, self.after_spaces(parameter.close + 1))
            }
            (b',', _) => TextEdit::delete(parameter.open, parameter.close),
            _ => TextEdit::delete(parameter.open + 1, parameter.close),
        }
    }

    fn after_spaces(&self, from: usize) -> usize {
        let spaces = self.text.bytes[from..]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        from + spaces
    }
}

impl RequestParameter {
    fn contains(&self, at: usize) -> bool {
        [self.open < at, at < self.close] == [true, true]
    }
}
