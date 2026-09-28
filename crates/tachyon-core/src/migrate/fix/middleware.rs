use super::edits::TextEdit;
use super::scan::SourceText;
use std::path::Path;

pub(super) enum MiddlewarePlan {
    Unchanged,
    Converted {
        edits: Vec<TextEdit>,
        notes: Vec<String>,
    },
    Manual {
        problem: String,
        action: String,
    },
}

impl MiddlewarePlan {
    const MANUAL_ACTION: &str = "Rename the class to Middleware. The old runtime called Handler.<HTTP method> both before and after the route, and the request operation ('middleware.before' or 'middleware.after') named the phase. Move the code that runs before the route into before(request) and the code that runs after it into after(request), then delete the HTTP method functions. Tachyon calls both for every request, whatever its HTTP method. A missing before or after lets the request pass.";

    pub(super) fn manual(reason: &str) -> Self {
        Self::Manual {
            problem: format!(
                "Root middleware declares class Handler with one function per HTTP method; {reason}, so it was not converted automatically."
            ),
            action: String::from(Self::MANUAL_ACTION),
        }
    }
}

struct Member {
    start: usize,
    name_start: usize,
    name_end: usize,
    end: usize,
}

pub(super) struct MiddlewareConversion<'t, 'a> {
    path: &'t Path,
    text: &'t SourceText<'a>,
}

impl<'t> MiddlewareConversion<'t, '_> {
    const OLD_NAME: &'static str = "Handler";
    const NEW_NAME: &'static str = "Middleware";
    const HTTP_METHODS: [&'static str; 7] =
        ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
    const LANGUAGES: [&'static str; 9] =
        ["js", "ts", "py", "php", "java", "kt", "cs", "rs", "dart"];

    pub(super) fn plan(relative: &str, source: &str) -> MiddlewarePlan {
        let path = Path::new(relative);
        let Ok(names) = crate::annotation_layout::declaration_names(path, source) else {
            return MiddlewarePlan::Unchanged;
        };
        if !names.iter().any(|name| name == Self::OLD_NAME) {
            return MiddlewarePlan::Unchanged;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !Self::LANGUAGES.contains(&extension) {
            return MiddlewarePlan::Unchanged;
        }
        if names.len() != 1 {
            return MiddlewarePlan::manual("the file declares more than one type");
        }
        let Some(text) = SourceText::read(extension, source) else {
            return MiddlewarePlan::manual("the file has an unfinished string or comment");
        };
        let conversion = MiddlewareConversion { path, text: &text };
        match conversion.convert() {
            Ok(plan) => plan,
            Err(reason) => MiddlewarePlan::manual(reason),
        }
    }

    fn convert(&self) -> Result<MiddlewarePlan, &'static str> {
        if !self.text.code_words(Self::NEW_NAME).is_empty() {
            return Err("the file already uses the name Middleware");
        }
        let methods = self.http_methods()?;
        let Some(first) = methods.first() else {
            return Err("class Handler has no function named after an HTTP method");
        };
        let first_code = self.code_without_name(first);
        for other in &methods[1..] {
            if self.code_without_name(other) != first_code {
                return Err("its HTTP method functions hold different code");
            }
        }
        let edits = self.edits(&methods);
        if !TextEdit::are_separate(&edits) {
            return Err("its changes would overlap");
        }
        let converted = TextEdit::apply(self.text.source, &edits);
        self.verify(&converted)?;
        Ok(MiddlewarePlan::Converted {
            notes: self.notes(&methods),
            edits,
        })
    }

    fn edits(&self, methods: &[Member]) -> Vec<TextEdit> {
        let first = &methods[0];
        let mut edits = vec![TextEdit::replace(
            first.name_start,
            first.name_end,
            "before",
        )];
        let mut copy = String::from("\n");
        if !self.text.source[..first.end].ends_with('\n') {
            copy.push('\n');
        }
        copy.push_str(&self.renamed_copy(first, "after"));
        edits.push(TextEdit::insert(first.end, &copy));
        let mut deleted = Vec::new();
        for other in &methods[1..] {
            deleted.push((self.removal_start(other), other.end));
        }
        for (start, end) in &deleted {
            edits.push(TextEdit::delete(*start, *end));
        }
        for at in self.text.code_words(Self::OLD_NAME) {
            let inside_deleted = deleted.iter().any(|(start, end)| *start <= at && at < *end);
            if !inside_deleted {
                edits.push(TextEdit::replace(
                    at,
                    at + Self::OLD_NAME.len(),
                    Self::NEW_NAME,
                ));
            }
        }
        edits
    }

    fn renamed_copy(&self, member: &Member, name: &str) -> String {
        let mut edits = vec![TextEdit::replace(
            member.name_start - member.start,
            member.name_end - member.start,
            name,
        )];
        for at in self.text.code_words(Self::OLD_NAME) {
            if member.start <= at && at < member.end {
                let start = at - member.start;
                edits.push(TextEdit::replace(
                    start,
                    start + Self::OLD_NAME.len(),
                    Self::NEW_NAME,
                ));
            }
        }
        TextEdit::apply(&self.text.source[member.start..member.end], &edits)
    }

    fn removal_start(&self, member: &Member) -> usize {
        match self.text.previous_line_start(member.start) {
            Some(previous) if self.text.is_blank_line(previous) => previous,
            _ => member.start,
        }
    }

    fn code_without_name(&self, member: &Member) -> String {
        let source = self.text.source;
        format!(
            "{}{}",
            &source[member.start..member.name_start],
            &source[member.name_end..member.end]
        )
    }

    fn notes(&self, methods: &[Member]) -> Vec<String> {
        let source = self.text.source;
        let first = &source[methods[0].name_start..methods[0].name_end];
        let mut notes = vec![format!(
            "Renamed class Handler to Middleware. Its {first} function became before(request) and after(request), which run the same code as {first} did in both phases."
        )];
        if methods.len() > 1 {
            let mut others = Vec::new();
            for member in &methods[1..] {
                others.push(&source[member.name_start..member.name_end]);
            }
            notes.push(format!(
                "Deleted {}, which held the same code as {first}.",
                others.join(", ")
            ));
        }
        notes.push(String::from(
            "Tachyon now runs this code for every HTTP method; before, a method the class did not define answered 405.",
        ));
        if source.contains("operation") {
            notes.push(String::from(
                "The code still reads the request operation to tell the phases apart. This still works, and you can now remove that check.",
            ));
        }
        notes
    }

    fn verify(&self, converted: &str) -> Result<(), &'static str> {
        let names = crate::annotation_layout::declaration_names(self.path, converted);
        if names.ok() != Some(vec![String::from(Self::NEW_NAME)]) {
            return Err("the converted file would not declare exactly one class Middleware");
        }
        if crate::annotation_layout::has_primary_role(self.path, converted).is_err() {
            return Err("the converted file would not pass the one-class check");
        }
        let Some(text) = SourceText::read(self.extension(), converted) else {
            return Err("the converted file would not scan cleanly");
        };
        for method in Self::HTTP_METHODS {
            if !text.code_words(method).is_empty() {
                return Err("other code refers to an HTTP method function by name");
            }
        }
        Ok(())
    }

    fn extension(&self) -> &'t str {
        self.path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    }

    fn http_methods(&self) -> Result<Vec<Member>, &'static str> {
        if self.extension() == "py" {
            return self.python_methods();
        }
        let (open, close) = self.class_body()?;
        let text = self.text;
        let mut methods = Vec::new();
        let mut depth = 0_usize;
        let mut at = open + 1;
        while at < close {
            if !text.is_code(at) {
                at += 1;
                continue;
            }
            match text.bytes[at] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0 && text.starts_word(at) {
                let end = text.word_end(at);
                if self.is_method_name(at, end) {
                    let member = self.braced_member(at, end)?;
                    at = member.end;
                    methods.push(member);
                    continue;
                }
                at = end;
                continue;
            }
            at += 1;
        }
        Ok(methods)
    }

    fn is_method_name(&self, start: usize, end: usize) -> bool {
        let word = &self.text.source[start..end];
        let opens_parameters = self
            .text
            .next_code(end)
            .is_some_and(|next| self.text.bytes[next] == b'(');
        Self::HTTP_METHODS.contains(&word) && opens_parameters
    }

    fn class_body(&self) -> Result<(usize, usize), &'static str> {
        let text = self.text;
        let mut bodies = Vec::new();
        for at in text.code_words(Self::OLD_NAME) {
            let before = text.previous_word(at);
            let after = at + Self::OLD_NAME.len();
            let opens_body = text
                .next_code(after)
                .is_some_and(|next| text.bytes[next] == b'{');
            let owns_body = match before {
                "class" | "object" => true,
                "impl" => opens_body,
                "for" => return Err("it implements a trait for Handler"),
                _ => false,
            };
            if owns_body {
                bodies.push(after);
            }
        }
        if bodies.len() != 1 {
            return Err("the class body could not be found exactly once");
        }
        let mut at = bodies[0];
        while at < text.len() && !(text.is_code(at) && text.bytes[at] == b'{') {
            at += 1;
        }
        if at == text.len() {
            return Err("the class body has no opening brace");
        }
        let Some(close) = text.matching(at) else {
            return Err("the class body has no closing brace");
        };
        Ok((at, close))
    }

    fn braced_member(&self, name_start: usize, name_end: usize) -> Result<Member, &'static str> {
        let text = self.text;
        let line_start = text.line_start(name_start);
        let prefix = &text.source[line_start..name_start];
        let prefix_is_code = (line_start..name_start).all(|at| text.is_code(at));
        if !prefix_is_code || prefix.contains(['=', '(', '.', ';', '{', '}']) {
            return Err("a function named after an HTTP method has an unusual declaration");
        }
        let Some(open) = text.next_code(name_end) else {
            return Err("a function named after an HTTP method has no parameters");
        };
        let Some(close) = text.matching(open) else {
            return Err("a function named after an HTTP method has unclosed parameters");
        };
        let body_end = self.body_end(close + 1, line_start)?;
        let end = self.after_line(body_end)?;
        self.balanced_member(self.attribute_start(line_start), name_start, name_end, end)
    }

    fn balanced_member(
        &self,
        start: usize,
        name_start: usize,
        name_end: usize,
        end: usize,
    ) -> Result<Member, &'static str> {
        if !self.text.is_balanced(start, end) {
            return Err(
                "a function named after an HTTP method could not be separated from the code around it",
            );
        }
        Ok(Member {
            start,
            name_start,
            name_end,
            end,
        })
    }

    fn body_end(&self, from: usize, line_start: usize) -> Result<usize, &'static str> {
        let text = self.text;
        let unclear =
            "a function named after an HTTP method has a body that is not changed automatically";
        let mut at = from;
        while at < text.len() {
            if !text.is_code(at) {
                at += 1;
                continue;
            }
            let next = text.bytes.get(at + 1).copied();
            match text.bytes[at] {
                b'{' => return text.matching(at).map(|close| close + 1).ok_or(unclear),
                b'=' if next == Some(b'>') => return self.statement_end(at + 2).ok_or(unclear),
                b'=' if self.extension() == "kt" => {
                    let indentation = text.indentation(line_start);
                    return Ok(self.indented_block_end(at, indentation));
                }
                b'=' | b';' => return Err(unclear),
                _ => at += 1,
            }
        }
        Err(unclear)
    }

    fn statement_end(&self, from: usize) -> Option<usize> {
        let text = self.text;
        let mut depth = 0_i64;
        for at in from..text.len() {
            if !text.is_code(at) {
                continue;
            }
            match text.bytes[at] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b';' if depth == 0 => return Some(at + 1),
                _ => {}
            }
            if depth < 0 {
                return None;
            }
        }
        None
    }

    fn indented_block_end(&self, from: usize, indentation: usize) -> usize {
        let text = self.text;
        let mut end = text.line_end(from);
        let mut line = end;
        while line < text.len() {
            let next = text.line_end(line);
            if text.is_blank_line(line) {
                line = next;
                continue;
            }
            if self.ends_block(line, indentation) {
                break;
            }
            end = next;
            line = next;
        }
        end
    }

    fn ends_block(&self, line: usize, indentation: usize) -> bool {
        let text = self.text;
        let first = line + text.indentation(line);
        let rest = &text.source[first..];
        let is_comment = rest.starts_with('#') || rest.starts_with("//");
        let is_code_or_comment = text.is_code(first) || is_comment;
        is_code_or_comment && text.indentation(line) <= indentation
    }

    fn after_line(&self, end: usize) -> Result<usize, &'static str> {
        let text = self.text;
        if end > 0 && text.bytes[end - 1] == b'\n' {
            return Ok(end);
        }
        let mut at = end;
        while matches!(text.bytes.get(at), Some(b' ' | b'\t' | b'\r')) {
            at += 1;
        }
        match text.bytes.get(at) {
            Some(b'\n') => Ok(at + 1),
            None => Ok(at),
            Some(_) => Err("code follows a function named after an HTTP method on the same line"),
        }
    }

    fn attribute_start(&self, line_start: usize) -> usize {
        let text = self.text;
        let mut start = line_start;
        while let Some(previous) = text.previous_line_start(start) {
            let line = text.line_text(previous).trim_start();
            let is_attribute = line.starts_with('@')
                || line.starts_with("#[")
                || (self.extension() == "cs" && line.starts_with('['));
            if !is_attribute {
                break;
            }
            start = previous;
        }
        start
    }

    fn python_methods(&self) -> Result<Vec<Member>, &'static str> {
        let text = self.text;
        let (body_start, body_end) = self.python_class_body()?;
        let Some(first_line) = self.first_code_line(body_start, body_end) else {
            return Err("the class body is empty");
        };
        let indentation = text.indentation(first_line);
        let mut methods = Vec::new();
        let mut line = body_start;
        while line < body_end {
            let next = text.line_end(line);
            let Some((name_start, name_end)) = self.python_method_name(line, indentation) else {
                line = next;
                continue;
            };
            let end = self.indented_block_end(name_end, indentation);
            let start = self.attribute_start(line);
            methods.push(self.balanced_member(start, name_start, name_end, end)?);
            line = end;
        }
        Ok(methods)
    }

    fn python_class_body(&self) -> Result<(usize, usize), &'static str> {
        let text = self.text;
        let mut classes = Vec::new();
        for at in text.code_words(Self::OLD_NAME) {
            if text.previous_word(at) == "class" {
                classes.push(at);
            }
        }
        if classes.len() != 1 {
            return Err("the class could not be found exactly once");
        }
        let class_line = text.line_start(classes[0]);
        let indentation = text.indentation(class_line);
        let body_start = text.line_end(classes[0]);
        Ok((body_start, self.indented_block_end(classes[0], indentation)))
    }

    fn first_code_line(&self, start: usize, end: usize) -> Option<usize> {
        let text = self.text;
        let mut line = start;
        while line < end {
            let first = line + text.indentation(line);
            if !text.is_blank_line(line) && text.is_code(first) {
                return Some(line);
            }
            line = text.line_end(line);
        }
        None
    }

    fn python_method_name(&self, line: usize, indentation: usize) -> Option<(usize, usize)> {
        let text = self.text;
        if text.is_blank_line(line) || text.indentation(line) != indentation {
            return None;
        }
        let first = line + indentation;
        let rest = text.line_text(line).trim_start();
        let after_def = rest
            .strip_prefix("def ")
            .or_else(|| rest.strip_prefix("async def "))?;
        let name_start = first + rest.len() - after_def.len();
        let name_start = text.next_code(name_start)?;
        let name_end = text.word_end(name_start);
        let is_method = text.is_code(first) && self.is_method_name(name_start, name_end);
        is_method.then_some((name_start, name_end))
    }
}
