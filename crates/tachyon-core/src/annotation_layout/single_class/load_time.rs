use super::{Failure, Parser};

impl Parser<'_, '_> {
    const CODE_KEYWORDS: [&'static str; 11] = [
        "new", "await", "yield", "delete", "void", "typeof", "function", "class", "import",
        "async", "lambda",
    ];

    fn runs_decorators_when_loaded(&self) -> bool {
        matches!(self.input.extension, "js" | "ts" | "py")
    }

    pub(super) fn check_marker_arguments(&self, open: usize, end: usize) -> Result<(), Failure> {
        if !self.runs_decorators_when_loaded() {
            return Ok(());
        }
        let Some(at) = self.first_code_between(open + 1, end - 1) else {
            return Ok(());
        };
        Err(self.refused(
            at,
            "an annotation argument that runs code when the file loads; use strings, numbers, true, false, null, arrays and objects of those, or a class name",
        ))
    }

    pub(super) fn check_python_bases(&self, open: usize, end: usize) -> Result<(), Failure> {
        let Some(at) = self.first_code_between(open + 1, end - 1) else {
            return Ok(());
        };
        Err(self.refused(
            at,
            "code in the class header that runs when the file loads; name each base class",
        ))
    }

    pub(super) fn check_javascript_header_word(&self) -> Result<(), Failure> {
        let plain_name =
            self.is_name(self.at) && !Self::CODE_KEYWORDS.contains(&self.word(self.at));
        if plain_name || matches!(self.word(self.at), "." | ",") {
            return Ok(());
        }
        Err(self.refused(
            self.at,
            "code in the class header that runs when the file loads; extend a class by its name",
        ))
    }

    pub(super) fn check_class_body(&self, first: usize, end: usize) -> Result<(), Failure> {
        if !self.runs_decorators_when_loaded() {
            return Ok(());
        }
        let javascript = self.input.extension != "py";
        for at in first..end {
            if javascript && self.word(at) == "static" && self.word(at + 1) == "{" {
                return Err(self.refused(
                    at,
                    "a static block, which runs code when the file loads; move its code into a method",
                ));
            }
            if let Some(arguments) = self.tokens.marker_arguments(at) {
                let arguments_end = self
                    .tokens
                    .after_group(arguments)
                    .ok_or_else(|| self.refused(at, "an unclosed annotation"))?;
                self.check_marker_arguments(arguments, arguments_end)?;
            }
        }
        Ok(())
    }

    fn first_code_between(&self, start: usize, end: usize) -> Option<usize> {
        (start..end).find(|at| !self.is_literal_argument_token(*at))
    }

    fn is_literal_argument_token(&self, at: usize) -> bool {
        let token = &self.tokens.items[at];
        if token.literal {
            return !(token.text.starts_with('`') && token.text.contains("${"));
        }
        if token.is_name() {
            return !Self::CODE_KEYWORDS.contains(&token.text);
        }
        if token.text.as_bytes()[0].is_ascii_digit() {
            return true;
        }
        if token.text == "=" {
            return self.input.extension == "py";
        }
        matches!(token.text, "." | "," | ":" | "-" | "[" | "]" | "{" | "}")
    }
}
