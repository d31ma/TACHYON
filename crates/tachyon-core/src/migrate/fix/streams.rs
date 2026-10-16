use super::ManualStep;
use super::conversion_plan::ConversionPlan;
use super::edits::TextEdit;
use super::markers::{Marker, MarkerLine};
use super::scan::SourceText;
use std::path::Path;

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

    pub(super) fn plan(relative: &'a str, source: &'a str) -> ConversionPlan {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(extension, source);
        text.map(|text| {
            let conversion = Self {
                relative,
                extension,
                text,
            };
            conversion.convert_all()
        })
        .unwrap_or_default()
    }

    fn convert_all(&self) -> ConversionPlan {
        let mut plan = ConversionPlan::default();
        let markers = Marker::lines(&self.text, self.extension, &Self::ROUTES);
        for stream in markers
            .iter()
            .filter(|marker| Marker::normalized(&marker.name) == "stream")
        {
            self.convert(stream, &markers, &mut plan);
        }
        plan
    }

    fn convert(&self, stream: &MarkerLine, markers: &[MarkerLine], plan: &mut ConversionPlan) {
        let declaration = Marker::next_declaration(&self.text, self.extension, stream.start);
        let converted = declaration.map(|declaration| self.converted(stream, declaration, markers));
        match converted {
            Some(Ok((edits, note))) => {
                plan.edits.extend(edits);
                plan.notes.push(note);
            }
            Some(Err(problem)) => plan.steps.push(self.manual(problem)),
            None => {}
        }
    }

    fn converted(
        &self,
        stream: &MarkerLine,
        declaration: usize,
        markers: &[MarkerLine],
    ) -> Result<(Vec<TextEdit>, String), String> {
        let line = self.text.line_number(stream.start);
        let get_route = self.get_route(stream, declaration, markers).ok_or_else(|| {
            format!("@Stream on line {line} is not next to one @GetRoute, so the fix cannot tell which route streams.")
        })?;
        let method = self.method_name(declaration).ok_or_else(|| {
            format!("The fix cannot find the name of the method under @Stream on line {line}.")
        })?;
        let renamed = self.renamed(&method)?;
        let mut edits = vec![
            TextEdit::delete(stream.line_start, self.text.line_end(stream.line_start)),
            TextEdit::replace(
                get_route.start,
                get_route.end,
                &Marker::spelled(self.extension, "StreamRoute", &self.arguments(get_route)),
            ),
        ];
        let method_end = method.start + method.name.len();
        let rename = renamed
            .as_ref()
            .map(|renamed| TextEdit::replace(method.start, method_end, renamed));
        edits.extend(rename);
        let note = Self::note(line, &method, renamed.as_deref());
        Ok((edits, note))
    }

    fn get_route<'m>(
        &self,
        stream: &MarkerLine,
        declaration: usize,
        markers: &'m [MarkerLine],
    ) -> Option<&'m MarkerLine> {
        let routes: Vec<&MarkerLine> = markers
            .iter()
            .filter(|marker| self.marks_declaration(marker, declaration))
            .collect();
        let [route] = routes.as_slice() else {
            return None;
        };
        let is_get = Marker::normalized(&route.name) == "getroute";
        let own_line = route.line_start != stream.line_start;
        ([is_get, own_line] == [true, true]).then_some(*route)
    }

    fn marks_declaration(&self, marker: &MarkerLine, declaration: usize) -> bool {
        let is_route = Marker::normalized(&marker.name) != "stream";
        let next = Marker::next_declaration(&self.text, self.extension, marker.start);
        [is_route, next == Some(declaration)] == [true, true]
    }

    fn note(line: usize, method: &MethodName, renamed: Option<&str>) -> String {
        let Some(renamed) = renamed else {
            return format!(
                "Changed @GetRoute and @Stream to @StreamRoute on line {line} (ADR 0048)."
            );
        };
        format!(
            "Changed @GetRoute and @Stream to @StreamRoute on line {line} and renamed {} to {renamed} (ADR 0048).",
            method.name
        )
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
        let head = &line[..before_call];
        let end = declaration + head.trim_end_matches(' ').len();
        let start = self.text.word_start(end);
        (start != end).then(|| MethodName {
            start,
            name: String::from(&self.text.source[start..end]),
        })
    }

    fn renamed(&self, method: &MethodName) -> Result<Option<String>, String> {
        let streams = Self::starts_with(&method.name, "stream");
        let renamed = (!streams).then(|| self.stream_name(method));
        renamed.transpose()
    }

    fn stream_name(&self, method: &MethodName) -> Result<String, String> {
        let name = &method.name;
        Self::starts_with(name, "get").then_some(()).ok_or_else(|| {
            format!("The stream method {name} does not start with 'get', so the fix cannot choose its new name.")
        })?;
        let prefix = if name.starts_with('G') {
            "Stream"
        } else {
            "stream"
        };
        let renamed = format!("{prefix}{}", &name[3..]);
        let uses = self.text.code_words(name).len();
        let free = [uses == 1, self.text.code_words(&renamed).is_empty()] == [true, true];
        let taken = format!(
            "{name} is used elsewhere in this file, or {renamed} already exists, so the fix does not rename it."
        );
        free.then_some(renamed).ok_or(taken)
    }

    fn starts_with(name: &str, verb: &str) -> bool {
        let prefix = name.get(..verb.len());
        let spelled = prefix.is_some_and(|prefix| prefix.eq_ignore_ascii_case(verb));
        let next = name.get(verb.len()..).and_then(|rest| rest.chars().next());
        let at_boundary = next.is_none_or(Self::starts_word_part);
        [spelled, at_boundary] == [true, true]
    }

    fn starts_word_part(next: char) -> bool {
        [
            next == '_',
            next.is_ascii_uppercase(),
            next.is_ascii_digit(),
        ]
        .contains(&true)
    }

    fn manual(&self, problem: String) -> ManualStep {
        ManualStep::new(self.relative, problem, String::from(Self::ACTION))
    }
}
