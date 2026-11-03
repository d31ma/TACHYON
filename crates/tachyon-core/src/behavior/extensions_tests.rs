#![allow(clippy::expect_used)]

use super::lower;
use std::path::Path;

struct TestLanguages;

impl TestLanguages {
    const GO_OWNER: &str = "__TEST_OWNER__";
    const ALL: [&str; 10] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "rs", "swift", "go",
    ];
}

#[test]
fn empty_required_constructors_do_not_relax_table_arguments() {
    for language in TestLanguages::ALL {
        let declaration = declaration(language, "Empty", empty_body(language));
        let constructor = format!("{}\n{declaration}", marker(language, "NeedsAll", ""));
        for environment in environments(language) {
            checked_lower(language, &constructor, environment)
                .expect("empty required constructors are supported")
                .expect("constructor lowering");
            let table = format!("{}\n{declaration}", marker(language, "Table", ""));
            let failure = checked_lower(language, &table, environment)
                .expect_err("Table still requires a nonempty table name");
            assert!(
                failure.to_string().contains("quoted identifiers"),
                "{language}: {failure}"
            );
        }
    }
}

#[test]
fn value_descriptors_reject_missing_paths_and_non_string_types_without_panicking() {
    for language in TestLanguages::ALL {
        for environment in environments(language) {
            let field = port_field(language);
            for descriptor in invalid_value_descriptors(environment) {
                let annotation = marker(language, "Setting", &descriptor);
                let body = format!("    {annotation}\n    {field}");
                let source = declaration(language, "Settings", &body);
                assert!(
                    checked_lower(language, &source, environment).is_err(),
                    "{language}/{environment}: {descriptor}"
                );
            }
        }
    }
}

fn invalid_value_descriptors(environment: &str) -> [String; 8] {
    let path = format!("{environment}.port");
    [
        "{}".to_owned(),
        r#"{"type":"integer"}"#.to_owned(),
        r#"{"path":null}"#.to_owned(),
        format!(r#"{{"path":"{path}","type":null}}"#),
        format!(r#"{{"path":"{path}","type":7}}"#),
        format!(r#"{{"path":"{path}","type":false}}"#),
        format!(r#"{{"path":"{path}","type":[]}}"#),
        format!(r#"{{"path":"{path}","type":{{}}}}"#),
    ]
}

fn port_field(language: &str) -> &'static str {
    match language {
        "js" => "port;",
        "ts" => "port: number;",
        "py" => "port: int = 0",
        "php" => "public int $port = 0;",
        "java" | "cs" => "int port;",
        "kt" | "swift" => "var port: Int = 0",
        "rs" => "port: i64,",
        "go" => "port int64",
        _ => unreachable!(),
    }
}

#[test]
fn value_supports_typescript_definite_assignment_and_optional_fields() {
    for environment in ["tac", "yon"] {
        let prefix = environment;
        for field in ["port!: number;", "port?: number;"] {
            let source = format!(
                "class Settings {{\n@Setting({{\"path\":\"{prefix}.port\",\"type\":\"integer\",\"default\":8080}})\n{field}\n}}"
            );
            let generated = checked_lower("ts", &source, environment)
                .expect("TypeScript field modifiers are valid")
                .expect("annotated source");
            assert!(generated.contains("\"values\":{\"port\":"));
        }
    }
}

#[test]
fn value_paths_use_only_canonical_framework_segments() {
    for environment in ["tac", "yon"] {
        for suffix in ["catalog.page.size", "api.url", "feature2.enabled"] {
            let source = format!(
                "class Settings {{\n@Setting(\"${{{environment}.{suffix}:fallback}}\")\nvalue;\n}}"
            );
            checked_lower("js", &source, environment).expect("canonical nested path");
        }
        for path in [
            "server.port",
            "properties.YON_SIZE",
            "yon.page_size",
            "yon..size",
            "yon.Size",
            "yon.size.",
            "yon.1size",
            "tac.__proto__",
            "yon.constructor",
        ] {
            let source = format!("class Settings {{\n@Setting(\"${{{path}}}\")\nvalue;\n}}");
            assert!(
                checked_lower("js", &source, environment).is_err(),
                "{environment}: {path}"
            );
        }
    }
}

fn marker(language: &str, name: &str, arguments: &str) -> String {
    match language {
        "php" | "rs" => format!("#[{name}({arguments})]"),
        "go" => format!("// @{name}({arguments})"),
        "cs" => format!("[{name}({arguments})]"),
        _ => format!("@{name}({arguments})"),
    }
}

fn environments(language: &str) -> &'static [&'static str] {
    match language {
        "swift" => &["tac"],
        "js" | "ts" | "kt" | "cs" | "rs" => &["yon", "tac"],
        _ => &["yon"],
    }
}

fn declaration(language: &str, name: &str, body: &str) -> String {
    match language {
        "py" => format!("class {name}:\n{body}\n"),
        "rs" => format!("struct {name} {{\n{body}\n}}\n"),
        "go" => format!("type {name} struct {{\n{body}\n}}\n"),
        _ => format!("class {name} {{\n{body}\n}}\n"),
    }
}

fn empty_body(language: &str) -> &'static str {
    if language == "py" { "    pass" } else { "" }
}

fn checked_lower(
    language: &str,
    source: &str,
    environment: &str,
) -> Result<Option<String>, crate::Failure> {
    lower(
        Path::new(&format!("source.{language}")),
        source,
        environment,
    )
}

fn assert_accepts(language: &str, source: &str) {
    for environment in environments(language) {
        let generated = checked_lower(language, source, environment)
            .unwrap_or_else(|error| {
                panic!("{language}/{environment} should accept:\n{source}\n{error:?}")
            })
            .expect("source contains executable behavior declarations");
        assert!(
            !generated.contains(&marker(language, "Checked", "")),
            "validation marker not erased: {language}"
        );
    }
}

fn assert_rejects(language: &str, source: &str) {
    for environment in environments(language) {
        assert!(
            checked_lower(language, source, environment).is_err(),
            "{language}/{environment} accepted invalid source:\n{source}"
        );
    }
}

fn dto(language: &str, constructor: bool) -> String {
    let mut output = format!("{}\n", marker(language, "Checked", ""));
    if constructor {
        output.push_str(&marker(language, "NeedsAll", "\"name\",\"count\""));
        output.push('\n');
    }
    let (name, count, comment) = dto_fields(language);
    let body = format!(
        "    {}\n    {}\n    {}\n    {name} {comment}\n    {}\n    {}\n    {count} {comment}",
        marker(language, "Required", ""),
        marker(language, "NotEmpty", ""),
        marker(language, "Length", r#"{"min":1,"max":20}"#),
        marker(language, "Min", "1"),
        marker(language, "Max", "10")
    );
    output.push_str(&declaration(language, "Input", &body));
    if language == "php" {
        output.insert_str(0, "<?php\n");
    }
    output
}

fn dto_fields(language: &str) -> (&'static str, &'static str, &'static str) {
    match language {
        "js" => ("name;", "count;", "// field comment"),
        "ts" => ("name: string;", "count: number;", "// field comment"),
        "py" => ("name: str", "count: int", "# field comment"),
        "php" => (
            "public string $name;",
            "public int $count;",
            "// field comment",
        ),
        "java" => ("String name;", "int count;", "// field comment"),
        "kt" | "swift" => ("var name: String", "var count: Int", "// field comment"),
        "cs" => (
            "public string name;",
            "public int count;",
            "// field comment",
        ),
        "rs" => ("name: String,", "count: i32,", "// field comment"),
        "go" => ("name string", "count int32", "// field comment"),
        _ => unreachable!(),
    }
}

#[test]
fn field_constraints_allow_inline_comments_in_every_language() {
    for language in TestLanguages::ALL {
        assert_accepts(language, &dto(language, false));
        for environment in environments(language) {
            let generated = checked_lower(language, &dto(language, false), environment)
                .expect("valid schema")
                .expect("generated schema");
            let unescaped = generated.replace("\\\"", "\"").replace("\"\"", "\"");
            let normalized = without_whitespace(&unescaped);
            let expected = expected_name_rules(language);
            assert!(
                normalized.contains(expected),
                "{language}/{environment}: comments redirected constraints away from the authored field"
            );
        }
    }
}

fn expected_name_rules(language: &str) -> &'static str {
    match language {
        "rs" => "name:\"name\",not_null:true,not_blank:true,size:Some((1,20))",
        "java" => "newRule(\"name\",true,true,1L,20L,null,null)",
        "kt" => "Rule(\"name\",true,true,1L,20L,null,null)",
        "go" => "{Name:\"name\",NotNull:true,NotBlank:true,HasSize:true,SizeMin:1,SizeMax:20}",
        _ => r#""name":{"notBlank":true,"notNull":true,"size":{"max":20,"min":1}}"#,
    }
}

#[test]
fn annotated_fields_compose_with_required_constructor_in_every_language() {
    for language in TestLanguages::ALL {
        assert_accepts(language, &dto(language, true));
        let expected = expected_constructor(language);
        for environment in environments(language) {
            let generated = checked_lower(language, &dto(language, true), environment)
                .expect("valid constructor")
                .expect("generated constructor");
            let compact = without_whitespace(&generated);
            if language == "go" {
                assert!(!compact.contains("funcNewInput"), "{generated}");
                continue;
            }
            assert!(
                compact.contains(expected),
                "{language}/{environment}: annotation/comment trivia corrupted generated constructor signature; expected {expected}; constructors: {:?}",
                constructor_lines(&generated)
            );
        }
    }
}

fn expected_constructor(language: &str) -> &'static str {
    match language {
        "js" | "ts" => "constructor(name,count)",
        "py" => "def__init__(self,name,count):",
        "php" => "function__construct(string$name,int$count)",
        "java" => "publicInput(Stringname,intcount)",
        "kt" => "constructor(name:String,count:Int)",
        "cs" => "publicInput(stringname,intcount)",
        "rs" => "pubfnnew(name:String,count:i32)",
        "swift" => "init(name:String,count:Int)",
        "go" => "",
        _ => unreachable!(),
    }
}

fn without_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn constructor_lines(generated: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    for line in generated.lines() {
        if line.contains("constructor(") || line.contains("fn new(") || line.contains("init(") {
            lines.push(line);
        }
    }
    lines
}

fn value_type(language: &str) -> &'static str {
    match language {
        "java" => "Object",
        "kt" => "Any?",
        "cs" | "swift" => "BehaviorJSON",
        "rs" | "go" => "BehaviorValue",
        _ => "object",
    }
}

fn method(language: &str, name: &str, parameters: &[&str], body: &str) -> String {
    let parameter_list = parameter_text(language, parameters);
    method_with(language, name, &parameter_list, body)
}

fn method_with(language: &str, name: &str, parameter_list: &str, body: &str) -> String {
    let separator = if parameter_list.is_empty() { "" } else { ", " };
    match language {
        "js" => format!("    async {name}({parameter_list}) {{ {body} }}\n"),
        "ts" => format!("    async {name}({parameter_list}): Promise<any> {{ {body} }}\n"),
        "py" => format!(
            "    async def {name}(self{separator}{parameter_list}):\n        {}\n",
            if body.is_empty() { "pass" } else { body }
        ),
        "php" => format!("    public function {name}({parameter_list}) {{ {body} }}\n"),
        "java" => format!("    Object {name}({parameter_list}) {{ {body} }}\n"),
        "kt" => format!("    fun {name}({parameter_list}): Any? {{ {body} }}\n"),
        "cs" => {
            format!("    public async Task<BehaviorJSON> {name}({parameter_list}) {{ {body} }}\n")
        }
        "swift" => {
            format!("    func {name}({parameter_list}) async throws -> BehaviorJSON {{ {body} }}\n")
        }
        "rs" => format!(
            "    fn {name}(&self{separator}{parameter_list}) -> BehaviorResult<BehaviorValue> {{ {body} }}\n"
        ),
        "go" => format!(
            "func (receiver *{}) {name}({parameter_list}) (BehaviorValue, error) {{ {body} }}\n",
            TestLanguages::GO_OWNER
        ),
        _ => unreachable!(),
    }
}

fn marked(language: &str, marker_name: &str, arguments: &str, name: &str) -> String {
    let written = if arguments.is_empty() {
        marker_name.to_owned()
    } else {
        format!("{marker_name}({arguments})")
    };
    let spelled = parameter_text(language, &[name]);
    match language {
        "php" | "rs" => format!("#[{written}] {spelled}"),
        "cs" => format!("[{written}] {spelled}"),
        "go" => format!("\n\t// @{written}\n\t{spelled}"),
        _ => format!("@{written} {spelled}"),
    }
}

fn parameter_list(language: &str, parameters: &[String]) -> String {
    let joined = parameters.join(", ");
    if language == "go" {
        return format!("{joined},\n");
    }
    joined
}

fn check_argument(language: &str) -> &'static str {
    if matches!(language, "js" | "ts" | "py") {
        ""
    } else {
        "Input"
    }
}

fn checked_parameter(language: &str) -> String {
    let parameter = marked(language, "Check", check_argument(language), "input");
    match language {
        "ts" => parameter.replace("input: any", "input: Input"),
        "py" => format!("{parameter}: Input"),
        _ => parameter,
    }
}

fn parameter_marker_service(language: &str, key_parameter: &str) -> String {
    let find = parameter_list(
        language,
        &[
            marked(language, "CacheKey", "", "id"),
            parameter_text(language, &["view"]),
        ],
    );
    let remove = parameter_list(
        language,
        &[
            parameter_text(language, &["view"]),
            parameter_text(language, &[key_parameter]),
        ],
    );
    let accept = parameter_list(language, &[checked_parameter(language)]);
    let page_options = r#"{"defaultSize":20,"maxSize":100,"sort":["id"]}"#;
    let list = parameter_list(
        language,
        &[marked(language, "PageDefaults", page_options, "page")],
    );
    let paged = if matches!(language, "js" | "ts") {
        method_with(language, "list", &list, "")
    } else {
        String::new()
    };
    let documentation = if language == "js" {
        "    /** @param {Input} input */\n"
    } else {
        ""
    };
    let methods = format!(
        "    {}\n{}    {}\n{}{documentation}{}{}",
        marker(language, "Cached", ""),
        method_with(language, "find", &find, ""),
        marker(language, "ClearCache", r#"{"method":"find"}"#),
        method_with(language, "remove", &remove, ""),
        method_with(language, "accept", &accept, ""),
        paged,
    );
    let settings = marker(
        language,
        "CacheSettings",
        r#"{"name":"items","ttlMs":1000,"maxBytes":1024,"maxEntries":8}"#,
    );
    let service = service(language, "Items", &methods, true);
    let dto = dto(language, false)
        .trim_start_matches("<?php\n")
        .to_owned();
    let opening = if language == "php" { "<?php\n" } else { "" };
    format!("{opening}{dto}{settings}\n{service}")
}

#[test]
fn parameter_markers_lower_in_every_language() {
    for language in TestLanguages::ALL {
        let source = parameter_marker_service(language, "id");
        assert_accepts(language, &source);
        let renamed = parameter_marker_service(language, "key");
        let environment = environments(language)[0];
        let failure = checked_lower(language, &renamed, environment)
            .expect_err(&format!("{language}: mutation without the key name"));
        assert!(
            failure.to_string().contains("named 'id'"),
            "{language}: {failure}"
        );
    }
}

#[test]
fn parameter_markers_record_indexes_as_policies() {
    let source = parameter_marker_service("ts", "id");
    let lowered = checked_lower("ts", &source, "yon")
        .expect("TypeScript parameter markers")
        .expect("lowered");
    for expected in [
        r#""cacheable":{"keyArgs":[0]}"#,
        r#""arguments":[1]"#,
        r#""valid":{"argument":0"#,
        r#""type":"Input""#,
        r#""pageable":{"argument":0,"defaultSize":20"#,
    ] {
        assert!(lowered.contains(expected), "{expected} missing:\n{lowered}");
    }
    assert!(!lowered.contains("@CacheKey") && !lowered.contains("@PageDefaults"));
}

#[test]
fn python_field_and_parameter_markers_lower_from_annotated_types() {
    let decorated = parameter_marker_service("py", "id");
    let mut source = format!("from typing import Annotated\n{decorated}");
    for (old, new) in [
        (
            "    @Required()\n    @NotEmpty()\n    @Length({\"min\":1,\"max\":20})\n    name: str",
            "    name: Annotated[str, required, not_empty, length({\"min\":1,\"max\":20})]",
        ),
        (
            "    @Min(1)\n    @Max(10)\n    count: int",
            "    count: Annotated[int, min(1), max(10)]",
        ),
        ("@CacheKey id", "id: Annotated[object, cache_key]"),
        ("@Check input: Input", "input: Annotated[Input, check]"),
    ] {
        assert!(source.contains(old), "{old} missing:\n{source}");
        source = source.replacen(old, new, 1);
    }
    let lowered = checked_lower("py", &source, "yon")
        .unwrap_or_else(|error| panic!("{error}\n{source}"))
        .expect("lowered");
    let expected = checked_lower("py", &decorated, "yon")
        .expect("decorated source")
        .expect("lowered");
    assert_eq!(
        lowered[source.len()..],
        expected[decorated.len()..],
        "{lowered}"
    );
    let authored = &lowered[..source.len()];
    for erased in ["Annotated[", "cache_key", "not_empty", "check]"] {
        assert!(!authored.contains(erased), "{erased} left in:\n{authored}");
    }
    let find = format!("def find(self, id:{}object", " ".repeat(11));
    assert!(authored.contains(&find), "{authored}");
}

#[test]
fn old_index_forms_are_refused_with_the_parameter_form() {
    for (source, expected) in [
        (
            "class Items {\n@Check({\"argument\":0,\"type\":\"Input\"})\nasync accept(input) {}\n}",
            "@Check now sits on the parameter",
        ),
        (
            "class Items {\n@PageDefaults({\"argument\":0,\"defaultSize\":1,\"maxSize\":1,\"sort\":[]})\nasync list(page) {}\n}",
            "@PageDefaults now sits on the parameter",
        ),
        (
            "class Items {\nasync list(@PageDefaults({\"argument\":0,\"defaultSize\":1,\"maxSize\":1,\"sort\":[]}) page) {}\n}",
            "no \"argument\" key",
        ),
        (
            "class Items {\nasync find(@CacheKey id) {}\n}",
            "add @Cached to the method",
        ),
        (
            "class Items {\nasync list(@PageDefaults page) {}\n}",
            "page parameter of a Store query",
        ),
    ] {
        let failure = checked_lower("js", source, "yon").expect_err(source);
        assert!(failure.to_string().contains(expected), "{failure}");
    }
    let java = "class Items {\n    Object accept(@Check Object input) { return input; }\n}\n";
    let failure = checked_lower("java", java, "yon").expect_err("Java Check without a class");
    assert!(
        failure
            .to_string()
            .contains("@Check(ProductInput.class) Object input"),
        "{failure}"
    );
}

fn parameter_text(language: &str, parameters: &[&str]) -> String {
    let parameter_type = value_type(language);
    let mut spelled = Vec::new();
    for name in parameters {
        let parameter = match language {
            "js" | "py" => (*name).to_owned(),
            "ts" => format!("{name}: any"),
            "php" => format!("${name}"),
            "java" | "cs" => format!("{parameter_type} {name}"),
            "swift" => format!("_ {name}: {parameter_type}"),
            "go" => format!("{name} {parameter_type}"),
            _ => format!("{name}: {parameter_type}"),
        };
        spelled.push(parameter);
    }
    spelled.join(", ")
}

fn service(language: &str, name: &str, methods: &str, context: bool) -> String {
    let field = if context {
        match language {
            "js" => "    behaviors;\n",
            "ts" => "    behaviors: any;\n",
            "py" => "    behaviors: object\n",
            "php" => "    public $behaviors;\n",
            "java" => "    Behaviors.Context behaviors;\n",
            "kt" => "    lateinit var behaviors: Behaviors.Context\n",
            "cs" => "    public BehaviorContext behaviors;\n",
            "swift" => "    var behaviors: BehaviorContext\n",
            "rs" => "    behaviors: BehaviorContext,\n",
            "go" => "    behaviors *BehaviorContext\n",
            _ => unreachable!(),
        }
    } else {
        ""
    };
    if language == "go" {
        let methods = methods.replace(TestLanguages::GO_OWNER, name);
        return format!("{}{methods}", declaration(language, name, field));
    }
    if language == "rs" {
        format!(
            "{}impl {name} {{\n{methods}}}\n",
            declaration(language, name, field)
        )
    } else {
        declaration(language, name, &format!("{field}{methods}"))
    }
}

#[test]
fn valid_rejects_missing_argument_and_unknown_schema_in_every_language() {
    for language in TestLanguages::ALL {
        for (argument, schema, parameters) in [
            (1, "Input", vec!["value"]),
            (0, "Missing", vec!["value"]),
            (0, "Input", vec![]),
        ] {
            let annotation = marker(
                language,
                "Check",
                &format!(r#"{{"argument":{argument},"type":"{schema}"}}"#),
            );
            let body = if language == "py" {
                "return None"
            } else {
                "return value;"
            };
            let check = method(language, "check", &parameters, body);
            let methods = format!("    {annotation}\n{check}");
            let service_source = service(language, "Service", &methods, false);
            let source = format!("{}{service_source}", dto(language, false));
            assert_rejects(language, &source);
        }
    }
}

#[test]
fn configuration_names_must_match_the_selected_framework_in_every_language() {
    for language in TestLanguages::ALL {
        for environment in environments(language) {
            let wrong = if *environment == "yon" {
                "TAC_"
            } else {
                "YON_"
            };
            let source = configuration_source(language, wrong, "");
            assert!(
                checked_lower(language, &source, environment).is_err(),
                "{language}/{environment}: wrong prefix accepted"
            );
        }
    }
}

#[test]
fn nested_duplicate_json_keys_are_rejected_in_every_language() {
    for language in TestLanguages::ALL {
        for environment in environments(language) {
            let prefix = if *environment == "yon" {
                "YON_"
            } else {
                "TAC_"
            };
            for options in [
                r#","names":{"port":"PORT","port":"OTHER"}"#,
                r#","prefix":"YON_OTHER_""#,
            ] {
                let source = configuration_source(language, prefix, options);
                assert!(
                    checked_lower(language, &source, environment).is_err(),
                    "{language}/{environment}: duplicate JSON keys accepted"
                );
            }
        }
    }
}

fn configuration_source(language: &str, prefix: &str, options: &str) -> String {
    let annotation = marker(
        language,
        "Settings",
        &format!(r#"{{"prefix":"{prefix}APP_"{options}}}"#),
    );
    let body = format!("    {}", settings_port_field(language));
    let settings = declaration(language, "Settings", &body);
    format!("{annotation}\n{settings}")
}

fn settings_port_field(language: &str) -> &'static str {
    match language {
        "js" => "port = 80;",
        "ts" => "port: number = 80;",
        "py" => "port: int = 80",
        "php" => "public int $port = 80;",
        "java" | "cs" => "public int port = 80;",
        "kt" | "swift" => "var port: Int = 80",
        "rs" => "port: i64,",
        "go" => "port int64",
        _ => unreachable!(),
    }
}

#[test]
fn settings_read_types_defaults_and_bounds_from_declared_fields() {
    for language in TestLanguages::ALL {
        for environment in environments(language) {
            let prefix = if *environment == "yon" {
                "YON_"
            } else {
                "TAC_"
            };
            let options = if matches!(language, "rs" | "go") {
                r#","defaults":{"port":80}"#
            } else {
                ""
            };
            let source = configuration_source(language, prefix, options).replacen(
                settings_port_field(language),
                &format!(
                    "{}\n    {}\n    {}",
                    marker(language, "Min", "1"),
                    marker(language, "Max", "90"),
                    settings_port_field(language)
                ),
                1,
            );
            let lowered = checked_lower(language, &source, environment)
                .unwrap_or_else(|error| panic!("{language}/{environment}: {error}"))
                .expect("lowered settings");
            for expected in ["PORT", "80", "90"] {
                assert!(
                    lowered.contains(expected),
                    "{language}: {expected} missing:\n{lowered}"
                );
            }
            let too_large = source.replace("90", "70");
            assert!(
                checked_lower(language, &too_large, environment).is_err(),
                "{language}/{environment}: a default above Max was accepted"
            );
        }
    }
}

#[test]
fn old_settings_descriptors_are_refused_with_the_field_form() {
    let source = "@Settings({\"prefix\":\"YON_APP_\",\"fields\":{\"port\":{\"name\":\"PORT\",\"type\":\"integer\"}}})\nclass Settings {}\n";
    let failure = checked_lower("js", source, "yon").expect_err("old descriptor form");
    assert!(
        failure
            .to_string()
            .contains("declare each setting as a field"),
        "{failure}"
    );
    let defaults = "@Settings({\"prefix\":\"YON_APP_\",\"defaults\":{\"port\":80}})\nclass Settings {\n  port = 80;\n}\n";
    let failure = checked_lower("js", defaults, "yon").expect_err("defaults outside Rust and Go");
    assert!(
        failure.to_string().contains("only for Rust, Go and Ruby"),
        "{failure}"
    );
    let required =
        "@Settings({\"prefix\":\"YON_APP_\"})\nclass Settings {\n  @Required\n  port = 80;\n}\n";
    let failure = checked_lower("js", required, "yon").expect_err("Required on Settings");
    assert!(
        failure.to_string().contains("only @Min and @Max"),
        "{failure}"
    );
}

fn repository_source(language: &str, fields: &str, methods: &str) -> String {
    let entity = format!(
        "{}\n{}\n{}",
        marker(language, "Entity", r#"{"adapter":"data"}"#),
        marker(language, "Table", "\"products\""),
        declaration(language, "Product", &id_body(language))
    );
    format!(
        "{entity}{}\n{}\n{}",
        marker(language, "NeedsAll", "\"behaviors\""),
        marker(
            language,
            "Store",
            &format!(r#"Product, {{"fields":{fields},"defaultSize":20,"maxSize":100}}"#)
        ),
        service(language, "Products", methods, true)
    )
}

fn id_body(language: &str) -> String {
    format!(
        "    {}\n    {}",
        marker(language, "Id", ""),
        id_field(language)
    )
}

#[test]
fn empty_query_bodies_lower_and_nonempty_bodies_fail_in_every_language() {
    for language in TestLanguages::ALL {
        let annotation = marker(
            language,
            "Query",
            r#"{"operation":"find","where":[{"field":"id","operator":"eq","argument":0}],"orderBy":[]}"#,
        );
        let empty = format!(
            "    {annotation}\n{}",
            method(language, "selected", &["id"], "")
        );
        assert_accepts(language, &repository_source(language, r#"["id"]"#, &empty));
        let body = if language == "py" {
            "return None"
        } else {
            "return id;"
        };
        let nonempty = format!(
            "    {annotation}\n{}",
            method(language, "selected", &["id"], body)
        );
        assert_rejects(
            language,
            &repository_source(language, r#"["id"]"#, &nonempty),
        );
    }
}

#[test]
fn adjacent_derived_and_explicit_queries_preserve_declaration_boundaries() {
    for language in TestLanguages::ALL {
        let annotation = marker(
            language,
            "Query",
            r#"{"operation":"exists","where":[{"field":"id","operator":"eq","argument":0}],"orderBy":[]}"#,
        );
        let methods = format!(
            "{}{}    {annotation}\n{}{}    {annotation}\n{}",
            method(language, "findById", &["id"], ""),
            method(language, "countById", &["id"], ""),
            method(language, "selected", &["id"], ""),
            method(language, "existsById", &["id"], ""),
            method(language, "another", &["id"], ""),
        );
        let source = format!(
            "{}{}",
            repository_source(language, r#"["id"]"#, &methods),
            dto(language, false).trim_start_matches("<?php\n")
        );
        assert_accepts(language, &source);
    }
}

#[test]
fn qualified_native_query_return_types_are_discovered_without_initializer_calls() {
    for language in ["java", "cs"] {
        let methods = qualified_query_methods(language);
        let source = repository_source(language, r#"["id"]"#, &methods);
        for environment in environments(language) {
            let generated = checked_lower(language, &source, environment)
                .expect("qualified native declarations are valid")
                .expect("query wrappers are emitted");
            for declaration in methods.lines().take(3) {
                assert!(
                    !generated.contains(declaration.trim()),
                    "{language}/{environment}: query body was not generated: {declaration}"
                );
            }
            assert!(generated.contains("client.findByMissing(null)"));
            assert!(generated.contains("Object other = findByMissing(null)"));
        }
    }
}

fn qualified_query_methods(language: &str) -> String {
    let mut methods = String::new();
    for name in ["findById", "countById", "existsById"] {
        let declaration = if language == "cs" {
            format!(
                "    public async System.Threading.Tasks.Task<BehaviorJSON> {name}(BehaviorJSON id) {{ }}\n"
            )
        } else {
            format!("    public java.lang.Object {name}(Object id) {{ }}\n")
        };
        methods.push_str(&declaration);
    }
    methods.push_str("    Object cached = client.findByMissing(null);\n");
    methods.push_str("    Object other = findByMissing(null);\n");
    methods
}

#[test]
fn repository_field_allowlist_requires_entity_identity_even_without_methods() {
    for language in TestLanguages::ALL {
        assert_rejects(language, &repository_source(language, r#"["status"]"#, ""));
    }
}

#[test]
fn derived_queries_reject_unknown_ambiguous_and_overloaded_methods() {
    for language in TestLanguages::ALL {
        assert_rejects(
            language,
            &repository_source(
                language,
                r#"["id"]"#,
                &method(language, "findByMissing", &["value"], ""),
            ),
        );
        assert_rejects(
            language,
            &repository_source(
                language,
                r#"["id","status","statusEquals"]"#,
                &method(language, "findByStatusEquals", &["value"], ""),
            ),
        );
        for parameters in [vec!["value"], vec!["value", "other"]] {
            let duplicated = format!(
                "{}{}",
                method(language, "findById", &["value"], ""),
                method(language, "findById", &parameters, "")
            );
            assert_rejects(
                language,
                &repository_source(language, r#"["id"]"#, &duplicated),
            );
        }
    }
}

#[test]
fn bare_repository_stays_yon_di_while_object_repository_is_pure_behavior() {
    for language in TestLanguages::ALL {
        if language == "swift" {
            continue;
        }
        let empty = empty_body(language);
        let bare = marker(language, "Store", "");
        let source = format!("{bare}\n{}", declaration(language, "Storage", empty));
        let lowered =
            checked_lower(language, &source, "yon").expect("bare provider is not a behavior error");
        assert!(
            lowered
                .as_deref()
                .is_none_or(|lowered| lowered.contains(&bare)),
            "{language}: existing DI marker was consumed"
        );
        let object = repository_source(language, r#"["id"]"#, "");
        let lowered = checked_lower(language, &object, "yon")
            .expect("object repository")
            .expect("behavior output");
        assert!(
            !lowered.contains(&bare),
            "{language}: object repository unexpectedly synthesized a DI provider"
        );
    }
}

#[test]
fn an_entity_takes_its_identifier_from_the_id_field() {
    for language in TestLanguages::ALL {
        let entity = marker(language, "Entity", r#"{"adapter":"catalog"}"#);
        let table = marker(language, "Table", "\"products\"");
        let source = format!(
            "{entity}\n{table}\n{}",
            declaration(language, "ProductEntity", &id_body(language))
        );
        for environment in environments(language) {
            checked_lower(language, &source, environment)
                .unwrap_or_else(|error| panic!("{language}/{environment}: {error}"));
        }
        let environment = environments(language)[0];
        let named = source.replace(
            r#"{"adapter":"catalog"}"#,
            r#"{"adapter":"catalog","id":"id"}"#,
        );
        let failure = checked_lower(language, &named, environment)
            .expect_err(&format!("{language}: Entity id key"));
        assert!(
            failure
                .to_string()
                .contains("mark the identifier field with @Id"),
            "{language}: {failure}"
        );
        let without_id = format!(
            "{entity}\n{table}\n{}",
            declaration(
                language,
                "ProductEntity",
                &format!("    {}", id_field(language))
            )
        );
        let failure = checked_lower(language, &without_id, environment)
            .expect_err(&format!("{language}: missing Id"));
        assert!(
            failure.to_string().contains("exactly one field marked @Id"),
            "{language}: {failure}"
        );
    }
}

fn id_field(language: &str) -> &'static str {
    match language {
        "js" => "id;",
        "ts" => "id: number;",
        "py" => "id: int = 0",
        "php" => "public int $id = 0;",
        "java" | "cs" => "int id;",
        "kt" | "swift" => "var id: Int = 0",
        "rs" => "id: i64,",
        "go" => "id int64",
        _ => unreachable!(),
    }
}

#[test]
fn a_settings_text_default_at_the_end_of_a_line_is_read() {
    for (language, field) in [
        ("kt", "var title: String = \"Catalog\""),
        ("swift", "var title: String = \"Catalog\""),
        ("py", "title: str = 'Catalog'"),
    ] {
        let annotation = marker(language, "Settings", r#"{"prefix":"TAC_APP_"}"#);
        let source = format!(
            "{annotation}\n{}",
            declaration(language, "Settings", &format!("    {field}"))
        );
        let environment = if language == "py" { "yon" } else { "tac" };
        let source = source.replace("TAC_", if environment == "yon" { "YON_" } else { "TAC_" });
        let lowered = checked_lower(language, &source, environment)
            .unwrap_or_else(|error| panic!("{language}: {error}"))
            .expect("lowered settings");
        assert!(lowered.contains("Catalog"), "{language}: {lowered}");
    }
}
