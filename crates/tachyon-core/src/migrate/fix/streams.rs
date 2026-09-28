use super::ManualStep;
use super::edits::TextEdit;
use super::markers::{Marker, MarkerLine};
use super::scan::SourceText;
use std::path::Path;

#[derive(Default)]
pub(super) struct StreamPlan {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) steps: Vec<ManualStep>,
}

pub(super) struct StreamConversion<'a> {
    relative: &'a str,
    extension: &'a str,
    text: SourceText<'a>,
}

struct MethodName {
    start: usize,
    name: String,
}

impl<'a> StreamConversion<'a> {
    const ROUTES: [&'static str; 6] = [
        "getroute",
        "postroute",
        "putroute",
        "patchroute",
        "deleteroute",
        "stream",
    ];
    const ACTION: &'static str = "Write @StreamRoute alone on the method and start the method name with 'stream', for example streamTicks (ADR 0048).";

    pub(super) fn plan(relative: &'a str, source: &'a str) -> StreamPlan {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(text) = SourceText::read(extension, source) else {
            return StreamPlan::default();
        };
        let conversion = Self {
            relative,
            extension,
            text,
        };
        let mut plan = StreamPlan::default();
        let markers = Marker::lines(&conversion.text, extension, &Self::ROUTES);
        for stream in markers
            .iter()
            .filter(|marker| Marker::normalized(&marker.name) == "stream")
        {
            conversion.convert(stream, &markers, &mut plan);
        }
        plan
    }

    fn convert(&self, stream: &MarkerLine, markers: &[MarkerLine], plan: &mut StreamPlan) {
        let line = self.text.line_number(stream.start);
        let Some(declaration) = Marker::next_declaration(&self.text, self.extension, stream.start)
        else {
            return;
        };
        let routes: Vec<&MarkerLine> = markers
            .iter()
            .filter(|marker| {
                Marker::normalized(&marker.name) != "stream"
                    && Marker::next_declaration(&self.text, self.extension, marker.start)
                        == Some(declaration)
            })
            .collect();
        let get_route = match routes.as_slice() {
            [route]
                if Marker::normalized(&route.name) == "getroute"
                    && route.line_start != stream.line_start =>
            {
                *route
            }
            _ => {
                plan.steps.push(self.manual(
                    format!("@Stream on line {line} is not next to one @GetRoute, so the fix cannot tell which route streams."),
                ));
                return;
            }
        };
        let Some(method) = self.method_name(declaration) else {
            plan.steps.push(self.manual(format!(
                "The fix cannot find the name of the method under @Stream on line {line}."
            )));
            return;
        };
        let mut edits = vec![
            TextEdit::delete(stream.line_start, self.text.line_end(stream.line_start)),
            TextEdit::replace(
                get_route.start,
                get_route.end,
                &Marker::spelled(self.extension, "StreamRoute", &self.arguments(get_route)),
            ),
        ];
        match self.renamed(&method) {
            Ok(Some(renamed)) => {
                edits.push(TextEdit::replace(
                    method.start,
                    method.start + method.name.len(),
                    &renamed,
                ));
                plan.notes.push(format!(
                    "Changed @GetRoute and @Stream to @StreamRoute on line {line} and renamed {} to {renamed} (ADR 0048).",
                    method.name
                ));
            }
            Ok(None) => plan.notes.push(format!(
                "Changed @GetRoute and @Stream to @StreamRoute on line {line} (ADR 0048)."
            )),
            Err(problem) => {
                plan.steps.push(self.manual(problem));
                return;
            }
        }
        plan.edits.extend(edits);
    }

    fn arguments(&self, marker: &MarkerLine) -> String {
        marker.arguments.map_or_else(String::new, |(start, end)| {
            String::from(&self.text.source[start..end])
        })
    }

    fn method_name(&self, declaration: usize) -> Option<MethodName> {
        let line_end = self.text.line_end(declaration);
        let line = &self.text.source[declaration..line_end];
        let before_call = line.find('(').map_or(line.trim_end().len(), |at| at);
        let mut end = declaration + before_call;
        while end > declaration && self.text.bytes[end - 1] == b' ' {
            end -= 1;
        }
        let start = self.text.word_start(end);
        if start == end {
            return None;
        }
        Some(MethodName {
            start,
            name: String::from(&self.text.source[start..end]),
        })
    }

    fn renamed(&self, method: &MethodName) -> Result<Option<String>, String> {
        if Self::starts_with(&method.name, "stream") {
            return Ok(None);
        }
        if !Self::starts_with(&method.name, "get") {
            return Err(format!(
                "The stream method {} does not start with 'get', so the fix cannot choose its new name.",
                method.name
            ));
        }
        let rest = &method.name[3..];
        let prefix = if method.name.starts_with('G') {
            "Stream"
        } else {
            "stream"
        };
        let renamed = format!("{prefix}{rest}");
        let uses = self.text.code_words(&method.name).len();
        if uses != 1 || !self.text.code_words(&renamed).is_empty() {
            return Err(format!(
                "{} is used elsewhere in this file, or {renamed} already exists, so the fix does not rename it.",
                method.name
            ));
        }
        Ok(Some(renamed))
    }

    fn starts_with(name: &str, verb: &str) -> bool {
        let Some(prefix) = name.get(..verb.len()) else {
            return false;
        };
        if !prefix.eq_ignore_ascii_case(verb) {
            return false;
        }
        name[verb.len()..]
            .chars()
            .next()
            .is_none_or(|next| next == '_' || next.is_ascii_uppercase() || next.is_ascii_digit())
    }

    fn manual(&self, problem: String) -> ManualStep {
        ManualStep::new(self.relative, problem, String::from(Self::ACTION))
    }
}
