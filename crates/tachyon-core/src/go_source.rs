mod go_function_results;
pub(crate) use go_function_results::GoFunctionResults;

use crate::Failure;
use crate::ascii_name::AsciiName;
use std::ops::Range;
use std::path::Path;

pub(crate) fn validate_markers(path: &Path, source: &str, code: &str) -> Result<(), Failure> {
    if let Some(at) = code.find("#[") {
        return Err(marker_failure(
            path,
            source,
            at,
            "Go annotations use `// @Name`, not `#[Name]`; run `ty migrate fix --write`",
        ));
    }
    if let Some((at, message)) = crate::lexical::code_mask("go", source).marker_problem {
        return Err(marker_failure(path, source, at, message));
    }
    let mut offset = 0;
    for line in code.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let at = offset + line.len() - trimmed.len();
        offset += line.len();
        let Some(rest) = trimmed.strip_prefix('@') else {
            continue;
        };
        let comment = at.checked_sub(3).and_then(|start| source.get(start..at));
        if comment != Some("// ") {
            return Err(marker_failure(
                path,
                source,
                at,
                "Go annotations are comments: write `// @Name` on its own line, not `@Name`",
            ));
        }
        if let Some(message) = GoMarkerName::refusal(GoMarkerName::read(rest)) {
            return Err(marker_failure(path, source, at, message));
        }
    }
    Ok(())
}

pub(crate) struct GoMarkerName;

impl GoMarkerName {
    const NAMESPACES: [&'static str; 2] = ["yon::", "tac::"];
    const SWAG_TAGS: [&'static str; 2] = ["Description", "ID"];
    const MAX_NAME: usize = 128;

    pub(crate) fn read(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
            .count();
        &text[..length]
    }

    pub(crate) fn is_marker(text: &str) -> bool {
        let name = Self::read(text);
        let after = &text[name.len()..];
        let swag_text = Self::SWAG_TAGS.contains(&name)
            && after.starts_with([' ', '\t'])
            && !after.trim().is_empty();
        if swag_text || !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
            return false;
        }
        name.contains("::")
            || (name.as_bytes()[0].is_ascii_uppercase()
                && (Self::known(name) || Self::suggestion(name).is_some()))
    }

    pub(crate) fn known(name: &str) -> bool {
        let plain = Self::NAMESPACES
            .iter()
            .find_map(|namespace| name.strip_prefix(namespace))
            .unwrap_or(name);
        if crate::behavior::framework_marker_names().any(|marker| marker == plain) {
            return true;
        }
        if crate::behavior::framework_marker_names()
            .any(|marker| marker.eq_ignore_ascii_case(plain))
        {
            return false;
        }
        crate::behavior::RenamedAnnotation::message(plain).is_some()
            || crate::behavior::RoutingRename::message(plain).is_some()
            || crate::yon::parser::is_yon_annotation(plain)
    }

    pub(crate) fn suggestion(name: &str) -> Option<&'static str> {
        let bounded: String = name.chars().take(Self::MAX_NAME).collect();
        let limit = if bounded.len() <= 5 { 1 } else { 2 };
        crate::behavior::framework_marker_names()
            .map(|candidate| (marker_distance(&bounded, candidate), candidate))
            .filter(|(distance, _)| *distance <= limit)
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, candidate)| candidate)
    }

    fn refusal(name: &str) -> Option<String> {
        if Self::known(name) {
            return None;
        }
        let written: String = name.chars().take(Self::MAX_NAME).collect();
        let (namespace, plain) = match written.rsplit_once("::") {
            Some((namespace, plain)) => (format!("{namespace}::"), plain.to_owned()),
            None => (String::new(), written.clone()),
        };
        let lowered = namespace.to_ascii_lowercase();
        if !namespace.is_empty()
            && namespace != lowered
            && Self::NAMESPACES.contains(&lowered.as_str())
        {
            return Some(format!(
                "unknown annotation `@{written}`; namespaces are lowercase: write `// @{lowered}{plain}`"
            ));
        }
        let help = Self::suggestion(&plain).map_or_else(String::new, |candidate| {
            format!("; did you mean `// @{namespace}{candidate}`?")
        });
        Some(format!("unknown annotation `@{written}`{help}"))
    }
}

fn marker_failure(path: &Path, source: &str, at: usize, message: impl Into<String>) -> Failure {
    Failure::one(crate::failure::diagnostic(
        1308,
        message,
        Some(String::from(
            "Write each Go annotation as `// @Name` on its own line before the declaration.",
        )),
        crate::failure::source_span(&path.to_string_lossy(), at, (at + 1).min(source.len())),
    ))
}

fn marker_distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_byte) in left.bytes().enumerate() {
        let mut current = vec![row + 1];
        for (column, right_byte) in right.bytes().enumerate() {
            let replacement = usize::from(!left_byte.eq_ignore_ascii_case(&right_byte));
            current.push(
                (previous[column] + replacement)
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GoTypeKind {
    Struct,
    Interface,
    Other,
}

#[derive(Clone, Debug)]
pub(crate) struct GoType {
    pub name: String,
    pub start: usize,
    pub keyword: Range<usize>,
    pub kind: GoTypeKind,
    pub body: Range<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct GoMethod {
    pub receiver_type: String,
    pub name: String,
    pub start: usize,
    pub parameters: Range<usize>,
    pub results: Range<usize>,
    pub body: Range<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct GoMethodSpec {
    pub name: String,
    pub start: usize,
    pub parameters: Range<usize>,
    pub results: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoField {
    pub name: String,
    pub type_source: String,
    pub start: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoParameter {
    pub name: String,
    pub type_source: String,
}

pub(crate) struct GoFile {
    pub name: String,
    pub path: String,
    pub start: usize,
    pub text: String,
}

impl GoFile {
    pub(crate) const MARKER: &'static str = "//tachyon:file ";
    const ENTRY_NAME: &'static str = "handler";

    pub(crate) fn role_separator(name: &str, path: &str) -> String {
        format!("\n{}{name} {path}\n", Self::MARKER)
    }

    pub(crate) fn split(program: &str) -> Vec<Self> {
        let mut files = vec![Self {
            name: String::from(Self::ENTRY_NAME),
            path: String::new(),
            start: 0,
            text: String::new(),
        }];
        let mut offset = 0;
        for line in program.split_inclusive('\n') {
            offset += line.len();
            let content = line.trim_end_matches(['\n', '\r']);
            let Some(label) = content.strip_prefix(Self::MARKER) else {
                if let Some(file) = files.last_mut() {
                    file.text.push_str(line);
                }
                continue;
            };
            let label = label.trim();
            let (name, path) = label.split_once(' ').unwrap_or((label, ""));
            files.push(Self {
                name: name.to_owned(),
                path: path.trim().to_owned(),
                start: offset,
                text: String::new(),
            });
        }
        files
    }

    pub(crate) fn locate(failure: &Failure, program: &str) -> Failure {
        let files = Self::split(program);
        let mut diagnostics = Vec::new();
        for diagnostic in failure.diagnostics() {
            let mut diagnostic = diagnostic.clone();
            for span in &mut diagnostic.spans {
                let start = usize::try_from(span.start).unwrap_or(usize::MAX);
                let Some(file) = files.iter().rev().find(|file| file.start <= start) else {
                    continue;
                };
                if file.path.is_empty() {
                    continue;
                }
                let moved = file.start as u64;
                span.file.clone_from(&file.path);
                span.start -= moved;
                span.end = span.end.saturating_sub(moved);
            }
            diagnostics.push(diagnostic);
        }
        Failure::new(diagnostics)
    }
}

pub(crate) fn types(code: &str) -> Vec<GoType> {
    let mut found = Vec::new();
    for (at, word) in top_level_words(code) {
        if word != "type" {
            continue;
        }
        if let Some(declared) = type_at(code, at) {
            found.push(declared);
        }
    }
    found
}

pub(crate) fn methods(code: &str) -> Vec<GoMethod> {
    let mut found = Vec::new();
    for (at, word) in top_level_words(code) {
        if word != "func" {
            continue;
        }
        if let Some(method) = method_at(code, at) {
            found.push(method);
        }
    }
    found
}

pub(crate) fn methods_of<'a>(methods: &'a [GoMethod], type_name: &str) -> Vec<&'a GoMethod> {
    let mut owned = Vec::new();
    for method in methods {
        if method.receiver_type == type_name {
            owned.push(method);
        }
    }
    owned
}

pub(crate) fn type_at(code: &str, at: usize) -> Option<GoType> {
    if word_at(code, at) != "type" {
        return None;
    }
    let name_at = skip_spaces(code, at + "type".len());
    let name = word_at(code, name_at);
    if name.is_empty() {
        return None;
    }
    let mut after_name = skip_spaces(code, name_at + name.len());
    if code.as_bytes().get(after_name) == Some(&b'=') {
        return None;
    }
    if code.as_bytes().get(after_name) == Some(&b'[') {
        after_name = skip_spaces(code, matching(code, after_name, b'[', b']')? + 1);
    }
    let keyword_word = word_at(code, after_name);
    let keyword = after_name..after_name + keyword_word.len();
    let kind = match keyword_word {
        "struct" => GoTypeKind::Struct,
        "interface" => GoTypeKind::Interface,
        _ => GoTypeKind::Other,
    };
    if kind == GoTypeKind::Other {
        let end = line_end(code, after_name);
        return Some(GoType {
            name: name.to_owned(),
            start: at,
            keyword,
            kind,
            body: end..end,
        });
    }
    let open = skip_spaces(code, keyword.end);
    if code.as_bytes().get(open) != Some(&b'{') {
        return None;
    }
    let close = matching(code, open, b'{', b'}')?;
    Some(GoType {
        name: name.to_owned(),
        start: at,
        keyword,
        kind,
        body: open + 1..close,
    })
}

pub(crate) fn method_at(code: &str, at: usize) -> Option<GoMethod> {
    if word_at(code, at) != "func" {
        return None;
    }
    let receiver_open = skip_spaces(code, at + "func".len());
    if code.as_bytes().get(receiver_open) != Some(&b'(') {
        return None;
    }
    let receiver_close = matching(code, receiver_open, b'(', b')')?;
    let (_, receiver_type) = receiver(&code[receiver_open + 1..receiver_close])?;
    let name_at = skip_spaces(code, receiver_close + 1);
    let name = word_at(code, name_at);
    if name.is_empty() {
        return None;
    }
    let parameters_open = skip_spaces(code, name_at + name.len());
    if code.as_bytes().get(parameters_open) != Some(&b'(') {
        return None;
    }
    let parameters_close = matching(code, parameters_open, b'(', b')')?;
    let body_open = body_open(code, parameters_close + 1)?;
    let body_close = matching(code, body_open, b'{', b'}')?;
    Some(GoMethod {
        receiver_type,
        name: name.to_owned(),
        start: at,
        parameters: parameters_open + 1..parameters_close,
        results: parameters_close + 1..body_open,
        body: body_open + 1..body_close,
    })
}

pub(crate) fn interface_methods(code: &str, body: Range<usize>) -> Vec<GoMethodSpec> {
    let mut found = Vec::new();
    for line in member_lines(code, body) {
        let name = word_at(code, line.start);
        let open = line.start + name.len();
        if name.is_empty() || code.as_bytes().get(open) != Some(&b'(') {
            continue;
        }
        let Some(close) = matching(code, open, b'(', b')') else {
            continue;
        };
        if close >= line.end {
            continue;
        }
        found.push(GoMethodSpec {
            name: name.to_owned(),
            start: line.start,
            parameters: open + 1..close,
            results: close + 1..line.end,
        });
    }
    found
}

pub(crate) fn fields(code: &str, body: Range<usize>) -> Vec<GoField> {
    let mut found = Vec::new();
    for line in member_lines(code, body) {
        let text = &code[line.clone()];
        if text.starts_with('@') {
            continue;
        }
        let words = text.split_whitespace().collect::<Vec<_>>();
        let mut names = Vec::new();
        let mut index = 0;
        while index < words.len() {
            let word = words[index];
            index += 1;
            if let Some(name) = word.strip_suffix(',') {
                names.push(name);
                continue;
            }
            names.push(word);
            break;
        }
        let type_source = words[index..].join(" ");
        if type_source.is_empty() {
            continue;
        }
        for name in names {
            found.push(GoField {
                name: name.to_owned(),
                type_source: type_source.clone(),
                start: line.start,
            });
        }
    }
    found
}

pub(crate) fn parameters(text: &str) -> Vec<GoParameter> {
    let mut found = Vec::new();
    let mut waiting_names = Vec::new();
    for part in split_top_level(text, b',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((name, type_source)) = part.split_once(char::is_whitespace) else {
            waiting_names.push(part.to_owned());
            continue;
        };
        let type_source = type_source.trim().to_owned();
        for waiting in waiting_names.drain(..) {
            found.push(GoParameter {
                name: waiting,
                type_source: type_source.clone(),
            });
        }
        found.push(GoParameter {
            name: name.to_owned(),
            type_source,
        });
    }
    for unnamed in waiting_names {
        found.push(GoParameter {
            name: String::new(),
            type_source: unnamed,
        });
    }
    found
}

pub(crate) fn split_top_level(text: &str, separator: u8) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i64;
    let mut start = 0;
    for (at, byte) in text.bytes().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if byte == separator && depth == 0 => {
                parts.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

pub(crate) fn lower_first(name: &str) -> String {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    let mut lowered = first.to_ascii_lowercase().to_string();
    lowered.push_str(characters.as_str());
    lowered
}

pub(crate) fn upper_first(name: &str) -> String {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    let mut raised = first.to_ascii_uppercase().to_string();
    raised.push_str(characters.as_str());
    raised
}

pub(crate) fn matching(code: &str, open: usize, left: u8, right: u8) -> Option<usize> {
    let mut depth = 0_usize;
    for (offset, byte) in code.as_bytes()[open..].iter().enumerate() {
        if *byte == left {
            depth += 1;
        } else if *byte == right {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(open + offset);
            }
        }
    }
    None
}

pub(crate) fn receiver_name(signature: &str) -> String {
    let Some(rest) = signature.trim_start().strip_prefix("func") else {
        return String::new();
    };
    let rest = rest.trim_start();
    if !rest.starts_with('(') {
        return String::new();
    }
    let Some(close) = matching(rest, 0, b'(', b')') else {
        return String::new();
    };
    receiver(&rest[1..close]).map_or_else(String::new, |(name, _)| name)
}

fn receiver(text: &str) -> Option<(String, String)> {
    let words = text
        .split(|character: char| character.is_whitespace() || character == '*')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let (name, type_name) = match words.as_slice() {
        [type_name] => ("", *type_name),
        [name, type_name] => (*name, *type_name),
        _ => return None,
    };
    let type_name = type_name.split('[').next().unwrap_or(type_name);
    if !is_identifier(type_name) {
        return None;
    }
    Some((name.to_owned(), type_name.to_owned()))
}

fn body_open(code: &str, mut at: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    let mut previous_word = "";
    while at < bytes.len() {
        match bytes[at] {
            b'(' => at = matching(code, at, b'(', b')')? + 1,
            b'[' => at = matching(code, at, b'[', b']')? + 1,
            b'{' if matches!(previous_word, "struct" | "interface") => {
                at = matching(code, at, b'{', b'}')? + 1;
                previous_word = "";
            }
            b'{' => return Some(at),
            b'\n' if GoFunctionResults::continues(code, at, previous_word) => at += 1,
            b'\n' | b';' | b')' | b']' | b'}' | b',' | b'=' | b'&' => return None,
            byte if AsciiName::is_name_byte(byte) => {
                previous_word = word_at(code, at);
                at += previous_word.len();
            }
            _ => at += 1,
        }
    }
    None
}

fn member_lines(code: &str, body: Range<usize>) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut depth = 0_i64;
    let mut start = body.start;
    for at in body.clone() {
        let byte = code.as_bytes()[at];
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'\n' | b';' if depth == 0 => {
                push_trimmed(code, start..at, &mut lines);
                start = at + 1;
            }
            _ => {}
        }
    }
    push_trimmed(code, start..body.end, &mut lines);
    lines
}

fn push_trimmed(code: &str, range: Range<usize>, lines: &mut Vec<Range<usize>>) {
    let text = &code[range.clone()];
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let start = range.start + text.len() - text.trim_start().len();
    lines.push(start..start + trimmed.len());
}

fn top_level_words(code: &str) -> Vec<(usize, &str)> {
    let bytes = code.as_bytes();
    let mut words = Vec::new();
    let mut depth = 0_i64;
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if AsciiName::is_name_byte(byte) {
            let word = word_at(code, at);
            if depth == 0 {
                words.push((at, word));
            }
            at += word.len();
            continue;
        }
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    words
}

fn word_at(code: &str, at: usize) -> &str {
    let bytes = code.as_bytes();
    if at > 0
        && bytes
            .get(at - 1)
            .is_some_and(|byte| AsciiName::is_name_byte(*byte))
    {
        return "";
    }
    let mut end = at;
    while end < bytes.len() && AsciiName::is_name_byte(bytes[end]) {
        end += 1;
    }
    &code[at..end]
}

fn skip_spaces(code: &str, mut at: usize) -> usize {
    let bytes = code.as_bytes();
    while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | b'\r') {
        at += 1;
    }
    at
}

fn line_end(code: &str, at: usize) -> usize {
    code[at..].find('\n').map_or(code.len(), |end| at + end)
}

fn is_identifier(text: &str) -> bool {
    let leading_digit = text.starts_with(|character: char| character.is_ascii_digit());
    !text.is_empty() && !leading_digit && text.bytes().all(AsciiName::is_name_byte)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    struct GoTooling;

    impl GoTooling {
        const SOURCE: &str = include_str!("../tests/fixtures/go-markers/OrdersController.go");

        fn run(
            command: &mut std::process::Command,
        ) -> Result<crate::external_command::ToolOutput, crate::external_command::ToolError>
        {
            // The bound only stops a hung tool. `go vet` compiles the module, which takes well over 30 s on a loaded machine.
            crate::external_command::run_sync(command, std::time::Duration::from_mins(3), 64 * 1024)
        }
    }

    #[test]
    fn go_comment_annotations_are_gofmt_stable_and_vet_clean() {
        let available = GoTooling::run(std::process::Command::new("go").arg("version"))
            .is_ok_and(|output| output.status.success());
        if !available {
            assert!(std::env::var_os("CI").is_none(), "Go is required in CI");
            eprintln!("skipping Go formatting and vet: Go is unavailable");
            return;
        }
        let project = tempfile::tempdir().expect("tooling project");
        std::fs::write(
            project.path().join("go.mod"),
            "module markerprobe\n\ngo 1.27\n",
        )
        .expect("module");
        let path = project.path().join("controller.go");
        std::fs::write(&path, GoTooling::SOURCE).expect("controller");
        let formatting =
            GoTooling::run(std::process::Command::new("gofmt").arg(&path)).expect("gofmt");
        assert!(formatting.status.success());
        assert_eq!(
            String::from_utf8(formatting.stdout).expect("formatted source"),
            GoTooling::SOURCE
        );
        let mut command = std::process::Command::new("go");
        command
            .args(["vet", "."])
            .current_dir(project.path())
            .env("GOTOOLCHAIN", "local")
            .env("GOPROXY", "off")
            .env("GOWORK", "off");
        let vet = GoTooling::run(&mut command).expect("go vet");
        assert!(
            vet.status.success(),
            "{}",
            String::from_utf8_lossy(&vet.stderr)
        );
    }

    use super::*;

    #[test]
    fn identifiers_keep_ascii_letter_or_underscore_start_and_word_continuation() {
        let alphabet = (0..=0x7f_u8)
            .map(char::from)
            .chain(['é', 'Ü', 'ß', '٣'])
            .collect::<Vec<_>>();
        let mut texts = vec![String::new()];
        for first in &alphabet {
            texts.push(first.to_string());
            texts.extend(alphabet.iter().map(|second| format!("{first}{second}")));
        }
        for text in texts {
            let mut bytes = text.bytes();
            let expected = bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
            assert_eq!(is_identifier(&text), expected, "{text:?}");
        }
        assert_eq!(
            receiver("catalog *Catalog[T]"),
            Some((String::from("catalog"), String::from("Catalog")))
        );
        assert_eq!(receiver("c *9Catalog"), None);
    }

    #[test]
    fn a_struct_and_its_methods_are_read_from_the_top_level() {
        let code = "package main\n\ntype Catalog struct {\n\tbehaviors *BehaviorContext\n\tcalls, misses int\n\tsync.Mutex\n}\n\nfunc (catalog *Catalog) List(page, size BehaviorValue) (BehaviorValue, error) {\n\tif true { return nil, nil }\n\treturn page, nil\n}\n\nfunc (Catalog) Kind() interface{ Name() string } { return nil }\n";
        let found = types(code);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Catalog");
        assert_eq!(found[0].kind, GoTypeKind::Struct);
        assert_eq!(&code[found[0].keyword.clone()], "struct");
        let members = fields(code, found[0].body.clone());
        let names = members
            .iter()
            .map(|field| (field.name.as_str(), field.type_source.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                ("behaviors", "*BehaviorContext"),
                ("calls", "int"),
                ("misses", "int")
            ]
        );
        let declared = methods(code);
        assert_eq!(declared.len(), 2);
        assert_eq!(
            receiver_name(&code[declared[0].start..declared[0].body.start]),
            "catalog"
        );
        assert_eq!(declared[0].receiver_type, "Catalog");
        assert_eq!(declared[0].name, "List");
        assert_eq!(
            code[declared[0].results.clone()].trim(),
            "(BehaviorValue, error)"
        );
        assert!(code[declared[0].body.clone()].contains("return page, nil"));
        assert_eq!(
            receiver_name(&code[declared[1].start..declared[1].body.start]),
            ""
        );
        assert_eq!(
            code[declared[1].results.clone()].trim(),
            "interface{ Name() string }"
        );
        assert_eq!(code[declared[1].body.clone()].trim(), "return nil");
    }

    #[test]
    fn grouped_and_unnamed_parameters_keep_their_types() {
        assert_eq!(
            parameters("mode, detail BehaviorValue, request *http.Request"),
            [
                GoParameter {
                    name: "mode".into(),
                    type_source: "BehaviorValue".into()
                },
                GoParameter {
                    name: "detail".into(),
                    type_source: "BehaviorValue".into()
                },
                GoParameter {
                    name: "request".into(),
                    type_source: "*http.Request".into()
                },
            ]
        );
        assert_eq!(parameters("string, int").len(), 2);
        assert_eq!(parameters("string, int")[0].name, "");
        assert!(parameters("").is_empty());
    }

    #[test]
    fn interface_method_specs_and_other_types_are_read() {
        let code = "type Mapper interface {\n\t// @Convert({})\n\tToSummary(product BehaviorValue) (BehaviorValue, error)\n}\ntype Status string\ntype Alias = int\n";
        let found = types(code);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, GoTypeKind::Interface);
        let specs = interface_methods(code, found[0].body.clone());
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "ToSummary");
        assert_eq!(
            code[specs[0].results.clone()].trim(),
            "(BehaviorValue, error)"
        );
        assert_eq!(found[1].kind, GoTypeKind::Other);
        assert_eq!(found[1].name, "Status");
    }

    #[test]
    fn a_program_splits_into_named_files() {
        let program = format!(
            "type Handler struct{{}}\n{}package main\nfunc helper() {{}}\n\n{}behavior\npackage main\n",
            GoFile::role_separator("Catalog", "server/services/Catalog.go"),
            GoFile::MARKER
        );
        let files = GoFile::split(&program);
        let names = files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["handler", "Catalog", "behavior"]);
        assert_eq!(files[1].path, "server/services/Catalog.go");
        assert_eq!(files[2].path, "");
        assert_eq!(files[0].text, "type Handler struct{}\n\n");
        assert_eq!(files[1].text, "package main\nfunc helper() {}\n\n");
    }

    #[test]
    fn a_failure_in_a_role_file_names_that_file() {
        let program = format!(
            "type TallyService struct{{}}\n{}\n// @Helper\ntype StepHelper struct{{}}\n",
            GoFile::role_separator("StepHelper", "client/helpers/StepHelper.go")
        );
        let files = GoFile::split(&program);
        assert_eq!(files[1].path, "client/helpers/StepHelper.go");
        let marker = program.find("@Helper").expect("marker");
        let failure = Failure::one(crate::failure::diagnostic(
            1308,
            "refused",
            None,
            crate::failure::source_span("client/services/TallyService.go", marker, marker + 1),
        ));
        let located = GoFile::locate(&failure, &program);
        let span = &located.diagnostics()[0].spans[0];
        assert_eq!(span.file, "client/helpers/StepHelper.go");
        assert_eq!((span.start, span.end), (4, 5));
    }
}
