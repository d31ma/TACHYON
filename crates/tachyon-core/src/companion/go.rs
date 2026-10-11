use crate::Failure;
use crate::go_source::{self, GoField, GoMethod, GoTypeKind};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GoResults {
    Nothing,
    Value,
    Failure,
    ValueAndFailure,
}

impl GoResults {
    pub(crate) fn read(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return Some(Self::Nothing);
        }
        let Some(inside) = text
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
        else {
            return Some(Self::single(text));
        };
        let results = go_source::parameters(inside);
        match results.as_slice() {
            [] => Some(Self::Nothing),
            [only] => Some(Self::single(&only.type_source)),
            [_, second] if second.type_source == "error" => Some(Self::ValueAndFailure),
            _ => None,
        }
    }

    fn single(type_source: &str) -> Self {
        if type_source == "error" {
            Self::Failure
        } else {
            Self::Value
        }
    }

    pub(crate) fn returned(self, call: &str) -> String {
        match self {
            Self::Nothing => format!("{call}\n\t\t\treturn nil, nil"),
            Self::Value => format!("return {call}, nil"),
            Self::Failure => format!("return nil, {call}"),
            Self::ValueAndFailure => format!("return {call}"),
        }
    }

    pub(crate) fn hook(self, call: &str) -> String {
        match self {
            Self::Nothing | Self::Value => format!("{call}\n\t\t\treturn nil"),
            Self::Failure => format!("return {call}"),
            Self::ValueAndFailure => format!("_, err := {call}\n\t\t\treturn err"),
        }
    }
}

pub(crate) struct GoTable<'a> {
    pub(crate) source: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) class: &'a str,
    pub(crate) excluded: &'a [String],
    pub(crate) extra: &'a str,
    pub(crate) needs: &'a str,
}

struct GoTypes;

impl GoTypes {
    const PUBLISHER_TYPES: [&str; 2] = ["ApplicationEventPublisher", "*ApplicationEventPublisher"];
    const UNPORTABLE_TYPE_STARTS: [&str; 4] = ["...", "func", "chan", "<-chan"];
}

pub(crate) fn masked_code(source: &str, source_path: &str) -> Result<String, Failure> {
    crate::annotation_layout::masked_source(Path::new(source_path), source)
}

pub(crate) fn without_markers(source: &str, source_path: &str) -> Result<String, Failure> {
    let code = masked_code(source, source_path)?;
    let mut output = String::with_capacity(source.len());
    for (line, masked) in source.split_inclusive('\n').zip(code.split_inclusive('\n')) {
        if !masked.trim_start().starts_with('@') {
            output.push_str(line);
            continue;
        }
        if line.ends_with('\n') {
            output.push('\n');
        }
    }
    Ok(output)
}

#[cfg(test)]
pub(crate) fn owner_table(table: &GoTable<'_>) -> Result<String, Failure> {
    owner_table_named(table, "TacCreate")
}

pub(crate) fn owner_table_named(table: &GoTable<'_>, factory: &str) -> Result<String, Failure> {
    let code = masked_code(table.source, table.source_path)?;
    let Some(body) = struct_body(&code, table.class) else {
        return Err(super::unnamed_service(
            table.source_path,
            Some(table.class),
            "struct",
        ));
    };
    let mut entries = Vec::new();
    for field in go_source::fields(&code, body) {
        if let Some(entry) = field_entry(table, &field) {
            entries.push(entry);
        }
    }
    for method in go_source::methods(&code) {
        if method.receiver_type != table.class {
            continue;
        }
        if let Some(entry) = method_entry(table, &code, &method) {
            entries.push(entry);
        }
    }
    entries.sort();
    if entries.is_empty() && table.extra.trim().is_empty() {
        return Err(super::exposes_nothing(table.source_path, table.class));
    }
    let start = crate::declared_contract::DeclaredContract::GO_NEEDS_START;
    let starts_needs = code.contains(&format!("func (service *{}) {start}() error", table.class));
    let injection = if starts_needs {
        format!("\tif err := instance.{start}(); err != nil {{\n\t\tpanic(err)\n\t}}\n")
    } else {
        String::new()
    };
    let mut output = format!(
        "func {factory}() []tac.Member {{\n\tinstance := &{}{{}}\n{injection}{}\treturn []tac.Member{{\n\t\t{{Instance: instance}},\n",
        table.class, table.needs
    );
    for (_, entry) in entries {
        output.push_str(&entry);
    }
    output.push_str(table.extra);
    output.push_str("\t}\n}\n");
    Ok(output)
}

fn struct_body(code: &str, class: &str) -> Option<std::ops::Range<usize>> {
    for declared in go_source::types(code) {
        if declared.name == class && declared.kind == GoTypeKind::Struct {
            return Some(declared.body);
        }
    }
    None
}

fn is_exposed(name: &str, excluded: &[String]) -> bool {
    let exported = name.chars().next().is_some_and(char::is_uppercase);
    exported && !excluded.iter().any(|excluded_name| excluded_name == name)
}

fn field_entry(table: &GoTable<'_>, field: &GoField) -> Option<(String, String)> {
    if !is_exposed(&field.name, table.excluded) {
        return None;
    }
    if GoTypes::PUBLISHER_TYPES.contains(&field.type_source.as_str()) {
        return None;
    }
    let name = &field.name;
    let key = go_source::lower_first(name);
    let entry = format!(
        "\t\t{{Name: {key:?}, Read: func() any {{ return instance.{name} }}, Write: func(value any) error {{ return tac.Assign(&instance.{name}, value) }}}},\n"
    );
    Some((key, entry))
}

fn method_entry(table: &GoTable<'_>, code: &str, method: &GoMethod) -> Option<(String, String)> {
    if !is_exposed(&method.name, table.excluded) {
        return None;
    }
    let results = GoResults::read(&code[method.results.clone()])?;
    let parameters = parameter_types(code, method)?;
    let key = go_source::lower_first(&method.name);
    let mut entry = format!("\t\t{{Name: {key:?}, Call: func(arguments []any) (any, error) {{\n");
    let mut passed = Vec::new();
    for (index, type_source) in parameters.iter().enumerate() {
        let _ = write!(
            entry,
            "\t\t\tvar argument{index} {type_source}\n\t\t\tif err := tac.Argument(arguments, {index}, &argument{index}); err != nil {{\n\t\t\t\treturn nil, err\n\t\t\t}}\n"
        );
        passed.push(format!("argument{index}"));
    }
    let call = format!("instance.{}({})", method.name, passed.join(", "));
    let _ = write!(entry, "\t\t\t{}\n\t\t}}}},\n", results.returned(&call));
    Some((key, entry))
}

pub(crate) fn parameter_types(code: &str, method: &GoMethod) -> Option<Vec<String>> {
    let mut types = Vec::new();
    for parameter in go_source::parameters(&code[method.parameters.clone()]) {
        let type_source = parameter.type_source.trim();
        let unportable = GoTypes::UNPORTABLE_TYPE_STARTS
            .iter()
            .any(|start| type_source.starts_with(start));
        if unportable {
            return None;
        }
        types.push(type_source.to_owned());
    }
    Some(types)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    struct Fixtures;

    impl Fixtures {
        const SERVICE: &str = "package main\n\nimport \"strings\"\n\ntype SearchService struct {\n\tResults []string\n\tQuery, Mode string `json:\"q\"`\n\thidden int\n\tPublisher ApplicationEventPublisher\n}\n\n\
             func (search *SearchService) Find(text string, limit int) ([]string, error) {\n\treturn nil, nil\n}\n\n\
             func (search SearchService) Clear() {\n}\n\n\
             func (search *SearchService) Check() error {\n\treturn nil\n}\n\n\
             func (search *SearchService) Pick(values ...string) string {\n\treturn strings.Join(values, \"\")\n}\n\n\
             func (search *SearchService) Pair() (int, int) {\n\treturn 1, 2\n}\n\n\
             func (search *SearchService) secret() {}\n\n\
             func (search *SearchService) Started() {}\n";
    }

    fn table(source: &str, excluded: &[String]) -> Result<String, Failure> {
        owner_table(&GoTable {
            source,
            source_path: "client/services/SearchService.go",
            class: "SearchService",
            excluded,
            extra: "",
            needs: "",
        })
    }

    #[test]
    fn page_composition_default_factory_keeps_native_caller_allocation_and_injection() {
        let source = "type SearchService struct { Count int }\n\nfunc (service *SearchService) tachyonInject() error { return nil }\n\nfunc (service *SearchService) Read() int { return service.Count }\n";
        let output =
            table(source, &[String::from("tachyonInject")]).expect("native default factory");
        assert!(
            output.starts_with("func TacCreate() []tac.Member {\n\tinstance := &SearchService{}\n")
        );
        assert_eq!(output.matches("func TacCreate()").count(), 1);
        assert!(output.contains("if err := instance.tachyonInject(); err != nil"));
        assert!(output.contains("{Name: \"count\", Read:"));
        assert!(output.contains("{Name: \"read\", Call:"));
        assert!(!output.contains("{Name: \"tachyonInject\""));
    }

    #[test]
    fn page_composition_owner_table_keeps_the_selected_class_hooks_and_publisher_types() {
        let source = "type WriterService struct { Count int; Publisher ApplicationEventPublisher }\n\ntype ReaderService struct { Label string; Publisher *ApplicationEventPublisher }\n\nfunc (writer *WriterService) Start() {}\nfunc (writer *WriterService) Stop() {}\nfunc (writer *WriterService) Increment() int { writer.Count++; return writer.Count }\n\nfunc (reader *ReaderService) Start() {}\nfunc (reader *ReaderService) Stop() {}\nfunc (reader *ReaderService) Read() string { return reader.Label }\n";
        for (class, field, method, other) in [
            ("WriterService", "count", "Increment", "ReaderService"),
            ("ReaderService", "label", "Read", "WriterService"),
        ] {
            let extra = "\t\t{Name: \"owned-start\", Hook: func() error { instance.Start(); return nil }},\n\t\t{Name: \"owned-stop\", Destroy: true, Hook: func() error { instance.Stop(); return nil }},\n";
            let output = owner_table(&GoTable {
                source,
                source_path: &format!("client/services/{class}.go"),
                class,
                excluded: &[String::from("Start"), String::from("Stop")],
                extra,
                needs: "",
            })
            .expect("selected owner");
            assert!(output.starts_with(&format!(
                "func TacCreate() []tac.Member {{\n\tinstance := &{class}{{}}\n"
            )));
            assert!(output.contains(&format!("{{Name: \"{field}\", Read:")));
            assert!(output.contains(&format!("instance.{method}()")));
            assert!(output.contains(extra));
            for hidden in ["start", "stop", "publisher"] {
                assert!(
                    !output.contains(&format!("{{Name: \"{hidden}\"")),
                    "{hidden}"
                );
            }
            assert!(!output.contains(other));
        }
    }

    #[test]
    fn exported_fields_and_methods_become_lower_first_members() {
        let output = table(Fixtures::SERVICE, &[String::from("Started")]).expect("table");
        assert!(
            output.starts_with("func TacCreate() []tac.Member {\n\tinstance := &SearchService{}\n")
        );
        assert!(output.contains("{Name: \"results\", Read: func() any { return instance.Results }, Write: func(value any) error { return tac.Assign(&instance.Results, value) }},"));
        assert!(output.contains("{Name: \"query\","));
        assert!(output.contains("{Name: \"mode\","));
        assert!(output.contains("var argument1 int\n"));
        assert!(output.contains("return instance.Find(argument0, argument1)\n"));
        assert!(output.contains("instance.Clear()\n\t\t\treturn nil, nil\n"));
        assert!(output.contains("return nil, instance.Check()\n"));
        for hidden in ["hidden", "publisher", "pick", "pair", "secret", "started"] {
            assert!(
                !output.contains(&format!("{{Name: \"{hidden}\"")),
                "{hidden}"
            );
        }
        let check = output.find("\"check\"").expect("check");
        let results = output.find("\"results\"").expect("results");
        assert!(check < results, "members are sorted by key");
    }

    #[test]
    fn a_service_without_its_struct_or_members_is_refused() {
        let missing = table("type OtherService struct{ Count int }\n", &[]).expect_err("missing");
        assert!(
            missing.diagnostics()[0]
                .message
                .contains("declares no struct named 'SearchService'")
        );
        let empty = table("type SearchService struct{ count int }\n", &[]).expect_err("empty");
        assert!(
            empty.diagnostics()[0]
                .message
                .contains("exposes no members")
        );
    }

    #[test]
    fn result_shapes_are_read_from_the_signature() {
        assert_eq!(GoResults::read(""), Some(GoResults::Nothing));
        assert_eq!(GoResults::read("int"), Some(GoResults::Value));
        assert_eq!(GoResults::read("error"), Some(GoResults::Failure));
        assert_eq!(
            GoResults::read("(value int, err error)"),
            Some(GoResults::ValueAndFailure)
        );
        assert_eq!(GoResults::read("(Guide)"), Some(GoResults::Value));
        assert_eq!(GoResults::read("(int, string)"), None);
    }

    #[test]
    fn marker_lines_become_blank_lines_and_strings_stay() {
        let source = "// @Store\ntype A struct {\n\t// @Needs\n\tb *B\n}\n\nfunc (a *A) Text() string {\n\treturn `\n#[kept]\n`\n}";
        let cleaned = without_markers(source, "A.go").expect("masked");
        assert_eq!(
            cleaned,
            "\ntype A struct {\n\n\tb *B\n}\n\nfunc (a *A) Text() string {\n\treturn `\n#[kept]\n`\n}"
        );
    }
}
