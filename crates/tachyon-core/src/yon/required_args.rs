mod fields;

use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

struct Input<'a> {
    path: &'a Path,
    source: &'a str,
    code: String,
    extension: &'a str,
}

impl Input<'_> {
    const MAX_SOURCE: usize = 1024 * 1024;
    const MAX_FIELDS: usize = 32;

    fn fail(&self, offset: usize, message: impl Into<String>) -> Failure {
        let help = "Use direct uninitialized instance fields with concrete provider types (Rust Arc<Provider>); JavaScript field names must exactly match lowerCamel provider names. NeedsAll accepts no arguments or an explicit ordered list of quoted fields.";
        let span = source_span(
            &self.path.to_string_lossy(),
            offset,
            (offset + 1).min(self.source.len()),
        );
        Failure::one(diagnostic(2015, message, Some(help.into()), span))
    }
}

struct Marker {
    name: String,
    arguments: String,
    span: Range<usize>,
}

struct Managed {
    name: String,
    body: Range<usize>,
    declaration: usize,
    indent: usize,
    constructor: Option<Marker>,
    injected: bool,
}

pub(crate) fn required_fields(
    path: &Path,
    source: &str,
    body: Range<usize>,
) -> Result<Vec<(String, String)>, Failure> {
    let input = Input {
        path,
        source,
        code: crate::annotation_layout::masked_source(path, source)?,
        extension: crate::groovy_source::GroovySource::syntax(
            path.extension()
                .and_then(|value| value.to_str())
                .unwrap_or(""),
        ),
    };
    fields::collect(&input, &body).map(|fields| {
        fields
            .into_iter()
            .filter(|field| field.eligible)
            .map(|field| (field.name, field.ty))
            .collect()
    })
}

pub(crate) fn prepare(path: &Path, source: &str) -> Result<Option<String>, Failure> {
    let manual = super::manual_helpers::ManualHelpers::read(path, source)?;
    prepare_with_manual(path, source, &manual)
}

pub(crate) fn prepare_with_manual(
    path: &Path,
    source: &str,
    manual: &BTreeSet<String>,
) -> Result<Option<String>, Failure> {
    if !mentions_constructor(source) {
        return Ok(None);
    }
    let extension = crate::groovy_source::GroovySource::syntax(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
    );
    if extension == "rb" {
        return Ok(None);
    }
    if source.len() > Input::MAX_SOURCE {
        return Err(Failure::one(diagnostic(
            2015,
            "Yon constructor source exceeds 1 MiB",
            None,
            None,
        )));
    }
    let code = crate::annotation_layout::masked_source(path, source)?;
    let input = Input {
        path,
        source,
        code,
        extension,
    };
    if !active_constructor(&input.code, extension) {
        return Ok(None);
    }
    expand_source(&input, manual)
}

fn expand_source(input: &Input<'_>, manual: &BTreeSet<String>) -> Result<Option<String>, Failure> {
    let managed = declarations(input)?;
    let providers: BTreeSet<_> = managed
        .iter()
        .map(|item| item.name.as_str())
        .chain(["DataSource", "RedisTemplate", "ApplicationEventPublisher"])
        .collect();
    let mut edits = Vec::new();
    for item in managed.iter().filter(|item| !manual.contains(&item.name)) {
        expand(input, item, &providers, &mut edits)?;
    }
    if edits.is_empty() {
        return Ok(None);
    }
    let mut output = input.source.to_owned();
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, replacement) in edits {
        output.replace_range(range, &replacement);
    }
    if output.len() > Input::MAX_SOURCE {
        return Err(input.fail(0, "Expanded Yon constructor source exceeds 1 MiB"));
    }
    Ok(Some(output))
}

fn expand(
    input: &Input<'_>,
    item: &Managed,
    providers: &BTreeSet<&str>,
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), Failure> {
    let body = &input.code[item.body.clone()].to_ascii_lowercase();
    if !body.contains("needs") && !body.contains("named") {
        if item.constructor.is_none() {
            return Ok(());
        }
        let candidates = fields::collect(input, &item.body)?;
        return expand_required_constructor(input, item, &candidates, providers, edits);
    }
    let (candidates, markers) = fields::collect_with_markers(input, &item.body)?;
    let selected = candidates
        .iter()
        .filter(|field| field.autowired)
        .collect::<Vec<_>>();
    for span in markers {
        let blanked = blank_marker(&input.source[span.clone()]);
        edits.push((span, blanked));
    }
    if selected.is_empty() || item.constructor.is_some() {
        return expand_required_constructor(input, item, &candidates, providers, edits);
    }
    let tokens = autowired_tokens(input, item, &selected, providers)?;
    let arguments = selected
        .iter()
        .map(|field| format!("\"{}\"", field.name))
        .collect::<Vec<_>>()
        .join(", ");
    let indent = " ".repeat(item.indent);
    let generated = format!(
        "{}\n{indent}{}\n{indent}",
        annotation(input.extension, "NeedsAll", &arguments),
        annotation(input.extension, "Needs", &tokens.join(", "))
    );
    edits.push((item.declaration..item.declaration, generated));
    Ok(())
}

fn blank_marker(source: &str) -> String {
    source
        .bytes()
        .map(|byte| match byte {
            b'\n' | b'\r' => char::from(byte),
            _ => ' ',
        })
        .collect()
}

fn autowired_tokens(
    input: &Input<'_>,
    item: &Managed,
    selected: &[&fields::Field],
    providers: &BTreeSet<&str>,
) -> Result<Vec<String>, Failure> {
    if item.body.is_empty()
        && input.code.as_bytes().get(item.body.end) != Some(&b'}')
        && input.extension != "py"
    {
        return Err(input.fail(
            item.declaration,
            "Generated constructors require a named type body enclosed in braces",
        ));
    }
    if selected.len() > Input::MAX_FIELDS {
        return Err(input.fail(
            item.declaration,
            "Needs selects at most 32 fields on one type",
        ));
    }
    let mut names = BTreeSet::new();
    let mut tokens = Vec::new();
    for field in selected {
        if !field.eligible {
            return Err(input.fail(
                item.declaration,
                format!(
                    "Needs field '{}' must be a direct uninitialized, non-static instance field",
                    field.name
                ),
            ));
        }
        if !names.insert(field.name.as_str()) {
            return Err(input.fail(item.declaration, "Needs fields must be unique"));
        }
        tokens.push(autowired_token(input, item, field, providers)?);
    }
    Ok(tokens)
}

fn autowired_token(
    input: &Input<'_>,
    item: &Managed,
    field: &fields::Field,
    providers: &BTreeSet<&str>,
) -> Result<String, Failure> {
    let Some(provider) = &field.qualifier else {
        return provider_token(input, item, field, providers);
    };
    if providers.contains(provider.as_str()) {
        return Ok(provider.clone());
    }
    Err(input.fail(
        item.declaration,
        format!("Named '{provider}' names no Yon provider in this entry scope"),
    ))
}

fn expand_required_constructor(
    input: &Input<'_>,
    item: &Managed,
    candidates: &[fields::Field],
    providers: &BTreeSet<&str>,
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), Failure> {
    let Some(marker) = &item.constructor else {
        return Ok(());
    };
    if item.injected && !marker.arguments.is_empty() {
        return Ok(());
    }
    let selected = selected_fields(marker, candidates);
    let tokens = required_tokens(input, item, candidates, &selected, providers)?;
    let arguments = selected
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let indent = " ".repeat(item.indent);
    let mut generated = annotation(input.extension, "NeedsAll", &arguments);
    if input.extension == "go" {
        generated.drain(..3);
    }
    if !item.injected {
        generated.push('\n');
        generated.push_str(&indent);
        generated.push_str(&annotation(input.extension, "Needs", &tokens.join(", ")));
    }
    edits.push((marker.span.clone(), generated));
    Ok(())
}

fn selected_fields(marker: &Marker, candidates: &[fields::Field]) -> Vec<String> {
    if marker.arguments.is_empty() {
        candidates
            .iter()
            .filter(|field| field.eligible)
            .map(|field| field.name.clone())
            .collect::<Vec<_>>()
    } else {
        marker
            .arguments
            .split(',')
            .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
            .filter(|value| !value.is_empty())
            .collect()
    }
}

fn required_tokens(
    input: &Input<'_>,
    item: &Managed,
    candidates: &[fields::Field],
    selected: &[String],
    providers: &BTreeSet<&str>,
) -> Result<Vec<String>, Failure> {
    if selected.len() > Input::MAX_FIELDS {
        return Err(input.fail(item.declaration, "NeedsAll selects at most 32 fields"));
    }
    let fields: std::collections::BTreeMap<_, _> = candidates
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect();
    if fields.len() != candidates.len() {
        return Err(input.fail(
            item.declaration,
            "Constructor fields must be uniquely declared",
        ));
    }
    let mut tokens = Vec::new();
    for name in selected {
        let field = fields
            .get(name.as_str())
            .filter(|field| field.eligible)
            .ok_or_else(|| input.fail(item.declaration, format!("Required field '{name}' must be a direct uninitialized, non-static instance field")))?;
        if !item.injected {
            tokens.push(provider_token(input, item, field, providers)?);
        }
    }
    Ok(tokens)
}

fn provider_token(
    input: &Input<'_>,
    item: &Managed,
    field: &fields::Field,
    providers: &BTreeSet<&str>,
) -> Result<String, Failure> {
    let wanted = if input.extension == "js" {
        upper_camel(&field.name)
    } else {
        let compact: String = field
            .ty
            .chars()
            .filter(|value| !value.is_whitespace())
            .collect();
        concrete_type(input.extension, &compact).to_owned()
    };
    let token = providers
        .iter()
        .filter(|provider| {
            if input.extension == "js" {
                lower_camel(provider) == field.name
            } else {
                **provider == wanted
            }
        })
        .copied()
        .collect::<Vec<_>>();
    match token.as_slice() {
        [provider] => Ok((*provider).to_owned()),
        [] => Err(input.fail(
            item.declaration,
            format!(
                "Unresolved dependency {wanted}: {} needs field '{}' of type {}, but no provider {wanted} is declared",
                item.name,
                field.name,
                if field.ty.is_empty() { &wanted } else { &field.ty }
            ),
        )),
        _ => Err(input.fail(
            item.declaration,
            format!(
                "{} needs field '{}', but more than one provider matches it; name one with Named",
                item.name, field.name
            ),
        )),
    }
}

fn upper_camel(name: &str) -> String {
    let mut characters = name.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + characters.as_str()
    })
}

fn concrete_type<'a>(extension: &str, compact: &'a str) -> &'a str {
    if extension == "go" {
        return compact.strip_prefix('*').unwrap_or(compact);
    }
    if extension != "rs" {
        return compact;
    }
    compact
        .strip_prefix("Arc<")
        .or_else(|| compact.strip_prefix("std::sync::Arc<"))
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or("")
}

fn lower_camel(name: &str) -> String {
    let run = name.bytes().take_while(u8::is_ascii_uppercase).count();
    let count = if run > 1 && run < name.len() {
        run - 1
    } else {
        run
    };
    let mut bytes = name.as_bytes().to_vec();
    bytes[..count].make_ascii_lowercase();
    String::from_utf8(bytes).unwrap_or_default()
}

fn mentions_constructor(source: &str) -> bool {
    let source = source.to_ascii_lowercase();
    source.contains("needs")
}

fn active_constructor(code: &str, extension: &str) -> bool {
    if extension == "py" {
        let annotated = crate::python_parameters::PythonAnnotated::markers(code, code, |written| {
            written.eq_ignore_ascii_case("needs")
        });
        if !annotated.markers.is_empty() {
            return true;
        }
    }
    let prefix = match extension {
        "php" | "rs" => "#[",
        "cs" => "[",
        _ => "@",
    };
    for line in code.lines() {
        let Some(tail) = line.trim_start().strip_prefix(prefix) else {
            continue;
        };
        let name = tail
            .split(|value: char| !value.is_ascii_alphanumeric() && !matches!(value, '_' | ':'))
            .next()
            .unwrap_or("");
        let name = name.strip_prefix("yon::").unwrap_or(name).replace('_', "");
        if name.eq_ignore_ascii_case("Needs") || name.eq_ignore_ascii_case("NeedsAll") {
            return true;
        }
    }
    false
}

fn annotation(extension: &str, name: &str, arguments: &str) -> String {
    match extension {
        "rs" => format!("#[yon::{name}({arguments})]"),
        "go" => format!("// @yon::{name}({arguments})"),
        "php" => format!("#[{name}({arguments})]"),
        "cs" => format!("[{name}({arguments})]"),
        _ => format!("@{name}({arguments})"),
    }
}

fn declarations(input: &Input<'_>) -> Result<Vec<Managed>, Failure> {
    let mut output = Vec::new();
    let mut pending = Vec::new();
    let mut cursor = 0;
    while cursor < input.code.len() {
        let end = input.code[cursor..]
            .find('\n')
            .map_or(input.code.len(), |at| cursor + at + 1);
        let line = &input.code[cursor..end];
        let start = cursor + line.len() - line.trim_start().len();
        if line.trim().is_empty() {
            cursor = end;
            continue;
        }
        if let Some(marker) = marker(input, start, end)? {
            pending.push(marker);
            cursor = end;
            continue;
        }
        let Some(name) = class_name(input.extension, line) else {
            pending.clear();
            cursor = end;
            continue;
        };
        let body = body(input, start)?;
        let item = Managed {
            name: name.to_owned(),
            body: body.clone(),
            declaration: start,
            indent: line.len() - line.trim_start().len(),
            constructor: None,
            injected: false,
        };
        append_managed(input, item, pending, &mut output)?;
        pending = Vec::new();
        cursor = if input.extension == "py" || (input.extension == "kt" && body.is_empty()) {
            body.end
        } else {
            body.end + 1
        };
    }
    Ok(output)
}

fn append_managed(
    input: &Input<'_>,
    mut item: Managed,
    markers: Vec<Marker>,
    output: &mut Vec<Managed>,
) -> Result<(), Failure> {
    let managed = markers
        .iter()
        .any(|marker| managed_role(&marker.name) && accepts_arguments(marker));
    if !managed {
        return Ok(());
    }
    if output.len() >= 128 {
        return Err(input.fail(
            item.declaration,
            "Yon permits at most 128 managed providers",
        ));
    }
    item.injected = markers.iter().any(|marker| marker.name == "needs");
    let mut constructors = markers
        .into_iter()
        .filter(|marker| marker.name == "needsall");
    item.constructor = constructors.next();
    if constructors.next().is_some() {
        return Err(input.fail(item.declaration, "Duplicate NeedsAll marker"));
    }
    output.push(item);
    Ok(())
}

fn marker_arguments(
    input: &Input<'_>,
    line: &str,
    start: usize,
) -> Result<Option<Range<usize>>, Failure> {
    let Some(opening) = line.find('(') else {
        return Ok(None);
    };
    let closing = matching(&input.code, start + opening, b'(', b')')
        .ok_or_else(|| input.fail(start, "Unbalanced constructor annotation"))?;
    if closing >= start + line.len() {
        return Err(input.fail(start, "Constructor markers must occupy one complete line"));
    }
    Ok(Some(start + opening + 1..closing))
}

fn validate_marker_suffix(
    input: &Input<'_>,
    start: usize,
    suffix: &str,
    prefix: &str,
) -> Result<(), Failure> {
    let expected = if prefix == "@" { "" } else { "]" };
    if suffix.trim() != expected {
        return Err(input.fail(start, "NeedsAll must occupy one complete annotation line"));
    }
    Ok(())
}

fn marker(input: &Input<'_>, start: usize, end: usize) -> Result<Option<Marker>, Failure> {
    let line = input.code[start..end].trim_end();
    let prefix = match input.extension {
        "rs" | "php" => "#[",
        "cs" => "[",
        _ => "@",
    };
    let Some(tail) = line.strip_prefix(prefix) else {
        return Ok(None);
    };
    let length = tail
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        .count();
    let name = tail[..length]
        .strip_prefix("yon::")
        .unwrap_or(&tail[..length])
        .replace('_', "")
        .to_ascii_lowercase();
    let marker_end = start + line.len();
    let arguments_span = marker_arguments(input, line, start)?;
    let arguments = arguments_span
        .as_ref()
        .map(|range| input.source[range.clone()].trim().to_owned())
        .unwrap_or_default();
    if name == "needsall" {
        let suffix = arguments_span
            .as_ref()
            .map_or(&line[prefix.len() + length..], |range| {
                &input.code[range.end + 1..marker_end]
            });
        validate_marker_suffix(input, start, suffix, prefix)?;
    }
    Ok(Some(Marker {
        name,
        arguments,
        span: start..marker_end,
    }))
}

fn accepts_arguments(marker: &Marker) -> bool {
    marker.arguments.is_empty()
        || matches!(
            marker.name.as_str(),
            "requestroute" | "socketroute" | "httpclient"
        )
}

fn managed_role(name: &str) -> bool {
    matches!(
        name,
        "service"
            | "requestroute"
            | "socketroute"
            | "store"
            | "client"
            | "httpclient"
            | "helper"
            | "converter"
    )
}

fn class_name<'a>(extension: &str, line: &'a str) -> Option<&'a str> {
    let mut tokens = line.split_whitespace();
    if extension == "go" {
        if tokens.next() != Some("type") {
            return None;
        }
        return tokens
            .next()?
            .split(|value: char| !value.is_ascii_alphanumeric() && value != '_')
            .next()
            .filter(|name| identifier(name));
    }
    while let Some(token) = tokens.next() {
        if matches!(token, "class" | "struct") {
            return tokens
                .next()?
                .split(|value: char| !value.is_ascii_alphanumeric() && value != '_')
                .next()
                .filter(|name| identifier(name));
        }
        if !matches!(
            token,
            "public"
                | "private"
                | "internal"
                | "sealed"
                | "final"
                | "abstract"
                | "pub"
                | "export"
                | "default"
                | "open"
                | "partial"
                | "readonly"
                | "data"
                | "pub(crate)"
                | "pub(super)"
        ) {
            return None;
        }
    }
    None
}

fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    !value.is_empty()
        && value.len() <= 128
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn body(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
    if input.extension == "py" {
        return python_body(input, start);
    }
    if input.extension == "kt" {
        let end = input.code[start..]
            .find('\n')
            .map_or(input.code.len(), |at| start + at + 1);
        if !input.code[start..end].contains('{') {
            return Ok(end..end);
        }
    }
    let opening = input.code[start..]
        .find(['{', ';'])
        .map(|at| start + at)
        .ok_or_else(|| input.fail(start, "Managed constructor types require named bodies"))?;
    if input.code.as_bytes()[opening] == b';' {
        return Ok(opening..opening);
    }
    let closing = matching(&input.code, opening, b'{', b'}')
        .ok_or_else(|| input.fail(start, "Unbalanced managed type body"))?;
    Ok(opening + 1..closing)
}

fn python_body(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
    let opening = input.code[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .ok_or_else(|| input.fail(start, "Python managed types require multiline bodies"))?;
    let mut end = opening;
    for line in input.code[opening..].split_inclusive('\n') {
        if !line.trim().is_empty() && !line.starts_with(char::is_whitespace) {
            break;
        }
        end += line.len();
    }
    Ok(opening..end)
}

fn matching(code: &str, start: usize, opening: u8, closing: u8) -> Option<usize> {
    let mut depth = 0_usize;
    for (offset, byte) in code.as_bytes()[start..].iter().enumerate() {
        if *byte == opening {
            depth += 1;
        }
        if *byte == closing {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_owned_helper_field_needs_retains_authored_construction() {
        let source = "[Helper]\nclass ProbeHelper {\n [Needs]\n public BehaviorContext behaviors;\n}\nclass Caller { object value = Behaviors.construct(typeof(ProbeHelper), context); }";
        let output =
            prepare(Path::new("yon.cs"), source).unwrap_or_else(|failure| panic!("{failure}"));
        assert!(output.is_none(), "{output:?}");
        assert!(source.contains("[Needs]"));
    }

    #[test]
    fn autowired_fields_generate_the_constructor() {
        let source = "@Store()\nclass OrderRepository {}\n@Service()\nclass OrderService {\n  @Needs()\n  orderRepository;\n  count = 0;\n}\n";
        let output = prepared("js", source);
        assert!(
            output.contains("@NeedsAll(\"orderRepository\")"),
            "{output}"
        );
        assert!(output.contains("@Needs(OrderRepository)"), "{output}");
        assert!(!output.contains("@Needs()"), "{output}");
    }

    #[test]
    fn a_typescript_declare_field_is_only_a_type() {
        let body = "class ChatClient {\n  declare send: SocketSend<ChatMessage>;\n  chatStore!: ChatStore;\n}\n";
        let inside = body.find('{').unwrap_or_default() + 1..body.rfind('}').unwrap_or_default();
        let fields = required_fields(Path::new("client/remotes/ChatClient.ts"), body, inside)
            .unwrap_or_default();
        let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["chatStore"]);
    }

    #[test]
    fn a_javascript_private_field_is_never_required() {
        let body = "class SnippetService {\n  static #FEWEST_CHOICES = 2\n  #platform = 'web'\n  files\n}\n";
        let inside = body.find('{').unwrap_or_default() + 1..body.rfind('}').unwrap_or_default();
        let fields = required_fields(Path::new("client/services/SnippetService.js"), body, inside)
            .unwrap_or_default();
        let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["files"]);
    }

    #[test]
    fn a_qualifier_names_the_provider() {
        let source = "@Store()\nclass OrderRepository {}\n@Service()\nclass OrderService {\n  @Needs()\n  @Named(\"OrderRepository\")\n  store;\n}\n";
        let output = prepared("js", source);
        assert!(output.contains("@Needs(OrderRepository)"), "{output}");
        assert!(output.contains("@NeedsAll(\"store\")"), "{output}");
        let unknown = source.replace("OrderRepository\")", "Absent\")");
        assert!(prepare(Path::new("yon.js"), &unknown).is_err());
    }

    #[test]
    fn python_fields_carry_needs_and_named_inside_annotated() {
        let source = "from typing import Annotated\n\n@store()\nclass OrderRepository:\n    pass\n@service()\nclass OrderService:\n    store: Annotated[OrderRepository, needs, named(\"OrderRepository\")]\n    count: Annotated[int, min(1)] = 1\n";
        let output = prepared("py", source);
        assert!(output.contains("@NeedsAll(\"store\")"), "{output}");
        assert!(output.contains("@Needs(OrderRepository)"), "{output}");
        let blank = " ".repeat("Annotated[".len());
        assert!(
            output.contains(&format!("    store: {blank}OrderRepository")),
            "{output}"
        );
        assert!(
            output.contains("    count: Annotated[int, min(1)] = 1"),
            "{output}"
        );
        assert!(!output.contains("needs"), "{output}");
    }

    fn prepared(extension: &str, source: &str) -> String {
        prepared_option(extension, source).unwrap_or_else(|| panic!("expected inference"))
    }

    fn prepared_option(extension: &str, source: &str) -> Option<String> {
        prepare(Path::new(&format!("yon.{extension}")), source)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn preserves_exact_source_when_blanking_autowired_markers() {
        let source = "@Store()\r\nclass OrderRepository {}\r\n@Service()\r\nclass OrderService {\r\n  @Needs()\r\n  orderRepository;\r\n  count = 0;\r\n}\r\n";
        let expected = "@Store()\r\nclass OrderRepository {}\r\n@Service()\r\n@NeedsAll(\"orderRepository\")\n@Needs(OrderRepository)\nclass OrderService {\r\n          \r\n  orderRepository;\r\n  count = 0;\r\n}\r\n";
        assert_eq!(prepared("js", source), expected);
    }

    #[test]
    fn preserves_exact_source_with_explicit_field_order() {
        let source = "@Service()\nclass LeftService {}\n@Service()\nclass RightService {}\n@RequestRoute(\"/money\")\n@NeedsAll('right','left')\nclass MoneyController {\nleft: LeftService;\nright: RightService;\n}";
        let expected = "@Service()\nclass LeftService {}\n@Service()\nclass RightService {}\n@RequestRoute(\"/money\")\n@NeedsAll(\"right\", \"left\")\n@Needs(RightService, LeftService)\nclass MoneyController {\nleft: LeftService;\nright: RightService;\n}";
        assert_eq!(prepared("ts", source), expected);
    }

    #[test]
    fn infers_fields_in_every_yon_language() {
        for (extension, field) in [
            ("js", "moneyService;"),
            ("ts", "readonly money!: MoneyService;"),
            ("java", "final MoneyService money;"),
            ("kt", "private val money: MoneyService"),
            ("cs", "readonly MoneyService money;"),
            ("php", "private MoneyService $money;"),
            ("rs", "money: std::sync::Arc<MoneyService>,"),
        ] {
            let kind = if extension == "rs" { "struct" } else { "class" };
            let source = format!(
                "{}\n{kind} MoneyService {{}}\n{}\n{}\n{kind} MoneyController {{\n{field}\n}}",
                annotation(extension, "Service", ""),
                annotation(extension, "RequestRoute", "\"/money\""),
                annotation(extension, "NeedsAll", "")
            );
            let output = prepared(extension, &source);
            assert!(
                output.contains("Needs(MoneyService)"),
                "{extension}: {output}"
            );
            assert!(output.contains(if extension == "js" {
                "NeedsAll(\"moneyService\")"
            } else {
                "NeedsAll(\"money\")"
            }));
        }
        let go = "// @Service\ntype MoneyService struct{}\n// @RequestRoute(\"/money\")\n// @NeedsAll\ntype MoneyController struct {\n\tmoney *MoneyService\n\tsync.Mutex\n}\n";
        let output = prepared("go", go);
        assert!(
            output.contains("// @yon::Needs(MoneyService)\n"),
            "{output}"
        );
        assert!(
            output.contains("// @yon::NeedsAll(\"money\")\n"),
            "{output}"
        );
        let python = "@service()\nclass MoneyService:\n    pass\n@request_route(\"/money\")\n@needs_all()\nclass MoneyController:\n    money: MoneyService\n";
        assert!(prepared("py", python).contains("@Needs(MoneyService)"));
    }

    #[test]
    fn a_needs_field_finds_a_converter() {
        for (extension, field) in [
            ("java", "ProductConverter converter;"),
            ("kt", "val converter: ProductConverter"),
            ("php", "private ProductConverter $converter;"),
        ] {
            let source = format!(
                "{}\nclass ProductConverter {{}}\n{}\n{}\nclass CatalogController {{\n{field}\n}}",
                annotation(extension, "Converter", ""),
                annotation(extension, "RequestRoute", "\"/catalog\""),
                annotation(extension, "NeedsAll", "")
            );
            let output = prepared(extension, &source);
            assert!(
                output.contains("Needs(ProductConverter)"),
                "{extension}: {output}"
            );
        }
    }

    #[test]
    fn explicit_field_order_controls_dependency_order() {
        let source = "@Service()\nclass LeftService {}\n@Service()\nclass RightService {}\n@RequestRoute(\"/money\")\n@NeedsAll('right','left')\nclass MoneyController {\nleft: LeftService;\nright: RightService;\n}";
        assert!(prepared("ts", source).contains("@Needs(RightService, LeftService)"));
    }

    #[test]
    fn preserves_initializers_static_fields_methods_and_annotation_arguments() {
        let source = "@Service()\nclass MoneyService {}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {\nstatic ignored;\nitems = [\n1,\n2\n];\nsettings = {nested:{value:1}};\ncallback = () => { return {value:1}; };\n@Cached({\"keyArgs\":[]})\nGET() { const local = {moneyService: 0}; }\nget other() { return {}; }\nmoneyService;\n}";
        let output = prepared("js", source);
        assert!(output.contains("NeedsAll(\"moneyService\")"));
        assert!(output.contains("items = [\n1,\n2\n]"));
    }

    #[test]
    fn preserves_legacy_explicit_injection_and_nonmanaged_constructors() {
        let source =
            "@RequestRoute(\"/money\")\n@NeedsAll('old')\n@Needs(Other)\nclass MoneyController {}";
        assert_eq!(prepared_option("js", source), None);
        let source = "@NeedsAll('value')\nclass Ordinary {}";
        assert_eq!(prepared_option("js", source), None);
    }

    #[test]
    fn rejects_ambiguous_missing_initialized_static_and_generic_dependencies() {
        for source in [
            "@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController { missingService; }",
            "@Service()\nclass MoneyService {}\n@Service()\nclass moneyService {}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController { moneyService; }",
            "@RequestRoute(\"/money\")\n@NeedsAll('moneyService')\nclass MoneyController { moneyService = 1; }",
            "@RequestRoute(\"/money\")\n@NeedsAll('moneyService')\nclass MoneyController { static moneyService; }",
            "@RequestRoute(\"/money\")\n@NeedsAll trailing\nclass MoneyController {}",
        ] {
            assert!(prepare(Path::new("yon.js"), source).is_err(), "{source}");
        }
        for ty in [
            "MoneyService | undefined",
            "Box<MoneyService>",
            "MoneyService[]",
        ] {
            let source = format!(
                "@Service()\nclass MoneyService {{}}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {{ money: {ty}; }}"
            );
            assert!(prepare(Path::new("yon.ts"), &source).is_err(), "{ty}");
        }
    }

    #[test]
    fn ignores_literal_markers_and_preserves_bodyless_kotlin_providers() {
        let source = "class Plain { text = '@NeedsAll()'; }";
        assert_eq!(prepared_option("js", source), None);
        let source = "# @needs_all()\nclass Helper: pass\nclass Handler: pass\n";
        assert_eq!(prepared_option("py", source), None);
        let source = "@Service()\nclass MoneyService\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {\nval money: MoneyService\n}";
        assert!(prepared("kt", source).contains("Needs(MoneyService)"));
    }

    #[test]
    fn enforces_dependency_count() {
        let fields = (0..33)
            .map(|index| format!("value{index}: MoneyService;"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!(
            "@Service()\nclass MoneyService {{}}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {{\n{fields}\n}}"
        );
        assert!(prepare(Path::new("yon.ts"), &source).is_err());
    }

    #[test]
    fn regex_literals_and_ordinary_identifier_mentions_are_inert() {
        let source = "@Service()\nclass MoneyService {}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {\npattern = /}/;\nmoneyService;\n}";
        assert!(prepared("js", source).contains("Needs(MoneyService)"));
        let source = "RequiredArgsConstructorDocs = 1\nclass Handler: GET = staticmethod(lambda request: {})";
        assert_eq!(prepared_option("py", source), None);
    }

    #[test]
    fn rejects_function_typed_required_fields_but_preserves_initialized_callbacks() {
        for (extension, field) in [
            ("ts", "callback: () => MoneyService;"),
            ("kt", "val callback: () -> MoneyService"),
            ("rs", "callback: fn() -> MoneyService,"),
        ] {
            let kind = if extension == "rs" { "struct" } else { "class" };
            let source = format!(
                "{}\n{kind} MoneyController {{\n{field}\n}}",
                [
                    annotation(extension, "RequestRoute", "\"/money\""),
                    annotation(extension, "NeedsAll", "")
                ]
                .join("\n")
            );
            assert!(
                prepare(Path::new(&format!("yon.{extension}")), &source).is_err(),
                "{extension}"
            );
        }
        let source = "@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {\ncallback: () => number = () => 1;\n}";
        assert!(prepared("ts", source).contains("Needs()"));
    }

    #[test]
    fn uses_acronym_lower_camel_and_retains_existing_provider_modifiers() {
        assert_eq!(lower_camel("HTTPClient"), "httpClient");
        assert_eq!(lower_camel("URL"), "url");
        let source = "@Client()\nclass HTTPClient {}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController { httpClient; }";
        assert!(prepared("js", source).contains("Needs(HTTPClient)"));
        let ambiguous = source.replace(
            "@RequestRoute(\"/money\")",
            "@Client()\nclass HttpClient {}\n@RequestRoute(\"/money\")",
        );
        assert!(prepare(Path::new("yon.js"), &ambiguous).is_err());
        let kotlin = "@Service()\nopen class MoneyService {}\n@RequestRoute(\"/money\")\n@NeedsAll()\nclass MoneyController {\nval money: MoneyService\n}";
        assert!(prepared("kt", kotlin).contains("Needs(MoneyService)"));
        let csharp = "[Service()]\npartial class MoneyService {}\n[RequestRoute(\"/money\")]\n[NeedsAll()]\nclass MoneyController { MoneyService money; }";
        assert!(prepared("cs", csharp).contains("Needs(MoneyService)"));
    }

    #[test]
    fn rust_constructor_validation_searches_all_inherent_impls_and_rejects_duplicates() {
        let source = "#[Service()]\nstruct MoneyService {}\nimpl MoneyService { fn amount(&self) -> i32 { 42 } }\nimpl MoneyService { fn new() -> Self { Self {} } }\n#[RequestRoute(\"/money\")]\n#[Needs(MoneyService)]\nstruct MoneyController { money: std::sync::Arc<MoneyService> }\nimpl MoneyController {\nfn new(money: std::sync::Arc<MoneyService>) -> Self { Self { money } }\n#[GetRoute]\nfn get(&self) -> Json { Json::Null }\n}";
        assert!(
            crate::yon::parser::parse(Path::new("server/controllers/MoneyController.rs"), source)
                .is_ok()
        );
        let duplicate =
            format!("{source}\nimpl MoneyService {{ fn new() -> Self {{ Self {{}} }} }}");
        assert!(
            crate::yon::parser::parse(
                Path::new("server/controllers/MoneyController.rs"),
                &duplicate
            )
            .is_err()
        );
    }

    #[test]
    fn needs_all_injects_only_fields_without_an_initial_value() {
        for (extension, fields) in [
            ("js", "money;\ncount = 0;"),
            (
                "ts",
                "money: MoneyService;\nlabel?: string;\ncount: number = 0;",
            ),
            ("java", "final MoneyService money;\nint count = 0;"),
            ("kt", "private val money: MoneyService\nvar count: Int = 0"),
            ("cs", "readonly MoneyService money;\nint count = 0;"),
            (
                "php",
                "private MoneyService $money;\nprivate int $count = 0;",
            ),
            ("dart", "final MoneyService money;\nint count = 0;"),
        ] {
            let fields = if extension == "js" {
                fields.replace("money;", "moneyService;")
            } else {
                fields.to_owned()
            };
            let source = format!(
                "{}\nclass MoneyService {{}}\n{}\n{}\nclass MoneyController {{\n{fields}\n}}",
                annotation(extension, "Service", ""),
                annotation(extension, "RequestRoute", "\"/money\""),
                annotation(extension, "NeedsAll", "")
            );
            let output = prepared(extension, &source);
            let expected = if extension == "js" {
                "NeedsAll(\"moneyService\")"
            } else {
                "NeedsAll(\"money\")"
            };
            assert!(output.contains(expected), "{extension}: {output}");
            assert!(
                output.contains("Needs(MoneyService)"),
                "{extension}: {output}"
            );
        }
        let python = "@service()\nclass MoneyService:\n    pass\n@request_route(\"/money\")\n@needs_all()\nclass MoneyController:\n    money: MoneyService\n    count: int = 0\n";
        assert!(prepared("py", python).contains("@NeedsAll(\"money\")"));
    }

    #[test]
    fn a_missing_provider_names_the_class_the_field_and_the_type() {
        for (extension, field, ty) in [
            ("js", "missingService;", "MissingService"),
            ("ts", "money: MissingService;", "MissingService"),
            ("java", "final MissingService money;", "MissingService"),
            ("kt", "val money: MissingService", "MissingService"),
            ("cs", "readonly MissingService money;", "MissingService"),
            ("php", "private MissingService $money;", "MissingService"),
            ("dart", "final MissingService money;", "MissingService"),
            (
                "rs",
                "money: std::sync::Arc<MissingService>,",
                "MissingService",
            ),
        ] {
            let kind = if extension == "rs" { "struct" } else { "class" };
            let source = format!(
                "{}\n{}\n{kind} MoneyController {{\n{field}\n}}",
                annotation(extension, "RequestRoute", "\"/money\""),
                annotation(extension, "NeedsAll", "")
            );
            let failure = prepare(Path::new(&format!("yon.{extension}")), &source)
                .err()
                .unwrap_or_else(|| panic!("{extension} accepted a missing provider"))
                .to_string();
            let name = if extension == "js" {
                "missingService"
            } else {
                "money"
            };
            assert!(
                failure.contains("MoneyController")
                    && failure.contains(&format!("'{name}'"))
                    && failure.contains(&format!("no provider {ty} is declared")),
                "{extension}: {failure}"
            );
        }
        let go = "// @RequestRoute(\"/money\")\n// @NeedsAll\ntype MoneyController struct {\n\tmoney *MissingService\n}\n";
        let failure = prepare(Path::new("yon.go"), go)
            .err()
            .map(|failure| failure.to_string())
            .unwrap_or_default();
        assert!(
            failure.contains("MoneyController needs field 'money' of type *MissingService"),
            "{failure}"
        );
    }
}
