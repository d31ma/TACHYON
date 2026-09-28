use super::{Input, Marker};
use crate::Failure;
use serde_json::{Value, json};
use std::collections::BTreeSet;

struct QueryLimits;

impl QueryLimits {
    const MAX_FIELDS: usize = 64;
    const MAX_PREDICATES: usize = 8;
    const MAX_SORT: usize = 4;
    const MAX_ARGUMENTS: usize = 32;
    const MAX_NAME: usize = 256;
    const MAX_PARSE_STEPS: usize = 4096;
}

pub(super) fn repository(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    let extension = input.extension();
    let example = format!(
        "Store({}, {{\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100}})",
        super::literal::class_reference_example(extension, "Product")
    );
    if marker.arguments.trim_start().starts_with('{') {
        return Err(input.failure(
            marker.span.start,
            format!("Store names its Entity class first, not as a string: write {example}"),
        ));
    }
    let (reference, options) = super::literal::reference_and_options(&marker.arguments);
    let entity = super::literal::class_reference(extension, reference);
    let (Some(entity), Some(options)) = (entity, options) else {
        return Err(input.failure(
            marker.span.start,
            format!("Store takes its Entity class and its options: write {example}"),
        ));
    };
    let options_marker = Marker {
        arguments: options.to_owned(),
        ..marker.clone()
    };
    let mut object = super::object(
        input,
        &options_marker,
        &["fields", "defaultSize", "maxSize"],
    )?;
    object.insert("entity".to_owned(), Value::String(entity.to_owned()));
    repository_object(input, marker, object)
}

fn repository_object(
    input: &Input<'_>,
    marker: &Marker,
    object: serde_json::Map<String, Value>,
) -> Result<Value, Failure> {
    super::named(input, marker, &object, "entity")?;
    super::bounded_integer(input, marker, &object, "defaultSize", 1, 100)?;
    super::bounded_integer(input, marker, &object, "maxSize", 1, 100)?;
    if object["defaultSize"].as_u64() > object["maxSize"].as_u64() {
        return Err(input.failure(marker.span.start, "Store.defaultSize cannot exceed maxSize"));
    }
    let Some(fields) = object["fields"].as_array() else {
        return Err(input.failure(
            marker.span.start,
            "Store.fields requires an explicit field allowlist",
        ));
    };
    let too_many = fields.len() > QueryLimits::MAX_FIELDS;
    if fields.is_empty() || too_many || !unique_identifiers(fields) {
        return Err(input.failure(
            marker.span.start,
            "Store.fields requires 1–64 unique safe identifiers",
        ));
    }
    Ok(Value::Object(object))
}

fn unique_identifiers(fields: &[Value]) -> bool {
    let mut seen = BTreeSet::new();
    for field in fields {
        let Some(name) = field.as_str() else {
            return false;
        };
        if !super::identifier(name) || !seen.insert(name) {
            return false;
        }
    }
    true
}

pub(super) fn explicit(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    let written = super::literal::parse(&marker.arguments).ok();
    if written
        .as_ref()
        .is_some_and(|value| value.get("pageArgument").is_some())
    {
        return Err(input.failure(
            marker.span.start,
            "Query has no pageArgument now: remove it and mark the page parameter with @PageDefaults",
        ));
    }
    let mut object = super::object(input, marker, &["operation", "where", "orderBy"])?;
    object.insert("pageArgument".to_owned(), Value::Null);
    let query = Value::Object(object);
    validate_query(input, marker.span.start, &query)?;
    Ok(query)
}

fn exact_keys(value: &Value, keys: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != keys.len() {
        return false;
    }
    object.keys().all(|key| keys.contains(&key.as_str()))
}

fn argument(value: &Value) -> Option<usize> {
    value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < QueryLimits::MAX_ARGUMENTS)
}

fn validate_query(input: &Input<'_>, at: usize, query: &Value) -> Result<(), Failure> {
    if !exact_keys(query, &["operation", "where", "pageArgument", "orderBy"]) {
        return Err(input.failure(
            at,
            "Query requires exactly operation, where, pageArgument and orderBy",
        ));
    }
    let operation = query["operation"].as_str().unwrap_or("");
    if !matches!(operation, "find" | "count" | "exists") {
        return Err(input.failure(
            at,
            "Query.operation must be find, count or exists; derived mutations are unsupported",
        ));
    }
    validate_predicates(input, at, &query["where"])?;
    if !query["pageArgument"].is_null() && argument(&query["pageArgument"]).is_none() {
        return Err(input.failure(
            at,
            "Query.pageArgument must be null or an argument index 0–31",
        ));
    }
    let order = validate_order(input, at, &query["orderBy"])?;
    if operation != "find" && (!query["pageArgument"].is_null() || !order.is_empty()) {
        return Err(input.failure(
            at,
            "Count and exists queries take no page parameter and use orderBy []",
        ));
    }
    Ok(())
}

fn validate_predicates(input: &Input<'_>, at: usize, predicates: &Value) -> Result<(), Failure> {
    let Some(predicates) = predicates.as_array() else {
        return Err(too_many_predicates(input, at));
    };
    if predicates.len() > QueryLimits::MAX_PREDICATES {
        return Err(too_many_predicates(input, at));
    }
    for predicate in predicates {
        validate_predicate(input, at, predicate)?;
    }
    Ok(())
}

fn too_many_predicates(input: &Input<'_>, at: usize) -> Failure {
    input.failure(
        at,
        "Query.where requires an array of at most eight AND predicates",
    )
}

fn validate_predicate(input: &Input<'_>, at: usize, predicate: &Value) -> Result<(), Failure> {
    let operator = predicate["operator"].as_str().unwrap_or("");
    let keys: &[&str] = match operator {
        "isNull" | "isNotNull" => &["field", "operator"],
        "eq" | "ne" | "lt" | "lte" | "gt" | "gte" | "in" => &["field", "operator", "argument"],
        _ => return Err(input.failure(at, "Unsupported Query predicate operator")),
    };
    let safe_field = predicate["field"].as_str().is_some_and(super::identifier);
    let needs_argument = keys.len() == 3;
    let missing_argument = needs_argument && argument(&predicate["argument"]).is_none();
    if !exact_keys(predicate, keys) || !safe_field || missing_argument {
        return Err(input.failure(at, "Query predicates require a safe field and operator; value predicates require an argument index 0–31, null predicates cannot bind arguments"));
    }
    Ok(())
}

fn validate_order<'query>(
    input: &Input<'_>,
    at: usize,
    order: &'query Value,
) -> Result<&'query Vec<Value>, Failure> {
    let Some(order) = order.as_array() else {
        return Err(too_many_sort_fields(input, at));
    };
    if order.len() > QueryLimits::MAX_SORT {
        return Err(too_many_sort_fields(input, at));
    }
    let mut seen = BTreeSet::new();
    for sort in order {
        let unique_safe_field = sort["field"]
            .as_str()
            .is_some_and(|name| super::identifier(name) && seen.insert(name));
        let known_direction = matches!(sort["direction"].as_str(), Some("asc" | "desc"));
        if !exact_keys(sort, &["field", "direction"]) || !unique_safe_field || !known_direction {
            return Err(input.failure(
                at,
                "Query.orderBy requires unique safe fields and asc/desc directions",
            ));
        }
    }
    Ok(order)
}

fn too_many_sort_fields(input: &Input<'_>, at: usize) -> Failure {
    input.failure(
        at,
        "Query.orderBy requires an array of at most four sort fields",
    )
}

pub(super) fn derives_name(name: &str) -> bool {
    derived_prefix(name).is_some()
}

fn derived_prefix(name: &str) -> Option<(&'static str, &str, bool)> {
    for (operation, camel, snake) in [
        ("find", "findBy", "find_by_"),
        ("count", "countBy", "count_by_"),
        ("exists", "existsBy", "exists_by_"),
    ] {
        if let Some(tail) = name.strip_prefix(camel) {
            return Some((operation, tail, false));
        }
        if let Some(tail) = name.strip_prefix(snake) {
            return Some((operation, tail, true));
        }
    }
    None
}

#[derive(Clone)]
struct Term {
    field: String,
    operator: &'static str,
}

struct DerivedParser {
    candidates: Vec<(String, Term)>,
    separator: &'static str,
    steps: usize,
    exhausted: bool,
    solutions: Vec<Vec<Term>>,
}

impl DerivedParser {
    fn new(fields: &[Value], snake: bool) -> Self {
        let suffixes = [
            ("", "", "eq"),
            ("Equals", "_equals", "eq"),
            ("NotEquals", "_not_equals", "ne"),
            ("LessThan", "_less_than", "lt"),
            ("LessThanEqual", "_less_than_equal", "lte"),
            ("GreaterThan", "_greater_than", "gt"),
            ("GreaterThanEqual", "_greater_than_equal", "gte"),
            ("In", "_in", "in"),
            ("IsNull", "_is_null", "isNull"),
            ("IsNotNull", "_is_not_null", "isNotNull"),
        ];
        let mut candidates = Vec::new();
        for field in fields.iter().filter_map(Value::as_str) {
            let spelling = field_spelling(field, snake);
            for (camel_suffix, snake_suffix, operator) in suffixes {
                let suffix = if snake { snake_suffix } else { camel_suffix };
                let term = Term {
                    field: field.to_owned(),
                    operator,
                };
                candidates.push((format!("{spelling}{suffix}"), term));
            }
        }
        Self {
            candidates,
            separator: if snake { "_and_" } else { "And" },
            steps: 0,
            exhausted: false,
            solutions: Vec::new(),
        }
    }

    fn should_stop(&self) -> bool {
        self.solutions.len() > 1 || self.exhausted
    }

    fn walk(&mut self, tail: &str, terms: &mut Vec<Term>) {
        if self.should_stop() || terms.len() >= QueryLimits::MAX_PREDICATES {
            return;
        }
        for index in 0..self.candidates.len() {
            self.steps += 1;
            if self.steps > QueryLimits::MAX_PARSE_STEPS {
                self.exhausted = true;
                return;
            }
            let (spelling, term) = &self.candidates[index];
            let Some(rest) = tail.strip_prefix(spelling) else {
                continue;
            };
            let term = term.clone();
            terms.push(term);
            if rest.is_empty() {
                self.solutions.push(terms.clone());
            } else if let Some(next) = rest
                .strip_prefix(self.separator)
                .filter(|next| !next.is_empty())
            {
                self.walk(next, terms);
            }
            terms.pop();
            if self.should_stop() {
                return;
            }
        }
    }
}

fn field_spelling(field: &str, snake: bool) -> String {
    if snake {
        snake_case_spelling(field)
    } else {
        camel_case_spelling(field)
    }
}

fn snake_case_spelling(field: &str) -> String {
    let mut result = String::new();
    for (index, character) in field.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 && !result.ends_with('_') {
            result.push('_');
        }
        result.push(character.to_ascii_lowercase());
    }
    result
}

fn camel_case_spelling(field: &str) -> String {
    let mut result = String::new();
    let mut uppercase = true;
    for character in field.chars() {
        if character == '_' {
            uppercase = true;
            continue;
        }
        if uppercase {
            result.push(character.to_ascii_uppercase());
        } else {
            result.push(character);
        }
        uppercase = false;
    }
    result
}

fn derived(input: &Input<'_>, at: usize, name: &str, fields: &[Value]) -> Result<Value, Failure> {
    let Some((operation, tail, snake)) = derived_prefix(name) else {
        return Err(input.failure(
            at,
            "Derived queries require findBy/countBy/existsBy or their snake_case prefixes",
        ));
    };
    if name.len() > QueryLimits::MAX_NAME || tail.is_empty() {
        return Err(input.failure(
            at,
            "Derived query names require a field predicate and at most 256 bytes",
        ));
    }
    let mut parser = DerivedParser::new(fields, snake);
    parser.walk(tail, &mut Vec::new());
    if parser.exhausted {
        return Err(input.failure(
            at,
            "Derived query parsing exceeds its bounded work budget; use explicit Query metadata",
        ));
    }
    if parser.solutions.len() != 1 {
        return Err(input.failure(at, "Derived query is unknown or ambiguous; use exact allowlisted fields, at most eight AND predicates, or explicit Query metadata"));
    }
    let predicates = predicates_from_terms(&parser.solutions[0]);
    Ok(json!({"operation":operation,"where":predicates,"pageArgument":null,"orderBy":[]}))
}

fn predicates_from_terms(terms: &[Term]) -> Vec<Value> {
    let mut next_argument = 0;
    let mut predicates = Vec::new();
    for term in terms {
        let mut value = json!({"field":term.field,"operator":term.operator});
        if !matches!(term.operator, "isNull" | "isNotNull") {
            value["argument"] = json!(next_argument);
            next_argument += 1;
        }
        predicates.push(value);
    }
    predicates
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve(
    input: &Input<'_>,
    at: usize,
    repository: &Value,
    entity_name: &str,
    entity: &Value,
    table: &str,
    method_name: &str,
    arity: usize,
    query: Option<&Value>,
    page: Option<&Value>,
) -> Result<Value, Failure> {
    let marker = repository_marker(at);
    let stored = repository.as_object().cloned().unwrap_or_default();
    let repository = repository_object(input, &marker, stored)?;
    if !exact_binding(&repository, entity_name, entity, table, arity) {
        return Err(input.failure(at, "Store requires an exact source-local Entity/Table binding and at most 32 method parameters"));
    }
    let fields = repository["fields"]
        .as_array()
        .ok_or_else(|| input.failure(at, "Store fields missing"))?;
    if !fields.contains(&entity["id"]) {
        return Err(input.failure(at, "Store.fields must include its Entity id field"));
    }
    let mut query = match query {
        Some(query) => query.clone(),
        None => derived(input, at, method_name, fields)?,
    };
    if let Some(page) = page {
        query["pageArgument"] = page.clone();
    }
    validate_query(input, at, &query)?;
    check_bindings(input, at, &query, fields, arity)?;
    let mut result = query.as_object().cloned().unwrap_or_default();
    if query["operation"] == "find" {
        add_id_tiebreaker(input, at, &mut result, &entity["id"])?;
    }
    result.insert(
        "entity".to_owned(),
        json!({"name":entity_name,"adapter":entity["adapter"],"id":entity["id"],"table":table}),
    );
    for key in ["fields", "defaultSize", "maxSize"] {
        result.insert(key.to_owned(), repository[key].clone());
    }
    Ok(Value::Object(result))
}

fn repository_marker(at: usize) -> Marker {
    Marker {
        name: "Store".to_owned(),
        arguments: String::new(),
        span: at..at,
        target: at,
        parameter: false,
    }
}

fn exact_binding(
    repository: &Value,
    entity_name: &str,
    entity: &Value,
    table: &str,
    arity: usize,
) -> bool {
    arity <= QueryLimits::MAX_ARGUMENTS
        && repository["entity"].as_str() == Some(entity_name)
        && super::identifier(entity_name)
        && super::identifier(table)
        && entity["adapter"].as_str().is_some_and(super::identifier)
        && entity["id"].as_str().is_some_and(super::identifier)
}

fn check_bindings(
    input: &Input<'_>,
    at: usize,
    query: &Value,
    fields: &[Value],
    arity: usize,
) -> Result<(), Failure> {
    let mut used = BTreeSet::new();
    for predicate in query["where"].as_array().into_iter().flatten() {
        if !fields.contains(&predicate["field"]) {
            return Err(input.failure(
                at,
                "Query predicate field is not in the exact Store.fields allowlist",
            ));
        }
        if let Some(index) = argument(&predicate["argument"]) {
            used.insert(index);
        }
    }
    for sort in query["orderBy"].as_array().into_iter().flatten() {
        if !fields.contains(&sort["field"]) {
            return Err(input.failure(
                at,
                "Query sort field is not in the exact Store.fields allowlist",
            ));
        }
    }
    if let Some(index) = argument(&query["pageArgument"])
        && !used.insert(index)
    {
        return Err(input.failure(
            at,
            "Query pagination argument cannot also supply a predicate value",
        ));
    }
    if used != (0..arity).collect() {
        return Err(input.failure(at, "Query bindings must use every declared parameter and cannot reference missing parameters"));
    }
    Ok(())
}

fn add_id_tiebreaker(
    input: &Input<'_>,
    at: usize,
    result: &mut serde_json::Map<String, Value>,
    id: &Value,
) -> Result<(), Failure> {
    let order = result
        .get_mut("orderBy")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| input.failure(at, "Query ordering missing"))?;
    if !order.iter().any(|sort| sort["field"] == *id) {
        order.push(json!({"field":id,"direction":"asc"}));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::path::Path;

    fn input() -> Input<'static> {
        Input {
            path: Path::new("query.rs"),
            source: "",
            code: String::new(),
        }
    }
    fn marker(name: &str, value: &Value) -> Marker {
        Marker {
            name: name.to_owned(),
            arguments: value.to_string(),
            span: 0..0,
            target: 0,
            parameter: false,
        }
    }
    fn repository_policy() -> Value {
        json!({"entity":"Product","fields":["id","status","price","deletedAt"],"defaultSize":20,"maxSize":100})
    }
    fn query() -> Value {
        json!({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"pageArgument":1,"orderBy":[{"field":"id","direction":"asc"}]})
    }
    fn product_entity() -> Value {
        json!({"adapter":"data","id":"id"})
    }
    fn resolve_query(name: &str, arity: usize, query: Option<&Value>) -> Result<Value, Failure> {
        let repository = repository_policy();
        resolve_derived(&repository, name, arity, query)
    }
    fn resolve_derived(
        repository: &Value,
        name: &str,
        arity: usize,
        query: Option<&Value>,
    ) -> Result<Value, Failure> {
        let entity = product_entity();
        resolve_binding(
            "Product", &entity, "products", repository, name, arity, query,
        )
    }
    fn resolve_binding(
        entity_name: &str,
        entity: &Value,
        table: &str,
        repository: &Value,
        name: &str,
        arity: usize,
        query: Option<&Value>,
    ) -> Result<Value, Failure> {
        resolve(
            &input(),
            0,
            repository,
            entity_name,
            entity,
            table,
            name,
            arity,
            query,
            None,
        )
    }
    fn check_repository(policy: &Value) -> Result<Value, Failure> {
        let mut options = policy.clone();
        let entity = options
            .as_object_mut()
            .and_then(|object| object.remove("entity"))
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        let mut store = marker("Store", &options);
        store.arguments = format!("{entity}, {options}");
        repository(&input(), &store)
    }
    fn check_query(query: &Value) -> Result<Value, Failure> {
        let mut written = query.clone();
        if let Some(object) = written.as_object_mut() {
            object.remove("pageArgument");
        }
        explicit(&input(), &marker("Query", &written))
    }

    #[test]
    fn resolves_structured_queries_without_executable_text() {
        let result = resolve_query("available", 2, Some(&query())).expect("valid query");
        assert_eq!(
            result["entity"],
            json!({"name":"Product","adapter":"data","id":"id","table":"products"})
        );
        assert_eq!(result["defaultSize"], 20);
        assert_eq!(result["where"], query()["where"]);
    }

    #[test]
    fn repository_rejects_unknown_fields_duplicates_and_bad_bounds() {
        for (key, value) in [
            ("entity", json!("a.b")),
            ("fields", json!([])),
            ("fields", json!(["id", "id"])),
            ("fields", json!(["id;drop"])),
            ("defaultSize", json!(101)),
            ("maxSize", json!(0)),
        ] {
            let mut policy = repository_policy();
            policy[key] = value;
            assert!(check_repository(&policy).is_err());
        }
        let mut policy = repository_policy();
        policy["unknown"] = json!(true);
        assert!(check_repository(&policy).is_err());
        policy = repository_policy();
        policy["maxSize"] = json!(10);
        assert!(check_repository(&policy).is_err());
    }

    #[test]
    fn a_store_names_its_entity_class_before_its_options() {
        let stored = check_repository(&repository_policy()).expect("new form");
        assert_eq!(stored, repository_policy());
        let old_form = marker("Store", &repository_policy());
        let refused = repository(&input(), &old_form).expect_err("old form");
        assert!(format!("{refused:?}").contains("Store(Product, {"));
        let mut without_options = marker("Store", &Value::Null);
        without_options.arguments = "Product".to_owned();
        assert!(repository(&input(), &without_options).is_err());
    }

    #[test]
    fn derived_camel_and_snake_share_exact_plans() {
        let camel = resolve_query(
            "findByStatusAndPriceGreaterThanEqualAndDeletedAtIsNull",
            2,
            None,
        )
        .expect("camel query");
        let snake = resolve_query(
            "find_by_status_and_price_greater_than_equal_and_deleted_at_is_null",
            2,
            None,
        )
        .expect("snake query");
        assert_eq!(camel, snake);
        assert_eq!(
            camel["where"][2],
            json!({"field":"deletedAt","operator":"isNull"})
        );
    }

    #[test]
    fn derived_operators_and_read_operations_are_explicit() {
        for (suffix, operator, arity) in [
            ("", "eq", 1),
            ("Equals", "eq", 1),
            ("NotEquals", "ne", 1),
            ("LessThan", "lt", 1),
            ("LessThanEqual", "lte", 1),
            ("GreaterThan", "gt", 1),
            ("GreaterThanEqual", "gte", 1),
            ("In", "in", 1),
            ("IsNull", "isNull", 0),
            ("IsNotNull", "isNotNull", 0),
        ] {
            for operation in ["find", "count", "exists"] {
                let result = resolve_query(&format!("{operation}ById{suffix}"), arity, None)
                    .expect("supported derived operator");
                assert_eq!(result["operation"], operation);
                assert_eq!(result["where"][0]["operator"], operator);
            }
        }
        for name in [
            "deleteById",
            "findBy",
            "findByUnknown",
            "findByIdOrStatus",
            "findByIdOrderByStatus",
        ] {
            assert!(resolve_query(name, 1, None).is_err());
        }
        assert!(derives_name("findBy"));
        assert!(derives_name("exists_by_id"));
        assert!(!derives_name("findAll"));
    }

    #[test]
    fn derived_ambiguity_is_rejected_instead_of_guessed() {
        let mut policy = repository_policy();
        policy["fields"] = json!(["id", "status", "statusEquals"]);
        assert!(resolve_derived(&policy, "findByStatusEquals", 1, None).is_err());
        policy["fields"] = json!(["id", "status", "price", "statusAndPrice"]);
        assert!(resolve_derived(&policy, "findByStatusAndPrice", 2, None).is_err());
    }

    #[test]
    fn bindings_allow_reuse_but_never_missing_unused_or_page_overlap() {
        assert!(resolve_query("available", 1, Some(&query())).is_err());
        assert!(resolve_query("available", 3, Some(&query())).is_err());
        let mut value = query();
        value["pageArgument"] = json!(0);
        assert!(resolve_query("available", 1, Some(&value)).is_err());
        value["pageArgument"] = Value::Null;
        value["where"]
            .as_array_mut()
            .expect("array")
            .push(json!({"field":"price","operator":"eq","argument":0}));
        assert!(resolve_query("available", 1, Some(&value)).is_ok());
        assert!(resolve_query("findById", 0, None).is_err());
        assert!(resolve_query("findById", 33, None).is_err());
    }

    #[test]
    fn rejects_unsafe_or_inexact_entity_and_field_bindings() {
        for (entity_name, entity, table) in [
            ("Other", json!({"adapter":"data","id":"id"}), "products"),
            (
                "Product",
                json!({"adapter":"data","id":"missing"}),
                "products",
            ),
            (
                "Product",
                json!({"adapter":"../data","id":"id"}),
                "products",
            ),
            (
                "Product",
                json!({"adapter":"data","id":"id"}),
                "schema.products",
            ),
        ] {
            let policy = repository_policy();
            let resolved =
                resolve_binding(entity_name, &entity, table, &policy, "findById", 1, None);
            assert!(resolved.is_err());
        }
        for location in ["where", "orderBy"] {
            let mut value = query();
            value[location][0]["field"] = json!("Status");
            assert!(resolve_query("available", 2, Some(&value)).is_err());
        }
    }

    #[test]
    fn strict_query_schema_and_aggregate_restrictions() {
        assert!(explicit(&input(), &marker("Query", &query())).is_err());
        for (key, value) in [
            ("operation", json!("delete")),
            ("where", json!({})),
            ("orderBy", json!({})),
        ] {
            let mut changed_query = query();
            changed_query[key] = value;
            assert!(check_query(&changed_query).is_err());
        }
        for operation in ["count", "exists"] {
            let mut value = query();
            value["operation"] = json!(operation);
            assert!(check_query(&value).is_err());
            value["pageArgument"] = Value::Null;
            value["orderBy"] = json!([]);
            assert!(resolve_query("aggregate", 1, Some(&value)).is_ok());
            let page = json!(1);
            let paged = resolve(
                &input(),
                0,
                &repository_policy(),
                "Product",
                &product_entity(),
                "products",
                "aggregate",
                2,
                Some(&value),
                Some(&page),
            );
            assert!(paged.is_err());
        }
        let mut value = query();
        value["where"][0]["operator"] = json!("isNull");
        assert!(check_query(&value).is_err());
        value = query();
        value["where"][0]["sql"] = json!("interpolation");
        assert!(check_query(&value).is_err());
    }

    #[test]
    fn query_collections_and_derived_work_are_bounded() {
        let mut value = query();
        value["where"] = json!(vec![query()["where"][0].clone(); 9]);
        assert!(check_query(&value).is_err());
        value = query();
        value["orderBy"] = json!(vec![query()["orderBy"][0].clone(); 2]);
        assert!(check_query(&value).is_err());
        let name = format!("findBy{}", ["Id"; 9].join("And"));
        assert!(resolve_query(&name, 9, None).is_err());
        assert!(resolve_query(&format!("findBy{}", "A".repeat(256)), 1, None).is_err());
    }

    #[test]
    fn find_ordering_has_one_deterministic_id_tiebreaker() {
        let derived = resolve_query("findById", 1, None).expect("derived query");
        assert_eq!(
            derived["orderBy"],
            json!([{"field":"id","direction":"asc"}])
        );
        let mut value = query();
        value["orderBy"] = json!([{"field":"status","direction":"desc"}]);
        let resolved = resolve_query("find", 2, Some(&value)).expect("ordered query");
        assert_eq!(
            resolved["orderBy"],
            json!([{"field":"status","direction":"desc"},{"field":"id","direction":"asc"}])
        );
        value["orderBy"] = json!([{"field":"id","direction":"desc"}]);
        let resolved = resolve_query("find", 2, Some(&value)).expect("id ordering");
        assert_eq!(resolved["orderBy"], value["orderBy"]);
    }
}
