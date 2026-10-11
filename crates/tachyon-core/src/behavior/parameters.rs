use super::{Input, Marker};
use crate::Failure;
use crate::ascii_name::AsciiName;
use serde_json::{Map, Value, json};
use std::ops::Range;

pub(super) struct ParameterMarkers;

impl ParameterMarkers {
    pub(super) const NAMES: [&str; 3] = ["Check", "PageDefaults", "CacheKey"];
    const TYPED_BY_DECLARATION: [&str; 3] = ["js", "ts", "py"];
    const RECEIVERS: [&str; 3] = ["self", "&self", "&mut self"];
}

pub(super) enum ParameterPolicy {
    Valid(Value),
    Page(Value),
    PageParameter(usize),
    CacheKey(usize),
}

pub(super) struct MarkedParameter {
    pub declaration: usize,
    pub index: usize,
}

fn canonical(language: &str, written: &str) -> Option<&'static str> {
    let normalized = written.replace('_', "").to_ascii_lowercase();
    for name in ParameterMarkers::NAMES {
        let same = if language == "py" {
            name.to_ascii_lowercase() == normalized
        } else {
            name == written
        };
        if same {
            return Some(name);
        }
    }
    None
}

fn prefix(language: &str) -> &'static str {
    match language {
        "rs" | "php" | "rb" => "#[",
        "cs" => "[",
        _ => "@",
    }
}

pub(super) fn read(
    input: &Input<'_>,
    language: &str,
    at: usize,
) -> Result<Option<Marker>, Failure> {
    let prefix = prefix(language);
    let Some(rest) = input.code[at..].strip_prefix(prefix) else {
        return Ok(None);
    };
    let length = AsciiName::leading(rest).len();
    let Some(name) = canonical(language, &rest[..length]) else {
        return Ok(None);
    };
    let mut end = at + prefix.len() + length;
    let mut arguments = String::new();
    if input.code[end..].starts_with('(') {
        let close = super::matching(&input.code, end, b'(', b')')
            .ok_or_else(|| input.failure(at, format!("Close the arguments of @{name}")))?;
        input.source[end + 1..close].clone_into(&mut arguments);
        end = close + 1;
    }
    if prefix != "@" {
        if !input.code[end..].starts_with(']') {
            return Err(input.failure(at, format!("Close @{name} with ']'")));
        }
        end += 1;
    }
    Ok(Some(Marker {
        name: name.to_owned(),
        arguments,
        span: at..end,
        target: end,
        parameter: true,
    }))
}

pub(super) fn marked_parameter(
    input: &Input<'_>,
    marker: &Marker,
) -> Result<MarkedParameter, Failure> {
    let code = &input.code;
    let not_a_parameter = || {
        input.failure(
            marker.span.start,
            format!("@{} marks one parameter of a method", marker.name),
        )
    };
    let open = open_parenthesis(code, marker.span.start).ok_or_else(not_a_parameter)?;
    let close = super::matching(code, open, b'(', b')').ok_or_else(not_a_parameter)?;
    let index =
        parameter_index(code, open + 1..close, marker.target).ok_or_else(not_a_parameter)?;
    let line_start = code[..open].rfind('\n').map_or(0, |end| end + 1);
    let line = &code[line_start..open];
    let declaration = line_start + line.len() - line.trim_start().len();
    Ok(MarkedParameter { declaration, index })
}

fn open_parenthesis(code: &str, at: usize) -> Option<usize> {
    let mut depth = 0_u32;
    for (position, byte) in code[..at].bytes().enumerate().rev() {
        if byte == b')' {
            depth += 1;
        }
        if byte == b'(' {
            if depth == 0 {
                return Some(position);
            }
            depth -= 1;
        }
    }
    None
}

fn parameter_index(code: &str, list: Range<usize>, at: usize) -> Option<usize> {
    let mut index = 0;
    for segment in segments(code, list) {
        let text = code[segment.clone()].trim();
        if segment.contains(&at) || segment.end == at {
            return Some(index);
        }
        if !ParameterMarkers::RECEIVERS.contains(&text) {
            index += 1;
        }
    }
    None
}

fn segments(code: &str, list: Range<usize>) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut depth = 0_i64;
    let mut start = list.start;
    for (position, byte) in code[list.clone()].bytes().enumerate() {
        let at = list.start + position;
        match byte {
            b'<' | b'(' | b'[' | b'{' => depth += 1,
            b'>' | b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                result.push(start..at);
                start = at + 1;
            }
            _ => {}
        }
    }
    if !code[start..list.end].trim().is_empty() {
        result.push(start..list.end);
    }
    result
}

pub(super) fn policy(
    input: &Input<'_>,
    marker: &Marker,
    index: usize,
    declared_type: &str,
) -> Result<ParameterPolicy, Failure> {
    let arguments = marker.arguments.trim();
    match marker.name.as_str() {
        "CacheKey" => {
            if !arguments.is_empty() {
                return Err(input.failure(
                    marker.span.start,
                    crate::annotation_rules::AnnotationRules::no_arguments("CacheKey"),
                ));
            }
            Ok(ParameterPolicy::CacheKey(index))
        }
        "PageDefaults" if arguments.is_empty() => Ok(ParameterPolicy::PageParameter(index)),
        "PageDefaults" => page_defaults(input, marker, index).map(ParameterPolicy::Page),
        _ => checked(input, marker, index, declared_type).map(ParameterPolicy::Valid),
    }
}

fn page_defaults(input: &Input<'_>, marker: &Marker, index: usize) -> Result<Value, Failure> {
    let parsed = super::literal::parse(&marker.arguments).ok();
    let Some(Value::Object(mut object)) = parsed else {
        return Err(input.failure(
            marker.span.start,
            "PageDefaults takes one JSON object, for example @PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})",
        ));
    };
    if object.contains_key("argument") {
        return Err(input.failure(
            marker.span.start,
            "PageDefaults sits on the page parameter, so it has no \"argument\" key: remove it",
        ));
    }
    object.insert("argument".to_owned(), json!(index));
    let page_marker = Marker {
        arguments: Value::Object(object).to_string(),
        ..marker.clone()
    };
    super::pageable_default(input, &page_marker)
}

fn checked(
    input: &Input<'_>,
    marker: &Marker,
    index: usize,
    declared_type: &str,
) -> Result<Value, Failure> {
    let language = input.extension();
    let arguments = marker.arguments.trim();
    let type_name = if ParameterMarkers::TYPED_BY_DECLARATION.contains(&language) {
        if !arguments.is_empty() {
            return Err(input.failure(marker.span.start, typed_check_advice(language)));
        }
        declared_type.trim()
    } else {
        if arguments.is_empty() {
            return Err(input.failure(marker.span.start, json_check_advice(language)));
        }
        super::literal::class_reference(language, arguments).unwrap_or("")
    };
    if !super::identifier(type_name) {
        let advice = if ParameterMarkers::TYPED_BY_DECLARATION.contains(&language) {
            typed_check_advice(language)
        } else {
            json_check_advice(language)
        };
        return Err(input.failure(marker.span.start, advice));
    }
    Ok(json!({"argument": index, "type": type_name}))
}

fn typed_check_advice(language: &str) -> String {
    let example = match language {
        "js" => {
            "add @param {ProductInput} input to the method's JSDoc and write submit(@Check input)"
        }
        "py" => "write def submit(self, @check input: ProductInput)",
        _ => "write submit(@Check input: ProductInput)",
    };
    format!("@Check reads the checked class from the parameter's type: {example}")
}

fn json_check_advice(language: &str) -> String {
    let example = match language {
        "java" => "@Check(ProductInput.class) Object input",
        "kt" => "@Check(ProductInput::class) input: Any?",
        "cs" => "[Check(typeof(ProductInput))] BehaviorJSON input",
        "swift" => "@Check(ProductInput) _ input: BehaviorJSON",
        "dart" => "@Check(ProductInput) Object? input",
        "rs" => "#[Check(ProductInput)] input: BehaviorValue",
        "go" => "// @Check(ProductInput) on its own line above input BehaviorValue",
        "rb" => {
            "#[Check(ProductInput)] on its own line above input, in a parameter list written over several lines"
        }
        "php" => "#[Check(ProductInput::class)] mixed $input",
        _ => "@Check(ProductInput) input",
    };
    format!(
        "In this language a checked parameter holds JSON, so @Check names the checked class: {example}"
    )
}

pub(super) fn every_parameter(count: usize) -> Value {
    let mut indexes = Vec::new();
    for index in 0..count {
        indexes.push(json!(index));
    }
    json!({ "keyArgs": indexes })
}

pub(super) fn record(
    input: &Input<'_>,
    at: usize,
    metadata: &mut Map<String, Value>,
    policy: ParameterPolicy,
) -> Result<(), Failure> {
    let (key, value) = match policy {
        ParameterPolicy::CacheKey(index) => {
            let keys = metadata
                .entry("cacheKeys")
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(keys) = keys {
                keys.push(json!(index));
            }
            return Ok(());
        }
        ParameterPolicy::Valid(value) => ("valid", value),
        ParameterPolicy::Page(value) => ("pageable", value),
        ParameterPolicy::PageParameter(index) => ("pageParameter", json!(index)),
    };
    let taken = metadata.contains_key("pageable") || metadata.contains_key("pageParameter");
    if (key != "valid" && taken) || metadata.insert(key.to_owned(), value).is_some() {
        return Err(input.failure(at, "A method has at most one @Check and one @PageDefaults"));
    }
    Ok(())
}

pub(super) fn finish(
    input: &Input<'_>,
    at: usize,
    metadata: &mut Map<String, Value>,
) -> Result<(), Failure> {
    if let Some(keys) = metadata.remove("cacheKeys") {
        let Some(cacheable) = metadata.get_mut("cacheable") else {
            return Err(input.failure(
                at,
                "@CacheKey marks a key parameter of a @Cached method; add @Cached to the method",
            ));
        };
        cacheable["keyArgs"] = keys;
    }
    if metadata.contains_key("pageParameter") {
        return Err(input.failure(
            at,
            "@PageDefaults without sizes marks the page parameter of a Store query; on other methods write the sizes: @PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})",
        ));
    }
    Ok(())
}

pub(super) fn documented_type(source: &str, code: &str, start: usize, name: &str) -> String {
    let Some(comment_end) = source[..start].rfind("*/") else {
        return String::new();
    };
    let between = &code[comment_end + 2..start];
    let only_markers = between
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('@'));
    let Some(comment_start) = source[..comment_end].rfind("/**") else {
        return String::new();
    };
    if !only_markers {
        return String::new();
    }
    let comment = &source[comment_start..comment_end];
    for (at, _) in comment.match_indices("@param") {
        let rest = comment[at + 6..].trim_start();
        let Some(rest) = rest.strip_prefix('{') else {
            continue;
        };
        let Some((kind, after)) = rest.split_once('}') else {
            continue;
        };
        if after.split_whitespace().next() == Some(name) {
            return kind.trim().to_owned();
        }
    }
    String::new()
}

pub(super) fn refuse_method_form(input: &Input<'_>, marker: &Marker) -> Failure {
    let example = match marker.name.as_str() {
        "Check" => "submit(@Check input)",
        "CacheKey" => "find(@CacheKey id, view)",
        _ => "list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) page)",
    };
    input.failure(
        marker.span.start,
        format!(
            "@{} now sits on the parameter it describes, not above the method: write {example}",
            marker.name
        ),
    )
}

pub(super) fn documented_field_type(source: &str, code: &str, start: usize) -> String {
    let Some(comment_end) = source[..start].rfind("*/") else {
        return String::new();
    };
    if !code[comment_end + 2..start].trim().is_empty() {
        return String::new();
    }
    let Some(comment_start) = source[..comment_end].rfind("/**") else {
        return String::new();
    };
    let comment = &source[comment_start..comment_end];
    let Some(at) = comment.find("@type") else {
        return String::new();
    };
    let rest = comment[at + 5..].trim_start();
    let Some(rest) = rest.strip_prefix('{') else {
        return String::new();
    };
    rest.split_once('}')
        .map_or_else(String::new, |(kind, _)| kind.trim().to_owned())
}
