use super::{Input, Marker};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Map, Value};
use std::path::Path;

pub(crate) struct GatewayLimits;

impl GatewayLimits {
    pub(crate) const MAX_ARGUMENTS: usize = 32;
    pub(crate) const MAX_BYTES: usize = 1024 * 1024;
    pub(crate) const MAX_DEPTH: usize = 32;
    pub(crate) const MAX_NODES: usize = 10_000;
    pub(crate) const MAX_CALLS: usize = 64;
    const MAX_COMMAND_ENTRIES: usize = 32;
    const MAX_ENTRY_BYTES: usize = 4096;
    const MAX_CLASS_PATH_BYTES: usize = 1024;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GatewayTarget {
    Class { path: String, method: String },
    Command { arguments: Vec<String> },
}

impl GatewayTarget {
    pub(crate) fn names_a_path(entry: &str) -> bool {
        let bytes = entry.as_bytes();
        let has_drive_prefix =
            bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        entry.contains('/') || has_drive_prefix
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GatewayDeclaration {
    pub key: String,
    pub owner: String,
    pub method: String,
    pub arity: usize,
    pub target: GatewayTarget,
}

impl GatewayDeclaration {
    fn from_policy(policy: &Value) -> Option<Self> {
        let key = policy.get("key")?.as_str()?;
        let (owner, method) = key.split_once('.')?;
        let arity = usize::try_from(policy.get("arity")?.as_u64()?).ok()?;
        let target = if let Some(command) = policy.get("command") {
            let mut arguments = Vec::new();
            for entry in command.as_array()? {
                arguments.push(entry.as_str()?.to_owned());
            }
            GatewayTarget::Command { arguments }
        } else {
            GatewayTarget::Class {
                path: policy.get("class")?.as_str()?.to_owned(),
                method: policy.get("method")?.as_str()?.to_owned(),
            }
        };
        Some(Self {
            key: key.to_owned(),
            owner: owner.to_owned(),
            method: method.to_owned(),
            arity,
            target,
        })
    }
}

pub(crate) struct TargetSignature {
    pub parameter_types: Vec<String>,
    pub result_type: String,
    pub is_static: bool,
    pub has_body: bool,
    pub has_supported_receiver: bool,
}

impl TargetSignature {
    fn is_callable(&self, arity: usize, language: &str) -> bool {
        self.has_body
            && !self.is_static
            && self.parameter_types.len() == arity
            && self.uses_json_values(language)
            && self.has_supported_receiver
    }

    pub(crate) fn uses_json_values(&self, language: &str) -> bool {
        let Some((parameter, result, required)) = JsonValueType::for_language(language) else {
            return true;
        };
        let parameters_match = self
            .parameter_types
            .iter()
            .all(|written| JsonValueType::accepts(parameter, written, required));
        let result_matches = JsonValueType::accepts(result, &self.result_type, required)
            || JsonValueType::accepts(parameter, &self.result_type, required);
        parameters_match && result_matches
    }
}

pub(crate) fn target_signature(
    path: &Path,
    source: &str,
    class: &str,
    method: &str,
    arity: usize,
) -> Result<Option<TargetSignature>, Failure> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let signatures = if matches!(extension, "js" | "ts") {
        super::javascript_target_signature(path, source, class, method)?
            .into_iter()
            .collect()
    } else if extension == "rb" {
        super::ruby::RubyBehaviors::target_signatures(source, class, method)
    } else {
        super::polyglot::target_signatures(path, source, class, method)?
    };
    select_target_signature(path, class, method, arity, signatures)
}

fn select_target_signature(
    path: &Path,
    class: &str,
    method: &str,
    arity: usize,
    mut signatures: Vec<TargetSignature>,
) -> Result<Option<TargetSignature>, Failure> {
    retain_matching_signatures(&mut signatures, |signature| {
        signature.has_body && !signature.is_static
    });
    retain_matching_signatures(&mut signatures, |signature| {
        signature.parameter_types.len() == arity
    });
    let language = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    retain_matching_signatures(&mut signatures, |signature| {
        signature.uses_json_values(language)
    });
    retain_matching_signatures(&mut signatures, |signature| {
        signature.has_supported_receiver
    });
    let callable = signatures
        .iter()
        .filter(|signature| signature.is_callable(arity, language))
        .count();
    if callable > 1 {
        return Err(target_failure(
            &path.to_string_lossy(),
            0,
            &format!(
                "Gateway target {class}.{method} has ambiguous JSON signatures with {arity} arguments"
            ),
        ));
    }
    signatures.sort_by_key(|signature| signature.parameter_types.len());
    Ok(signatures.into_iter().next())
}

fn retain_matching_signatures(
    signatures: &mut Vec<TargetSignature>,
    matches: impl Fn(&TargetSignature) -> bool,
) {
    if signatures.iter().any(&matches) {
        signatures.retain(matches);
    }
}

pub(super) struct GatewayMethod<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub language: &'a str,
    pub parameter_types: Vec<&'a str>,
    pub result_type: &'a str,
    pub has_body: bool,
}

struct JsonValueType;

impl JsonValueType {
    const TYPESCRIPT_EMPTY_ANSWER: &'static str = "Promise<BehaviorJsonValue | void>";

    fn for_language(language: &str) -> Option<(&'static str, &'static str, bool)> {
        match language {
            "ts" => Some(("BehaviorJsonValue", "Promise<BehaviorJsonValue>", false)),
            "php" => Some(("mixed", "mixed", false)),
            "java" => Some(("Object", "Object", true)),
            "kt" => Some(("Any?", "Any?", true)),
            "cs" => Some(("BehaviorJSON", "BehaviorJSON", true)),
            "rs" => Some(("BehaviorValue", "BehaviorResult<BehaviorValue>", true)),
            "go" => Some(("BehaviorValue", "(BehaviorValue, error)", true)),
            "dart" => Some(("Object?", "Future<Object?>", true)),
            _ => None,
        }
    }

    fn accepts(expected: &str, written: &str, required: bool) -> bool {
        let written = written.trim();
        if written.is_empty() {
            return !required;
        }
        written.replace(' ', "") == expected.replace(' ', "")
    }

    fn accepts_result(language: &str, expected: &str, written: &str, required: bool) -> bool {
        Self::accepts(expected, written, required)
            || Self::is_typescript_empty_answer(language, written)
    }

    fn is_typescript_empty_answer(language: &str, written: &str) -> bool {
        language == "ts" && Self::accepts(Self::TYPESCRIPT_EMPTY_ANSWER, written, true)
    }
}

pub(super) fn policy(
    input: &Input<'_>,
    marker: &Marker,
    method: &GatewayMethod<'_>,
) -> Result<Value, Failure> {
    let at = marker.span.start;
    if super::Method::HTTP_VERBS.contains(&method.name) {
        return Err(input.failure(
            at,
            "Gateway belongs on a service method, never an HTTP verb endpoint",
        ));
    }
    if method.has_body {
        return Err(input.failure(
            at,
            "Gateway declares a method that another language answers; write its signature with an empty body",
        ));
    }
    if method.parameter_types.len() > GatewayLimits::MAX_ARGUMENTS {
        return Err(input.failure(at, "Gateway methods take at most 32 arguments"));
    }
    check_types(input, marker, method)?;
    let mut policy = target(input, marker)?;
    if policy.contains_key("class") && !policy.contains_key("method") {
        policy.insert("method".to_owned(), Value::String(method.name.to_owned()));
    }
    policy.insert(
        "key".to_owned(),
        Value::String(format!("{}.{}", method.owner, method.name)),
    );
    policy.insert(
        "arity".to_owned(),
        Value::from(method.parameter_types.len()),
    );
    Ok(Value::Object(policy))
}

fn check_types(
    input: &Input<'_>,
    marker: &Marker,
    method: &GatewayMethod<'_>,
) -> Result<(), Failure> {
    let Some((parameter, result, required)) = JsonValueType::for_language(method.language) else {
        return Ok(());
    };
    let parameters_match = method
        .parameter_types
        .iter()
        .all(|written| JsonValueType::accepts(parameter, written, required));
    let result_matches =
        JsonValueType::accepts_result(method.language, result, method.result_type, required);
    if parameters_match && result_matches {
        return Ok(());
    }
    let path = input.path.to_string_lossy();
    Err(target_failure(
        &path,
        marker.span.start,
        &format!(
            "Gateway method '{}' must take {parameter} parameters and return {result}",
            method.name
        ),
    ))
}

pub(crate) fn target_failure(path: &str, at: usize, message: &str) -> Failure {
    Failure::one(diagnostic(
        1312,
        message,
        Some(String::from(
            "A Gateway target is a role file in another Yon language or a program under server/helpers; see docs/BEHAVIOR_ANNOTATIONS.md.",
        )),
        source_span(path, at, at + 1),
    ))
}

fn target(input: &Input<'_>, marker: &Marker) -> Result<Map<String, Value>, Failure> {
    let at = marker.span.start;
    let value = super::literal::parse(&marker.arguments)
        .map_err(|_| input.failure(at, "Gateway configuration must be a strict JSON object"))?;
    let Some(object) = value.as_object() else {
        return Err(input.failure(at, "Gateway configuration must be a JSON object"));
    };
    if object.contains_key("command") {
        return command_target(input, at, object);
    }
    class_target(input, at, object)
}

fn class_target(
    input: &Input<'_>,
    at: usize,
    object: &Map<String, Value>,
) -> Result<Map<String, Value>, Failure> {
    let only_class_keys = object
        .keys()
        .all(|key| matches!(key.as_str(), "class" | "method"));
    if !only_class_keys {
        return Err(input.failure(
            at,
            "Gateway takes either class with an optional method, or command",
        ));
    }
    let valid_path = object
        .get("class")
        .and_then(Value::as_str)
        .is_some_and(|path| {
            !path.is_empty()
                && path.len() <= GatewayLimits::MAX_CLASS_PATH_BYTES
                && !path.contains('\0')
        });
    if !valid_path {
        return Err(input.failure(
            at,
            "Gateway.class names a role file such as server/services/ReportService.java",
        ));
    }
    if let Some(method) = object.get("method")
        && !method.as_str().is_some_and(super::identifier)
    {
        return Err(input.failure(at, "Gateway.method must be a method name"));
    }
    Ok(object.clone())
}

fn command_target(
    input: &Input<'_>,
    at: usize,
    object: &Map<String, Value>,
) -> Result<Map<String, Value>, Failure> {
    if object.len() != 1 {
        return Err(input.failure(
            at,
            "Gateway takes either class with an optional method, or command",
        ));
    }
    let Some(entries) = object["command"]
        .as_array()
        .filter(|entries| (1..=GatewayLimits::MAX_COMMAND_ENTRIES).contains(&entries.len()))
    else {
        return Err(input.failure(at, "Gateway.command is a list of 1 to 32 strings"));
    };
    for entry in entries {
        if !entry.as_str().is_some_and(valid_command_entry) {
            return Err(input.failure(
                at,
                "Gateway.command entries are non-empty strings of at most 4096 bytes without NUL or backslash",
            ));
        }
    }
    if entries[0].as_str().is_some_and(GatewayTarget::names_a_path) {
        return Err(input.failure(
            at,
            "Gateway.command starts with a program name on PATH; files under server/helpers follow it",
        ));
    }
    Ok(object.clone())
}

fn valid_command_entry(entry: &str) -> bool {
    !entry.is_empty()
        && entry.len() <= GatewayLimits::MAX_ENTRY_BYTES
        && !entry.contains(['\0', '\\'])
}

pub(super) fn reject_combined(
    input: &Input<'_>,
    at: usize,
    metadata: &Map<String, Value>,
) -> Result<(), Failure> {
    if metadata.contains_key("gateway") && metadata.len() > 1 {
        return Err(input.failure(
            at,
            "Gateway cannot be combined with another method annotation",
        ));
    }
    Ok(())
}

pub(super) fn reject_in_tac(
    input: &Input<'_>,
    at: usize,
    environment: &str,
) -> Result<(), Failure> {
    if environment == "tac" {
        return Err(input.failure(
            at,
            "Gateway is a Yon annotation; a Tac service cannot call another server language",
        ));
    }
    Ok(())
}

pub(crate) fn declarations(path: &Path, source: &str) -> Result<Vec<GatewayDeclaration>, Failure> {
    crate::fsharp_source::FsharpView::either(path, source, written_declarations)
}

fn written_declarations(path: &Path, source: &str) -> Result<Vec<GatewayDeclaration>, Failure> {
    if !source.contains("Gateway") && !source.contains("gateway") {
        return Ok(Vec::new());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let policies = if matches!(extension, "js" | "ts") {
        super::javascript_gateway_policies(path, source)?
    } else {
        super::polyglot::gateway_policies(path, source)?
    };
    let mut declarations = Vec::new();
    for policy in &policies {
        if let Some(declaration) = GatewayDeclaration::from_policy(policy) {
            declarations.push(declaration);
        }
    }
    Ok(declarations)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{GatewayDeclaration, GatewayTarget, declarations, target_signature};
    use std::path::Path;

    fn lower_error(path: &str, source: &str, environment: &str) -> String {
        crate::behavior::lower(Path::new(path), source, environment)
            .expect_err("gateway refusal")
            .to_string()
    }

    fn javascript(annotation: &str, method: &str) -> String {
        format!("class ReportClient {{\n  {annotation}\n  {method}\n}}\n")
    }

    fn python(annotation: &str, method: &str) -> String {
        format!("class ReportClient:\n    {annotation}\n    {method}\n")
    }

    #[test]
    fn a_command_declaration_records_its_key_arity_and_arguments() {
        let source = javascript(
            r#"@Gateway({"command":["ruby","server/helpers/report.rb"]})"#,
            "async summarize(period, currency) {}",
        );
        let found = declarations(Path::new("server/clients/ReportClient.js"), &source)
            .expect("declarations");
        assert_eq!(
            found,
            vec![GatewayDeclaration {
                key: "ReportClient.summarize".to_owned(),
                owner: "ReportClient".to_owned(),
                method: "summarize".to_owned(),
                arity: 2,
                target: GatewayTarget::Command {
                    arguments: vec!["ruby".to_owned(), "server/helpers/report.rb".to_owned()],
                },
            }]
        );
    }

    #[test]
    fn a_class_declaration_defaults_its_method_to_the_annotated_name() {
        let source = python(
            r#"@gateway({"class":"server/services/ReportService.java"})"#,
            "def summarize(self, period): ...",
        );
        let found = declarations(Path::new("server/clients/ReportClient.py"), &source)
            .expect("declarations");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].arity, 1);
        assert_eq!(
            found[0].target,
            GatewayTarget::Class {
                path: "server/services/ReportService.java".to_owned(),
                method: "summarize".to_owned(),
            }
        );
    }

    #[test]
    fn misused_gateways_fail_with_ty1308() {
        let cases = [
            (
                r#"@Gateway({"command":["ruby"]})"#,
                "async GET(request) {}",
                "HTTP verb",
            ),
            (
                r#"@Gateway({"command":["ruby"]})"#,
                "async summarize(period) { return period }",
                "empty body",
            ),
            (
                r#"@Gateway({"command":["ruby"]})"#,
                "summarize(period) {}",
                "async",
            ),
            (
                r#"@Gateway({"command":[]})"#,
                "async summarize() {}",
                "1 to 32",
            ),
            (
                r#"@Gateway({"command":["./bin/report"]})"#,
                "async summarize() {}",
                "program name",
            ),
            (
                r#"@Gateway({"command":["C:report"]})"#,
                "async summarize() {}",
                "program name",
            ),
            (
                r#"@Gateway({"command":["ruby","server\\helpers\\a.rb"]})"#,
                "async summarize() {}",
                "backslash",
            ),
            (
                r#"@Gateway({"class":"server/services/A.py","command":["ruby"]})"#,
                "async summarize() {}",
                "either class",
            ),
            (
                r#"@Gateway({"method":"x"})"#,
                "async summarize() {}",
                "role file",
            ),
            (
                r#"@Gateway({"class":"server/services/A.py","method":"not a name"})"#,
                "async summarize() {}",
                "method name",
            ),
            ("@Gateway(summarize)", "async summarize() {}", "strict JSON"),
        ];
        for (annotation, method, expected) in cases {
            let rendered = lower_error(
                "server/clients/ReportClient.js",
                &javascript(annotation, method),
                "yon",
            );
            assert!(rendered.contains("TY1308"), "{annotation}: {rendered}");
            assert!(rendered.contains(expected), "{annotation}: {rendered}");
        }
    }

    #[test]
    fn a_gateway_cannot_share_its_method_or_reach_tac() {
        let combined = javascript(
            "@Gateway({\"command\":[\"ruby\"]})\n  @Retry({\"maxRetries\":1})",
            "async summarize() {}",
        );
        let rendered = lower_error("server/clients/ReportClient.js", &combined, "yon");
        assert!(rendered.contains("combined"), "{rendered}");
        let tac = lower_error(
            "client/services/ReportService.js",
            &javascript(r#"@Gateway({"command":["ruby"]})"#, "async summarize() {}"),
            "tac",
        );
        assert!(
            tac.contains("TY1308") && tac.contains("Yon annotation"),
            "{tac}"
        );
    }

    #[test]
    fn typed_callers_must_use_the_json_value_type() {
        let typed = javascript(
            r#"@Gateway({"command":["ruby"]})"#,
            "async summarize(period: number): Promise<BehaviorJsonValue> {}",
        );
        let rendered = lower_error("server/clients/ReportClient.ts", &typed, "yon");
        assert!(rendered.contains("TY1312"), "{rendered}");
        let untyped = javascript(
            r#"@Gateway({"command":["ruby"]})"#,
            "async summarize(period: BehaviorJsonValue): Promise<BehaviorJsonValue> {}",
        );
        assert!(
            crate::behavior::lower(Path::new("server/clients/ReportClient.ts"), &untyped, "yon")
                .is_ok()
        );
        let empty_answer = javascript(
            r#"@Gateway({"command":["ruby"]})"#,
            "async summarize(period: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}",
        );
        assert!(
            crate::behavior::lower(
                Path::new("server/clients/ReportClient.ts"),
                &empty_answer,
                "yon"
            )
            .is_ok()
        );
    }

    #[test]
    fn python_gateways_lower_without_a_behavior_context() {
        let source = python(
            r#"@gateway({"command":["python3","server/helpers/report.py"]})"#,
            "def summarize(self, period): ...",
        );
        let lowered =
            crate::behavior::lower(Path::new("server/clients/ReportClient.py"), &source, "yon")
                .expect("lowered")
                .expect("changed");
        assert!(lowered.contains("ReportClient.summarize"), "{lowered}");
        assert!(
            !lowered.contains("def __init__(self, behaviors)"),
            "{lowered}"
        );
    }

    #[test]
    fn a_python_target_uses_the_last_definition_of_a_method() {
        let source = "class ReportService:\n    def summarize(self, period):\n        return period\n\n    def summarize(self, period, currency):\n        return currency\n";
        let path = Path::new("server/services/ReportService.py");
        let signature = target_signature(path, source, "ReportService", "summarize", 1)
            .expect("valid signature")
            .expect("last definition");
        assert_eq!(signature.parameter_types.len(), 2);
    }

    #[test]
    fn gateway_overload_selection_is_shared_by_jvm_and_csharp() {
        let cases = [
            (
                "java",
                "String summarize(String mode, String detail) { return mode; }",
                "Object summarize(Object mode) { return mode; }",
                "Object summarize(Object mode, Object detail) { return detail; }",
            ),
            (
                "kt",
                "fun summarize(mode: String, detail: String): String = mode",
                "fun summarize(mode: Any?): Any? = mode",
                "fun summarize(mode: Any?, detail: Any?): Any? = detail",
            ),
            (
                "cs",
                "string summarize(string mode, string detail) { return mode; }",
                "BehaviorJSON summarize(BehaviorJSON mode) { return mode; }",
                "BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail) { return detail; }",
            ),
        ];
        for (language, typed, short, matching) in cases {
            for methods in [[typed, short, matching], [matching, short, typed]] {
                let source = format!(
                    "class ReportService {{\n    {}\n}}\n",
                    methods.join("\n    ")
                );
                let path = format!("server/services/ReportService.{language}");
                let signature =
                    target_signature(Path::new(&path), &source, "ReportService", "summarize", 2)
                        .expect("valid signature")
                        .expect("selected overload");
                assert_eq!(signature.parameter_types.len(), 2, "{language}");
                assert!(signature.uses_json_values(language), "{language}");
                assert!(!signature.is_static && signature.has_body, "{language}");
            }
        }
    }
}
