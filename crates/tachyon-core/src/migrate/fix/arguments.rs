use super::ManualStep;
use super::conversion_plan::ConversionPlan;
use super::edits::TextEdit;
use super::markers::{Marker, MarkerLine};
use super::python_parameters::PythonParameterEdits;
use super::scan::SourceText;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::ops::Range;
use std::path::Path;

pub(super) struct ArgumentConversion<'a> {
    relative: &'a str,
    extension: &'a str,
    text: SourceText<'a>,
}

struct Parameters {
    open: usize,
    close: usize,
    items: Vec<Range<usize>>,
    names: Vec<String>,
}

enum MarkerChange {
    Rewrite(String),
    Remove,
}

#[derive(Default)]
struct ParameterEdits {
    markers: BTreeMap<usize, Vec<String>>,
    checked_types: BTreeMap<usize, String>,
    documentation: Vec<String>,
}

impl<'a> ArgumentConversion<'a> {
    const NAMES: [&'static str; 12] = [
        "test",
        "tachyontest",
        "store",
        "entity",
        "convert",
        "cached",
        "check",
        "pagedefaults",
        "clearcache",
        "updatecache",
        "query",
        "settings",
    ];
    const RECEIVERS: [&'static str; 3] = ["self", "&self", "&mut self"];
    const TYPED_BY_DECLARATION: [&'static str; 3] = ["js", "ts", "py"];

    pub(super) fn plan(relative: &'a str, source: &'a str) -> ConversionPlan {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(text) = SourceText::read(extension, source) else {
            return ConversionPlan::default();
        };
        let conversion = Self {
            relative,
            extension,
            text,
        };
        conversion.convert()
    }

    fn convert(&self) -> ConversionPlan {
        let mut plan = ConversionPlan::default();
        let markers = Marker::lines(&self.text, self.extension, &Self::NAMES);
        let methods = self.method_parameters(&markers);
        let mut parameters: BTreeMap<usize, ParameterEdits> = BTreeMap::new();
        for marker in &markers {
            let Some(arguments) = self.arguments(marker) else {
                continue;
            };
            let line = self.text.line_number(marker.start);
            let before = plan.edits.len();
            self.convert_marker(marker, &arguments, &methods, &mut parameters, &mut plan);
            if plan.edits.len() > before {
                plan.notes.push(format!(
                    "Changed the {} arguments on line {line} to the typed form (ADR 0055).",
                    marker.name
                ));
            }
        }
        for (declaration, edits) in &parameters {
            self.parameter_edits(*declaration, edits, &mut plan);
        }
        plan
    }

    fn arguments(&self, marker: &MarkerLine) -> Option<Map<String, Value>> {
        let (start, end) = marker.arguments?;
        let parsed = crate::behavior::literal::parse(&self.text.source[start..end]).ok()?;
        parsed.as_object().cloned()
    }

    fn convert_marker(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        methods: &BTreeMap<String, Vec<String>>,
        parameters: &mut BTreeMap<usize, ParameterEdits>,
        plan: &mut ConversionPlan,
    ) {
        match Marker::normalized(&marker.name).as_str() {
            "test" | "tachyontest" => self.test(marker, arguments, plan),
            "store" => self.store(marker, arguments, plan),
            "entity" => self.entity(marker, arguments, plan),
            "convert" => self.convert_mapping(marker, arguments, plan),
            "settings" => self.settings(marker, arguments, plan),
            "clearcache" | "updatecache" => self.mutation(marker, arguments, methods, plan),
            _ => self.method_marker(marker, arguments, parameters, plan),
        }
    }

    fn replace_arguments(marker: &MarkerLine, replacement: &str, plan: &mut ConversionPlan) {
        let Some((start, end)) = marker.arguments else {
            return;
        };
        plan.edits.push(TextEdit::replace(start, end, replacement));
    }

    fn reference(&self, name: &str) -> String {
        crate::behavior::literal::class_reference_example(self.extension, name)
    }

    fn test(&self, marker: &MarkerLine, arguments: &Map<String, Value>, plan: &mut ConversionPlan) {
        let Some(configuration) = arguments.get("configuration").and_then(Value::as_str) else {
            return;
        };
        let Some((start, end)) = marker.arguments else {
            return;
        };
        let reference = self.reference(configuration);
        let (before, after) = reference.split_once(configuration).unwrap_or(("", ""));
        let mut tail = after.to_owned();
        if let Some(partition) = arguments.get("partition") {
            let _ = write!(tail, ", {{\"partition\":{partition}}}");
        }
        let written = &self.text.source[start..end];
        let key = "{\"configuration\":\"";
        let value_end = key.len() + configuration.len();
        let name_first =
            written.starts_with(key) && written[key.len()..].starts_with(configuration);
        if !name_first || written.as_bytes().get(value_end) != Some(&b'"') {
            let replacement = format!("{before}{configuration}{tail}");
            Self::replace_arguments(marker, &replacement, plan);
            return;
        }
        plan.edits
            .push(TextEdit::replace(start, start + key.len(), before));
        plan.edits
            .push(TextEdit::replace(start + value_end, end, &tail));
    }

    fn store(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        plan: &mut ConversionPlan,
    ) {
        let Some(entity) = arguments.get("entity").and_then(Value::as_str) else {
            return;
        };
        let mut options = arguments.clone();
        options.remove("entity");
        let replacement = format!("{}, {}", self.reference(entity), Value::Object(options));
        Self::replace_arguments(marker, &replacement, plan);
    }

    fn entity(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        plan: &mut ConversionPlan,
    ) {
        let Some(identifier) = arguments.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some(field_line) = self.field_line(marker, identifier) else {
            plan.steps.push(self.step(
                marker,
                format!("Entity names '{identifier}' as its identifier, but the fix cannot find that field."),
                format!("Declare the field '{identifier}', put @Id on it, and remove \"id\" from Entity."),
            ));
            return;
        };
        let mut options = arguments.clone();
        options.remove("id");
        Self::replace_arguments(marker, &Value::Object(options).to_string(), plan);
        let indent = Marker::indentation(&self.text, field_line);
        let id_marker = Marker::spelled(self.extension, "Id", "");
        if self.extension == "py" {
            let line = self.text.line_text(field_line);
            let declaration =
                PythonParameterEdits::annotated_declaration(line.trim(), &[id_marker]);
            plan.edits.push(TextEdit::replace(
                field_line,
                field_line + line.len(),
                &format!("{indent}{declaration}"),
            ));
            return;
        }
        plan.edits.push(TextEdit::insert(
            field_line,
            &format!("{indent}{id_marker}\n"),
        ));
    }

    fn field_line(&self, marker: &MarkerLine, name: &str) -> Option<usize> {
        let declaration = Marker::next_declaration(&self.text, self.extension, marker.line_start)?;
        let mut line_start = self.text.line_end(declaration);
        let body_indent = self.text.indentation(self.text.line_start(declaration));
        while line_start < self.text.len() {
            let line = self.text.line_text(line_start);
            let trimmed = line.trim();
            if !trimmed.is_empty() && self.text.indentation(line_start) <= body_indent {
                return None;
            }
            if Self::declares_field(self.extension, trimmed, name) {
                return Some(line_start);
            }
            line_start = self.text.line_end(line_start);
        }
        None
    }

    fn declares_field(extension: &str, line: &str, name: &str) -> bool {
        let plain = [line.contains('('), line.starts_with(['@', '#', '[', '/'])] == [false, false];
        let declaration = line.split('=').next().unwrap_or("").trim();
        let declaration = declaration.trim_end_matches([';', ',']);
        [plain, Self::declared_name(extension, declaration) == name] == [true, true]
    }

    fn convert_mapping(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        plan: &mut ConversionPlan,
    ) {
        let Some(argument) = arguments.get("argument") else {
            return;
        };
        if argument.as_u64() != Some(0) {
            plan.steps.push(self.step(
                marker,
                String::from("Convert reads a parameter other than the first one."),
                String::from("Give the Convert method one parameter, the value it converts, and remove \"argument\"."),
            ));
            return;
        }
        let mut options = arguments.clone();
        options.remove("argument");
        Self::replace_arguments(marker, &Value::Object(options).to_string(), plan);
    }

    fn step(&self, marker: &MarkerLine, problem: String, action: String) -> ManualStep {
        let line = self.text.line_number(marker.start);
        let mut text = format!("Line {line}: ");
        text.push_str(&(problem + " (ADR 0055)."));
        ManualStep::new(self.relative, text, action)
    }
}

impl ArgumentConversion<'_> {
    fn parameters(&self, declaration: usize) -> Option<Parameters> {
        let mut open = declaration + self.text.source[declaration..].find('(')?;
        if self.extension == "go" && self.text.source[declaration..].starts_with("func (") {
            let receiver_close = self.text.matching(open)?;
            open = receiver_close + 1 + self.text.source[receiver_close + 1..].find('(')?;
        }
        let close = self.text.matching(open)?;
        let mut items = Vec::new();
        let mut names = Vec::new();
        for item in self.items(open, close) {
            let piece = self.text.source[item.clone()].trim();
            if Self::RECEIVERS.contains(&piece) {
                continue;
            }
            names.push(Self::parameter_name(self.extension, piece));
            items.push(item);
        }
        Some(Parameters {
            open,
            close,
            items,
            names,
        })
    }

    fn items(&self, open: usize, close: usize) -> Vec<Range<usize>> {
        let mut items = Vec::new();
        let mut depth = 0_i64;
        let mut start = open + 1;
        for at in open + 1..close {
            if !self.text.is_code(at) {
                continue;
            }
            match self.text.bytes[at] {
                b'(' | b'[' | b'{' | b'<' => depth += 1,
                b')' | b']' | b'}' | b'>' => depth -= 1,
                b',' if depth == 0 => {
                    items.push(start..at);
                    start = at + 1;
                }
                _ => {}
            }
        }
        if !self.text.source[start..close].trim().is_empty() {
            items.push(start..close);
        }
        items
    }

    fn parameter_name(extension: &str, piece: &str) -> String {
        let declaration = piece.split('=').next().unwrap_or("").trim();
        Self::declared_name(extension, declaration).to_owned()
    }

    fn declared_name<'b>(extension: &str, declaration: &'b str) -> &'b str {
        let name = match extension {
            "go" => declaration.split_whitespace().next().unwrap_or(""),
            "php" => declaration.rsplit('$').next().unwrap_or(""),
            "ts" | "py" | "kt" | "swift" | "rs" | "js" => {
                let before_type = declaration.split(':').next().unwrap_or("");
                before_type.split_whitespace().last().unwrap_or("")
            }
            _ => declaration.split_whitespace().last().unwrap_or(""),
        };
        name.trim_end_matches(['?', '!'])
    }

    fn method_parameters(&self, markers: &[MarkerLine]) -> BTreeMap<String, Vec<String>> {
        let mut methods = BTreeMap::new();
        for marker in markers {
            let Some(declaration) =
                Marker::next_declaration(&self.text, self.extension, marker.line_start)
            else {
                continue;
            };
            let Some(parameters) = self.parameters(declaration) else {
                continue;
            };
            let before_open = self.text.source[declaration..parameters.open].trim_end();
            let name_start = self.text.word_start(declaration + before_open.len());
            let name = &self.text.source[name_start..declaration + before_open.len()];
            methods.insert(name.to_owned(), parameters.names);
        }
        methods
    }

    fn mutation(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        methods: &BTreeMap<String, Vec<String>>,
        plan: &mut ConversionPlan,
    ) {
        let Some(Value::Array(indexes)) = arguments.get("arguments") else {
            return;
        };
        let target = arguments
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("");
        let own_parameters =
            Marker::next_declaration(&self.text, self.extension, marker.line_start)
                .and_then(|declaration| self.parameters(declaration))
                .map(|parameters| parameters.names)
                .unwrap_or_default();
        let target_parameters = methods.get(target).cloned().unwrap_or_default();
        for (position, index) in indexes.iter().enumerate() {
            let own_name = index
                .as_u64()
                .and_then(|number| own_parameters.get(usize::try_from(number).ok()?));
            let target_name = target_parameters.get(position);
            if own_name != target_name {
                plan.steps.push(self.step(
                    marker,
                    format!(
                        "{} now matches parameters by name, and the names differ from '{target}'.",
                        marker.name
                    ),
                    format!(
                        "Rename this method's parameters to match the key parameters of '{target}'."
                    ),
                ));
                break;
            }
        }
        let mut options = arguments.clone();
        options.remove("arguments");
        Self::replace_arguments(marker, &Value::Object(options).to_string(), plan);
    }
}

impl ArgumentConversion<'_> {
    fn method_marker(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        parameters: &mut BTreeMap<usize, ParameterEdits>,
        plan: &mut ConversionPlan,
    ) {
        let Some(declaration) =
            Marker::next_declaration(&self.text, self.extension, marker.line_start)
        else {
            return;
        };
        let Some(list) = self.parameters(declaration) else {
            return;
        };
        let name = Marker::normalized(&marker.name);
        let edits = parameters.entry(declaration).or_default();
        let converted = match name.as_str() {
            "cached" => Self::cached(arguments, &list, edits, self.extension),
            "check" => self.checked(arguments, &list, edits),
            "pagedefaults" => Self::page_defaults(arguments, edits, self.extension),
            _ => Self::query(arguments, edits, self.extension),
        };
        let Some(change) = converted else {
            return;
        };
        let line_end = self.text.line_end(marker.line_start);
        match change {
            MarkerChange::Rewrite(arguments) => {
                let line = Marker::spelled(self.extension, Self::written_name(&name), &arguments);
                plan.edits
                    .push(TextEdit::replace(marker.start, marker.end, &line));
            }
            MarkerChange::Remove => plan
                .edits
                .push(TextEdit::delete(marker.line_start, line_end)),
        }
    }

    fn written_name(normalized: &str) -> &'static str {
        match normalized {
            "cached" => "Cached",
            "query" => "Query",
            "check" => "Check",
            _ => "PageDefaults",
        }
    }

    fn marker_text(extension: &str, name: &str, arguments: &str) -> String {
        let spelled = Marker::spelled(extension, name, arguments);
        spelled.trim_start_matches("// ").to_owned()
    }

    fn cached(
        arguments: &Map<String, Value>,
        list: &Parameters,
        edits: &mut ParameterEdits,
        extension: &str,
    ) -> Option<MarkerChange> {
        let Some(Value::Array(keys)) = arguments.get("keyArgs") else {
            return None;
        };
        let every: Vec<Value> = (0..list.items.len()).map(Value::from).collect();
        if *keys != every {
            for key in keys {
                let index = usize::try_from(key.as_u64()?).ok()?;
                let marker = Self::marker_text(extension, "CacheKey", "");
                edits.markers.entry(index).or_default().push(marker);
            }
        }
        Some(MarkerChange::Rewrite(String::new()))
    }

    fn checked(
        &self,
        arguments: &Map<String, Value>,
        list: &Parameters,
        edits: &mut ParameterEdits,
    ) -> Option<MarkerChange> {
        let index = usize::try_from(arguments.get("argument")?.as_u64()?).ok()?;
        let checked_type = arguments.get("type")?.as_str()?;
        let parameter = list.names.get(index)?;
        let written = if Self::TYPED_BY_DECLARATION.contains(&self.extension) {
            edits.checked_types.insert(index, checked_type.to_owned());
            if self.extension == "js" {
                edits
                    .documentation
                    .push(format!("/** @param {{{checked_type}}} {parameter} */"));
            }
            String::new()
        } else {
            self.reference(checked_type)
        };
        let marker = Self::marker_text(self.extension, "Check", &written);
        edits.markers.entry(index).or_default().push(marker);
        Some(MarkerChange::Remove)
    }

    fn page_defaults(
        arguments: &Map<String, Value>,
        edits: &mut ParameterEdits,
        extension: &str,
    ) -> Option<MarkerChange> {
        let index = usize::try_from(arguments.get("argument")?.as_u64()?).ok()?;
        let mut options = arguments.clone();
        options.remove("argument");
        let marker = Self::marker_text(
            extension,
            "PageDefaults",
            &Value::Object(options).to_string(),
        );
        edits.markers.entry(index).or_default().push(marker);
        Some(MarkerChange::Remove)
    }

    fn query(
        arguments: &Map<String, Value>,
        edits: &mut ParameterEdits,
        extension: &str,
    ) -> Option<MarkerChange> {
        let page = arguments.get("pageArgument")?;
        if let Some(index) = page.as_u64().and_then(|index| usize::try_from(index).ok()) {
            let marker = Self::marker_text(extension, "PageDefaults", "");
            edits.markers.entry(index).or_default().push(marker);
        }
        let mut options = arguments.clone();
        options.remove("pageArgument");
        Some(MarkerChange::Rewrite(Value::Object(options).to_string()))
    }
}

impl ArgumentConversion<'_> {
    fn parameter_edits(
        &self,
        declaration: usize,
        edits: &ParameterEdits,
        plan: &mut ConversionPlan,
    ) {
        let Some(list) = self.parameters(declaration) else {
            return;
        };
        if edits.markers.is_empty() {
            return;
        }
        let line_start = self.text.line_start(declaration);
        let indent = Marker::indentation(&self.text, line_start);
        for documentation in &edits.documentation {
            let text = format!("{indent}{documentation}\n");
            plan.edits
                .push(TextEdit::insert(self.block_start(line_start), &text));
        }
        if self.extension == "go" {
            plan.edits
                .push(self.go_parameter_list(&list, edits, indent));
            return;
        }
        for (index, item) in list.items.iter().enumerate() {
            let piece = &self.text.source[item.clone()];
            let written = piece.trim_start();
            let lead = &piece[..piece.len() - written.len()];
            let mut replacement = written.trim_end().to_owned();
            if let Some(kind) = edits.checked_types.get(&index) {
                replacement = Self::with_type(self.extension, &replacement, kind);
            }
            if let Some(markers) = edits.markers.get(&index) {
                replacement = if self.extension == "py" {
                    PythonParameterEdits::annotated_declaration(&replacement, markers)
                } else {
                    format!("{} {replacement}", markers.join(" "))
                };
            }
            if replacement != written.trim_end() {
                let end = item.start + lead.len() + written.trim_end().len();
                plan.edits.push(TextEdit::replace(
                    item.start + lead.len(),
                    end,
                    &replacement,
                ));
            }
        }
    }

    fn block_start(&self, declaration_line: usize) -> usize {
        let mut start = declaration_line;
        while let Some(previous) = self.text.previous_line_start(start) {
            if Marker::at_line(&self.text, self.extension, previous).is_none() {
                break;
            }
            start = previous;
        }
        start
    }

    fn with_type(extension: &str, parameter: &str, kind: &str) -> String {
        if extension == "js" {
            return parameter.to_owned();
        }
        let name = parameter.split(':').next().unwrap_or(parameter).trim_end();
        format!("{name}: {kind}")
    }

    fn go_parameter_list(
        &self,
        list: &Parameters,
        edits: &ParameterEdits,
        indent: &str,
    ) -> TextEdit {
        let mut lines = String::from("\n");
        for (index, item) in list.items.iter().enumerate() {
            for marker in edits.markers.get(&index).into_iter().flatten() {
                let _ = writeln!(lines, "{indent}\t// {marker}");
            }
            let piece = self.text.source[item.clone()].trim();
            let _ = writeln!(lines, "{indent}\t{piece},");
        }
        lines.push_str(indent);
        TextEdit::replace(list.open + 1, list.close, &lines)
    }
}

impl ArgumentConversion<'_> {
    fn settings(
        &self,
        marker: &MarkerLine,
        arguments: &Map<String, Value>,
        plan: &mut ConversionPlan,
    ) {
        let Some(Value::Object(fields)) = arguments.get("fields") else {
            return;
        };
        let Some(declaration) =
            Marker::next_declaration(&self.text, self.extension, marker.line_start)
        else {
            return;
        };
        let Some(body) = self.empty_body(declaration) else {
            plan.steps.push(self.step(
                marker,
                String::from("Settings lists its fields in the annotation, and the class already has members."),
                String::from("Declare each setting as a field with its type and default, put @Min/@Max on it, and keep only \"prefix\" in Settings."),
            ));
            return;
        };
        let Some(lines) = self.settings_fields(fields) else {
            plan.steps.push(self.step(
                marker,
                String::from("A Settings field has a type or name the fix cannot write."),
                String::from("Declare the fields by hand and keep only \"prefix\" in Settings."),
            ));
            return;
        };
        let options = Self::settings_options(arguments, fields, self.extension);
        Self::replace_arguments(marker, &Value::Object(options).to_string(), plan);
        let indent = Marker::indentation(&self.text, self.text.line_start(declaration));
        plan.edits.push(TextEdit::replace(
            body.start,
            body.end,
            &Self::settings_body(self.extension, &lines, indent),
        ));
    }

    fn empty_body(&self, declaration: usize) -> Option<Range<usize>> {
        let line_end = self.text.line_end(declaration);
        let line = &self.text.source[declaration..line_end];
        if self.extension == "py" {
            let next = self.text.line_text(line_end).trim();
            if line.trim_end().ends_with(':') && next == "pass" {
                let pass_end = self.text.line_end(line_end);
                let end = pass_end - usize::from(self.text.source[..pass_end].ends_with('\n'));
                return Some(declaration + line.trim_end().len()..end);
            }
            return None;
        }
        let open = declaration + line.find('{')?;
        let close = self.text.matching(open)?;
        let inside = &self.text.source[open + 1..close];
        if !inside.trim().is_empty() {
            return None;
        }
        Some(open..close + 1)
    }

    fn settings_options(
        arguments: &Map<String, Value>,
        fields: &Map<String, Value>,
        extension: &str,
    ) -> Map<String, Value> {
        let mut options = Map::new();
        if let Some(prefix) = arguments.get("prefix") {
            options.insert("prefix".to_owned(), prefix.clone());
        }
        let mut names = Map::new();
        let mut defaults = Map::new();
        for (field, descriptor) in fields {
            let name = descriptor["name"].as_str().unwrap_or("");
            if name != Self::environment_suffix(field) {
                names.insert(field.clone(), Value::String(name.to_owned()));
            }
            if matches!(extension, "rs" | "go")
                && let Some(default) = descriptor.get("default")
            {
                defaults.insert(field.clone(), default.clone());
            }
        }
        if !names.is_empty() {
            options.insert("names".to_owned(), Value::Object(names));
        }
        if !defaults.is_empty() {
            options.insert("defaults".to_owned(), Value::Object(defaults));
        }
        options
    }

    fn environment_suffix(field: &str) -> String {
        let mut name = String::new();
        let mut previous_lowercase = false;
        for character in field.chars() {
            if character.is_ascii_uppercase() && previous_lowercase {
                name.push('_');
            }
            previous_lowercase = character.is_ascii_lowercase() || character.is_ascii_digit();
            name.push(character.to_ascii_uppercase());
        }
        name
    }

    fn settings_fields(&self, fields: &Map<String, Value>) -> Option<Vec<String>> {
        let mut lines = Vec::new();
        for (field, descriptor) in fields {
            let mut bounds = Vec::new();
            for bound in ["min", "max"] {
                if let Some(value) = descriptor.get(bound) {
                    let name = if bound == "min" { "Min" } else { "Max" };
                    bounds.push(Marker::spelled(self.extension, name, &value.to_string()));
                }
            }
            let declaration = self.settings_field(field, descriptor)?;
            if self.extension == "py" && !bounds.is_empty() {
                lines.push(PythonParameterEdits::annotated_declaration(
                    &declaration,
                    &bounds,
                ));
                continue;
            }
            lines.extend(bounds);
            lines.push(declaration);
        }
        Some(lines)
    }

    fn settings_field(&self, field: &str, descriptor: &Value) -> Option<String> {
        let kind = descriptor["type"].as_str()?;
        let spelled = SettingsTypes::spelled(self.extension, kind)?;
        let default = descriptor
            .get("default")
            .filter(|_| !matches!(self.extension, "rs" | "go"))
            .map(|value| SettingsTypes::literal(self.extension, kind, value));
        Some(SettingsTypes::declaration(
            self.extension,
            field,
            spelled,
            default.as_deref(),
        ))
    }

    fn settings_body(extension: &str, lines: &[String], indent: &str) -> String {
        let member_indent = if extension == "go" {
            format!("{indent}\t")
        } else {
            format!("{indent}    ")
        };
        let mut body = String::new();
        for line in lines {
            for part in line.split('\n') {
                body.push('\n');
                body.push_str(&member_indent);
                body.push_str(part);
            }
        }
        if extension == "py" {
            return body;
        }
        format!(" {{{body}\n{indent}}}")
    }
}

struct SettingsTypes;

impl SettingsTypes {
    fn spelled(extension: &str, kind: &str) -> Option<&'static str> {
        let names: [&'static str; 4] = match extension {
            "java" => ["String", "int", "double", "boolean"],
            "kt" => ["String", "Int", "Double", "Boolean"],
            "cs" => ["string", "int", "double", "bool"],
            "swift" => ["String", "Int", "Double", "Bool"],
            "dart" => ["String", "int", "double", "bool"],
            "py" => ["str", "int", "float", "bool"],
            "php" => ["string", "int", "float", "bool"],
            "js" | "ts" => ["string", "integer", "number", "boolean"],
            "rs" => ["String", "i64", "f64", "bool"],
            "go" => ["string", "int64", "float64", "bool"],
            _ => return None,
        };
        match kind {
            "string" => Some(names[0]),
            "integer" => Some(names[1]),
            "number" => Some(names[2]),
            "boolean" => Some(names[3]),
            _ => None,
        }
    }

    fn literal(extension: &str, kind: &str, value: &Value) -> String {
        match value {
            Value::Bool(true) if extension == "py" => String::from("True"),
            Value::Bool(false) if extension == "py" => String::from("False"),
            Value::String(text) if matches!(extension, "js" | "ts" | "py" | "php" | "dart") => {
                format!("'{text}'")
            }
            Value::Number(number) if kind == "number" && number.is_i64() => {
                format!("{number}.0")
            }
            _ => value.to_string(),
        }
    }

    fn declaration(extension: &str, field: &str, spelled: &str, default: Option<&str>) -> String {
        let assigned = default.map_or_else(String::new, |value| format!(" = {value}"));
        match (extension, default) {
            ("java", _) | ("dart", Some(_)) => format!("{spelled} {field}{assigned};"),
            ("cs", _) => format!("public {spelled} {field}{assigned};"),
            ("php", _) => format!("public {spelled} ${field}{assigned};"),
            ("dart", None) => format!("{spelled}? {field};"),
            ("kt" | "swift", Some(_)) => format!("var {field}: {spelled}{assigned}"),
            ("kt", None) => format!("var {field}: {spelled}? = null"),
            ("swift", None) => format!("var {field}: {spelled}?"),
            ("py", _) => format!("{field}: {spelled}{assigned}"),
            ("ts", Some(_)) => format!("{field}: {spelled}{assigned};"),
            ("ts", None) => format!("{field}!: {spelled};"),
            ("js", Some(_)) => format!("{field}{assigned};"),
            ("js", None) => format!("/** @type {{{spelled}}} */\n{field};"),
            ("rs", _) => format!("{field}: {spelled},"),
            _ => format!("{field} {spelled}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArgumentConversion;

    #[test]
    fn field_lines_are_matched_by_the_declared_name_of_each_language() {
        let cases = [
            ("go", "Name string", "Name", true),
            ("go", "Other string", "Name", false),
            ("php", "public $name;", "name", true),
            ("ts", "private name?: string;", "name", true),
            ("kt", "val name: String = \"\"", "name", true),
            ("swift", "var name!: String", "name", true),
            ("py", "name: str", "name", true),
            ("rs", "pub name: String,", "name", true),
            ("cs", "public string name,", "name", true),
            ("java", "private String name;", "name", true),
            ("java", "private String name = \"x\";", "name", true),
            ("java", "String name(int x)", "name", false),
            ("java", "@Id String name;", "name", false),
            ("java", "// String name;", "name", false),
            ("java", "[Key] string name;", "name", false),
            ("java", "# name", "name", false),
        ];
        for (extension, line, name, expected) in cases {
            assert_eq!(
                ArgumentConversion::declares_field(extension, line, name),
                expected,
                "{extension} {line:?}"
            );
        }
    }

    #[test]
    fn parameter_names_are_the_declared_name_of_each_language() {
        let cases = [
            ("go", "name string", "name"),
            ("php", "$name = 1", "name"),
            ("ts", "name?: string", "name"),
            ("kt", "count: Int = 0", "count"),
            ("java", "final int count", "count"),
            ("py", "value: int = 3", "value"),
            ("rs", "mut total: u64", "total"),
            ("cs", "string label!", "label"),
            ("dart", "", ""),
        ];
        for (extension, piece, expected) in cases {
            assert_eq!(
                ArgumentConversion::parameter_name(extension, piece),
                expected,
                "{extension} {piece:?}"
            );
        }
    }
}
