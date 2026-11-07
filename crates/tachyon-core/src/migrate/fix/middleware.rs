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
    const UNCLEAR_BODY: &'static str =
        "a function named after an HTTP method has a body that is not changed automatically";
    const BODY_MARKS: &'static [u8] = b"{=;";
    const BLANKS: &'static [u8] = b" \t\r";

    pub(super) fn plan(relative: &str, source: &str) -> MiddlewarePlan {
        let path = Path::new(relative);
        let names = crate::annotation_layout::declaration_names(path, source).unwrap_or_default();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self::candidate_plan(path, extension, source, &names)
    }

    fn candidate_plan(
        path: &Path,
        extension: &str,
        source: &str,
        names: &[String],
    ) -> MiddlewarePlan {
        let declares_handler = names.iter().any(|name| name == Self::OLD_NAME);
        let supported = Self::LANGUAGES.contains(&extension);
        if [declares_handler, supported] != [true, true] {
            return MiddlewarePlan::Unchanged;
        }
        let converted = Self::converted(path, extension, source, names);
        converted.unwrap_or_else(MiddlewarePlan::manual)
    }

    fn converted(
        path: &Path,
        extension: &str,
        source: &str,
        names: &[String],
    ) -> Result<MiddlewarePlan, &'static str> {
        Self::require(names.len() == 1, "the file declares more than one type")?;
        let text = SourceText::read(extension, source)
            .ok_or("the file has an unfinished string or comment")?;
        let conversion = MiddlewareConversion { path, text: &text };
        conversion.convert()
    }

    fn require(condition: bool, reason: &'static str) -> Result<(), &'static str> {
        condition.then_some(()).ok_or(reason)
    }

    fn convert(&self) -> Result<MiddlewarePlan, &'static str> {
        let unused_name = self.text.code_words(Self::NEW_NAME).is_empty();
        Self::require(unused_name, "the file already uses the name Middleware")?;
        let methods = self.http_methods()?;
        let first = methods
            .first()
            .ok_or("class Handler has no function named after an HTTP method")?;
        let first_code = self.code_without_name(first);
        let same_code = methods[1..]
            .iter()
            .all(|other| self.code_without_name(other) == first_code);
        Self::require(same_code, "its HTTP method functions hold different code")?;
        let edits = self.edits(&methods);
        Self::require(TextEdit::are_separate(&edits), "its changes would overlap")?;
        let converted = TextEdit::apply(self.text.source, &edits);
        self.verify(&converted)?;
        Ok(MiddlewarePlan::Converted {
            notes: self.notes(&methods),
            edits,
        })
    }

    fn edits(&self, methods: &[Member]) -> Vec<TextEdit> {
        let first = &methods[0];
        let mut edits = vec![
            TextEdit::replace(first.name_start, first.name_end, "before"),
            TextEdit::insert(first.end, &self.after_copy(first)),
        ];
        let deleted: Vec<(usize, usize)> = methods[1..]
            .iter()
            .map(|other| (self.removal_start(other), other.end))
            .collect();
        edits.extend(
            deleted
                .iter()
                .map(|(start, end)| TextEdit::delete(*start, *end)),
        );
        let renamed = self
            .text
            .code_words(Self::OLD_NAME)
            .into_iter()
            .filter(|at| {
                !deleted
                    .iter()
                    .any(|(start, end)| (*start..*end).contains(at))
            })
            .map(|at| TextEdit::replace(at, at + Self::OLD_NAME.len(), Self::NEW_NAME));
        edits.extend(renamed);
        edits
    }

    fn after_copy(&self, first: &Member) -> String {
        let mut copy = String::from("\n");
        if !self.text.source[..first.end].ends_with('\n') {
            copy.push('\n');
        }
        copy.push_str(&self.renamed_copy(first, "after"));
        copy
    }

    fn renamed_copy(&self, member: &Member, name: &str) -> String {
        let mut edits = vec![TextEdit::replace(
            member.name_start - member.start,
            member.name_end - member.start,
            name,
        )];
        let inside = self
            .text
            .code_words(Self::OLD_NAME)
            .into_iter()
            .filter(|at| (member.start..member.end).contains(at));
        let renamed = inside.map(|at| {
            let start = at - member.start;
            TextEdit::replace(start, start + Self::OLD_NAME.len(), Self::NEW_NAME)
        });
        edits.extend(renamed);
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
        notes.extend(self.deleted_note(methods, first));
        notes.push(String::from(
            "Tachyon now runs this code for every HTTP method; before, a method the class did not define answered 405.",
        ));
        let reads_operation = source.contains("operation");
        notes.extend(reads_operation.then(|| {
            String::from(
                "The code still reads the request operation to tell the phases apart. This still works, and you can now remove that check.",
            )
        }));
        notes
    }

    fn deleted_note(&self, methods: &[Member], first: &str) -> Option<String> {
        let source = self.text.source;
        let others: Vec<&str> = methods[1..]
            .iter()
            .map(|member| &source[member.name_start..member.name_end])
            .collect();
        (!others.is_empty()).then(|| {
            format!(
                "Deleted {}, which held the same code as {first}.",
                others.join(", ")
            )
        })
    }

    fn verify(&self, converted: &str) -> Result<(), &'static str> {
        let names = crate::annotation_layout::declaration_names(self.path, converted);
        Self::require(
            names.ok() == Some(vec![String::from(Self::NEW_NAME)]),
            "the converted file would not declare exactly one class Middleware",
        )?;
        let one_class = crate::annotation_layout::has_primary_role(self.path, converted).is_ok();
        Self::require(
            one_class,
            "the converted file would not pass the one-class check",
        )?;
        let text = SourceText::read(self.extension(), converted)
            .ok_or("the converted file would not scan cleanly")?;
        let unnamed = Self::HTTP_METHODS
            .iter()
            .all(|method| text.code_words(method).is_empty());
        Self::require(
            unnamed,
            "other code refers to an HTTP method function by name",
        )
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
        self.braced_methods()
    }

    fn braced_methods(&self) -> Result<Vec<Member>, &'static str> {
        let (open, close) = self.class_body()?;
        let mut methods = Vec::new();
        let mut depth = 0_usize;
        let mut at = open + 1;
        while at < close {
            depth = self.depth_at(at, depth);
            at = self.after_position(at, depth, &mut methods)?;
        }
        Ok(methods)
    }

    fn depth_at(&self, at: usize, depth: usize) -> usize {
        match (self.text.is_code(at), self.text.bytes[at]) {
            (true, b'{') => depth + 1,
            (true, b'}') => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn after_position(
        &self,
        at: usize,
        depth: usize,
        methods: &mut Vec<Member>,
    ) -> Result<usize, &'static str> {
        let member_start = [self.text.is_code(at), depth == 0, self.text.starts_word(at)];
        if member_start != [true; 3] {
            return Ok(at + 1);
        }
        self.word_or_member(at, methods)
    }

    fn word_or_member(&self, at: usize, methods: &mut Vec<Member>) -> Result<usize, &'static str> {
        let end = self.text.word_end(at);
        let is_method = self.is_method_name(at, end);
        let member = is_method.then(|| self.braced_member(at, end)).transpose()?;
        let next = member.as_ref().map_or(end, |member| member.end);
        methods.extend(member);
        Ok(next)
    }

    fn is_method_name(&self, start: usize, end: usize) -> bool {
        let word = &self.text.source[start..end];
        let opens_parameters = self
            .text
            .next_code(end)
            .is_some_and(|next| self.text.bytes[next] == b'(');
        [Self::HTTP_METHODS.contains(&word), opens_parameters] == [true, true]
    }

    fn class_body(&self) -> Result<(usize, usize), &'static str> {
        let bodies = self.class_bodies()?;
        let body = bodies
            .first()
            .filter(|_| bodies.len() == 1)
            .copied()
            .ok_or("the class body could not be found exactly once")?;
        let open = (body..self.text.len())
            .find(|at| self.is_code_byte(*at, b'{'))
            .ok_or("the class body has no opening brace")?;
        let close = self
            .text
            .matching(open)
            .ok_or("the class body has no closing brace")?;
        Ok((open, close))
    }

    fn is_code_byte(&self, at: usize, byte: u8) -> bool {
        [self.text.is_code(at), self.text.bytes[at] == byte] == [true, true]
    }

    fn class_bodies(&self) -> Result<Vec<usize>, &'static str> {
        let mut bodies = Vec::new();
        for at in self.text.code_words(Self::OLD_NAME) {
            bodies.extend(self.owned_body(at)?);
        }
        Ok(bodies)
    }

    fn owned_body(&self, at: usize) -> Result<Option<usize>, &'static str> {
        let text = self.text;
        let after = at + Self::OLD_NAME.len();
        let opens_body = text
            .next_code(after)
            .is_some_and(|next| text.bytes[next] == b'{');
        let owns_body = match text.previous_word(at) {
            "class" | "object" => true,
            "impl" => opens_body,
            "for" => return Err("it implements a trait for Handler"),
            _ => false,
        };
        Ok(owns_body.then_some(after))
    }

    fn braced_member(&self, name_start: usize, name_end: usize) -> Result<Member, &'static str> {
        let text = self.text;
        let line_start = text.line_start(name_start);
        let prefix = &text.source[line_start..name_start];
        let prefix_is_code = (line_start..name_start).all(|at| text.is_code(at));
        let plain_prefix = !prefix.contains(['=', '(', '.', ';', '{', '}']);
        Self::require(
            [prefix_is_code, plain_prefix] == [true, true],
            "a function named after an HTTP method has an unusual declaration",
        )?;
        let open = text
            .next_code(name_end)
            .ok_or("a function named after an HTTP method has no parameters")?;
        let close = text
            .matching(open)
            .ok_or("a function named after an HTTP method has unclosed parameters")?;
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
        Self::require(
            self.text.is_balanced(start, end),
            "a function named after an HTTP method could not be separated from the code around it",
        )?;
        Ok(Member {
            start,
            name_start,
            name_end,
            end,
        })
    }

    fn body_end(&self, from: usize, line_start: usize) -> Result<usize, &'static str> {
        let text = self.text;
        let mark = (from..text.len())
            .find(|at| self.is_body_mark(*at))
            .ok_or(Self::UNCLEAR_BODY)?;
        let next = text.bytes.get(mark + 1).copied();
        match text.bytes[mark] {
            b'{' => text
                .matching(mark)
                .map(|close| close + 1)
                .ok_or(Self::UNCLEAR_BODY),
            b'=' if next == Some(b'>') => self.arrow_body_end(mark),
            b'=' if self.extension() == "kt" => {
                Ok(self.indented_block_end(mark, text.indentation(line_start)))
            }
            _ => Err(Self::UNCLEAR_BODY),
        }
    }

    fn is_body_mark(&self, at: usize) -> bool {
        let mark = Self::BODY_MARKS.contains(&self.text.bytes[at]);
        [self.text.is_code(at), mark] == [true, true]
    }

    fn arrow_body_end(&self, arrow: usize) -> Result<usize, &'static str> {
        self.statement_end(arrow + 2).ok_or(Self::UNCLEAR_BODY)
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
        let first_end = text.line_end(from);
        let lines = text.line_starts(first_end).into_iter();
        let content = lines.filter(|line| !text.is_blank_line(*line));
        let inside = content.take_while(|line| !self.ends_block(*line, indentation));
        inside.last().map_or(first_end, |line| text.line_end(line))
    }

    fn ends_block(&self, line: usize, indentation: usize) -> bool {
        let text = self.text;
        let first = line + text.indentation(line);
        let rest = &text.source[first..];
        let is_comment = [rest.starts_with('#'), rest.starts_with("//")].contains(&true);
        let is_code_or_comment = [text.is_code(first), is_comment].contains(&true);
        [is_code_or_comment, text.indentation(line) <= indentation] == [true, true]
    }

    fn after_line(&self, end: usize) -> Result<usize, &'static str> {
        let text = self.text;
        let ends_line = end
            .checked_sub(1)
            .is_some_and(|before| text.bytes[before] == b'\n');
        let rest = text.bytes.get(end..).unwrap_or_default();
        let blanks = rest
            .iter()
            .take_while(|byte| Self::BLANKS.contains(byte))
            .count();
        let at = end + blanks;
        match (ends_line, text.bytes.get(at)) {
            (true, _) => Ok(end),
            (false, Some(b'\n')) => Ok(at + 1),
            (false, None) => Ok(at),
            (false, Some(_)) => {
                Err("code follows a function named after an HTTP method on the same line")
            }
        }
    }

    fn attribute_start(&self, line_start: usize) -> usize {
        let text = self.text;
        let first = text.previous_line_start(line_start);
        let previous_lines = std::iter::successors(first, |at| text.previous_line_start(*at));
        let attributes = previous_lines.take_while(|previous| self.is_attribute_line(*previous));
        attributes.last().unwrap_or(line_start)
    }

    fn is_attribute_line(&self, line_start: usize) -> bool {
        let line = self.text.line_text(line_start).trim_start();
        let cs_attribute = [self.extension() == "cs", line.starts_with('[')] == [true, true];
        [line.starts_with('@'), line.starts_with("#["), cs_attribute].contains(&true)
    }

    fn python_methods(&self) -> Result<Vec<Member>, &'static str> {
        let (body_start, body_end) = self.python_class_body()?;
        let first_line = self
            .first_code_line(body_start, body_end)
            .ok_or("the class body is empty")?;
        let indentation = self.text.indentation(first_line);
        let mut methods = Vec::new();
        let mut line = body_start;
        while line < body_end {
            line = self.after_python_line(line, indentation, &mut methods)?;
        }
        Ok(methods)
    }

    fn after_python_line(
        &self,
        line: usize,
        indentation: usize,
        methods: &mut Vec<Member>,
    ) -> Result<usize, &'static str> {
        let Some((name_start, name_end)) = self.python_method_name(line, indentation) else {
            return Ok(self.text.line_end(line));
        };
        let end = self.indented_block_end(name_end, indentation);
        let start = self.attribute_start(line);
        methods.push(self.balanced_member(start, name_start, name_end, end)?);
        Ok(end)
    }

    fn python_class_body(&self) -> Result<(usize, usize), &'static str> {
        let text = self.text;
        let classes: Vec<usize> = text
            .code_words(Self::OLD_NAME)
            .into_iter()
            .filter(|at| text.previous_word(*at) == "class")
            .collect();
        let class = classes
            .first()
            .filter(|_| classes.len() == 1)
            .copied()
            .ok_or("the class could not be found exactly once")?;
        let class_line = text.line_start(class);
        let indentation = text.indentation(class_line);
        let body_start = text.line_end(class);
        Ok((body_start, self.indented_block_end(class, indentation)))
    }

    fn first_code_line(&self, start: usize, end: usize) -> Option<usize> {
        self.text
            .line_starts(start)
            .into_iter()
            .take_while(|line| *line < end)
            .find(|line| self.is_code_line(*line))
    }

    fn is_code_line(&self, line: usize) -> bool {
        let first = line + self.text.indentation(line);
        [self.text.is_blank_line(line), self.text.is_code(first)] == [false, true]
    }

    fn python_method_name(&self, line: usize, indentation: usize) -> Option<(usize, usize)> {
        let text = self.text;
        let blank = text.is_blank_line(line);
        let level = text.indentation(line) == indentation;
        ([blank, level] == [false, true]).then_some(())?;
        let first = line + indentation;
        let rest = text.line_text(line).trim_start();
        let after_def = rest
            .strip_prefix("def ")
            .or_else(|| rest.strip_prefix("async def "))?;
        let name_start = first + rest.len() - after_def.len();
        let name_start = text.next_code(name_start)?;
        let name_end = text.word_end(name_start);
        let is_method = [
            text.is_code(first),
            self.is_method_name(name_start, name_end),
        ];
        (is_method == [true, true]).then_some((name_start, name_end))
    }
}
